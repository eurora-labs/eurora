//! WebSocket chat endpoint.
//!
//! The frontend opens one WebSocket per chat turn. The wire protocol is the
//! [`ChatClientMessage`] / [`ChatServerMessage`] enum pair from `thread-core`.
//!
//! The server expects exactly one command frame to start a turn, then runs
//! that turn to completion (or cancellation). The accepted initial frames are:
//!
//! - [`ChatClientMessage::Send`] — start a turn from a new human message. The
//!   server persists the human message, emits
//!   [`ChatServerMessage::ConfirmedHumanMessage`], and spawns the agent loop.
//! - [`ChatClientMessage::Regenerate`] — re-roll the AI response of an
//!   existing AI message. The server resolves the AI's parent (a human
//!   message), rewinds `active_leaf` to it, and spawns the agent loop on the
//!   existing context. The new AI response lands as a sibling variant of the
//!   original.
//!
//! Once the turn is running, the agent loop streams [`ChatServerMessage::Chunk`]
//! frames and ends with [`ChatServerMessage::Final`]. At any point the client
//! can send [`ChatClientMessage::Cancel`] (or just drop the socket) to abort.
//!
//! Token gating is enforced by the surrounding `be-authz` middleware before
//! the upgrade handshake completes.

use std::collections::HashMap;
use std::sync::Arc;

