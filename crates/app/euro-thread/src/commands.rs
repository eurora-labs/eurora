//! Tauri-specta IPC surface for thread CRUD/search and the chat stream.
//!
//! Both `euro-tauri` (desktop) and `euro-mobile` register this surface into
//! their tauri-specta builders so the JS-side bindings expose one canonical
//! `thread_*` / `chat_*` API regardless of platform. Per-platform divergence
//! (timeline-driven context on desktop, native-picker assets on mobile)
//! lives behind the [`ChatContextProvider`] trait — each app installs its
//! own implementation as Tauri state at startup.
//!
//! Only compiled when the `tauri` cargo feature is on so the rest of
//! `euro-thread` (a plain HTTP/WebSocket client) stays free of the Tauri
//! / specta dependency graph.
//!
//! Apps register the IPC commands by listing each function under its
//! submodule path inside `tauri_specta::collect_commands!`. Re-exporting
//! the functions at this module's root would *not* propagate the sibling
//! `__cmd__$name` and `__specta__fn__$name` items that `#[tauri::command]`
//! / `#[specta::specta]` emit alongside the function — those live in the
//! defining submodule and are looked up by path next to each function.

pub mod chat;
pub mod context;
pub mod error;
pub mod state;
pub mod thread;

pub use context::{
    ChatContext, ChatContextProvider, NoopChatContextProvider, SharedChatContextProvider,
};
pub use error::{StreamError, ThreadError};
pub use state::{ActiveStreamTokens, SharedThreadManager};
