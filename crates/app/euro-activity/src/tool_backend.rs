//! Bridge from the active [`ActivityStrategy`] to [`ToolBackend`].
//!
//! `euro-thread`'s `ChatBridge` consumes an `Arc<dyn ToolBackend>` so it
//! has no knowledge of activity strategies. The wrapper here is the thin
//! adapter that grabs a read guard on the shared strategy and forwards
//! each call.

use std::sync::Arc;

use agent_chain_core::messages::ContentBlock;
use async_trait::async_trait;
use serde_json::Value;
use thread_core::{ToolBackend, ToolBackendCall, ToolErrorWire, WireToolDescriptor};
use tokio::sync::RwLock;

use crate::ActivityStrategy;
use crate::strategies::ActivityStrategyFunctionality;

/// `ToolBackend` implementation that delegates to whichever
/// [`ActivityStrategy`] is currently active.
///
/// Holds an `Arc` to the same `RwLock` `CollectorService` swaps on focus
/// changes, so the chat side always sees the freshest strategy without
/// any reconnection. Reads only — strategies expose `get_tools` /
/// `dispatch_tool` as `&self` so tool dispatch never blocks focus
/// updates.
pub struct ActivityToolBackend {
    strategy: Arc<RwLock<ActivityStrategy>>,
}

impl ActivityToolBackend {
    pub fn new(strategy: Arc<RwLock<ActivityStrategy>>) -> Self {
        Self { strategy }
    }
}

#[async_trait]
impl ToolBackend for ActivityToolBackend {
    async fn list_tools(&self) -> Vec<WireToolDescriptor> {
        match self.strategy.read().await.get_tools().await {
            Ok(tools) => tools,
            Err(err) => {
                tracing::warn!("strategy get_tools failed: {err}");
                Vec::new()
            }
        }
    }

    async fn collect_system_blocks(&self) -> Vec<ContentBlock> {
        match self.strategy.read().await.get_context().await {
            Ok(blocks) => blocks.into_inner(),
            Err(err) => {
                tracing::debug!("strategy get_context failed: {err}");
                Vec::new()
            }
        }
    }

    async fn dispatch(&self, call: ToolBackendCall) -> Result<Value, ToolErrorWire> {
        self.strategy.read().await.dispatch_tool(call).await
    }
}
