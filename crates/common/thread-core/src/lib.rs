//! Shared wire types for the Eurora thread HTTP/WebSocket service.
//!
//! This crate is the single source of truth for the JSON contract between
//! `be-thread-service` (Axum + WebSocket) and `euro-tauri` (reqwest +
//! tokio-tungstenite), and is also the input to the TypeScript bindings
//! emitted by the workspace-level `euro-api-codegen` orchestrator
//! (`pnpm specta:backend`).
//!
//! Types are pure data with `serde` derives; the optional `specta` feature
//! adds `specta::Type` so the same definitions can be re-exported as TS.
//! No HTTP, database, gRPC, or LLM dependencies live here on purpose.
//!
//! Rich `agent-chain` payloads (message bodies, content blocks, AI message
//! chunks) are typed end-to-end via the `agent-chain-core` types so the
//! TypeScript bindings emit proper discriminated unions instead of `unknown`.

use agent_chain_core::messages::{AIMessageChunk, AnyMessage, ContentBlock};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "specta")]
use specta::Type;

/// A persisted thread row as returned to the client.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct Thread {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub active_leaf_id: Option<Uuid>,
}

/// Request body for `POST /threads`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct CreateThreadRequest {
    #[serde(default)]
    pub title: Option<String>,
}

/// Response body for `POST /threads`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct CreateThreadResponse {
    pub thread: Thread,
}

/// Query parameters for `GET /threads`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ListThreadsQuery {
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
}

/// Response body for `GET /threads`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ListThreadsResponse {
    pub threads: Vec<Thread>,
}

/// Response body for `GET /threads/{thread_id}`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct GetThreadResponse {
    pub thread: Thread,
}

/// Response body for `DELETE /threads/{thread_id}`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct DeleteThreadResponse {}

/// One node in the message tree returned by message-list endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct MessageNode {
    #[serde(default)]
    pub parent_id: Option<Uuid>,
    pub message: AnyMessage,
    #[serde(default)]
    pub children: Vec<MessageNode>,
    pub sibling_index: i32,
    pub depth: i32,
}

/// Query parameters for `GET /threads/{thread_id}/messages`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct GetMessagesQuery {
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
}

/// Response body for endpoints that return the message tree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct GetMessagesResponse {
    pub messages: Vec<MessageNode>,
}

/// Request body for `POST /threads/{thread_id}/messages/switch-branch`.
///
/// `direction` is `-1`, `0`, or `1` — left sibling, no sibling change, right
/// sibling — matching the gRPC contract this replaces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct SwitchBranchRequest {
    pub message_id: Uuid,
    pub direction: i32,
}

/// Request body for `POST /threads/{thread_id}/title`.
///
/// The endpoint reads recent thread history server-side, so no payload is
/// needed. An empty body type keeps the request well-typed for code-gen and
/// leaves room to add fields later.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct GenerateThreadTitleRequest {}

/// Response body for `POST /threads/{thread_id}/title`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct GenerateThreadTitleResponse {
    pub thread: Thread,
}

/// Query parameters for `GET /threads/search`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct SearchThreadsQuery {
    pub q: String,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
}

/// Response body for `GET /threads/search`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct SearchThreadsResponse {
    pub results: Vec<SearchThreadResult>,
}

/// One thread hit returned by full-text search.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct SearchThreadResult {
    pub id: Uuid,
    pub title: String,
    pub rank: f32,
    pub updated_at: DateTime<Utc>,
}

/// Query parameters for `GET /threads/messages/search`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct SearchMessagesQuery {
    pub q: String,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
}

/// Response body for `GET /threads/messages/search`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct SearchMessagesResponse {
    pub results: Vec<SearchMessageResult>,
}

/// One message hit returned by full-text search.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct SearchMessageResult {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub message_type: String,
    pub rank: f32,
    pub created_at: DateTime<Utc>,
    pub snippet: String,
}

/// Frame sent by the client over the chat WebSocket.
///
/// Bidirectional from day one; the current set is `Send` (start a turn from
/// a new human message), `Regenerate` (re-roll an existing AI response under
/// the same human parent so it becomes a sibling variant), and `Cancel`
/// (interrupt the in-flight turn). New variants can be added without breaking
/// older clients because serde rejects unknown tagged variants only on
/// deserialize, never on encode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatClientMessage {
    Send(ChatSendRequest),
    Regenerate(RegenerateRequest),
    Cancel,
}

/// Payload of a [`ChatClientMessage::Send`] frame.
///
/// When `parent_message_id` is present the turn is interpreted as an edit of
/// an existing branch; the service rewinds `active_leaf` accordingly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ChatSendRequest {
    pub content_blocks: Vec<ContentBlock>,
    #[serde(default)]
    pub parent_message_id: Option<Uuid>,
    #[serde(default)]
    pub asset_chips_json: Option<String>,
}

/// Payload of a [`ChatClientMessage::Regenerate`] frame.
///
/// The server resolves the AI message's parent (a human message), rewinds
/// `active_leaf` to that parent, and runs the agent loop on the existing
/// context. The newly produced AI message lands as a sibling of the original.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct RegenerateRequest {
    pub ai_message_id: Uuid,
}

