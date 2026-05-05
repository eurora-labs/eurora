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
//! Where rich `agent-chain` payloads cross the wire (message bodies, content
//! blocks, AI message chunks), they are typed as [`serde_json::Value`]. They
//! are produced/consumed via `agent-chain`'s existing `serde` impls on the
//! Rust side; on the TypeScript side the existing `message-converter` layer
//! consumes them as opaque JSON and yields typed domain models. Embedding the
//! agent-chain types directly here would require draping specta over the
//! crate's hand-rolled serde, which is out of scope for the wire contract.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[cfg(feature = "specta")]
use specta::Type;
#[cfg(feature = "specta")]
use specta_typescript::Unknown;

/// A persisted thread row as returned to the client.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct Thread {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_leaf_id: Option<Uuid>,
}

/// Request body for `POST /threads`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct CreateThreadRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
///
/// `message` is an `agent_chain::AnyMessage` serialized as JSON. We type the
/// field as [`serde_json::Value`] so this crate stays free of the agent-chain
/// dependency, and override the TypeScript representation to `unknown` so
/// the frontend converter narrows it explicitly. The same trick on
/// `children` works around `specta-typescript`'s lack of recursive type
/// references at this version (it would inline the type and stack-overflow);
/// the runtime JSON shape is still a real recursive `MessageNode` tree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct MessageNode {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Uuid>,
    #[cfg_attr(feature = "specta", specta(type = Unknown))]
    pub message: Value,
    #[serde(default)]
    #[cfg_attr(feature = "specta", specta(type = Vec<Unknown>))]
    pub children: Vec<MessageNode>,
    pub sibling_index: i32,
    pub depth: i32,
}

/// Query parameters for `GET /threads/{thread_id}/messages`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct GetMessagesQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(default)]
    pub all_variants: bool,
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct GenerateThreadTitleRequest {
    pub content: String,
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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

/// Request body for `POST /threads/{thread_id}/preliminary-blocks`.
///
/// `content_blocks` carries `agent_chain` `ContentBlock` JSON values; the
/// service rewrites large in-line payloads into asset references and returns
/// the rewritten blocks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct SavePreliminaryContentBlocksRequest {
    #[cfg_attr(feature = "specta", specta(type = Vec<Unknown>))]
    pub content_blocks: Vec<Value>,
}

/// Response body for `POST /threads/{thread_id}/preliminary-blocks`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct SavePreliminaryContentBlocksResponse {
    #[cfg_attr(feature = "specta", specta(type = Vec<Unknown>))]
    pub content_blocks: Vec<Value>,
}

/// Frame sent by the client over the chat WebSocket.
///
/// Bidirectional from day one; the current set is `Send` (start a turn) and
/// `Cancel` (interrupt the in-flight turn). New variants can be added without
/// breaking older clients because serde rejects unknown tagged variants only
/// on deserialize, never on encode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatClientMessage {
    Send(ChatSendRequest),
    Cancel,
}

/// Payload of a [`ChatClientMessage::Send`] frame.
///
/// `content_blocks` carries a `Vec<agent_chain::ContentBlock>` as JSON. When
/// `parent_message_id` is present the turn is interpreted as an edit of an
/// existing branch; the service rewinds `active_leaf` accordingly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ChatSendRequest {
    #[cfg_attr(feature = "specta", specta(type = Vec<Unknown>))]
    pub content_blocks: Vec<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_message_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_chips_json: Option<String>,
}

/// Frame sent by the server over the chat WebSocket.
///
/// `chunk` carries an `AIMessageChunk` JSON; the client should accumulate
/// chunks (using agent-chain's chunk-merge semantics) and replace placeholder
/// state with the `final_messages` payload when the turn ends.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatServerMessage {
    /// The user's message has been persisted; clients should display it.
    ConfirmedHumanMessage { message: MessageNode },
    /// One streaming chunk from the AI.
    Chunk {
        #[cfg_attr(feature = "specta", specta(type = Unknown))]
        chunk: Value,
    },
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// Build a [`specta::TypeCollection`] containing every thread wire type the
/// desktop app needs. Used by the codegen binary to emit `thread.ts`.
#[cfg(feature = "specta")]
pub fn type_collection() -> specta::TypeCollection {
    specta::TypeCollection::default()
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
        .register::<SavePreliminaryContentBlocksRequest>()
        .register::<SavePreliminaryContentBlocksResponse>()
        .register::<ChatClientMessage>()
        .register::<ChatSendRequest>()
        .register::<ChatServerMessage>()
        .register::<ThreadErrorResponse>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn create_thread_request_omits_optional_title() {
        let req = CreateThreadRequest::default();
        let s = serde_json::to_string(&req).unwrap();
        assert_eq!(s, "{}");
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

    #[test]
    fn message_node_round_trips() {
        let node = MessageNode {
            parent_id: Some(Uuid::nil()),
            message: json!({"type": "human", "content": [{"type": "text", "text": "hi"}]}),
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
            content_blocks: vec![json!({"type": "text", "text": "hi"})],
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
    fn chat_server_message_round_trips_each_variant() {
        let cases = vec![
            ChatServerMessage::ConfirmedHumanMessage {
                message: MessageNode {
                    parent_id: None,
                    message: json!({}),
                    children: vec![],
                    sibling_index: 0,
                    depth: 0,
                },
            },
            ChatServerMessage::Chunk { chunk: json!({}) },
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
            .map(|ndt| ndt.name().to_string())
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
