//! Generic web-page adapter for Eurora's unified tool-execution
//! architecture.
//!
//! This crate declares the [`WebAdapter`] trait — the client-side
//! contract for tools that operate on whichever `http(s)` tab the user
//! has focused — together with the argument and return types every
//! method exchanges with the agent loop.
//!
//! The `#[adapter]` macro in [`adapter`](self::adapter) generates a
//! static [`WEB_DESCRIPTORS`] table and a [`WebDispatcher<T>`] around
//! any user-supplied [`WebAdapter`] implementation. The server never
//! instantiates the trait; it consumes `WEB_DESCRIPTORS` (via
//! [`ToolDescriptor::to_wire`](eurora_tools::ToolDescriptor::to_wire))
//! and routes calls through [`eurora_tools::RemoteToolBus`]. The
//! bridge-backed `WebBridgeImpl` lands in Phase 12 behind the `bridge`
//! cargo feature.

mod adapter;
#[cfg(feature = "bridge")]
mod bridge;
mod types;

pub use adapter::{WEB_DESCRIPTORS, WebAdapter, WebAdapterLocal, WebDispatcher};
pub use types::*;