/// Frame sent by the server over the chat WebSocket.
///
/// Clients should accumulate `Chunk` payloads (using agent-chain's chunk-merge
/// semantics) and replace placeholder state with the `Final.messages` payload
/// when the turn ends.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatServerMessage {
    /// The user's message has been persisted; clients should display it.
    ConfirmedHumanMessage { message: MessageNode },
    /// One streaming chunk from the AI.
    Chunk { chunk: AIMessageChunk },
    /// The turn ended successfully; tree positions for everything that was
    /// persisted during this turn (human + AI + any tool messages).
    Final { messages: Vec<MessageNode> },
    /// The turn aborted with an error. The connection is closed after this.
    Error { kind: String, message: String },
}

/// JSON error body returned by the thread service on non-2xx responses.
///
/// Mirrors the shape used by [`activity-core`](https://docs.rs/activity-core)
/// so the desktop client can decode failures uniformly across HTTP services.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ThreadErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(default)]
    pub details: Option<String>,
}

/// Per-asset context chip surfaced alongside [`ChatContext`] content blocks.
///
/// Lives here (rather than in `euro-activity`) because both desktop and
/// mobile chat IPC layers emit it: desktop populates it from the timeline,
/// mobile populates it from native pickers. Keeping the wire shape in
/// `thread-core` lets the IPC commands stay app-agnostic and avoids
/// dragging the desktop-only `euro-activity` graph into mobile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ContextChip {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub domain: Option<String>,
}

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
        .register::<ChatSendRequest>()
        .register::<RegenerateRequest>()
        .register::<ChatServerMessage>()
        .register::<ThreadErrorResponse>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_thread_request_serializes_optional_title_as_null() {
        let req = CreateThreadRequest::default();
        let s = serde_json::to_string(&req).unwrap();
        assert_eq!(s, r#"{"title":null}"#);
        let back: CreateThreadRequest = serde_json::from_str(&s).unwrap();
        assert!(back.title.is_none());
    }

    #[test]
    fn create_thread_request_decodes_with_missing_title() {
        // Forward-compat: older clients that omit the field still parse.
        let back: CreateThreadRequest = serde_json::from_str("{}").unwrap();
        assert!(back.title.is_none());
    }

    #[test]
    fn list_threads_query_round_trips() {
        let q = ListThreadsQuery {
            limit: Some(10),
            offset: Some(5),
        };
        let s = serde_json::to_string(&q).unwrap();
        let back: ListThreadsQuery = serde_json::from_str(&s).unwrap();
        assert_eq!(q, back);
    }

    fn sample_human_message() -> AnyMessage {
        AnyMessage::HumanMessage(
            agent_chain_core::messages::HumanMessage::builder()
                .content("hi")
                .build(),
        )
    }

    fn sample_text_block() -> ContentBlock {
        ContentBlock::Text(
            agent_chain_core::messages::TextContentBlock::builder()
                .text("hi")
                .build(),
        )
    }

    fn sample_ai_chunk() -> AIMessageChunk {
        AIMessageChunk::builder().content("").build()
    }

    #[test]
    fn message_node_round_trips() {
        let node = MessageNode {
            parent_id: Some(Uuid::nil()),
            message: sample_human_message(),
            children: vec![],
            sibling_index: 0,
            depth: 0,
        };
        let s = serde_json::to_string(&node).unwrap();
        let back: MessageNode = serde_json::from_str(&s).unwrap();
        assert_eq!(node, back);
    }

    #[test]
    fn chat_client_message_serializes_send_with_tag() {
        let m = ChatClientMessage::Send(ChatSendRequest {
            content_blocks: vec![sample_text_block()],
            parent_message_id: None,
            asset_chips_json: None,
        });
        let s = serde_json::to_string(&m).unwrap();
        assert!(s.contains("\"type\":\"send\""));
        assert!(s.contains("\"content_blocks\""));
    }

    #[test]
    fn chat_client_message_serializes_unit_cancel() {
        let s = serde_json::to_string(&ChatClientMessage::Cancel).unwrap();
        assert_eq!(s, "{\"type\":\"cancel\"}");
    }

    #[test]
    fn chat_client_message_serializes_regenerate_with_tag() {
        let m = ChatClientMessage::Regenerate(RegenerateRequest {
            ai_message_id: Uuid::nil(),
        });
        let s = serde_json::to_string(&m).unwrap();
        assert!(s.contains("\"type\":\"regenerate\""));
        assert!(s.contains("\"ai_message_id\""));
        let back: ChatClientMessage = serde_json::from_str(&s).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn chat_server_message_round_trips_each_variant() {
        let cases = vec![
            ChatServerMessage::ConfirmedHumanMessage {
                message: MessageNode {
                    parent_id: None,
                    message: sample_human_message(),
                    children: vec![],
                    sibling_index: 0,
                    depth: 0,
                },
            },
            ChatServerMessage::Chunk {
                chunk: sample_ai_chunk(),
            },
            ChatServerMessage::Final { messages: vec![] },
            ChatServerMessage::Error {
                kind: "internal".into(),
                message: "boom".into(),
            },
        ];
        for case in cases {
            let s = serde_json::to_string(&case).unwrap();
            let back: ChatServerMessage = serde_json::from_str(&s).unwrap();
            assert_eq!(case, back);
        }
    }

    #[cfg(feature = "specta")]
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
        ] {
            assert!(
                names.iter().any(|n| n == expected),
                "missing {expected} from collection: {names:?}"
            );
        }
    }
}
