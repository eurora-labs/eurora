//! Agent loop: drive a chat model through one or more streaming rounds,
//! dispatch tool calls it emits, and force a final text answer if it
//! exhausts the tool-call budget.

use std::collections::HashMap;
use std::sync::Arc;

use agent_chain::{
    AIMessage, AnyMessage, BaseChatModel, BaseTool, SystemMessage,
    language_models::{ToolChoice, ToolLike},
    messages::{AIMessageChunk, ToolCall, ToolMessage, ToolStatus},
};
use be_remote_db::{DatabaseManager, MessageType};
use serde_json::Value;
use thread_core::{ChatServerMessage, MessageNode};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::conversion::convert_db_message_to_base_message;

/// Appended as a system message on the forced-synthesis turn so the model
/// understands why its tools have been taken away.
const SYNTHESIS_NUDGE: &str = "You have reached the maximum number of tool calls for this turn. \
Based on the information you have gathered so far, provide your complete final answer to the \
user now. Do not request additional tools.";

/// Running totals of an AI response as it is streamed across one or more LLM rounds.
#[derive(Default)]
struct ChatAccumulator {
    content: String,
    reasoning: String,
    input_tokens: i64,
    output_tokens: i64,
    reasoning_tokens: i64,
    cache_creation_tokens: i64,
    cache_read_tokens: i64,
}

impl ChatAccumulator {
    fn absorb(&mut self, chunk: &AIMessageChunk) {
        if let Some(reasoning) = chunk
            .additional_kwargs
            .get("reasoning_content")
            .and_then(|v| v.as_str())
        {
            self.reasoning.push_str(reasoning);
        }
        if let Some(usage) = chunk.usage_metadata.as_ref() {
            self.input_tokens += usage.input_tokens;
            self.output_tokens += usage.output_tokens;
            if let Some(details) = usage.output_token_details.as_ref() {
                self.reasoning_tokens += details.reasoning.unwrap_or(0);
            }
            if let Some(details) = usage.input_token_details.as_ref() {
                self.cache_creation_tokens += details.cache_creation.unwrap_or(0);
                self.cache_read_tokens += details.cache_read.unwrap_or(0);
            }
        }
    }

    fn push_content(&mut self, text: &str) {
        self.content.push_str(text);
    }

    fn has_content(&self) -> bool {
        !self.content.is_empty() || !self.reasoning.is_empty()
    }

    fn to_content_value(&self) -> Value {
        let mut blocks = Vec::new();
        if !self.reasoning.is_empty() {
            blocks.push(serde_json::json!({"type": "reasoning", "reasoning": self.reasoning}));
        }
        if !self.content.is_empty() {
            blocks.push(serde_json::json!({"type": "text", "text": self.content}));
        }
        Value::Array(blocks)
    }
}

/// Outcome of a single streaming round.
struct RoundResult {
    content: String,
    tool_calls: Vec<ToolCall>,
    cancelled: bool,
}

async fn run_round(
    chat_model: &(dyn BaseChatModel + Send + Sync),
    messages: &[AnyMessage],
    tx: &mpsc::Sender<ChatServerMessage>,
    token: &CancellationToken,
    acc: &mut ChatAccumulator,
) -> Result<RoundResult, String> {
    // `BaseChatModel::stream` takes ownership of its message vec, so we must
    // clone here. The clone is bounded by the chat history length and the
    // tool-call budget; if this becomes a hotspot, push the slice through
    // the trait instead.
    let provider_stream = tokio::select! {
        result = chat_model.stream(messages.to_vec(), None, None) => {
            result.map_err(|e| {
                tracing::error!("Error starting chat stream: {e}");
                e.to_string()
            })?
        }
        () = token.cancelled() => {
            tracing::info!("Chat stream cancelled before provider stream started");
            return Ok(RoundResult {
                content: String::new(),
                tool_calls: Vec::new(),
                cancelled: true,
            });
        }
    };

    tokio::pin!(provider_stream);
    let mut round_content = String::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();

    loop {
        let next = tokio::select! {
            chunk = provider_stream.next() => chunk,
            () = token.cancelled() => {
                tracing::info!("Chat stream cancelled during provider streaming");
                acc.push_content(&round_content);
                return Ok(RoundResult {
                    content: round_content,
                    tool_calls,
                    cancelled: true,
                });
            }
        };

        let Some(result) = next else { break };

        let mut chunk = result.map_err(|e| e.to_string())?;

        let chunk_text = chunk.content.to_string();
        if !chunk_text.is_empty() {
            round_content.push_str(&chunk_text);
        }
        acc.absorb(&chunk);

        let outbound_chunk = chunk.clone();

        // Move tool calls out of the chunk into our running list. Done after
        // the clone above so the streamed chunk keeps the tool calls visible.
        if !chunk.tool_calls.is_empty() {
            tool_calls.append(&mut chunk.tool_calls);
        }

        if tx
            .send(ChatServerMessage::Chunk {
                chunk: outbound_chunk,
            })
            .await
            .is_err()
        {
            tracing::info!("Chat stream receiver dropped, client disconnected");
            acc.push_content(&round_content);
            return Ok(RoundResult {
                content: round_content,
                tool_calls,
                cancelled: true,
            });
        }
    }

    acc.push_content(&round_content);
    Ok(RoundResult {
        content: round_content,
        tool_calls,
        cancelled: false,
    })
}

