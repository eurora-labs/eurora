use agent_chain_core::messages::{ContentBlock, ContentBlocks, TextContentBlock};
use euro_activity::types::ContextChip;
use euro_timeline::TimelineManager;
use tauri::{Manager, Runtime, ipc::Channel};
use thread_core::{ChatSendRequest, ChatServerMessage};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::error::AppError;
use crate::shared_types::{ActiveStreamTokens, SharedThreadManager};

#[taurpc::ipc_type]
pub struct Query {
    text: String,
    assets: Vec<String>,
    parent_message_id: Option<String>,
    preserved_asset_chips: Option<Vec<ContextChip>>,
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
        let timeline_state: tauri::State<Mutex<TimelineManager>> = app_handle
            .try_state()
            .ok_or(AppError::Unavailable("Timeline"))?;
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

        tracing::debug!(
            "send_query: assets={:?}, preserved_chips={}",
            query.assets,
            query
                .preserved_asset_chips
                .as_ref()
                .map(|c| c.len())
                .unwrap_or(0)
        );

        let is_edit = query.preserved_asset_chips.is_some();

        let (chip, asset_blocks, snapshot_blocks) = if is_edit {
            (None, ContentBlocks::new(), ContentBlocks::new())
        } else {
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

        let asset_chips_json: Option<String> = if let Some(chips) = &query.preserved_asset_chips {
            if chips.is_empty() {
                None
            } else {
                serde_json::to_string(chips).ok()
            }
        } else if let Some(chip) = chip {
            serde_json::to_string(&[chip]).ok()
        } else {
            None
        };

        let mut context_blocks = ContentBlocks::new();

        let mut all_blocks = ContentBlocks::new();
        all_blocks.extend(asset_blocks.into_inner());
        all_blocks.extend(snapshot_blocks.into_inner());

        if !all_blocks.is_empty() {
            match thread_state
                .save_preliminary_content_blocks(thread_uuid, all_blocks.into_inner())
                .await
            {
                Ok(returned) => {
                    context_blocks = ContentBlocks::from(returned);
                }
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

        let request = ChatSendRequest {
            content_blocks: context_blocks.into_inner(),
            parent_message_id: parent_message_uuid,
            asset_chips_json,
        };

        tracing::debug!("Opening chat WebSocket");
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
