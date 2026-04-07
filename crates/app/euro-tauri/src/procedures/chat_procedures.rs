use agent_chain_core::messages::{ContentBlock, ContentBlocks, TextContentBlock};
use agent_chain_core::proto::ChatStreamResponse;
use euro_timeline::TimelineManager;
use futures::StreamExt;
use tauri::{Manager, Runtime, ipc::Channel};
use tokio::sync::Mutex;
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
        let timeline_state: tauri::State<Mutex<TimelineManager>> = app_handle
            .try_state()
            .ok_or(AppError::Unavailable("Timeline"))?;
        let tokens_state: tauri::State<ActiveStreamTokens> = app_handle
            .try_state()
            .ok_or(AppError::Unavailable("Active stream tokens"))?;

        tracing::debug!("send_query: assets={:?}", query.assets);

        let (chip, asset_blocks, snapshot_blocks) = {
            let timeline = timeline_state.lock().await;
            let _ = timeline.refresh_current_activity().await;

            let save_ok = timeline.save_current_activity_to_service().await.is_ok();
            tracing::debug!(
                "send_query: save_ok={save_ok}, assets_empty={}",
                query.assets.is_empty()
            );

            if save_ok && !query.assets.is_empty() {
                let chip = timeline.get_context_chip().await;
                let asset_blocks = timeline.construct_messages_from_last_asset().await;
                let snapshot_blocks = timeline.construct_messages_from_last_snapshot().await;
                tracing::debug!(
                    "send_query: chip={:?}, asset_blocks={}, snapshot_blocks={}",
                    chip.is_some(),
                    asset_blocks.len(),
                    snapshot_blocks.len()
                );
                (chip, asset_blocks, snapshot_blocks)
            } else {
                (None, ContentBlocks::new(), ContentBlocks::new())
            }
        };

        let mut asset_chips_json: Option<String> = None;
        let mut context_blocks = ContentBlocks::new();

        if let Some(chip) = chip {
            asset_chips_json = serde_json::to_string(&[chip]).ok();
        }

        let mut all_blocks = ContentBlocks::new();
        all_blocks.extend(asset_blocks.into_inner());
        all_blocks.extend(snapshot_blocks.into_inner());

        if !all_blocks.is_empty() {
            let mut thread_manager = thread_state.lock().await;
            match thread_manager
                .save_preliminary_content_blocks(thread_id.clone(), all_blocks)
                .await
            {
                Ok(returned) => context_blocks = returned,
                Err(e) => tracing::warn!("Failed to save preliminary blocks: {e}"),
            }
        }

        let user_text_block: ContentBlock =
            TextContentBlock::builder().text(&query.text).build().into();
        context_blocks.push(user_text_block);

        let cancel = CancellationToken::new();
        tokens_state
            .lock()
            .await
            .insert(thread_id.clone(), cancel.clone());

        tracing::debug!("Sending chat stream");
        let mut stream = {
            let mut thread_manager = thread_state.lock().await;
            thread_manager
                .chat_stream(
                    thread_id.clone(),
                    context_blocks,
                    query.parent_message_id.clone(),
                    asset_chips_json,
                    cancel.clone(),
                )
                .await
                .map_err(|e| format!("Failed to create chat stream: {e}"))?
        };

        tracing::debug!("Starting to consume stream...");

        let stream_future = async {
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
            Ok(Ok(())) => {
                tracing::debug!("Stream completed successfully");
                Ok(())
            }
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
            tracing::debug!("Cancelled stream for thread {thread_id}");
        }

        Ok(())
    }
}
