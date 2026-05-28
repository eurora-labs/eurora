use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use specta::Type;
use thread_core::ContextChip;
use uuid::Uuid;

use super::error::StreamError;

/// Per-turn host metadata returned by `chat_collect_context`.
///
/// Only the UI-facing chip set lives here. The LLM-facing prelude that
/// describes the user's current activity is delivered separately, via
/// the `system_blocks` field on `CapabilityUpdatePayload`, and is
/// pulled by the chat bridge from the [`thread_core::ToolBackend`] at
/// turn start — no round trip through the UI is required for that.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChatContext {
    pub asset_chips: Vec<ContextChip>,
}

/// Platform-specific source of per-turn chat context.
///
/// Desktop populates this from the timeline (the active activity's
/// chip). Mobile populates it from per-thread state seeded by native
/// picker commands. The IPC layer just forwards whatever the impl
/// returns into the chat-prep flow.
#[async_trait]
pub trait ChatContextProvider: Send + Sync + 'static {
    async fn collect(&self, thread_id: Uuid) -> Result<ChatContext, StreamError>;
}

/// Tauri-state alias for a registered [`ChatContextProvider`].
pub type SharedChatContextProvider = Arc<dyn ChatContextProvider>;

/// Provider that contributes no context. Useful for platforms (or test
/// harnesses) that don't have a richer source wired up yet — the chat
/// turn proceeds without any chips.
pub struct NoopChatContextProvider;

#[async_trait]
impl ChatContextProvider for NoopChatContextProvider {
    async fn collect(&self, _thread_id: Uuid) -> Result<ChatContext, StreamError> {
        Ok(ChatContext::default())
    }
}
