use std::sync::Arc;

use agent_chain_core::messages::ContentBlock;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use specta::Type;
use thread_core::ContextChip;
use uuid::Uuid;

use super::error::StreamError;

/// Per-turn host context returned by `chat_collect_context`.
///
/// `content_blocks` are inlined directly — large payloads are rewritten
/// into asset references server-side at chat-turn time, so the wire
/// format here can carry raw bytes/text without the client having to
/// round-trip them.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChatContext {
    pub content_blocks: Vec<ContentBlock>,
    pub asset_chips: Vec<ContextChip>,
}

/// Platform-specific source of per-turn chat context.
///
/// Desktop populates this from the timeline (active app, recent
/// browser snapshot, last-saved asset). Mobile populates it from
/// per-thread state seeded by native picker commands. The IPC layer
/// just forwards whatever the impl returns into the chat turn.
#[async_trait]
pub trait ChatContextProvider: Send + Sync + 'static {
    async fn collect(&self, thread_id: Uuid) -> Result<ChatContext, StreamError>;
}

/// Tauri-state alias for a registered [`ChatContextProvider`].
pub type SharedChatContextProvider = Arc<dyn ChatContextProvider>;

/// Provider that contributes no context. Useful for platforms (or test
/// harnesses) that don't have a richer source wired up yet — the chat
/// turn proceeds with whatever blocks the client passed in directly.
pub struct NoopChatContextProvider;

#[async_trait]
impl ChatContextProvider for NoopChatContextProvider {
    async fn collect(&self, _thread_id: Uuid) -> Result<ChatContext, StreamError> {
        Ok(ChatContext::default())
    }
}