enum ToolExecOutcome {
    Completed(Vec<AnyMessage>),
    Cancelled(Vec<AnyMessage>),
}

async fn execute_tool_calls(
    tools: &HashMap<String, Arc<dyn BaseTool>>,
    calls: Vec<ToolCall>,
    token: &CancellationToken,
) -> ToolExecOutcome {
    let mut results = Vec::with_capacity(calls.len());

    for call in calls {
        let tool_name = call.name.clone();
        let Some(tool) = tools.get(&tool_name) else {
            tracing::error!("Unknown tool: {tool_name}");
            let tool_call_id = call.id.clone().unwrap_or_default();
            results.push(
                ToolMessage::builder()
                    .content(format!("Error: unknown tool '{tool_name}'"))
                    .tool_call_id(tool_call_id)
                    .status(ToolStatus::Error)
                    .build()
                    .into(),
            );
            continue;
        };

        if call.id.is_none() {
            tracing::warn!("Tool call '{tool_name}' has no id");
        }

        let result_msg = tokio::select! {
            msg = tool.invoke_tool_call(call) => msg,
            () = token.cancelled() => {
                tracing::info!("Chat stream cancelled during tool invocation");
                return ToolExecOutcome::Cancelled(results);
            }
        };
        results.push(result_msg);
    }

    ToolExecOutcome::Completed(results)
}

async fn run_forced_synthesis(
    chat_model: &(dyn BaseChatModel + Send + Sync),
    tool_likes: &[ToolLike],
    base_messages: &[AnyMessage],
    tx: &mpsc::Sender<ChatServerMessage>,
    token: &CancellationToken,
    acc: &mut ChatAccumulator,
) -> agent_chain::Result<bool> {
    let bound = chat_model.bind_tools(tool_likes, Some(ToolChoice::none()))?;
    let synthesis_model: Arc<dyn BaseChatModel + Send + Sync> =
        Arc::from(bound as Box<dyn BaseChatModel + Send + Sync>);

    let mut synthesis_messages = Vec::with_capacity(base_messages.len() + 1);
    synthesis_messages.extend_from_slice(base_messages);
    synthesis_messages.push(
        SystemMessage::builder()
            .content(SYNTHESIS_NUDGE)
            .build()
            .into(),
    );

    let result = run_round(&*synthesis_model, &synthesis_messages, tx, token, acc)
        .await
        .map_err(agent_chain::Error::other)?;

    Ok(result.cancelled)
}

async fn save_accumulated_message(
    db: &DatabaseManager,
    thread_id: Uuid,
    user_id: Uuid,
    acc: &ChatAccumulator,
) -> Option<be_remote_db::Message> {
    let content_value = acc.to_content_value();

    let ai_message = match db
        .create_message()
        .thread_id(thread_id)
        .user_id(user_id)
        .message_type(MessageType::Ai)
        .content(content_value)
        .call()
        .await
    {
        Ok(msg) => msg,
        Err(e) => {
            tracing::error!("Failed to save AI message to database: {e}");
            return None;
        }
    };

    if (acc.input_tokens > 0 || acc.output_tokens > 0)
        && let Err(e) = db
            .record_token_usage()
            .user_id(user_id)
            .thread_id(thread_id)
            .message_id(ai_message.id)
            .input_tokens(acc.input_tokens)
            .output_tokens(acc.output_tokens)
            .reasoning_tokens(acc.reasoning_tokens)
            .cache_creation_tokens(acc.cache_creation_tokens)
            .cache_read_tokens(acc.cache_read_tokens)
            .call()
            .await
    {
        tracing::error!("Failed to record token usage: {e}");
    }

    Some(ai_message)
}

