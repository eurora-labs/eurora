use tauri::{AppHandle, Manager, ipc::Channel};
use thread_core::{ChatSendRequest, ChatServerMessage};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::error::ResultExt;
use crate::shared_types::{ActiveStreamTokens, SharedThreadManager};

const STREAM_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

fn thread_manager(app_handle: &AppHandle) -> Result<tauri::State<'_, SharedThreadManager>, String> {
    app_handle
        .try_state::<SharedThreadManager>()
        .ok_or_else(|| "Thread manager not available".to_string())
}

fn active_stream_tokens(
    app_handle: &AppHandle,
) -> Result<tauri::State<'_, ActiveStreamTokens>, String> {
    app_handle
        .try_state::<ActiveStreamTokens>()
        .ok_or_else(|| "Active stream tokens not available".to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn chat_send_query(
    app_handle: AppHandle,
    thread_id: Uuid,
    channel: Channel<ChatServerMessage>,
    request: ChatSendRequest,
) -> Result<(), String> {
    let thread_state = thread_manager(&app_handle)?;
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
) -> Result<(), String> {
    let thread_state = thread_manager(&app_handle)?;
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
pub async fn chat_cancel_query(app_handle: AppHandle, thread_id: Uuid) -> Result<(), String> {
    let tokens_state = active_stream_tokens(&app_handle)?;

    if let Some(token) = tokens_state.lock().await.remove(&thread_id) {
        token.cancel();
        tracing::debug!("Cancelled stream for thread {thread_id}");
    }

    Ok(())
}

async fn forward_chat_stream(
    open_result: euro_thread::Result<
        mpsc::UnboundedReceiver<euro_thread::Result<ChatServerMessage>>,
    >,
    cancel: CancellationToken,
    channel: &Channel<ChatServerMessage>,
) -> Result<(), String> {
    let mut rx = open_result.ctx("Failed to open chat stream")?;

    let stream_future = async {
        while let Some(item) = rx.recv().await {
            match item {
                Ok(event) => {
                    let is_terminal = matches!(
                        &event,
                        ChatServerMessage::Final { .. } | ChatServerMessage::Error { .. }
                    );
                    channel
                        .send(event)
                        .map_err(|e| format!("Failed to forward chat event: {e}"))?;
                    if is_terminal {
                        return Ok(());
                    }
                }
                Err(euro_thread::Error::Cancelled) => return Ok(()),
                Err(e) => return Err(format!("Stream error: {e}")),
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
            Err(format!(
                "Stream processing timed out after {} seconds",
                STREAM_TIMEOUT.as_secs()
            ))
        }
    }
}
