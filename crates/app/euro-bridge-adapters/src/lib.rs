//! App-side bridge-backed implementations of Eurora's tool adapters.
//!
//! Every adapter declared in
//! [`eurora_tools_browser`](::eurora_tools_browser) has a trait
//! implementation here that translates a typed adapter method into a
//! [`euro_bridge::BridgeService::send_request`] round trip. The
//! transport details (payload framing, response decoding, error
//! mapping, action constants) live in this crate — never in the common
//! tool crates — so the dependency graph stays strictly common → app.
//!
//! # Layout
//!
//! - [`client`] — [`BridgeClient`], the one place every adapter funnels
//!   `send_request` through. Handles payload framing, response decoding,
//!   and `BridgeError → ToolError` mapping so each adapter impl is a
//!   thin trait-method-to-action mapping.
//! - [`browser`] — bridge-backed implementations of the
//!   [`eurora_tools_browser`] adapter traits plus the bridge action
//!   constants the browser extension routes on.
//!
//! New transports follow the same shape: a `*Client` for shared
//! transport plumbing and per-namespace submodules holding the trait
//! impls and action constants.

pub mod browser;
pub mod client;

pub use client::BridgeClient;
