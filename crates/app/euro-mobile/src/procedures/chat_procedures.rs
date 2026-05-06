use agent_chain_core::messages::{ContentBlock, ContentBlocks, TextContentBlock};
use tauri::{Manager, Runtime, ipc::Channel};
use thread_core::{ChatSendRequest, ChatServerMessage};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::error::AppError;
use crate::shared_types::{ActiveStreamTokens, SharedThreadManager};

#[taurpc::ipc_type]
pub struct Query {
    text: String,
    assets: Vec<String>,
    parent_message_id: Option<String>,
}

#[taurpc::procedures(path = "chat")]
pub trait ChatApi {
    async fn send_query<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        channel: Channel<ChatServerMessage>,
        query: Query,
    ) -> Result<(), String>;

    async fn cancel_query<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
    ) -> Result<(), String>;
}

#[derive(Clone)]
pub struct ChatApiImpl;

#[taurpc::resolvers]
impl ChatApi for ChatApiImpl {
    async fn send_query<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        channel: Channel<ChatServerMessage>,
        query: Query,
    ) -> Result<(), String> {
        let thread_state: tauri::State<SharedThreadManager> = app_handle
            .try_state()
            .ok_or(AppError::Unavailable("Thread manager"))?;
        let tokens_state: tauri::State<ActiveStreamTokens> = app_handle
            .try_state()
            .ok_or(AppError::Unavailable("Active stream tokens"))?;

        let thread_uuid =
            Uuid::parse_str(&thread_id).map_err(|e| format!("Invalid thread_id: {e}"))?;
        let parent_message_uuid = match query.parent_message_id.as_deref() {
            Some(s) if !s.is_empty() => {
                Some(Uuid::parse_str(s).map_err(|e| format!("Invalid parent_message_id: {e}"))?)
            }
            _ => None,
        };

        // Mobile skips the timeline/asset management that the desktop does;
        // just wrap the user's text as a single content block.
        let user_text_block: ContentBlock =
            TextContentBlock::builder().text(&query.text).build().into();
        let mut context_blocks = ContentBlocks::new();
        context_blocks.push(user_text_block);

        let cancel = CancellationToken::new();
        tokens_state
            .lock()
            .await
            .insert(thread_id.clone(), cancel.clone());

        let request = ChatSendRequest {
            content_blocks: context_blocks.into_inner(),
            parent_message_id: parent_message_uuid,
            asset_chips_json: None,
        };

        let mut rx = thread_state
            .chat_stream(thread_uuid, request, cancel.clone())
            .await
            .map_err(|e| format!("Failed to open chat stream: {e}"))?;

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

        let timeout = std::time::Duration::from_secs(300);
        let result = match tokio::time::timeout(timeout, stream_future).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => {
                cancel.cancel();
                Err(e)
            }
            Err(_) => {
                cancel.cancel();
                Err("Stream processing timed out after 5 minutes".to_string())
            }
        };

        tokens_state.lock().await.remove(&thread_id);
        result
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
