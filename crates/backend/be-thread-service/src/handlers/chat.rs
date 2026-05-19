//! WebSocket chat endpoint.
//!
//! The frontend opens one WebSocket per chat turn. The wire protocol is the
//! [`ChatClientMessage`] / [`ChatServerMessage`] enum pair from `thread-core`.
//!
//! The frame-ordering contract is strict:
//!
//! 1. **First**, the client sends [`ChatClientMessage::CapabilityUpdate`]
//!    declaring its remote-dispatchable tools and any contexts that are live
//!    (e.g. the currently-focused YouTube watch page). The server uses this
//!    to assemble the per-turn tool catalog and to render the active-context
//!    system message.
//! 2. **Second**, the client sends either [`ChatClientMessage::Send`] to
//!    start a turn from a new human message or [`ChatClientMessage::Regenerate`]
//!    to re-roll an existing AI response.
//!
//! Anything else in the first two slots — `Cancel`, `ToolResponse`, a second
//! `CapabilityUpdate`, malformed JSON, or a timeout — is a
//! [`ThreadServiceError::ProtocolViolation`]. The server emits one
//! `Error { kind: "protocol", ... }` frame and closes.
//!
//! Once the turn is running, the agent loop streams [`ChatServerMessage::Chunk`]
//! frames and ends with [`ChatServerMessage::Final`]. Tool dispatch flows
//! through [`ChatRemoteBus`]: the server emits
//! [`ChatServerMessage::ToolRequest`] / [`ChatServerMessage::ToolCancel`]
//! on the same socket and the client returns [`ChatClientMessage::ToolResponse`]
//! frames the bus correlates by `call_id`. At any point the client can send
//! [`ChatClientMessage::Cancel`] (or just drop the socket) to abort.
//!
//! Token gating is enforced by the surrounding `be-authz` middleware before
//! the upgrade handshake completes.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use agent_chain::HumanMessage;
use agent_chain::messages::{AnyMessage, ContentBlock};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::Response;
use be_remote_db::{MessageType, PaginationParams};
use futures::Stream;
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use thread_core::{
    CapabilityUpdatePayload, ChatClientMessage, ChatSendRequest, ChatServerMessage, MessageNode,
    RegenerateRequest,
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
use crate::remote_tool_bus::ChatRemoteBus;
use crate::service::AppState;

/// Trailing messages from the active branch fed back to the LLM as
/// context for the next turn. Small on purpose — long histories blow
/// the token budget, and recall of older messages is a job for the
/// summarisation pipeline (when it lands), not the chat prelude.
const CONTEXT_MESSAGE_LIMIT: u32 = 5;

/// Hard cap on tool-call rounds per turn. Guards against an agent
/// looping over its own tool calls without converging on an answer.
/// On exhaustion the loop runs one forced synthesis with
/// `tool_choice=none` and finalises whatever it has accumulated.
const MAX_TOOL_ROUNDS: usize = 15;

/// Buffer depth for the agent-loop → WebSocket-writer channel. Sized
/// to absorb a small burst of `Chunk` frames before the writer
/// flushes; beyond this, backpressure parks the agent loop cleanly.
const SERVER_CHANNEL_DEPTH: usize = 32;

/// Budget for each of the two prelude frames (`CapabilityUpdate` then
/// `Send`/`Regenerate`). The client should send both immediately after
/// connecting — anything slower than this points at a stalled or
/// misbehaving peer rather than a slow network.
const CAPABILITY_FRAME_TIMEOUT: Duration = Duration::from_secs(5);

/// One of the command frames that may legally open a chat turn.
#[derive(Debug)]
enum InitialCommand {
    Send(ChatSendRequest),
    Regenerate(RegenerateRequest),
}

/// Typed protocol fault surfaced by the prelude reader.
///
/// Each variant carries enough context to render an actionable message back
/// to the client; the conversion to [`ThreadServiceError::ProtocolViolation`]
/// preserves that wording.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
enum ProtocolError {
    /// We received a known client-frame variant but it was not legal at the
    /// current stage (e.g. `Send` before `CapabilityUpdate`).
    #[error("expected {expected} as the {stage} frame, received {got}")]
    UnexpectedFrame {
        stage: &'static str,
        expected: &'static str,
        got: &'static str,
    },
    /// The text payload didn't parse as a `ChatClientMessage`.
    #[error("failed to decode {stage} frame: {detail}")]
    Malformed { stage: &'static str, detail: String },
    /// The peer dropped the socket before sending the expected frame.
    #[error("socket closed before the {stage} frame arrived")]
    PrematureClose { stage: &'static str },
    /// The client took longer than [`CAPABILITY_FRAME_TIMEOUT`] to send the
    /// expected frame.
    #[error("timed out waiting for the {stage} frame")]
    Timeout { stage: &'static str },
}

impl From<ProtocolError> for ThreadServiceError {
    fn from(err: ProtocolError) -> Self {
        ThreadServiceError::ProtocolViolation(err.to_string())
    }
}

/// Decision returned by [`handle_inbound_frame`].
#[derive(Debug, PartialEq, Eq)]
enum InboundDecision {
    /// Keep reading the next inbound frame.
    Continue,
    /// Tear down the reader loop; no further frames should be processed.
    Stop,
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

    // Strict prelude: `CapabilityUpdate` then `Send`/`Regenerate`. Anything
    // else closes the socket with an `Error { kind: "protocol", ... }`.
    let (capability, command) = match wait_for_capability_then_initial_command(&mut receiver).await
    {
        Ok(pair) => pair,
        Err(err) => {
            let detail = err.to_string();
            tracing::info!(error = %detail, "Chat protocol violation");
            let _ = sender
                .send(serialize_event(&ChatServerMessage::Error {
                    kind: "protocol".to_string(),
                    message: detail,
                }))
                .await;
            let _ = sender.send(Message::Close(None)).await;
            return;
        }
    };

    let cancel = CancellationToken::new();
    let (tx, mut rx) = mpsc::channel::<ChatServerMessage>(SERVER_CHANNEL_DEPTH);
    // The bus is shared between the reader task (which calls `resolve` on
    // `ToolResponse` frames) and the agent loop (which calls `call` to
    // dispatch a remote tool). Cleanup at the end of the turn calls
    // `shutdown()` to wake any callers parked on a `oneshot` that's never
    // going to resolve.
    let bus = ChatRemoteBus::new(tx.clone(), cancel.clone());

    let reader_task = tokio::spawn(read_inbound_frames(receiver, cancel.clone(), bus.clone()));

    let dispatch_result = match command {
        InitialCommand::Send(req) => {
            run_turn(
                state.clone(),
                user_id,
                thread_id,
                req,
                capability,
                tx,
                cancel.clone(),
                bus.clone(),
            )
            .await
        }
        InitialCommand::Regenerate(req) => {
            regenerate_ai_response(
                state.clone(),
                user_id,
                thread_id,
                req,
                capability,
                tx,
                cancel.clone(),
                bus.clone(),
            )
            .await
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
        bus.shutdown();
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

    bus.shutdown();
    cancel.cancel();
    let _ = sender.send(Message::Close(None)).await;
    reader_task.abort();
}

/// Wait for the strict two-frame prelude: a [`ChatClientMessage::CapabilityUpdate`]
/// followed by a [`ChatClientMessage::Send`] or [`ChatClientMessage::Regenerate`].
///
/// Generic over the inbound stream so unit tests can drive it with a
/// hand-rolled frame source; the production caller passes the receiver half
/// of a split [`WebSocket`]. Each frame must arrive within
/// [`CAPABILITY_FRAME_TIMEOUT`]; otherwise the prelude fails with
/// [`ProtocolError::Timeout`].
async fn wait_for_capability_then_initial_command<S>(
    receiver: &mut S,
) -> Result<(CapabilityUpdatePayload, InitialCommand), ProtocolError>
where
    S: Stream<Item = Result<Message, axum::Error>> + Unpin,
{
    let capability = match read_client_message(receiver, "capability_update").await? {
        ChatClientMessage::CapabilityUpdate(payload) => payload,
        ChatClientMessage::Send(_) => {
            return Err(ProtocolError::UnexpectedFrame {
                stage: "first",
                expected: "capability_update",
                got: "send",
            });
        }
        ChatClientMessage::Regenerate(_) => {
            return Err(ProtocolError::UnexpectedFrame {
                stage: "first",
                expected: "capability_update",
                got: "regenerate",
            });
        }
        ChatClientMessage::Cancel => {
            return Err(ProtocolError::UnexpectedFrame {
                stage: "first",
                expected: "capability_update",
                got: "cancel",
            });
        }
        ChatClientMessage::ToolResponse { .. } => {
            return Err(ProtocolError::UnexpectedFrame {
                stage: "first",
                expected: "capability_update",
                got: "tool_response",
            });
        }
    };

    let command = match read_client_message(receiver, "send_or_regenerate").await? {
        ChatClientMessage::Send(req) => InitialCommand::Send(req),
        ChatClientMessage::Regenerate(req) => InitialCommand::Regenerate(req),
        ChatClientMessage::CapabilityUpdate(_) => {
            return Err(ProtocolError::UnexpectedFrame {
                stage: "second",
                expected: "send_or_regenerate",
                got: "capability_update",
            });
        }
        ChatClientMessage::Cancel => {
            return Err(ProtocolError::UnexpectedFrame {
                stage: "second",
                expected: "send_or_regenerate",
                got: "cancel",
            });
        }
        ChatClientMessage::ToolResponse { .. } => {
            return Err(ProtocolError::UnexpectedFrame {
                stage: "second",
                expected: "send_or_regenerate",
                got: "tool_response",
            });
        }
    };

    Ok((capability, command))
}

/// Read the next text frame from `receiver` and decode it as a
/// [`ChatClientMessage`]. Non-text frames before the first text frame are
/// silently dropped (browsers and proxies emit `Ping`/`Pong` frames during
/// the upgrade hand-off). `Binary` is treated as a protocol violation: the
/// chat wire is text-only.
async fn read_client_message<S>(
    receiver: &mut S,
    stage: &'static str,
) -> Result<ChatClientMessage, ProtocolError>
where
    S: Stream<Item = Result<Message, axum::Error>> + Unpin,
{
    loop {
        let frame = match tokio::time::timeout(CAPABILITY_FRAME_TIMEOUT, receiver.next()).await {
            Ok(Some(frame)) => frame,
            Ok(None) => return Err(ProtocolError::PrematureClose { stage }),
            Err(_) => return Err(ProtocolError::Timeout { stage }),
        };

        match frame {
            Ok(Message::Text(text)) => {
                return serde_json::from_str::<ChatClientMessage>(&text).map_err(|e| {
                    ProtocolError::Malformed {
                        stage,
                        detail: e.to_string(),
                    }
                });
            }
            Ok(Message::Binary(_)) => {
                return Err(ProtocolError::Malformed {
                    stage,
                    detail: "binary frames are not accepted on the chat WebSocket".into(),
                });
            }
            Ok(Message::Close(_)) => return Err(ProtocolError::PrematureClose { stage }),
            Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => continue,
            Err(e) => {
                return Err(ProtocolError::Malformed {
                    stage,
                    detail: e.to_string(),
                });
            }
        }
    }
}

/// Drain the receiver for the rest of the turn, routing inbound frames to
/// the cancellation token and the remote-tool bus.
///
/// Generic over the inbound stream so the loop is testable with a stub
/// stream; production callers pass the receiver half of a split
/// [`WebSocket`].
async fn read_inbound_frames<S>(mut receiver: S, cancel: CancellationToken, bus: Arc<ChatRemoteBus>)
where
    S: Stream<Item = Result<Message, axum::Error>> + Unpin,
{
    while let Some(frame) = receiver.next().await {
        match handle_inbound_frame(frame, &cancel, &bus) {
            InboundDecision::Continue => continue,
            InboundDecision::Stop => break,
        }
    }
}

/// Per-frame routing for the post-prelude inbound reader. Pulled out as a
/// pure function so the matrix of accepted frame shapes can be unit-tested
/// without driving a real WebSocket.
fn handle_inbound_frame(
    frame: Result<Message, axum::Error>,
    cancel: &CancellationToken,
    bus: &ChatRemoteBus,
) -> InboundDecision {
    match frame {
        Ok(Message::Text(text)) => match serde_json::from_str::<ChatClientMessage>(&text) {
            Ok(ChatClientMessage::Cancel) => {
                tracing::info!("Client requested cancel");
                cancel.cancel();
                InboundDecision::Stop
            }
            Ok(ChatClientMessage::ToolResponse { call_id, result }) => {
                bus.resolve(call_id, result);
                InboundDecision::Continue
            }
            Ok(ChatClientMessage::Send(_))
            | Ok(ChatClientMessage::Regenerate(_))
            | Ok(ChatClientMessage::CapabilityUpdate(_)) => {
                // Re-declaring capabilities or starting a second turn on the
                // same socket isn't supported in v1. Log and ignore rather
                // than tearing down an in-flight turn over client misuse.
                tracing::debug!("Ignoring out-of-protocol command frame mid-turn");
                InboundDecision::Continue
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to decode client frame");
                InboundDecision::Continue
            }
        },
        Ok(Message::Close(_)) => {
            cancel.cancel();
            InboundDecision::Stop
        }
        Err(e) => {
            tracing::debug!(error = %e, "Inbound WebSocket transport error; cancelling turn");
            cancel.cancel();
            InboundDecision::Stop
        }
        Ok(Message::Binary(_)) | Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {
            InboundDecision::Continue
        }
    }
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

#[allow(clippy::too_many_arguments)]
async fn run_turn(
    state: Arc<AppState>,
    user_id: Uuid,
    thread_id: Uuid,
    request: ChatSendRequest,
    capability: CapabilityUpdatePayload,
    tx: mpsc::Sender<ChatServerMessage>,
    cancel: CancellationToken,
    bus: Arc<ChatRemoteBus>,
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
    // row with no AI response) in the thread history. The catalog is built
    // from the per-turn `CapabilityUpdate` plus the server-local tool set.
    let CapabilityUpdatePayload {
        tools: remote_tools,
        contexts: active_contexts,
    } = capability;
    let LlmContext {
        messages: llm_messages,
        chat_model,
        catalog,
    } = prepare_llm_context(
        &state.providers,
        &state.asset_service,
        messages,
        remote_tools,
        &active_contexts,
    )
    .await?;

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
            .catalog(catalog)
            .remote_bus(bus)
            .messages(llm_messages)
            .thread_id(thread_id)
            .user_id(user_id)
            .human_message_id(human_message_id)
            .max_tool_rounds(MAX_TOOL_ROUNDS)
            .call(),
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn regenerate_ai_response(
    state: Arc<AppState>,
    user_id: Uuid,
    thread_id: Uuid,
    request: RegenerateRequest,
    capability: CapabilityUpdatePayload,
    tx: mpsc::Sender<ChatServerMessage>,
    cancel: CancellationToken,
    bus: Arc<ChatRemoteBus>,
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

    let CapabilityUpdatePayload {
        tools: remote_tools,
        contexts: active_contexts,
    } = capability;
    let LlmContext {
        messages: llm_messages,
        chat_model,
        catalog,
    } = prepare_llm_context(
        &state.providers,
        &state.asset_service,
        messages,
        remote_tools,
        &active_contexts,
    )
    .await?;

    let db = state.db.clone();
    tokio::spawn(
        run_agent_loop()
            .tx(tx)
            .token(cancel)
            .db(db)
            .chat_model(chat_model)
            .catalog(catalog)
            .remote_bus(bus)
            .messages(llm_messages)
            .thread_id(thread_id)
            .user_id(user_id)
            .human_message_id(human_parent_id)
            .max_tool_rounds(MAX_TOOL_ROUNDS)
            .call(),
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use eurora_tools::{RemoteToolBus, ToolError};
    use serde_json::json;
    use thread_core::{
        CapabilityUpdatePayload, ChatSendRequest, RegenerateRequest, ToolErrorWire, ToolSource,
        WireToolDescriptor,
    };
    use tokio::sync::mpsc;

    /// Encode a `ChatClientMessage` as a text WebSocket frame, matching the
    /// shape the production stream yields.
    fn frame(msg: &ChatClientMessage) -> Result<Message, axum::Error> {
        let text = serde_json::to_string(msg).expect("encode");
        Ok(Message::Text(text.into()))
    }

    fn send_request() -> ChatSendRequest {
        ChatSendRequest {
            content_blocks: vec![],
            parent_message_id: None,
            asset_chips_json: None,
            activity_id: None,
        }
    }

    fn regenerate_request() -> RegenerateRequest {
        RegenerateRequest {
            ai_message_id: Uuid::nil(),
        }
    }

    fn sample_descriptor() -> WireToolDescriptor {
        WireToolDescriptor {
            definition: agent_chain_core::tools::ToolDefinition {
                name: "browser::test::echo".to_string(),
                description: "x".to_string(),
                parameters: json!({"type": "object"}),
            },
            output_schema: json!({"type": "object"}),
            timeout_ms: 60_000,
            source: ToolSource::Bridge {
                app_kind: "browser".to_string(),
            },
            required_contexts: vec![],
            requires_user_approval: false,
        }
    }

    fn make_bus() -> (
        Arc<ChatRemoteBus>,
        mpsc::Receiver<ChatServerMessage>,
        CancellationToken,
    ) {
        let (tx, rx) = mpsc::channel(8);
        let cancel = CancellationToken::new();
        let bus = ChatRemoteBus::new(tx, cancel.clone());
        (bus, rx, cancel)
    }

    // ------------------------------------------------------------------
    // wait_for_capability_then_initial_command
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn prelude_accepts_capability_then_send() {
        let cap = CapabilityUpdatePayload::default();
        let mut stream = futures::stream::iter(vec![
            frame(&ChatClientMessage::CapabilityUpdate(cap.clone())),
            frame(&ChatClientMessage::Send(send_request())),
        ]);
        let (capability, command) = wait_for_capability_then_initial_command(&mut stream)
            .await
            .expect("prelude is happy-path");
        assert_eq!(capability, cap);
        assert!(matches!(command, InitialCommand::Send(_)));
    }

    #[tokio::test]
    async fn prelude_accepts_capability_then_regenerate() {
        let mut stream = futures::stream::iter(vec![
            frame(&ChatClientMessage::CapabilityUpdate(
                CapabilityUpdatePayload::default(),
            )),
            frame(&ChatClientMessage::Regenerate(regenerate_request())),
        ]);
        let (_, command) = wait_for_capability_then_initial_command(&mut stream)
            .await
            .expect("prelude is happy-path");
        assert!(matches!(command, InitialCommand::Regenerate(_)));
    }

    #[tokio::test]
    async fn prelude_rejects_send_as_first_frame() {
        let mut stream =
            futures::stream::iter(vec![frame(&ChatClientMessage::Send(send_request()))]);
        let err = wait_for_capability_then_initial_command(&mut stream)
            .await
            .unwrap_err();
        assert_eq!(
            err,
            ProtocolError::UnexpectedFrame {
                stage: "first",
                expected: "capability_update",
                got: "send"
            }
        );
    }

    #[tokio::test]
    async fn prelude_rejects_regenerate_as_first_frame() {
        let mut stream = futures::stream::iter(vec![frame(&ChatClientMessage::Regenerate(
            regenerate_request(),
        ))]);
        let err = wait_for_capability_then_initial_command(&mut stream)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            ProtocolError::UnexpectedFrame {
                stage: "first",
                got: "regenerate",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn prelude_rejects_cancel_as_first_frame() {
        let mut stream = futures::stream::iter(vec![frame(&ChatClientMessage::Cancel)]);
        let err = wait_for_capability_then_initial_command(&mut stream)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            ProtocolError::UnexpectedFrame {
                stage: "first",
                got: "cancel",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn prelude_rejects_tool_response_as_first_frame() {
        let mut stream = futures::stream::iter(vec![frame(&ChatClientMessage::ToolResponse {
            call_id: 0,
            result: Ok(json!({})),
        })]);
        let err = wait_for_capability_then_initial_command(&mut stream)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            ProtocolError::UnexpectedFrame {
                stage: "first",
                got: "tool_response",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn prelude_rejects_capability_as_second_frame() {
        let mut stream = futures::stream::iter(vec![
            frame(&ChatClientMessage::CapabilityUpdate(
                CapabilityUpdatePayload::default(),
            )),
            frame(&ChatClientMessage::CapabilityUpdate(
                CapabilityUpdatePayload::default(),
            )),
        ]);
        let err = wait_for_capability_then_initial_command(&mut stream)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            ProtocolError::UnexpectedFrame {
                stage: "second",
                got: "capability_update",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn prelude_rejects_cancel_as_second_frame() {
        let mut stream = futures::stream::iter(vec![
            frame(&ChatClientMessage::CapabilityUpdate(
                CapabilityUpdatePayload::default(),
            )),
            frame(&ChatClientMessage::Cancel),
        ]);
        let err = wait_for_capability_then_initial_command(&mut stream)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            ProtocolError::UnexpectedFrame {
                stage: "second",
                got: "cancel",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn prelude_rejects_tool_response_as_second_frame() {
        let mut stream = futures::stream::iter(vec![
            frame(&ChatClientMessage::CapabilityUpdate(
                CapabilityUpdatePayload::default(),
            )),
            frame(&ChatClientMessage::ToolResponse {
                call_id: 0,
                result: Ok(json!({})),
            }),
        ]);
        let err = wait_for_capability_then_initial_command(&mut stream)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            ProtocolError::UnexpectedFrame {
                stage: "second",
                got: "tool_response",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn prelude_rejects_malformed_first_frame() {
        let mut stream = futures::stream::iter(vec![
            Ok(Message::Text("not json".into())) as Result<Message, axum::Error>
        ]);
        let err = wait_for_capability_then_initial_command(&mut stream)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            ProtocolError::Malformed {
                stage: "capability_update",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn prelude_rejects_binary_frame() {
        let mut stream = futures::stream::iter(vec![
            Ok(Message::Binary(vec![1u8, 2, 3].into())) as Result<Message, axum::Error>
        ]);
        let err = wait_for_capability_then_initial_command(&mut stream)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            ProtocolError::Malformed {
                stage: "capability_update",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn prelude_rejects_premature_close() {
        let mut stream = futures::stream::iter(Vec::<Result<Message, axum::Error>>::new());
        let err = wait_for_capability_then_initial_command(&mut stream)
            .await
            .unwrap_err();
        assert_eq!(
            err,
            ProtocolError::PrematureClose {
                stage: "capability_update"
            }
        );
    }

    /// Ping/Pong control frames may arrive while we're waiting for the
    /// prelude — typical for browsers/proxies during the upgrade hand-off.
    /// They must be skipped, not treated as protocol violations.
    #[tokio::test]
    async fn prelude_skips_ping_pong_until_text_frame() {
        let mut stream = futures::stream::iter(vec![
            Ok(Message::Ping(vec![1u8, 2, 3].into())) as Result<Message, axum::Error>,
            Ok(Message::Pong(vec![4u8, 5, 6].into())),
            frame(&ChatClientMessage::CapabilityUpdate(
                CapabilityUpdatePayload::default(),
            )),
            frame(&ChatClientMessage::Send(send_request())),
        ]);
        let result = wait_for_capability_then_initial_command(&mut stream).await;
        assert!(result.is_ok(), "ping/pong should be tolerated: {result:?}");
    }

    #[tokio::test(start_paused = true)]
    async fn prelude_times_out_when_no_frame_arrives() {
        let mut stream = futures::stream::pending::<Result<Message, axum::Error>>();
        let err = wait_for_capability_then_initial_command(&mut stream)
            .await
            .unwrap_err();
        assert_eq!(
            err,
            ProtocolError::Timeout {
                stage: "capability_update"
            }
        );
    }

    #[tokio::test(start_paused = true)]
    async fn prelude_times_out_on_second_frame() {
        // The first frame arrives immediately; the second never does — we
        // must time out on the *second* stage, not collapse into a
        // capability-stage timeout.
        let capability_frame = frame(&ChatClientMessage::CapabilityUpdate(
            CapabilityUpdatePayload::default(),
        ));
        let stream = futures::stream::iter(vec![capability_frame])
            .chain(futures::stream::pending::<Result<Message, axum::Error>>());
        let mut stream = Box::pin(stream);
        let err = wait_for_capability_then_initial_command(&mut stream)
            .await
            .unwrap_err();
        assert_eq!(
            err,
            ProtocolError::Timeout {
                stage: "send_or_regenerate"
            }
        );
    }

    /// `ProtocolError` converts to `ThreadServiceError::ProtocolViolation`
    /// so the chat handler can surface it as `Error { kind: "protocol" }`.
    #[test]
    fn protocol_error_converts_to_thread_service_error() {
        let err: ThreadServiceError = ProtocolError::PrematureClose {
            stage: "capability_update",
        }
        .into();
        assert_eq!(err.error_kind(), "protocol");
    }

    // ------------------------------------------------------------------
    // handle_inbound_frame
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn inbound_cancel_trips_token_and_stops() {
        let (bus, _rx, cancel) = make_bus();
        let decision = handle_inbound_frame(frame(&ChatClientMessage::Cancel), &cancel, &bus);
        assert_eq!(decision, InboundDecision::Stop);
        assert!(cancel.is_cancelled());
    }

    #[tokio::test]
    async fn inbound_tool_response_ok_resolves_pending_call() {
        let (bus, mut rx, cancel) = make_bus();
        let descriptor = sample_descriptor();
        let bus_clone = bus.clone();
        let pending = tokio::spawn(async move { bus_clone.call(&descriptor, json!({})).await });

        let call_id = match rx.recv().await {
            Some(ChatServerMessage::ToolRequest { call_id, .. }) => call_id,
            other => panic!("expected ToolRequest, got {other:?}"),
        };

        let response = frame(&ChatClientMessage::ToolResponse {
            call_id,
            result: Ok(json!({"answer": 42})),
        });
        let decision = handle_inbound_frame(response, &cancel, &bus);
        assert_eq!(decision, InboundDecision::Continue);
        assert!(!cancel.is_cancelled());

        let outcome = pending
            .await
            .expect("task didn't panic")
            .expect("resolved ok");
        assert_eq!(outcome, json!({"answer": 42}));
    }

    #[tokio::test]
    async fn inbound_tool_response_err_resolves_with_error() {
        let (bus, mut rx, cancel) = make_bus();
        let descriptor = sample_descriptor();
        let bus_clone = bus.clone();
        let pending = tokio::spawn(async move { bus_clone.call(&descriptor, json!({})).await });

        let call_id = match rx.recv().await {
            Some(ChatServerMessage::ToolRequest { call_id, .. }) => call_id,
            other => panic!("expected ToolRequest, got {other:?}"),
        };

        let response = frame(&ChatClientMessage::ToolResponse {
            call_id,
            result: Err(ToolErrorWire::Timeout),
        });
        let decision = handle_inbound_frame(response, &cancel, &bus);
        assert_eq!(decision, InboundDecision::Continue);
        assert!(!cancel.is_cancelled());

        let outcome = pending.await.expect("task didn't panic");
        assert!(matches!(outcome, Err(ToolError::Timeout)), "{outcome:?}");
    }

    #[tokio::test]
    async fn inbound_mid_turn_send_is_ignored() {
        let (bus, _rx, cancel) = make_bus();
        let decision = handle_inbound_frame(
            frame(&ChatClientMessage::Send(send_request())),
            &cancel,
            &bus,
        );
        assert_eq!(decision, InboundDecision::Continue);
        assert!(!cancel.is_cancelled());
    }

    #[tokio::test]
    async fn inbound_mid_turn_regenerate_is_ignored() {
        let (bus, _rx, cancel) = make_bus();
        let decision = handle_inbound_frame(
            frame(&ChatClientMessage::Regenerate(regenerate_request())),
            &cancel,
            &bus,
        );
        assert_eq!(decision, InboundDecision::Continue);
        assert!(!cancel.is_cancelled());
    }

    #[tokio::test]
    async fn inbound_mid_turn_capability_update_is_ignored() {
        let (bus, _rx, cancel) = make_bus();
        let decision = handle_inbound_frame(
            frame(&ChatClientMessage::CapabilityUpdate(
                CapabilityUpdatePayload::default(),
            )),
            &cancel,
            &bus,
        );
        assert_eq!(decision, InboundDecision::Continue);
        assert!(!cancel.is_cancelled());
    }

    #[tokio::test]
    async fn inbound_close_frame_trips_token_and_stops() {
        let (bus, _rx, cancel) = make_bus();
        let decision = handle_inbound_frame(Ok(Message::Close(None)), &cancel, &bus);
        assert_eq!(decision, InboundDecision::Stop);
        assert!(cancel.is_cancelled());
    }

    #[tokio::test]
    async fn inbound_decode_error_is_logged_and_ignored() {
        let (bus, _rx, cancel) = make_bus();
        let decision = handle_inbound_frame(Ok(Message::Text("not json".into())), &cancel, &bus);
        assert_eq!(decision, InboundDecision::Continue);
        assert!(!cancel.is_cancelled());
    }

    #[tokio::test]
    async fn inbound_binary_ping_pong_are_ignored() {
        for frame_val in [
            Message::Binary(vec![1u8].into()),
            Message::Ping(vec![1u8].into()),
            Message::Pong(vec![1u8].into()),
        ] {
            let (bus, _rx, cancel) = make_bus();
            let decision = handle_inbound_frame(Ok(frame_val), &cancel, &bus);
            assert_eq!(decision, InboundDecision::Continue);
            assert!(!cancel.is_cancelled());
        }
    }
}
