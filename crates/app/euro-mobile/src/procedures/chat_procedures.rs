use agent_chain_core::messages::{ContentBlock, ContentBlocks, TextContentBlock};
use agent_chain_core::proto::ChatStreamResponse;
use futures::StreamExt;
use tauri::{Manager, Runtime, ipc::Channel};
use tokio_util::sync::CancellationToken;

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
        channel: Channel<ChatStreamResponse>,
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
        channel: Channel<ChatStreamResponse>,
        query: Query,
    ) -> Result<(), String> {
        let thread_state: tauri::State<SharedThreadManager> = app_handle
            .try_state()
            .ok_or(AppError::Unavailable("Thread manager"))?;
        let tokens_state: tauri::State<ActiveStreamTokens> = app_handle
            .try_state()
            .ok_or(AppError::Unavailable("Active stream tokens"))?;

        // For mobile, we skip the timeline/asset management that the desktop does.
        // Just build content blocks from the user's text.
        let user_text_block: ContentBlock =
            TextContentBlock::builder().text(&query.text).build().into();
        let mut context_blocks = ContentBlocks::new();
        context_blocks.push(user_text_block);

        let cancel = CancellationToken::new();
        tokens_state
            .lock()
            .await
            .insert(thread_id.clone(), cancel.clone());

        let mut stream = {
            let mut thread_manager = thread_state.lock().await;
            thread_manager
                .chat_stream(
                    thread_id.clone(),
                    context_blocks,
                    query.parent_message_id.clone(),
                    None,
                    cancel.clone(),
                )
                .await
                .map_err(|e| format!("Failed to create chat stream: {e}"))?
        };

        let stream_future = async {
            match stream.next().await {
                Some(Ok(first)) => {
                    if let Err(e) = channel.send(first) {
                        return Err(format!("Failed to send confirmed human message: {e}"));
                    }
                }
                Some(Err(e)) => return Err(format!("Stream error: {e}")),
                None => return Ok(()),
            }

            loop {
                tokio::select! {
                    biased;
                    () = cancel.cancelled() => {
                        tracing::debug!("Stream cancelled for thread {thread_id}");
                        drop(stream);
                        return Ok(());
                    }
                    item = stream.next() => {
                        match item {
                            Some(Ok(response)) => {
                                if let Err(e) = channel.send(response) {
                                    return Err(format!("Failed to send response chunk: {e}"));
                                }
                            }
                            Some(Err(e)) => {
                                return Err(format!("Stream error: {e}"));
                            }
                            None => return Ok(()),
                        }
                    }
                }
            }
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
