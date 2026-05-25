use std::sync::Arc;

use euro_transport_policy::CHAT_STREAM_TIMEOUT;
use tauri::{AppHandle, Manager, ipc::Channel};
use thread_core::{ChatSendRequest, ChatServerMessage, RegenerateRequest, ToolBackend};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::chat_bridge::{ChatBridge, ChatSinkError, TurnOpening};

use super::context::{ChatContext, SharedChatContextProvider};
use super::error::StreamError;
use super::state::ActiveStreamTokens;
use super::thread::thread_manager;

fn active_stream_tokens(
    app_handle: &AppHandle,
) -> Result<tauri::State<'_, ActiveStreamTokens>, StreamError> {
    app_handle
        .try_state::<ActiveStreamTokens>()
        .ok_or(StreamError::StateUnavailable("active stream tokens"))
}

fn context_provider(
    app_handle: &AppHandle,
) -> Result<tauri::State<'_, SharedChatContextProvider>, StreamError> {
    app_handle
        .try_state::<SharedChatContextProvider>()
        .ok_or(StreamError::StateUnavailable("chat context provider"))
}

fn tool_backend(
    app_handle: &AppHandle,
) -> Result<tauri::State<'_, Arc<dyn ToolBackend>>, StreamError> {
    app_handle
        .try_state::<Arc<dyn ToolBackend>>()
        .ok_or(StreamError::StateUnavailable("tool backend"))
}

#[tauri::command]
#[specta::specta]
pub async fn chat_collect_context(
    app_handle: AppHandle,
    thread_id: Uuid,
) -> Result<ChatContext, StreamError> {
    let provider = context_provider(&app_handle)?;
    provider.collect(thread_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn chat_send_query(
    app_handle: AppHandle,
    thread_id: Uuid,
    channel: Channel<ChatServerMessage>,
    request: ChatSendRequest,
) -> Result<(), StreamError> {
    run_turn_command(&app_handle, thread_id, channel, TurnOpening::Send(request)).await
}

#[tauri::command]
#[specta::specta]
pub async fn chat_regenerate(
    app_handle: AppHandle,
    thread_id: Uuid,
    ai_message_id: Uuid,
    channel: Channel<ChatServerMessage>,
) -> Result<(), StreamError> {
    run_turn_command(
        &app_handle,
        thread_id,
        channel,
        TurnOpening::Regenerate(RegenerateRequest { ai_message_id }),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn chat_cancel_query(app_handle: AppHandle, thread_id: Uuid) -> Result<(), StreamError> {
    let tokens_state = active_stream_tokens(&app_handle)?;

    if let Some(token) = tokens_state.lock().await.remove(&thread_id) {
        token.cancel();
        tracing::debug!("Cancelled stream for thread {thread_id}");
    }

    Ok(())
}

/// Drive one chat turn end-to-end:
///
/// 1. Allocate a per-thread cancel token and register it so
///    `chat_cancel_query` can fire it.
/// 2. Open the chat WebSocket.
/// 3. Hand the socket to a fresh [`ChatBridge`] that queries the active
///    [`ToolBackend`] for the per-turn tool surface, sends the opening
///    frame, and forwards inbound chat frames to `channel` while routing
///    tool calls back through the same backend.
/// 4. Tear down the per-thread cancel token whether the turn finished,
///    timed out, or errored.
async fn run_turn_command(
    app_handle: &AppHandle,
    thread_id: Uuid,
    channel: Channel<ChatServerMessage>,
    opening: TurnOpening,
) -> Result<(), StreamError> {
    let thread_manager = thread_manager(app_handle, StreamError::StateUnavailable)?;
    let tokens_state = active_stream_tokens(app_handle)?;
    let backend = tool_backend(app_handle)?;

    let cancel = CancellationToken::new();
    tokens_state.lock().await.insert(thread_id, cancel.clone());

    let result = open_and_drive(
        thread_manager.inner().clone(),
        backend.inner().clone(),
        thread_id,
        channel,
        opening,
        cancel.clone(),
    )
    .await;

    tokens_state.lock().await.remove(&thread_id);
    result
}

async fn open_and_drive(
    thread_manager: super::state::SharedThreadManager,
    backend: Arc<dyn ToolBackend>,
    thread_id: Uuid,
    channel: Channel<ChatServerMessage>,
    opening: TurnOpening,
    cancel: CancellationToken,
) -> Result<(), StreamError> {
    let socket = thread_manager
        .open_chat_socket(thread_id, cancel.clone())
        .await?;
    let bridge = ChatBridge::new(backend);

    // `Channel<T>` is internally an `Arc`, so each `send` is independent
    // and the wrapping closure only needs `Fn` — no mut state.
    let sink = move |event: ChatServerMessage| {
        channel
            .send(event)
            .map_err(|e| ChatSinkError(e.to_string()))
    };

    let drive_future = bridge.run_turn(socket, opening, cancel.clone(), &sink);

    match tokio::time::timeout(CHAT_STREAM_TIMEOUT, drive_future).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(err)) => {
            cancel.cancel();
            Err(err.into())
        }
        Err(_) => {
            cancel.cancel();
            Err(StreamError::Timeout(CHAT_STREAM_TIMEOUT.as_secs() as u32))
        }
    }
}
