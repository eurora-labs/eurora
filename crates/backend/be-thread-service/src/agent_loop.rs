//! Agent loop: drive a chat model through one or more streaming rounds,
//! dispatch tool calls it emits, and force a final text answer if it
//! exhausts the tool-call budget.

use std::sync::Arc;

use agent_chain::{
    AIMessage, AnyMessage, BaseChatModel, SystemMessage,
    language_models::{ToolChoice, ToolLike},
    messages::{AIMessageChunk, ToolCall, ToolMessage, ToolStatus},
};
use be_remote_db::{DatabaseManager, MessageType};
use serde_json::Value;
use thread_core::{ChatServerMessage, MessageNode, ToolErrorWire};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::conversion::convert_db_message_to_base_message;
use crate::remote_tool_bus::RemoteToolBus;
use crate::tool_catalog::{TurnCatalog, TurnEntry};

/// Appended as a system message on the forced-synthesis turn so the model
/// understands why its tools have been taken away.
const SYNTHESIS_NUDGE: &str = "You have reached the maximum number of tool calls for this turn. \
Based on the information you have gathered so far, provide your complete final answer to the \
user now. Do not request additional tools.";

/// Appended as a system message when the provider reports
/// `finish_reason: \"tool_calls\"` but emits no actual tool-call deltas.
/// This is a known GLM-family failure mode: the model decides to call a
/// tool mid-stream but never emits the call. The retry asks the model to
/// either follow through with the function-calling interface or answer
/// directly — both outcomes recover the turn from what would otherwise be
/// a silent termination.
const EMPTY_TOOL_CALL_NUDGE: &str = "Your previous response indicated a tool call but no tool call was emitted. \
Either call a tool now using the function-calling interface, or answer the user \
directly without tools.";

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
///
/// `finish_reason` carries the provider-reported stop signal from the
/// final SSE chunk (e.g. `"stop"`, `"tool_calls"`, `"length"`). The
/// orchestrator uses it to disambiguate "the model finished a normal turn
/// with no tools to call" from "the model said it wanted to call tools
/// but emitted no deltas" — the latter is a known GLM-family failure
/// mode and triggers a retry-with-nudge in [`drive_turn`].
#[derive(Debug)]
struct RoundResult {
    content: String,
    tool_calls: Vec<ToolCall>,
    cancelled: bool,
    finish_reason: Option<String>,
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
                finish_reason: None,
            });
        }
    };

    tokio::pin!(provider_stream);
    let mut round_content = String::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();
    // Providers attach `finish_reason` to the final SSE chunk; intermediate
    // chunks carry `None`. The last non-`None` value across the round wins.
    let mut finish_reason: Option<String> = None;

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
                    finish_reason,
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
        // `finish_reason` is conventionally placed on the final SSE chunk by
        // OpenAI-compatible providers. Different clients put it in different
        // map keys: the OpenAI client copies it into `response_metadata`,
        // some other adapters land it in `additional_kwargs`. Check both so
        // the retry detector below sees the value regardless of which path
        // produced the chunk. The last non-`None` value across the round
        // wins, in case the provider emits it on an intermediate chunk.
        if let Some(reason) = extract_finish_reason(&chunk) {
            finish_reason = Some(reason);
        }

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
                finish_reason,
            });
        }
    }

    acc.push_content(&round_content);
    Ok(RoundResult {
        content: round_content,
        tool_calls,
        cancelled: false,
        finish_reason,
    })
}

enum ToolExecOutcome {
    Completed(Vec<AnyMessage>),
    Cancelled(Vec<AnyMessage>),
}

async fn execute_tool_calls<B>(
    catalog: &TurnCatalog,
    bus: &B,
    calls: Vec<ToolCall>,
    token: &CancellationToken,
) -> ToolExecOutcome
where
    B: RemoteToolBus,
{
    let mut results = Vec::with_capacity(calls.len());

    for call in calls {
        let tool_name = call.name.clone();
        if call.id.is_none() {
            tracing::warn!("Tool call '{tool_name}' has no id");
        }
        let tool_call_id = call.id.clone().unwrap_or_default();

        let Some(entry) = catalog.get(&tool_name) else {
            tracing::error!("Unknown tool: {tool_name}");
            results.push(unknown_tool_message(&tool_name, &tool_call_id));
            continue;
        };

        let result_msg = match entry {
            TurnEntry::ServerLocal { tool } => {
                tokio::select! {
                    msg = tool.invoke_tool_call(call) => msg,
                    () = token.cancelled() => {
                        tracing::info!("Chat stream cancelled during tool invocation");
                        return ToolExecOutcome::Cancelled(results);
                    }
                }
            }
            TurnEntry::Remote { descriptor } => {
                let arguments = call.args.clone();
                let outcome = tokio::select! {
                    res = bus.call(descriptor, arguments) => res,
                    () = token.cancelled() => {
                        tracing::info!("Chat stream cancelled while awaiting remote tool result");
                        return ToolExecOutcome::Cancelled(results);
                    }
                };
                match outcome {
                    Ok(value) => remote_success_message(&tool_call_id, value),
                    Err(ToolErrorWire::Cancelled) if token.is_cancelled() => {
                        return ToolExecOutcome::Cancelled(results);
                    }
                    Err(err) => remote_error_message(&tool_name, &tool_call_id, err),
                }
            }
        };
        results.push(result_msg);
    }

    ToolExecOutcome::Completed(results)
}

