//! Desktop client for the Eurora thread service (HTTP + WebSocket).
//!
//! Wire types come from [`thread_core`]; this crate is a thin reqwest /
//! tokio-tungstenite adapter that handles auth, base-URL resolution, and
//! transport-level error mapping. Tauri procedures consume [`ThreadManager`]
//! as a dependency-injected service.

mod error;
mod manager;

#[cfg(feature = "tauri")]
pub mod commands;

pub use error::{Error, Result};
pub use manager::ThreadManager;
pub use thread_core::{
    ChatClientMessage, ChatSendRequest, ChatServerMessage, MessageNode, SearchMessageResult,
    SearchMessagesResponse, SearchThreadResult, SearchThreadsResponse, Thread,
};
