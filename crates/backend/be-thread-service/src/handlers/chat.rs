//! WebSocket chat endpoint.
//!
//! The frontend opens one WebSocket per chat turn (via the desktop's Tauri
//! bridge). The wire protocol is the [`ChatClientMessage`] / [`ChatServerMessage`]
//! enum pair from `thread-core`. The server dispatch is:
//!
//! 1. The client sends [`ChatClientMessage::Send`] with the human turn.
//! 2. The server persists the human message, emits
//!    [`ChatServerMessage::ConfirmedHumanMessage`], and spawns the agent loop.
//! 3. The agent loop streams [`ChatServerMessage::Chunk`] frames as the model
//!    produces them and ends with [`ChatServerMessage::Final`].
//! 4. The server then closes the socket.
//!
//! At any point the client can send [`ChatClientMessage::Cancel`] (or just
//! drop the socket) to abort the turn — both paths cancel the agent loop's
//! [`CancellationToken`].
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
use thread_core::{ChatClientMessage, ChatSendRequest, ChatServerMessage, MessageNode};
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

    // First inbound frame must be `Send`. `Cancel` before any in-flight turn
    // is meaningless and we treat it as a protocol error to keep the state
    // machine simple.
    let send_request = match wait_for_send(&mut receiver).await {
        Ok(req) => req,
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
    // messages other than `Cancel` — accepting more `Send`s mid-turn is a
    // future-bidirectional feature, not a current one.
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
                    Ok(ChatClientMessage::Send(_)) => {
                        tracing::warn!("Ignoring extra Send frame mid-turn");
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
    if let Err(err) = run_turn(
        state.clone(),
        user_id,
        thread_id,
        send_request,
        tx,
        cancel.clone(),
    )
    .await
    {
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

async fn wait_for_send(
    receiver: &mut futures::stream::SplitStream<WebSocket>,
) -> Result<ChatSendRequest, String> {
    while let Some(frame) = receiver.next().await {
        match frame {
            Ok(Message::Text(text)) => match serde_json::from_str::<ChatClientMessage>(&text) {
                Ok(ChatClientMessage::Send(req)) => return Ok(req),
                Ok(ChatClientMessage::Cancel) => {
                    return Err("Cannot cancel before a turn has started".to_string());
                }
                Err(e) => return Err(format!("Failed to decode initial frame: {e}")),
            },
            Ok(Message::Close(_)) | Err(_) => return Err("Connection closed".to_string()),
            Ok(Message::Binary(_)) | Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => continue,
        }
    }
    Err("Connection closed before any Send frame".to_string())
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

async fn run_turn(
    state: Arc<AppState>,
    user_id: Uuid,
    thread_id: Uuid,
    request: ChatSendRequest,
    tx: mpsc::Sender<ChatServerMessage>,
    cancel: CancellationToken,
) -> ThreadServiceResult<()> {
    // An explicit parent means this turn is an edit or re-roll: rewind the
    // active leaf to that parent so the new human message branches off it.
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

    // Pull recent context (DESC then reverse to get chronological order).
    let mut recent_messages = state
        .db
        .list_messages()
        .thread_id(thread_id)
        .user_id(user_id)
        .params(PaginationParams::new(0, CONTEXT_MESSAGE_LIMIT, "DESC"))
        .call()
        .await?;
    recent_messages.reverse();

    let mut messages: Vec<AnyMessage> = recent_messages
        .into_iter()
        .filter_map(|msg| {
            convert_db_message_to_base_message(msg)
                .map_err(|e| tracing::warn!("Skipping unconvertible message: {e}"))
                .ok()
        })
        .collect();

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
