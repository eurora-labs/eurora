use tauri::{AppHandle, Manager, ipc::Channel};
use thread_core::{ChatSendRequest, ChatServerMessage};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::context::{ChatContext, SharedChatContextProvider};
use super::error::StreamError;
use super::state::ActiveStreamTokens;
use super::thread::thread_manager;

const STREAM_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

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
    let thread_state = thread_manager(&app_handle, StreamError::StateUnavailable)?;
    let tokens_state = active_stream_tokens(&app_handle)?;

    let cancel = CancellationToken::new();
    tokens_state.lock().await.insert(thread_id, cancel.clone());

    let stream_result = forward_chat_stream(
        thread_state
            .chat_stream(thread_id, request, cancel.clone())
            .await,
        cancel.clone(),
        &channel,
    )
    .await;

    tokens_state.lock().await.remove(&thread_id);
    stream_result
}

#[tauri::command]
#[specta::specta]
pub async fn chat_regenerate(
    app_handle: AppHandle,
    thread_id: Uuid,
    ai_message_id: Uuid,
    channel: Channel<ChatServerMessage>,
) -> Result<(), StreamError> {
    let thread_state = thread_manager(&app_handle, StreamError::StateUnavailable)?;
    let tokens_state = active_stream_tokens(&app_handle)?;

    let cancel = CancellationToken::new();
    tokens_state.lock().await.insert(thread_id, cancel.clone());

    let stream_result = forward_chat_stream(
        thread_state
            .chat_regenerate(thread_id, ai_message_id, cancel.clone())
            .await,
        cancel.clone(),
        &channel,
    )
    .await;

    tokens_state.lock().await.remove(&thread_id);
    stream_result
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

async fn forward_chat_stream(
    open_result: crate::Result<mpsc::UnboundedReceiver<crate::Result<ChatServerMessage>>>,
    cancel: CancellationToken,
    channel: &Channel<ChatServerMessage>,
) -> Result<(), StreamError> {
    let mut rx = open_result?;

    let stream_future = async {
        while let Some(item) = rx.recv().await {
            let event = item?;
            let is_terminal = matches!(
                &event,
                ChatServerMessage::Final { .. } | ChatServerMessage::Error { .. }
            );
            channel
                .send(event)
                .map_err(|e| StreamError::Channel(e.to_string()))?;
            if is_terminal {
                return Ok(());
            }
        }
        Ok(())
    };

    match tokio::time::timeout(STREAM_TIMEOUT, stream_future).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => {
            cancel.cancel();
            Err(e)
        }
        Err(_) => {
            cancel.cancel();
            Err(StreamError::Timeout(STREAM_TIMEOUT.as_secs() as u32))
        }
    }
}
