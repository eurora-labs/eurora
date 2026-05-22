//! Shared wire types for the Eurora thread HTTP/WebSocket service.
//!
//! This crate is the single source of truth for the JSON contract between
//! `be-thread-service` (Axum + WebSocket) and `euro-tauri` (reqwest +
//! tokio-tungstenite), and is also the input to the TypeScript bindings
//! emitted by the workspace-level `euro-codegen` orchestrator
//! (`pnpm specta`).
//!
//! Types are pure data with `serde` derives; the optional `specta` feature
//! adds `specta::Type` so the same definitions can be re-exported as TS.
//! No HTTP, database, gRPC, or LLM dependencies live here on purpose.
//!
//! Rich `agent-chain` payloads (message bodies, content blocks, AI message
//! chunks) are typed end-to-end via the `agent-chain-core` types so the
//! TypeScript bindings emit proper discriminated unions instead of `unknown`.
//!
//! ## Module layout
//!
//! - [`thread`] — thread CRUD + search response shapes.
//! - [`messages`] — message tree, branch switch, message search.
//! - [`chat`] — chat WebSocket frame enums and their payloads.
//! - [`tool_wire`] — wire-side primitives for the unified tool-execution
//!   architecture (`ToolSource`, `ToolErrorWire`, `WireToolDescriptor`,
//!   `WireActiveContext`).
//! - [`error`] — HTTP error envelope.
//! - [`context_chip`] — per-asset chip metadata surfaced alongside chat
//!   content blocks.
//!
//! Every type is re-exported at the crate root for backwards-compatible
//! `use thread_core::Foo;` imports.

pub mod chat;
pub mod context_chip;
pub mod error;
pub mod messages;
pub mod thread;
pub mod tool_wire;

pub use chat::{
    CapabilityUpdatePayload, ChatClientMessage, ChatSendRequest, ChatServerMessage,
    RegenerateRequest,
};
pub use context_chip::ContextChip;
pub use error::ThreadErrorResponse;
pub use messages::{
    GetMessagesQuery, GetMessagesResponse, MessageNode, SearchMessageResult, SearchMessagesQuery,
    SearchMessagesResponse, SwitchBranchRequest,
};
pub use thread::{
    CreateThreadRequest, CreateThreadResponse, DeleteThreadResponse, GenerateThreadTitleRequest,
    GenerateThreadTitleResponse, GetThreadResponse, ListThreadsQuery, ListThreadsResponse,
    SearchThreadResult, SearchThreadsQuery, SearchThreadsResponse, Thread,
};
pub use tool_wire::{ToolErrorWire, ToolSource, WireActiveContext, WireToolDescriptor};

/// Build a [`specta::Types`] containing every thread wire type the desktop
/// app needs. Used by the codegen binary to emit `thread.ts`.
#[cfg(feature = "specta")]
pub fn type_collection() -> specta::Types {
    specta::Types::default()
        .register::<Thread>()
        .register::<CreateThreadRequest>()
        .register::<CreateThreadResponse>()
        .register::<ListThreadsQuery>()
        .register::<ListThreadsResponse>()
        .register::<GetThreadResponse>()
        .register::<DeleteThreadResponse>()
        .register::<MessageNode>()
        .register::<GetMessagesQuery>()
        .register::<GetMessagesResponse>()
        .register::<SwitchBranchRequest>()
        .register::<GenerateThreadTitleRequest>()
        .register::<GenerateThreadTitleResponse>()
        .register::<SearchThreadsQuery>()
        .register::<SearchThreadsResponse>()
        .register::<SearchThreadResult>()
        .register::<SearchMessagesQuery>()
        .register::<SearchMessagesResponse>()
        .register::<SearchMessageResult>()
        .register::<ChatClientMessage>()
        .register::<CapabilityUpdatePayload>()
        .register::<ChatSendRequest>()
        .register::<RegenerateRequest>()
        .register::<ChatServerMessage>()
        .register::<ThreadErrorResponse>()
        .register::<WireToolDescriptor>()
        .register::<ToolSource>()
        .register::<ToolErrorWire>()
        .register::<WireActiveContext>()
}

#[cfg(all(test, feature = "specta"))]
mod tests {
    use super::*;

    #[test]
    fn type_collection_contains_all_wire_types() {
        let types = type_collection();
        let names: Vec<String> = types
            .into_unsorted_iter()
            .map(|ndt| ndt.name.to_string())
            .collect();
        for expected in [
            "Thread",
            "CreateThreadRequest",
            "ListThreadsQuery",
            "MessageNode",
            "GetMessagesResponse",
            "SwitchBranchRequest",
            "ChatClientMessage",
            "ChatServerMessage",
            "ThreadErrorResponse",
            "WireToolDescriptor",
            "ToolSource",
            "ToolErrorWire",
            "WireActiveContext",
        ] {
            assert!(
                names.iter().any(|n| n == expected),
                "missing {expected} from collection: {names:?}"
            );
        }
    }
}