/// Detect the GLM-family failure mode where the provider reports
/// `finish_reason: "tool_calls"` but the stream contained no tool-call
/// deltas. The accumulator-side conditions (`content.is_empty()`,
/// `tool_calls.is_empty()`) rule out the legitimate cases of "model
/// answered and is also calling tools" and "model answered without
/// tools"; only the pathological combination triggers a retry.
fn is_empty_tool_call_signal(result: &RoundResult) -> bool {
    result.tool_calls.is_empty()
        && result.content.is_empty()
        && result.finish_reason.as_deref() == Some("tool_calls")
}

/// Pull `finish_reason` off a streaming chunk. OpenAI-compatible adapters
/// agree the value belongs in metadata; they disagree about which map.
/// Check `response_metadata` first (the canonical OpenAI client path),
/// then `additional_kwargs` (some Anthropic/Ollama adapters), then return
/// `None`.
fn extract_finish_reason(chunk: &agent_chain::messages::AIMessageChunk) -> Option<String> {
    chunk
        .response_metadata
        .get("finish_reason")
        .or_else(|| chunk.additional_kwargs.get("finish_reason"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
}

fn unknown_tool_message(tool_name: &str, tool_call_id: &str) -> AnyMessage {
    ToolMessage::builder()
        .content(format!("Error: unknown tool '{tool_name}'"))
        .tool_call_id(tool_call_id.to_string())
        .status(ToolStatus::Error)
        .build()
        .into()
}

fn remote_success_message(tool_call_id: &str, value: Value) -> AnyMessage {
    let content = match &value {
        Value::String(s) => s.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| other.to_string()),
    };
    ToolMessage::builder()
        .content(content)
        .tool_call_id(tool_call_id.to_string())
        .status(ToolStatus::Success)
        .build()
        .into()
}

fn remote_error_message(tool_name: &str, tool_call_id: &str, err: ToolErrorWire) -> AnyMessage {
    tracing::warn!(tool = %tool_name, error = %err, "Remote tool call failed");
    ToolMessage::builder()
        .content(format!("Error: {err}"))
        .tool_call_id(tool_call_id.to_string())
        .status(ToolStatus::Error)
        .build()
        .into()
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

/// What a turn produced, viewed by the outer orchestrator. Carries the
/// information the wire layer needs to choose between `Final`, `Error`,
/// or no terminal frame at all.
///
/// `MessageNode` is large (~470 B) so it is boxed in `Completed`: keeps
/// the enum's stack footprint to one pointer when present and zero when
/// absent, instead of inlining the full node into every variant.
enum AgentTurnOutcome {
    /// The turn finished normally. `ai_node` is `Some` when the model
    /// produced content that was persisted and projected into wire shape;
    /// `None` when the model returned nothing (e.g. an empty refusal) or
    /// the DB row exists but projection failed — in the latter case the
    /// row is in the DB and the client will reconcile on its next
    /// message fetch, but we have no node to put in `Final.messages`.
    Completed { ai_node: Option<Box<MessageNode>> },
    /// The turn ended because the user cancelled mid-stream (or the
    /// socket dropped). Any partial response was persisted on a
    /// best-effort basis; the client owns its placeholder.
    Cancelled,
    /// The turn aborted before completing. The pair propagates verbatim
    /// to `ChatServerMessage::Error`.
    Errored { kind: String, message: String },
}

/// Persist the accumulated AI response and project it into a wire node.
///
/// Returns `Ok(None)` when there is nothing to save (empty accumulator)
/// or when the DB row was written but the wire projection failed (logged;
/// the client will reconcile via the next message fetch). Returns
/// `Err(msg)` when the DB write itself failed. The wire node is boxed so
/// the caller can hand it straight to [`AgentTurnOutcome::Completed`]
/// without copying the ~470-byte payload back onto the stack.
async fn save_turn_result(
    db: &DatabaseManager,
    thread_id: Uuid,
    user_id: Uuid,
    human_message_id: Uuid,
    acc: &ChatAccumulator,
) -> Result<Option<Box<MessageNode>>, String> {
    if !acc.has_content() {
        return Ok(None);
    }
    let Some(ai_message) = save_accumulated_message(db, thread_id, user_id, acc).await else {
        return Err("Failed to save AI message".to_string());
    };
    match convert_db_message_to_base_message(ai_message) {
        Ok(message) => Ok(Some(Box::new(MessageNode {
            parent_id: Some(human_message_id),
            message,
            children: vec![],
            sibling_index: 0,
            depth: 0,
        }))),
        Err(e) => {
            tracing::error!("Failed to project AI message for final frame: {e}");
            Ok(None)
        }
    }
}

/// Drive the agent loop to a single [`AgentTurnOutcome`]. Streams `Chunk`
/// frames over `tx` during the loop but never emits a terminal frame —
/// the caller maps the outcome to `Final`/`Error`/silence so any
/// post-turn work (e.g. auto-title) can land between the loop body and
/// the terminal.
#[allow(clippy::too_many_arguments)]
async fn drive_turn<B>(
    tx: &mpsc::Sender<ChatServerMessage>,
    token: &CancellationToken,
    db: &DatabaseManager,
    chat_model: &(dyn BaseChatModel + Send + Sync),
    catalog: &TurnCatalog,
    remote_bus: &B,
    mut messages: Vec<AnyMessage>,
    thread_id: Uuid,
    user_id: Uuid,
    human_message_id: Uuid,
    max_tool_rounds: usize,
) -> AgentTurnOutcome
where
    B: RemoteToolBus + Send + Sync,
{
    let mut acc = ChatAccumulator::default();
    let mut cancelled = false;
    let mut budget_exhausted = false;
    // GLM-family providers occasionally signal `finish_reason: "tool_calls"`
    // without emitting any tool-call deltas, leaving the loop with nothing
    // to dispatch and nothing to show the user. We retry the round once
    // with a system-message nudge before giving up. One-shot guard so a
    // pathological provider can't burn the whole tool-call budget on
    // empty-call rounds.
    let mut empty_tool_call_retry_used = false;

    for round in 0..max_tool_rounds {
        let result = match run_round(chat_model, &messages, tx, token, &mut acc).await {
            Ok(r) => r,
            Err(detail) => {
                return AgentTurnOutcome::Errored {
                    kind: "internal_error".to_string(),
                    message: detail,
                };
            }
        };

        if result.cancelled {
            cancelled = true;
            break;
        }
        if result.tool_calls.is_empty() {
            // An empty turn (no tool calls, no text) is always a failure —
            // either the GLM `finish_reason: tool_calls` shape we know how
            // to recover from, or some other provider hiccup. Either way,
            // surface a single warn with the visible signals so the
            // condition is diagnosable from logs without bumping verbosity.
            if result.content.is_empty() {
                tracing::warn!(
                    thread_id = %thread_id,
                    round = round,
                    finish_reason = ?result.finish_reason,
                    retry_available = !empty_tool_call_retry_used,
                    "Round ended with no content and no tool calls"
                );
            }
            if !empty_tool_call_retry_used && is_empty_tool_call_signal(&result) {
                tracing::warn!(
                    thread_id = %thread_id,
                    round = round,
                    finish_reason = ?result.finish_reason,
                    "Provider reported finish_reason=tool_calls with no tool-call deltas; \
                     retrying with nudge"
                );
                empty_tool_call_retry_used = true;
                messages.push(
                    SystemMessage::builder()
                        .content(EMPTY_TOOL_CALL_NUDGE)
                        .build()
                        .into(),
                );
                continue;
            }
            break;
        }

        messages.push(
            AIMessage::builder()
                .content(&result.content)
                .tool_calls(result.tool_calls.clone())
                .build()
                .into(),
        );

        match execute_tool_calls(catalog, remote_bus, result.tool_calls, token).await {
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
        let tool_likes = catalog.tool_likes();
        match run_forced_synthesis(chat_model, &tool_likes, &messages, tx, token, &mut acc).await {
            Ok(synth_cancelled) => cancelled = synth_cancelled,
            Err(e) => {
                tracing::warn!("Forced synthesis failed: {e}; saving accumulated response as-is");
            }
        }
    }

    match save_turn_result(db, thread_id, user_id, human_message_id, &acc).await {
        Ok(_) if cancelled => AgentTurnOutcome::Cancelled,
        Ok(node) => AgentTurnOutcome::Completed { ai_node: node },
        Err(_) if cancelled => {
            // Save failures during a user cancel are best-effort — the
            // client tore the turn down deliberately and we don't want
            // to overlay an error on top of that.
            AgentTurnOutcome::Cancelled
        }
        Err(msg) => AgentTurnOutcome::Errored {
            kind: "internal_error".to_string(),
            message: msg,
        },
    }
}

/// Run the full agent loop: up to `max_tool_rounds` tool-using rounds,
/// followed by a forced-synthesis round if the budget is exhausted with
/// pending tool calls. Streamed chunks land on `tx` during the loop; the
/// terminal frame (`Final`/`Error`) and the auto-title (`TitleUpdated`)
/// land on the same channel after the loop settles.
///
/// Title generation is wedged between the loop body and the terminal
/// frame so the new title arrives over the same WebSocket as the
/// response — no extra HTTP round-trip on the happy path, and cancelled
/// turns get titled too (the user message is persisted in `run_turn`
/// before this loop spawns, so the title model always has something to
/// summarise).
///
/// `title_model` is the dedicated title-generation provider from
/// [`crate::llm::Providers::title`]; threaded through here rather than
/// looked up at use site so the agent loop has no dependency on
/// `AppState`. `catalog` is the per-turn tool catalog produced by
/// [`crate::tool_catalog::TurnCatalog::build`]; `remote_bus` is the
/// [`crate::remote_tool_bus::RemoteToolBus`] used to dispatch tools
/// whose [`TurnEntry`] is `Remote`. The bus is taken as a concrete
/// `Arc<B>` so the agent loop can be exercised with stub buses in
/// tests; production callers pass [`crate::remote_tool_bus::ChatRemoteBus`].
#[bon::builder]
pub async fn run_agent_loop<B>(
    title_model: Arc<dyn BaseChatModel + Send + Sync>,
    tx: mpsc::Sender<ChatServerMessage>,
    token: CancellationToken,
    db: Arc<DatabaseManager>,
    chat_model: Arc<dyn BaseChatModel + Send + Sync>,
    catalog: Arc<TurnCatalog>,
    remote_bus: Arc<B>,
    messages: Vec<AnyMessage>,
    thread_id: Uuid,
    user_id: Uuid,
    human_message_id: Uuid,
    max_tool_rounds: usize,
) where
    B: RemoteToolBus + Send + Sync + 'static,
{
    let outcome = drive_turn(
        &tx,
        &token,
        db.as_ref(),
        chat_model.as_ref(),
        catalog.as_ref(),
        remote_bus.as_ref(),
        messages,
        thread_id,
        user_id,
        human_message_id,
        max_tool_rounds,
    )
    .await;

    // Auto-title runs in every outcome — completed, cancelled, or
    // errored. The user message is already in the DB (persisted by
    // `run_turn` before this loop spawned), so even a cancel before the
    // first chunk has something for the title model to summarise.
    // Failures here never tear down the turn; the thread keeps its
    // placeholder title until the next manual rename.
    match crate::title::auto_generate_title_if_needed(
        db.as_ref(),
        title_model.as_ref(),
        thread_id,
        user_id,
    )
    .await
    {
        Ok(Some(title)) => {
            let _ = tx.send(ChatServerMessage::TitleUpdated { title }).await;
        }
        Ok(None) => {}
        Err(e) => {
            tracing::warn!(
                thread_id = %thread_id,
                error = %e,
                "Auto-title generation failed during turn finalisation"
            );
        }
    }

    match outcome {
        AgentTurnOutcome::Completed {
            ai_node: Some(node),
        } => {
            let _ = tx
                .send(ChatServerMessage::Final {
                    messages: vec![*node],
                })
                .await;
        }
        AgentTurnOutcome::Completed { ai_node: None } => {
            // Nothing accumulated — let the channel drop so `handle_socket`
            // notices the empty queue and closes the WebSocket cleanly.
        }
        AgentTurnOutcome::Cancelled => {
            // The client tore the turn down and is already in its
            // post-cancel state; no terminal frame to send.
        }
        AgentTurnOutcome::Errored { kind, message } => {
            let _ = tx.send(ChatServerMessage::Error { kind, message }).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Mutex;

    use agent_chain::async_trait;
    use agent_chain::callbacks::manager::CallbackManagerForToolRun;
    use agent_chain::error::Result as ChainResult;
    use agent_chain::runnables::RunnableConfig;
    use agent_chain::tools::{ArgsSchema, BaseTool, ToolInput, ToolOutput};
    use serde_json::json;
    use thread_core::WireToolDescriptor;

    #[derive(Debug)]
    struct RecordingTool {
        name: String,
        args_schema: ArgsSchema,
        last_call: Mutex<Option<ToolInput>>,
        result: String,
    }

    impl RecordingTool {
        fn new(name: &str, result: &str) -> Arc<Self> {
            Arc::new(Self {
                name: name.to_string(),
                args_schema: ArgsSchema::JsonSchema(json!({"type": "object"})),
                last_call: Mutex::new(None),
                result: result.to_string(),
            })
        }
    }

    #[async_trait]
    impl BaseTool for RecordingTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "recording test tool"
        }

        fn args_schema(&self) -> Option<&ArgsSchema> {
            Some(&self.args_schema)
        }

        async fn tool_run(
            &self,
            input: ToolInput,
            _run_manager: Option<&CallbackManagerForToolRun>,
            _config: &RunnableConfig,
        ) -> ChainResult<ToolOutput> {
            *self.last_call.lock().unwrap() = Some(input);
            Ok(ToolOutput::String(self.result.clone()))
        }
    }

    /// Stub bus that records descriptor + args and returns a canned value
    /// or error. Used to verify that the agent loop's remote-dispatch arm
    /// reaches the bus with the right inputs and surfaces the result back
    /// to the `ToolMessage`.
    struct StubBus {
        recorded: Mutex<Vec<(String, Value)>>,
        outcome: Mutex<Box<dyn FnMut() -> Result<Value, ToolErrorWire> + Send>>,
    }

    impl StubBus {
        fn ok(value: Value) -> Arc<Self> {
            Arc::new(Self {
                recorded: Mutex::new(Vec::new()),
                outcome: Mutex::new(Box::new(move || Ok(value.clone()))),
            })
        }

        fn err(err: ToolErrorWire) -> Arc<Self> {
            let err = Mutex::new(Some(err));
            Arc::new(Self {
                recorded: Mutex::new(Vec::new()),
                outcome: Mutex::new(Box::new(move || {
                    Err(err.lock().unwrap().take().expect("err used twice"))
                })),
            })
        }

        fn recorded(&self) -> Vec<(String, Value)> {
            self.recorded.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl RemoteToolBus for StubBus {
        async fn call(
            &self,
            descriptor: &WireToolDescriptor,
            arguments: Value,
        ) -> Result<Value, ToolErrorWire> {
            self.recorded
                .lock()
                .unwrap()
                .push((descriptor.name().to_string(), arguments));
            (self.outcome.lock().unwrap())()
        }
    }

    fn remote_descriptor(name: &str) -> WireToolDescriptor {
        crate::test_support::bridge_descriptor(name, 5_000)
    }

    fn tool_call(name: &str, args: Value, id: &str) -> ToolCall {
        ToolCall::builder()
            .name(name.to_string())
            .args(args)
            .id(id.to_string())
            .build()
    }

    fn extract_tool_message(msg: &AnyMessage) -> &ToolMessage {
        match msg {
            AnyMessage::ToolMessage(t) => t,
            other => panic!("expected ToolMessage, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn dispatch_routes_server_local_calls_to_basetool() {
        let tool = RecordingTool::new("firecrawl_search", "search-result-body");
        let catalog =
            Arc::new(TurnCatalog::build([tool.clone() as Arc<dyn BaseTool>], [], &[]).unwrap());
        let bus = StubBus::ok(json!({}));
        let cancel = CancellationToken::new();

        let outcome = execute_tool_calls(
            &catalog,
            &*bus,
            vec![tool_call(
                "firecrawl_search",
                json!({"query": "rust"}),
                "c1",
            )],
            &cancel,
        )
        .await;

        let msgs = match outcome {
            ToolExecOutcome::Completed(m) => m,
            ToolExecOutcome::Cancelled(_) => panic!("unexpected cancel"),
        };
        assert_eq!(msgs.len(), 1);
        let msg = extract_tool_message(&msgs[0]);
        assert_eq!(msg.tool_call_id, "c1");
        assert_eq!(msg.status, ToolStatus::Success);

        assert!(tool.last_call.lock().unwrap().is_some());
        assert!(
            bus.recorded().is_empty(),
            "server-local should not touch the bus"
        );
    }

    #[tokio::test]
    async fn dispatch_routes_remote_calls_to_bus_and_propagates_success() {
        let catalog = Arc::new(
            TurnCatalog::build(
                [],
                [remote_descriptor("browser_youtube_get_current_timestamp")],
                &[],
            )
            .unwrap(),
        );
        let bus = StubBus::ok(json!({"timestamp_seconds": 12.5}));
        let cancel = CancellationToken::new();

        let outcome = execute_tool_calls(
            &catalog,
            &*bus,
            vec![tool_call(
                "browser_youtube_get_current_timestamp",
                json!({}),
                "c1",
            )],
            &cancel,
        )
        .await;

        let msgs = match outcome {
            ToolExecOutcome::Completed(m) => m,
            ToolExecOutcome::Cancelled(_) => panic!("unexpected cancel"),
        };
        assert_eq!(msgs.len(), 1);
        let msg = extract_tool_message(&msgs[0]);
        assert_eq!(msg.tool_call_id, "c1");
        assert_eq!(msg.status, ToolStatus::Success);
        let content_text = msg.content.iter().fold(String::new(), |mut acc, b| {
            if let agent_chain::messages::ContentBlock::Text(t) = b {
                acc.push_str(&t.text);
            }
            acc
        });
        assert!(content_text.contains("12.5"));

        let recorded = bus.recorded();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].0, "browser_youtube_get_current_timestamp");
    }

    #[tokio::test]
    async fn dispatch_propagates_remote_error_as_error_tool_message() {
        let catalog = Arc::new(
            TurnCatalog::build([], [remote_descriptor("browser::test::fail")], &[]).unwrap(),
        );
        let bus = StubBus::err(ToolErrorWire::Remote {
            code: 500,
            message: "upstream boom".to_string(),
            details: None,
        });
        let cancel = CancellationToken::new();

        let outcome = execute_tool_calls(
            &catalog,
            &*bus,
            vec![tool_call("browser::test::fail", json!({}), "c1")],
            &cancel,
        )
        .await;

        let msgs = match outcome {
            ToolExecOutcome::Completed(m) => m,
            ToolExecOutcome::Cancelled(_) => panic!("unexpected cancel"),
        };
        let msg = extract_tool_message(&msgs[0]);
        assert_eq!(msg.status, ToolStatus::Error);
        let content_text = msg.content.iter().fold(String::new(), |mut acc, b| {
            if let agent_chain::messages::ContentBlock::Text(t) = b {
                acc.push_str(&t.text);
            }
            acc
        });
        assert!(content_text.contains("upstream boom"));
    }

    #[tokio::test]
    async fn dispatch_returns_cancelled_when_remote_call_observes_cancellation() {
        let catalog = Arc::new(
            TurnCatalog::build([], [remote_descriptor("browser::test::slow")], &[]).unwrap(),
        );
        let bus = StubBus::err(ToolErrorWire::Cancelled);
        let cancel = CancellationToken::new();
        cancel.cancel();

        let outcome = execute_tool_calls(
            &catalog,
            &*bus,
            vec![tool_call("browser::test::slow", json!({}), "c1")],
            &cancel,
        )
        .await;

        assert!(
            matches!(outcome, ToolExecOutcome::Cancelled(_)),
            "expected ToolExecOutcome::Cancelled"
        );
    }

    /// Guards the `if token.is_cancelled()` arm in `execute_tool_calls`: a
    /// bus that returns `ToolErrorWire::Cancelled` *without* the chat token
    /// having fired is a misbehaving bus, not a cancellation. The round
    /// must continue and surface a normal error tool message rather than
    /// aborting the turn.
    #[tokio::test]
    async fn dispatch_treats_bus_cancelled_without_token_cancel_as_error() {
        let catalog = Arc::new(
            TurnCatalog::build([], [remote_descriptor("browser::test::cancel")], &[]).unwrap(),
        );
        let bus = StubBus::err(ToolErrorWire::Cancelled);
        let cancel = CancellationToken::new();

        let outcome = execute_tool_calls(
            &catalog,
            &*bus,
            vec![tool_call("browser::test::cancel", json!({}), "c1")],
            &cancel,
        )
        .await;

        let msgs = match outcome {
            ToolExecOutcome::Completed(m) => m,
            ToolExecOutcome::Cancelled(_) => {
                panic!("bus.Cancelled without token-cancel should not abort the round")
            }
        };
        assert_eq!(msgs.len(), 1);
        let msg = extract_tool_message(&msgs[0]);
        assert_eq!(msg.status, ToolStatus::Error);
        let content_text = msg.content.iter().fold(String::new(), |mut acc, b| {
            if let agent_chain::messages::ContentBlock::Text(t) = b {
                acc.push_str(&t.text);
            }
            acc
        });
        assert!(
            content_text.contains("tool call cancelled"),
            "expected ToolErrorWire::Cancelled rendering, got {content_text:?}"
        );
    }

    #[tokio::test]
    async fn dispatch_reports_unknown_tool_as_error_message() {
        let catalog = Arc::new(TurnCatalog::build([], [], &[]).unwrap());
        let bus = StubBus::ok(json!({}));
        let cancel = CancellationToken::new();

        let outcome = execute_tool_calls(
            &catalog,
            &*bus,
            vec![tool_call("ghost::tool", json!({}), "c1")],
            &cancel,
        )
        .await;

        let msgs = match outcome {
            ToolExecOutcome::Completed(m) => m,
            ToolExecOutcome::Cancelled(_) => panic!("unexpected cancel"),
        };
        let msg = extract_tool_message(&msgs[0]);
        assert_eq!(msg.status, ToolStatus::Error);
        let content_text = msg.content.iter().fold(String::new(), |mut acc, b| {
            if let agent_chain::messages::ContentBlock::Text(t) = b {
                acc.push_str(&t.text);
            }
            acc
        });
        assert!(content_text.contains("unknown tool 'ghost::tool'"));
    }

    /// Truth table for [`is_empty_tool_call_signal`]. The detector exists
    /// to recognise the GLM-family failure mode — `finish_reason:
    /// "tool_calls"` with no actual tool-call deltas — without
    /// misfiring on the three adjacent shapes (model answered normally,
    /// model answered and is also calling tools, model is calling tools
    /// without prose). All four cases are exercised here so a future
    /// tweak to the detector can't quietly broaden it.
    fn round_result(
        content: &str,
        finish_reason: Option<&str>,
        tool_calls: Vec<ToolCall>,
    ) -> RoundResult {
        RoundResult {
            content: content.to_string(),
            tool_calls,
            cancelled: false,
            finish_reason: finish_reason.map(str::to_string),
        }
    }

    #[test]
    fn empty_tool_call_signal_fires_on_glm_failure_shape() {
        let result = round_result("", Some("tool_calls"), Vec::new());
        assert!(is_empty_tool_call_signal(&result));
    }

    #[test]
    fn empty_tool_call_signal_ignores_normal_text_turn() {
        // Plain answer: model said its piece and stopped. No retry warranted.
        let result = round_result("Here is the answer.", Some("stop"), Vec::new());
        assert!(!is_empty_tool_call_signal(&result));
    }

    #[test]
    fn empty_tool_call_signal_ignores_normal_tool_call_turn() {
        // Model emitted at least one tool call — the legitimate
        // tool-calls finish_reason path. Must not retry.
        let result = round_result(
            "",
            Some("tool_calls"),
            vec![tool_call(
                "firecrawl_search",
                json!({"query": "rust"}),
                "c1",
            )],
        );
        assert!(!is_empty_tool_call_signal(&result));
    }

    #[test]
    fn empty_tool_call_signal_ignores_mixed_text_and_tool_call_turn() {
        // Model answered AND scheduled a tool call. Also a legitimate
        // shape that the detector must let through.
        let result = round_result(
            "Let me check.",
            Some("tool_calls"),
            vec![tool_call(
                "firecrawl_search",
                json!({"query": "rust"}),
                "c1",
            )],
        );
        assert!(!is_empty_tool_call_signal(&result));
    }

    #[test]
    fn empty_tool_call_signal_ignores_missing_finish_reason() {
        // Some providers (or mocked streams) end without ever sending a
        // `finish_reason`. That's ambiguous, not a known failure mode —
        // the detector stays conservative and does not retry.
        let result = round_result("", None, Vec::new());
        assert!(!is_empty_tool_call_signal(&result));
    }

    #[test]
    fn empty_tool_call_signal_ignores_unknown_finish_reasons() {
        // Only `tool_calls` triggers the retry; `length`, `stop`,
        // `content_filter`, and anything else mean something different
        // and must not retry.
        for reason in ["stop", "length", "content_filter", "function_call"] {
            let result = round_result("", Some(reason), Vec::new());
            assert!(
                !is_empty_tool_call_signal(&result),
                "finish_reason={reason:?} must not trigger retry",
            );
        }
    }

    /// End-to-end propagation tests for [`run_round`]: when a chat model
    /// yields chunks that carry `finish_reason` in `response_metadata`
    /// (the OpenAI-compatible client's contract — see
    /// `agent_chain::providers::openai`), [`RoundResult::finish_reason`]
    /// must reflect that value. The retry-with-nudge path in
    /// [`drive_turn`] depends on this; if propagation breaks, the GLM
    /// empty-tool-calls failure mode silently terminates the turn.
    mod run_round_propagation {
        use super::*;
        use agent_chain::AIMessage;
        use agent_chain::caches::BaseCache;
        use agent_chain::callbacks::Callbacks;
        use agent_chain::callbacks::manager::CallbackManagerForLLMRun;
        use agent_chain::error::Result as ChainResult;
        use agent_chain::language_models::{
            BaseLanguageModel, ChatGenerationStream, ChatModelConfig, LangSmithParams,
            LanguageModelConfig,
        };
        use agent_chain::messages::AnyMessage;
        use agent_chain::outputs::{ChatGeneration, ChatGenerationChunk, ChatResult, LLMResult};
        use serde_json::{Value, json};
        use std::collections::HashMap;

        /// Scripted chat model: hands back the AIMessages it was
        /// constructed with as a `_stream`. The default `BaseChatModel::stream`
        /// impl then handles the `ChatGenerationChunk → AIMessageChunk`
        /// conversion — same path the real OpenAI client uses, so this
        /// exercises the exact propagation the production code relies on.
        #[derive(Debug)]
        struct ScriptedChatModel {
            chunks: Vec<AIMessage>,
            config: ChatModelConfig,
        }

        impl ScriptedChatModel {
            fn new(chunks: Vec<AIMessage>) -> Self {
                Self {
                    chunks,
                    config: ChatModelConfig::builder().build(),
                }
            }
        }

        #[async_trait]
        impl BaseLanguageModel for ScriptedChatModel {
            fn llm_type(&self) -> &str {
                "scripted-chat-model"
            }

            fn model_name(&self) -> &str {
                "scripted"
            }

            fn config(&self) -> &LanguageModelConfig {
                &self.config.base
            }

            fn cache(&self) -> Option<&dyn BaseCache> {
                None
            }

            fn callbacks(&self) -> Option<&Callbacks> {
                None
            }

            async fn generate_prompt(
                &self,
                _prompts: Vec<Vec<AnyMessage>>,
                _stop: Option<Vec<String>>,
                _callbacks: Option<Callbacks>,
            ) -> ChainResult<LLMResult> {
                Ok(LLMResult::builder().generations(Vec::new()).build())
            }

            fn identifying_params(&self) -> HashMap<String, Value> {
                HashMap::new()
            }

            fn get_ls_params(&self, _stop: Option<&[String]>) -> LangSmithParams {
                LangSmithParams::default()
            }
        }

        #[async_trait]
        impl BaseChatModel for ScriptedChatModel {
            fn chat_config(&self) -> &ChatModelConfig {
                &self.config
            }

            async fn _generate(
                &self,
                _messages: Vec<AnyMessage>,
                _stop: Option<Vec<String>>,
                _run_manager: Option<&CallbackManagerForLLMRun>,
            ) -> ChainResult<ChatResult> {
                let generations: Vec<ChatGeneration> = self
                    .chunks
                    .iter()
                    .map(|m| {
                        ChatGeneration::builder()
                            .message(AnyMessage::AIMessage(m.clone()))
                            .build()
                    })
                    .collect();
                Ok(ChatResult::builder().generations(generations).build())
            }

            fn has_stream_impl(&self) -> bool {
                true
            }

            async fn _stream(
                &self,
                _messages: Vec<AnyMessage>,
                _stop: Option<Vec<String>>,
                _run_manager: Option<&CallbackManagerForLLMRun>,
            ) -> ChainResult<ChatGenerationStream> {
                let chunks: Vec<ChainResult<ChatGenerationChunk>> = self
                    .chunks
                    .iter()
                    .cloned()
                    .map(|message| {
                        Ok(ChatGenerationChunk::builder()
                            .message(AnyMessage::AIMessage(message))
                            .build())
                    })
                    .collect();
                Ok(Box::pin(futures::stream::iter(chunks)))
            }
        }

        fn ai_chunk_with_finish_reason(content: &str, reason: &str) -> AIMessage {
            let mut metadata = HashMap::new();
            metadata.insert("finish_reason".to_string(), json!(reason));
            AIMessage::builder()
                .content(content)
                .response_metadata(metadata)
                .build()
        }

        async fn drain_round(model: ScriptedChatModel) -> RoundResult {
            let (tx, mut rx) = mpsc::channel(64);
            let token = CancellationToken::new();
            let mut acc = ChatAccumulator::default();
            // Drain the receiver concurrently so the round's `tx.send`
            // calls don't deadlock on a full channel.
            let drain = tokio::spawn(async move { while rx.recv().await.is_some() {} });
            let result = run_round(&model, &[], &tx, &token, &mut acc)
                .await
                .expect("scripted stream never errors");
            drop(tx);
            drain.await.expect("drain task panicked");
            result
        }

        /// The GLM failure shape: provider yields a single empty chunk
        /// with `finish_reason: "tool_calls"` in `response_metadata` and
        /// no tool-call deltas. `run_round` must surface that
        /// `finish_reason` on the returned [`RoundResult`] so the
        /// retry-with-nudge path in [`drive_turn`] can detect the
        /// condition via [`is_empty_tool_call_signal`].
        #[tokio::test]
        async fn run_round_captures_finish_reason_from_response_metadata() {
            let model = ScriptedChatModel::new(vec![ai_chunk_with_finish_reason("", "tool_calls")]);
            let result = drain_round(model).await;
            assert_eq!(result.finish_reason.as_deref(), Some("tool_calls"));
            assert!(result.tool_calls.is_empty());
            assert!(result.content.is_empty());
            // End-to-end coherence: the upstream signal must satisfy the
            // downstream detector, otherwise the retry path is dead code.
            assert!(
                is_empty_tool_call_signal(&result),
                "round result {result:?} should trigger retry-with-nudge",
            );
        }

        /// Many providers spread `finish_reason` across the SSE stream:
        /// `None` on the content chunks, populated only on the final
        /// chunk. The accumulator in `run_round` must keep the latest
        /// non-`None` value rather than letting a `None` from a
        /// trailing-empty chunk clobber it.
        #[tokio::test]
        async fn run_round_keeps_finish_reason_across_trailing_none_chunks() {
            let model = ScriptedChatModel::new(vec![
                ai_chunk_with_finish_reason("", "tool_calls"),
                // Trailing empty chunk with no response_metadata —
                // mimics the synthesized last chunk that
                // `BaseChatModel::stream` appends.
                AIMessage::builder().content("").build(),
            ]);
            let result = drain_round(model).await;
            assert_eq!(result.finish_reason.as_deref(), Some("tool_calls"));
        }

        /// Sanity check the other direction: a normal `stop` turn must
        /// not look like the failure mode. Combined with the
        /// `empty_tool_call_signal_*` truth-table tests, this pins down
        /// both ends of the retry-trigger contract.
        #[tokio::test]
        async fn run_round_normal_stop_does_not_trigger_retry() {
            let model = ScriptedChatModel::new(vec![ai_chunk_with_finish_reason(
                "Here is the answer.",
                "stop",
            )]);
            let result = drain_round(model).await;
            assert_eq!(result.finish_reason.as_deref(), Some("stop"));
            assert!(!is_empty_tool_call_signal(&result));
        }

        /// Some non-OpenAI adapters place `finish_reason` in
        /// `additional_kwargs` rather than `response_metadata`.
        /// [`extract_finish_reason`] checks both maps; this pins that
        /// behavior so a future cleanup can't quietly narrow it.
        #[tokio::test]
        async fn run_round_reads_finish_reason_from_additional_kwargs_too() {
            let mut additional = HashMap::new();
            additional.insert("finish_reason".to_string(), json!("tool_calls"));
            let message = AIMessage::builder()
                .content("")
                .additional_kwargs(additional)
                .build();
            let model = ScriptedChatModel::new(vec![message]);
            let result = drain_round(model).await;
            assert_eq!(result.finish_reason.as_deref(), Some("tool_calls"));
            assert!(is_empty_tool_call_signal(&result));
        }
    }
}
