//! Native-messaging host that bridges a browser extension's stdio
//! [native-messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging)
//! channel to the desktop's WebSocket app bridge.
//!
//! The Chrome side speaks the standard length-prefixed JSON protocol on
//! stdio; the desktop side speaks JSON-encoded [`Frame`]s over a
//! WebSocket. This crate is the connective tissue.

pub mod bridge_client;
pub mod parent_pid;
pub mod types;
pub mod utils;

pub use bridge_client::{BridgeClient, BridgeReader, BridgeWriter};
pub use euro_bridge_protocol::Frame;
pub use types::*;

/// Maximum size of a Chrome native-messaging frame on stdio (1 GiB). Chrome
/// caps stdio frames at 1 MB inbound and 4 GB outbound; we pick a generous
/// limit that comfortably exceeds anything the extension actually sends.
pub const MAX_FRAME_SIZE: usize = 1024 * 1024 * 1024;

/// Loopback WebSocket URL of the desktop's app bridge.
pub const BRIDGE_URL: &str = "ws://[::1]:1431/";
