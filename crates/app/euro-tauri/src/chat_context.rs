//! Desktop implementation of [`euro_thread::commands::ChatContextProvider`].
//!
//! Returns the single [`ContextChip`] describing the user's current
//! activity, used by the UI to render a chip alongside the in-flight
//! human message and to persist the chip set on `ChatSendRequest.asset_chips_json`.
//!
//! The LLM-facing prelude (`"The user is currently watching ..."`) is
//! delivered separately — it ships in the `system_blocks` field of the
//! chat bridge's `CapabilityUpdate` frame, pulled directly from the
//! active activity strategy via the `ToolBackend::collect_system_blocks`
//! hook in `euro-activity`. No round trip through the UI is required.
//!
//! Activity rows themselves are pushed to the remote service by the
//! collector at creation time — there is no duplicate upload here.
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

        let asset_chips = timeline
            .get_context_chip()
            .await
            .map(|chip| vec![chip])
            .unwrap_or_default();

        Ok(ChatContext { asset_chips })
    }
}