async fn finalize(
    tx: &mpsc::Sender<ChatServerMessage>,
    db: &DatabaseManager,
    thread_id: Uuid,
    user_id: Uuid,
    human_message_id: Uuid,
    acc: &ChatAccumulator,
    cancelled: bool,
) {
    if !acc.has_content() {
        return;
    }

    let Some(ai_message) = save_accumulated_message(db, thread_id, user_id, acc).await else {
        if !cancelled {
            let _ = tx
                .send(ChatServerMessage::Error {
                    kind: "internal_error".to_string(),
                    message: "Failed to save AI message".to_string(),
                })
                .await;
        }
        return;
    };

    if cancelled {
        return;
    }

    let ai_node = match convert_db_message_to_base_message(ai_message) {
        Ok(message) => MessageNode {
            parent_id: Some(human_message_id),
            message,
            children: vec![],
            sibling_index: 0,
            depth: 0,
        },
        Err(e) => {
            tracing::error!("Failed to project AI message for final frame: {e}");
            return;
        }
    };

    let _ = tx
        .send(ChatServerMessage::Final {
            messages: vec![ai_node],
        })
        .await;
}

/// Run the full agent loop: up to `max_tool_rounds` tool-using rounds,
/// followed by a forced-synthesis round if the budget is exhausted with
/// pending tool calls. Streamed chunks and the terminal `Final` envelope are
/// forwarded to `tx`. The aggregated AI message is persisted on completion.
#[bon::builder]
pub async fn run_agent_loop(
    tx: mpsc::Sender<ChatServerMessage>,
    token: CancellationToken,
    db: Arc<DatabaseManager>,
    chat_model: Arc<dyn BaseChatModel + Send + Sync>,
    tools: HashMap<String, Arc<dyn BaseTool>>,
    mut messages: Vec<AnyMessage>,
    thread_id: Uuid,
    user_id: Uuid,
    human_message_id: Uuid,
    max_tool_rounds: usize,
) {
    let mut acc = ChatAccumulator::default();
    let mut cancelled = false;
    let mut budget_exhausted = false;

    for round in 0..max_tool_rounds {
        let result = match run_round(&*chat_model, &messages, &tx, &token, &mut acc).await {
            Ok(r) => r,
            Err(detail) => {
                let _ = tx
                    .send(ChatServerMessage::Error {
                        kind: "internal_error".to_string(),
                        message: detail,
                    })
                    .await;
                return;
            }
        };

        if result.cancelled {
            cancelled = true;
            break;
        }
        if result.tool_calls.is_empty() {
            break;
        }

        messages.push(
            AIMessage::builder()
                .content(&result.content)
                .tool_calls(result.tool_calls.clone())
                .build()
                .into(),
        );

        match execute_tool_calls(&tools, result.tool_calls, &token).await {
            ToolExecOutcome::Completed(tool_msgs) => messages.extend(tool_msgs),
            ToolExecOutcome::Cancelled(tool_msgs) => {
                messages.extend(tool_msgs);
                cancelled = true;
                break;
            }
        }

        if round + 1 == max_tool_rounds {
            budget_exhausted = true;
        }
    }

    if !cancelled && budget_exhausted {
        tracing::info!(
            thread_id = %thread_id,
            rounds = max_tool_rounds,
            "Tool-call budget exhausted; running forced synthesis with tool_choice=none"
        );
        let tool_likes: Vec<ToolLike> = tools.values().cloned().map(ToolLike::Tool).collect();
        match run_forced_synthesis(&*chat_model, &tool_likes, &messages, &tx, &token, &mut acc)
            .await
        {
            Ok(synth_cancelled) => cancelled = synth_cancelled,
            Err(e) => {
                tracing::warn!("Forced synthesis failed: {e}; saving accumulated response as-is");
            }
        }
    }

    finalize(
        &tx,
        &db,
        thread_id,
        user_id,
        human_message_id,
        &acc,
        cancelled,
    )
    .await;
}