use agent_chain::HumanMessage;
use agent_chain::messages::{AnyMessage, ContentBlock};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::Response;
use be_remote_db::{MessageType, PaginationParams};
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use thread_core::{
    ChatClientMessage, ChatSendRequest, ChatServerMessage, MessageNode, RegenerateRequest,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use be_auth_core::AuthUser;

use crate::agent_loop::run_agent_loop;
use crate::conversion::convert_db_message_to_base_message;
use crate::error::{ThreadServiceError, ThreadServiceResult};
use crate::llm::{LlmContext, prepare_llm_context};
use crate::preliminary::rewrite_preliminary_blocks;
use crate::service::AppState;

const CONTEXT_MESSAGE_LIMIT: u32 = 5;
const MAX_TOOL_ROUNDS: usize = 15;
const SERVER_CHANNEL_DEPTH: usize = 32;

/// One of the command frames that may legally open a chat turn.
enum InitialCommand {
    Send(ChatSendRequest),
    Regenerate(RegenerateRequest),
}

/// Axum entry point — upgrades the HTTP request to a WebSocket and hands
/// the socket off to [`handle_socket`].
#[tracing::instrument(skip(state, user, ws), fields(thread_id = %thread_id))]
pub async fn chat_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(thread_id): Path<Uuid>,
) -> ThreadServiceResult<Response> {
    let user_id = user.user_id()?;
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, user_id, thread_id)))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>, user_id: Uuid, thread_id: Uuid) {
    let (mut sender, mut receiver) = socket.split();

    // First inbound frame must be `Send` or `Regenerate`. `Cancel` before any
    // in-flight turn is meaningless and we treat it as a protocol error to
    // keep the state machine simple.
    let command = match wait_for_initial_command(&mut receiver).await {
        Ok(cmd) => cmd,
        Err(detail) => {
            let _ = sender
                .send(serialize_event(&ChatServerMessage::Error {
                    kind: "invalid_argument".to_string(),
                    message: detail,
                }))
                .await;
            return;
        }
    };

    let cancel = CancellationToken::new();
    let (tx, mut rx) = mpsc::channel::<ChatServerMessage>(SERVER_CHANNEL_DEPTH);

    // A second task watches for client-initiated `Cancel` frames (or socket
    // close) and trips the token. We deliberately do not forward inbound
    // command frames mid-turn — accepting more `Send`/`Regenerate`s in the
    // same socket is a future-bidirectional feature, not a current one.
    let cancel_for_reader = cancel.clone();
    let reader_task = tokio::spawn(async move {
        while let Some(frame) = receiver.next().await {
            match frame {
                Ok(Message::Text(text)) => match serde_json::from_str::<ChatClientMessage>(&text) {
                    Ok(ChatClientMessage::Cancel) => {
                        tracing::info!("Client requested cancel");
                        cancel_for_reader.cancel();
                        break;
                    }
                    Ok(ChatClientMessage::Send(_)) | Ok(ChatClientMessage::Regenerate(_)) => {
                        tracing::warn!("Ignoring extra command frame mid-turn");
                    }
                    Ok(ChatClientMessage::CapabilityUpdate { .. })
                    | Ok(ChatClientMessage::ToolResponse { .. }) => {
                        // Tool-routing frames are declared in the wire types so
                        // the protocol is stable, but the server-side dispatcher
                        // isn't wired yet (lands in later phases). Drop them
                        // with a log so we have observability when a client
                        // upgrades ahead of the server.
                        tracing::debug!(
                            "Ignoring tool-routing frame; server-side dispatcher is not wired yet"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to decode client frame");
                    }
                },
                Ok(Message::Close(_)) | Err(_) => {
                    cancel_for_reader.cancel();
                    break;
                }
                Ok(Message::Binary(_)) | Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
            }
        }
    });

    // Run the turn. Errors that surface here become a final `Error` frame
    // before we close the socket.
    let dispatch_result = match command {
        InitialCommand::Send(req) => {
            run_turn(state.clone(), user_id, thread_id, req, tx, cancel.clone()).await
        }
        InitialCommand::Regenerate(req) => {
            regenerate_ai_response(state.clone(), user_id, thread_id, req, tx, cancel.clone()).await
        }
    };

    if let Err(err) = dispatch_result {
        tracing::warn!(error = %err, "Chat turn failed before agent loop dispatch");
        let _ = sender
            .send(serialize_event(&ChatServerMessage::Error {
                kind: err.error_kind().to_string(),
                message: err.to_string(),
            }))
            .await;
        cancel.cancel();
        let _ = sender.send(Message::Close(None)).await;
        reader_task.abort();
        return;
    }

    while let Some(event) = rx.recv().await {
        let frame = serialize_event(&event);
        let is_terminal = matches!(
            &event,
            ChatServerMessage::Final { .. } | ChatServerMessage::Error { .. }
        );
        if sender.send(frame).await.is_err() {
            cancel.cancel();
            break;
        }
        if is_terminal {
            break;
        }
    }

    let _ = sender.send(Message::Close(None)).await;
    cancel.cancel();
    reader_task.abort();
}

async fn wait_for_initial_command(
    receiver: &mut futures::stream::SplitStream<WebSocket>,
) -> Result<InitialCommand, String> {
    while let Some(frame) = receiver.next().await {
        match frame {
            Ok(Message::Text(text)) => match serde_json::from_str::<ChatClientMessage>(&text) {
                Ok(ChatClientMessage::Send(req)) => return Ok(InitialCommand::Send(req)),
                Ok(ChatClientMessage::Regenerate(req)) => {
                    return Ok(InitialCommand::Regenerate(req));
                }
                Ok(ChatClientMessage::Cancel) => {
                    return Err("Cannot cancel before a turn has started".to_string());
                }
                Ok(ChatClientMessage::CapabilityUpdate { .. }) => {
                    // The unified tool-execution architecture will require
                    // `CapabilityUpdate` as the first frame, but that
                    // enforcement (and the matching server-side consumer)
                    // lands in a later phase. Until then, accepting it here
                    // would silently advance to the next frame without
                    // actually binding the catalog — clearer to reject.
                    return Err(
                        "CapabilityUpdate is declared but not yet handled by this server"
                            .to_string(),
                    );
                }
                Ok(ChatClientMessage::ToolResponse { .. }) => {
                    return Err(
                        "Received ToolResponse before any tool request was issued".to_string()
                    );
                }
                Err(e) => return Err(format!("Failed to decode initial frame: {e}")),
            },
            Ok(Message::Close(_)) | Err(_) => return Err("Connection closed".to_string()),
            Ok(Message::Binary(_)) | Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => continue,
        }
    }
    Err("Connection closed before any command frame".to_string())
}

fn serialize_event(event: &ChatServerMessage) -> Message {
    match serde_json::to_string(event) {
        Ok(s) => Message::Text(s.into()),
        Err(e) => {
            tracing::error!(error = %e, "Failed to serialize chat server event");
            Message::Text(
                serde_json::to_string(&ChatServerMessage::Error {
                    kind: "internal_error".to_string(),
                    message: "Failed to serialize event".to_string(),
                })
                .expect("error envelope serializes")
                .into(),
            )
        }
    }
}

/// Load the active branch's recent message history (oldest → newest) as the
/// LLM-facing message vector.
///
/// Reads `CONTEXT_MESSAGE_LIMIT` rows DESC then reverses; rows that fail to
/// project into an `AnyMessage` are dropped with a warning rather than
/// aborting the turn — a single corrupt row should never silently break chat.
async fn load_active_branch_context(
    state: &AppState,
    user_id: Uuid,
    thread_id: Uuid,
) -> ThreadServiceResult<Vec<AnyMessage>> {
    let mut recent_messages = state
        .db
        .list_messages()
        .thread_id(thread_id)
        .user_id(user_id)
        .params(PaginationParams::new(0, CONTEXT_MESSAGE_LIMIT, "DESC"))
        .call()
        .await?;
    recent_messages.reverse();

    Ok(recent_messages
        .into_iter()
        .filter_map(|msg| {
            convert_db_message_to_base_message(msg)
                .map_err(|e| tracing::warn!("Skipping unconvertible message: {e}"))
                .ok()
        })
        .collect())
}

async fn run_turn(
    state: Arc<AppState>,
    user_id: Uuid,
    thread_id: Uuid,
    request: ChatSendRequest,
    tx: mpsc::Sender<ChatServerMessage>,
    cancel: CancellationToken,
) -> ThreadServiceResult<()> {
    // An explicit parent means this turn is an edit: rewind the active leaf
    // to that parent so the new human message branches off it.
    if let Some(parent_id) = request.parent_message_id {
        state
            .db
            .set_active_leaf()
            .id(thread_id)
            .user_id(user_id)
            .active_leaf_id(parent_id)
            .call()
            .await?;
    }

    let mut messages = load_active_branch_context(&state, user_id, thread_id).await?;

    // Rewrite inline payloads (large text, base64 images) into asset
    // references before we persist or feed them to the LLM. This used to be
    // a separate `POST /threads/{id}/preliminary-blocks` round trip; doing
    // it inline keeps the chat turn to a single round trip and removes the
    // class of bugs where the rewrite step silently failed and the chat
    // proceeded with raw payloads.
    let content_blocks: Vec<ContentBlock> =
        rewrite_preliminary_blocks(&state, user_id, request.content_blocks).await?;

    let mut human_additional_kwargs: HashMap<String, Value> = HashMap::new();
    if let Some(ref chips_json) = request.asset_chips_json
        && let Ok(chips_value) = serde_json::from_str::<Value>(chips_json)
    {
        human_additional_kwargs.insert("asset_chips".to_string(), chips_value);
    }

    let human_message = HumanMessage::builder()
        .content(content_blocks)
        .additional_kwargs(human_additional_kwargs)
        .build();

    // Serialize the persistable form of the human message *before* the
    // borrow of `human_message.content` is released to `prepare_llm_context`
    // — that way we don't have to hold the message past the LLM-context call.
    let content = serde_json::to_value(&human_message.content)
        .map_err(|e| ThreadServiceError::Internal(format!("Failed to serialize content: {e}")))?;
    let additional_kwargs =
        serde_json::to_value(&human_message.additional_kwargs).map_err(|e| {
            ThreadServiceError::Internal(format!("Failed to serialize additional_kwargs: {e}"))
        })?;

    messages.push(human_message.into());

    // Prepare the LLM context *before* persisting the human message so that
    // a context-prep failure doesn't leave a half-completed turn (a human
    // row with no AI response) in the thread history.
    let LlmContext {
        messages: llm_messages,
        chat_model,
        tools,
    } = prepare_llm_context(&state.providers, &state.asset_service, messages).await?;

    let human_db_message = state
        .db
        .create_message()
        .thread_id(thread_id)
        .user_id(user_id)
        .message_type(MessageType::Human)
        .content(content)
        .additional_kwargs(additional_kwargs)
        .call()
        .await?;

    // Link the thread to the activity the client was in when the message was
    // sent. Idempotent — the composite PK (activity_id, thread_id) absorbs
    // repeat sends from the same activity. Failure here must not poison the
    // turn (chat works without the link), so we log and continue.
    if let Some(activity_id) = request.activity_id
        && let Err(err) = state
            .db
            .link_activity_to_thread()
            .activity_id(activity_id)
            .thread_id(thread_id)
            .user_id(user_id)
            .call()
            .await
    {
        tracing::warn!(
            error = %err,
            activity_id = %activity_id,
            thread_id = %thread_id,
            "Failed to link activity to thread"
        );
    }

    let human_message_id = human_db_message.id;
    let human_parent_id = human_db_message.parent_message_id;
    let human_node = MessageNode {
        parent_id: human_parent_id,
        message: convert_db_message_to_base_message(human_db_message)?,
        children: vec![],
        sibling_index: 0,
        depth: 0,
    };

    if tx
        .send(ChatServerMessage::ConfirmedHumanMessage {
            message: human_node,
        })
        .await
        .is_err()
    {
        // Receiver dropped — caller will close the socket. Cancel the loop
        // we're about to spawn so nothing runs past this point.
        cancel.cancel();
        return Ok(());
    }

    let db = state.db.clone();
    tokio::spawn(
        run_agent_loop()
            .tx(tx)
            .token(cancel)
            .db(db)
            .chat_model(chat_model)
            .tools(tools)
            .messages(llm_messages)
            .thread_id(thread_id)
            .user_id(user_id)
            .human_message_id(human_message_id)
            .max_tool_rounds(MAX_TOOL_ROUNDS)
            .call(),
    );

    Ok(())
}

async fn regenerate_ai_response(
    state: Arc<AppState>,
    user_id: Uuid,
    thread_id: Uuid,
    request: RegenerateRequest,
    tx: mpsc::Sender<ChatServerMessage>,
    cancel: CancellationToken,
) -> ThreadServiceResult<()> {
    // Resolve the AI message and its parent. We require the target to be an
    // AI row with a parent so the new variant has somewhere to attach.
    let ai_message = state
        .db
        .get_message(thread_id, user_id, request.ai_message_id)
        .await?
        .ok_or_else(|| ThreadServiceError::not_found("AI message"))?;

    if ai_message.message_type != MessageType::Ai {
        return Err(ThreadServiceError::invalid_argument(
            "ai_message_id must reference an AI message",
        ));
    }

    let human_parent_id = ai_message.parent_message_id.ok_or_else(|| {
        ThreadServiceError::invalid_argument("AI message has no parent to regenerate from")
    })?;

    // Rewind the active leaf to the human parent so the agent loop's new AI
    // row is created as a sibling of the original AI message.
    state
        .db
        .set_active_leaf()
        .id(thread_id)
        .user_id(user_id)
        .active_leaf_id(human_parent_id)
        .call()
        .await?;

    let messages = load_active_branch_context(&state, user_id, thread_id).await?;

    let LlmContext {
        messages: llm_messages,
        chat_model,
        tools,
    } = prepare_llm_context(&state.providers, &state.asset_service, messages).await?;

    let db = state.db.clone();
    tokio::spawn(
        run_agent_loop()
            .tx(tx)
            .token(cancel)
            .db(db)
            .chat_model(chat_model)
            .tools(tools)
            .messages(llm_messages)
            .thread_id(thread_id)
            .user_id(user_id)
            .human_message_id(human_parent_id)
            .max_tool_rounds(MAX_TOOL_ROUNDS)
            .call(),
    );

    Ok(())
}
