use tauri::{Manager, Runtime, ipc::Channel};
use thread_core::{ChatSendRequest, ChatServerMessage};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::error::AppError;
use crate::shared_types::{ActiveStreamTokens, SharedThreadManager};

#[taurpc::procedures(path = "chat")]
pub trait ChatApi {
    async fn send_query<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        channel: Channel<ChatServerMessage>,
        request: ChatSendRequest,
    ) -> Result<(), String>;

    async fn regenerate<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        ai_message_id: String,
        channel: Channel<ChatServerMessage>,
    ) -> Result<(), String>;

    async fn cancel_query<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
    ) -> Result<(), String>;
}

#[derive(Clone)]
pub struct ChatApiImpl;

const STREAM_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

#[taurpc::resolvers]
impl ChatApi for ChatApiImpl {
    async fn send_query<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        channel: Channel<ChatServerMessage>,
        request: ChatSendRequest,
    ) -> Result<(), String> {
        let thread_state: tauri::State<SharedThreadManager> = app_handle
            .try_state()
            .ok_or(AppError::Unavailable("Thread manager"))?;
        let tokens_state: tauri::State<ActiveStreamTokens> = app_handle
            .try_state()
            .ok_or(AppError::Unavailable("Active stream tokens"))?;

        let thread_uuid =
            Uuid::parse_str(&thread_id).map_err(|e| format!("Invalid thread_id: {e}"))?;

        let cancel = CancellationToken::new();
        tokens_state
            .lock()
            .await
            .insert(thread_id.clone(), cancel.clone());

        let stream_result = forward_chat_stream(
            thread_state
                .chat_stream(thread_uuid, request, cancel.clone())
                .await,
            cancel.clone(),
            &channel,
        )
        .await;

        tokens_state.lock().await.remove(&thread_id);
        stream_result
    }

    async fn regenerate<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        ai_message_id: String,
        channel: Channel<ChatServerMessage>,
    ) -> Result<(), String> {
        let thread_state: tauri::State<SharedThreadManager> = app_handle
            .try_state()
            .ok_or(AppError::Unavailable("Thread manager"))?;
        let tokens_state: tauri::State<ActiveStreamTokens> = app_handle
            .try_state()
            .ok_or(AppError::Unavailable("Active stream tokens"))?;

        let thread_uuid =
            Uuid::parse_str(&thread_id).map_err(|e| format!("Invalid thread_id: {e}"))?;
        let ai_message_uuid =
            Uuid::parse_str(&ai_message_id).map_err(|e| format!("Invalid ai_message_id: {e}"))?;

        let cancel = CancellationToken::new();
        tokens_state
            .lock()
            .await
            .insert(thread_id.clone(), cancel.clone());

        let stream_result = forward_chat_stream(
            thread_state
                .chat_regenerate(thread_uuid, ai_message_uuid, cancel.clone())
                .await,
            cancel.clone(),
            &channel,
        )
        .await;

        tokens_state.lock().await.remove(&thread_id);
        stream_result
    }

    async fn cancel_query<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
    ) -> Result<(), String> {
        let tokens_state: tauri::State<ActiveStreamTokens> = app_handle
            .try_state()
            .ok_or(AppError::Unavailable("Active stream tokens"))?;

        if let Some(token) = tokens_state.lock().await.remove(&thread_id) {
            token.cancel();
        }

        Ok(())
    }
}

async fn forward_chat_stream(
    open_result: euro_thread::Result<
        mpsc::UnboundedReceiver<euro_thread::Result<ChatServerMessage>>,
    >,
    cancel: CancellationToken,
    channel: &Channel<ChatServerMessage>,
) -> Result<(), String> {
    let mut rx = open_result.map_err(|e| format!("Failed to open chat stream: {e}"))?;

    let stream_future = async {
        while let Some(item) = rx.recv().await {
            match item {
                Ok(event) => {
                    let is_terminal = matches!(
                        &event,
                        ChatServerMessage::Final { .. } | ChatServerMessage::Error { .. }
                    );
                    if let Err(e) = channel.send(event) {
                        return Err(format!("Failed to forward chat event: {e}"));
                    }
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
