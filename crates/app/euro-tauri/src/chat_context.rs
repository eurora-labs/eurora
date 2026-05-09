//! Desktop implementation of [`euro_thread::commands::ChatContextProvider`].
//!
//! Pulls per-turn chat context from the timeline: refreshes the active
//! activity (best-effort), persists any new asset/snapshot to the
//! activity service, and surfaces the most recent asset/snapshot
//! content blocks plus a single context chip for the current activity.
//!
//! Mobile has its own provider (currently `NoopChatContextProvider`)
//! and will eventually source from native picker state instead.

use async_trait::async_trait;
use euro_thread::commands::{ChatContext, ChatContextProvider, StreamError};
use euro_timeline::TimelineManager;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct TimelineChatContextProvider {
    app_handle: AppHandle,
}

impl TimelineChatContextProvider {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

#[async_trait]
impl ChatContextProvider for TimelineChatContextProvider {
    async fn collect(&self, _thread_id: Uuid) -> Result<ChatContext, StreamError> {
        let timeline_state = self
            .app_handle
            .try_state::<Mutex<TimelineManager>>()
            .ok_or(StreamError::StateUnavailable("timeline"))?;

        let timeline = timeline_state.lock().await;

        // Refreshing the activity is best-effort: a missing tab or stale
        // browser bridge shouldn't abort the chat turn — we just contribute
        // no fresh context for it.
        if let Err(e) = timeline.refresh_current_activity().await {
            tracing::debug!("collect_context: refresh failed: {e}");
        }
        if timeline.save_current_activity_to_service().await.is_err() {
            return Ok(ChatContext::default());
        }

        let asset_blocks = timeline.construct_messages_from_last_asset().await;
        let snapshot_blocks = timeline.construct_messages_from_last_snapshot().await;

        let mut content_blocks = Vec::with_capacity(asset_blocks.len() + snapshot_blocks.len());
        content_blocks.extend(asset_blocks.into_inner());
        content_blocks.extend(snapshot_blocks.into_inner());

        let asset_chips = timeline
            .get_context_chip()
            .await
            .map(|chip| vec![chip])
            .unwrap_or_default();

        Ok(ChatContext {
            content_blocks,
            asset_chips,
        })
    }
}
