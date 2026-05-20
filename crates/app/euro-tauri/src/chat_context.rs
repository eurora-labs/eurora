//! Desktop implementation of [`euro_thread::commands::ChatContextProvider`].
//!
//! Surfaces a single [`ContextChip`] describing the current activity so the
//! UI can render it; the LLM receives the active-context summary through
//! the system message built by `be-thread-service::tool_catalog::build_context_system_message`
//! and pulls page contents through granular tool calls. The desktop no
//! longer speculatively bundles asset/snapshot blocks into every turn.
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

        Ok(ChatContext {
            content_blocks: Vec::new(),
            asset_chips,
        })
    }
}
