//! Client-side abstraction for the active tool surface.
//!
//! `ChatBridge` does not know about activity strategies, browsers, or the
//! native-messaging bridge: it talks to whatever implements [`ToolBackend`].
//! The desktop wires it to an activity-strategy wrapper; tests stub it
//! directly.

use async_trait::async_trait;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::tool_wire::{ToolErrorWire, WireToolDescriptor};

/// One inbound tool call routed from the LLM through `ChatBridge` to a
/// backend. The bridge correlates `call_id` to the `ToolResponse` it
/// emits; `cancel` is a child of the per-call cancellation token so the
/// backend can race its work against `ToolCancel { call_id }` or a
/// turn-level cancel.
#[derive(Debug)]
pub struct ToolBackendCall {
    pub call_id: u32,
    pub name: String,
    pub arguments: Value,
    pub cancel: CancellationToken,
}

/// The thing `ChatBridge` queries for the per-turn tool surface and to
/// which it routes every inbound `ToolRequest` frame.
///
/// Implementations are `Send + Sync` because the bridge shares them as
/// `Arc<dyn ToolBackend>` across the dispatch tasks it spawns.
#[async_trait]
pub trait ToolBackend: Send + Sync {
    /// Snapshot of every tool the LLM should see for the upcoming turn.
    /// Called once at turn start; the bridge advertises the result via
    /// `CapabilityUpdate`.
    async fn list_tools(&self) -> Vec<WireToolDescriptor>;

    /// Execute one tool call and return the structured result. Errors
    /// land in the `ToolResponse` frame's `Err` arm verbatim.
    async fn dispatch(&self, call: ToolBackendCall) -> Result<Value, ToolErrorWire>;
}
