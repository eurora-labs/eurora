//! WebSocket frame types for the per-turn chat protocol.
//!
//! The frames here multiplex two streams over a single connection:
//!
//! - **Turn lifecycle** — `Send`/`Regenerate`/`Cancel` from the client,
//!   `ConfirmedHumanMessage`/`Chunk`/`Final`/`Error` from the server.
//! - **Tool routing** — `CapabilityUpdate` and `ToolResponse` from the
//!   client; `ToolRequest` and `ToolCancel` from the server. These let
//!   the server-side LLM invoke tools that execute on the user's machine
//!   (the browser, a native app) using the same socket as the chat stream.
//!
//! The tool variants are wire-only here; the framework (`eurora-tools`),
//! the per-turn catalog (`be-thread-service`), and the agent-loop wiring
//! that actually exercises them land in later phases.

use agent_chain_core::messages::AIMessageChunk;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "specta")]
use specta::Type;
#[cfg(feature = "specta")]
use specta_typescript::Unknown;

/// Specta-only proxy for the `ToolResponse.result` field's TypeScript shape.
///
/// Rust uses `Result<serde_json::Value, ToolErrorWire>` which serde encodes
/// as `{"Ok": ...}` / `{"Err": ...}` (the externally-tagged default). The
/// auto-generated `Result<T, E>` that `specta-typescript` emits for
/// `std::result::Result` is `{ ok: T, err: E }` — both fields present, no
/// discriminator — which doesn't match the wire. We override the field
/// with this proxy so the TS binding renders as `{ Ok: unknown } |
/// { Err: ToolErrorWire }`, matching what serde actually produces.
///
/// Only the type information matters — specta never serializes or
/// deserializes this; the original field handles all I/O.
#[cfg(feature = "specta")]
#[derive(Type)]
pub enum WireToolResult {
    Ok(Unknown),
    Err(ToolErrorWire),
}

use crate::messages::MessageNode;
use crate::tool_wire::{ToolErrorWire, WireActiveContext, WireToolDescriptor};

/// Frame sent by the client over the chat WebSocket.
///
/// Bidirectional from day one; the current set is `Send` (start a turn from
/// a new human message), `Regenerate` (re-roll an existing AI response under
/// the same human parent so it becomes a sibling variant), `Cancel`
/// (interrupt the in-flight turn), and the tool-routing pair
/// `CapabilityUpdate` + `ToolResponse`. New variants can be added without
/// breaking older clients because serde rejects unknown tagged variants only
/// on deserialize, never on encode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatClientMessage {
    Send(ChatSendRequest),
    Regenerate(RegenerateRequest),
    Cancel,
    /// Declaration of the client's available tools and active contexts for
    /// the upcoming turn. Sent before every `Send`/`Regenerate` so the
    /// catalog reflects the freshest state of the user's session.
    CapabilityUpdate(CapabilityUpdatePayload),
    /// Resolution of a previously-issued `ToolRequest`. The result uses
    /// serde's default `Result` repr (`{"Ok": ...}` / `{"Err": ...}`); the
    /// shape is pinned by tests in this module. See [`WireToolResult`] for
    /// why the specta override is needed.
    ToolResponse {
        call_id: u32,
        #[cfg_attr(feature = "specta", specta(type = WireToolResult))]
        result: Result<serde_json::Value, ToolErrorWire>,
    },
}

/// Payload of a [`ChatClientMessage::CapabilityUpdate`] frame.
///
/// The `tools` list is the client's catalog of remote-dispatch descriptors
/// for the upcoming turn; the `contexts` list is the set of live contexts
/// (e.g. the currently-focused YouTube watch page). Both are filtered into
/// the server's per-turn catalog and rendered into the LLM context.
///
/// Flattened on the wire so the JSON shape stays
/// `{"type":"capability_update","tools":[...],"contexts":[...]}` — the
/// `#[serde(flatten)]` on the enum variant lifts the fields up to the same
/// level as the discriminator, matching the pre-struct payload shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct CapabilityUpdatePayload {
    #[serde(default)]
    pub tools: Vec<WireToolDescriptor>,
    #[serde(default)]
    pub contexts: Vec<WireActiveContext>,
}

/// Payload of a [`ChatClientMessage::Send`] frame.
///
/// When `parent_message_id` is present the turn is interpreted as an edit of
/// an existing branch; the service rewinds `active_leaf` accordingly.
///
/// `activity_id` captures the desktop client's currently-tracked activity
/// when the user sent the message, so the server can record the link in
/// `activity_threads`. Optional because non-desktop clients (web, mobile)
/// have no timeline; absent values skip the link step entirely.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct ChatSendRequest {
    pub content_blocks: Vec<agent_chain_core::messages::ContentBlock>,
    #[serde(default)]
    pub parent_message_id: Option<Uuid>,
    #[serde(default)]
    pub asset_chips_json: Option<String>,
    #[serde(default)]
    pub activity_id: Option<Uuid>,
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
/// when the turn ends. The tool-routing pair `ToolRequest` + `ToolCancel`
/// drives remote-tool RPC over the same socket.
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
    /// Request the client to execute a tool on its side and return the
    /// result via [`ChatClientMessage::ToolResponse`] correlated by
    /// `call_id`.
    ToolRequest {
        call_id: u32,
        descriptor: WireToolDescriptor,
        #[cfg_attr(feature = "specta", specta(type = Unknown))]
        arguments: serde_json::Value,
    },
    /// Abort an in-flight `ToolRequest`. Sent when the user cancels the
    /// turn or the server's tool budget is exhausted before the client's
    /// response arrives.
    ToolCancel { call_id: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_wire::ToolSource;
    use agent_chain_core::messages::{ContentBlock, HumanMessage, TextContentBlock};
    use chrono::{DateTime, Utc};
    use serde_json::{Value, json};

    fn sample_text_block() -> ContentBlock {
        ContentBlock::Text(TextContentBlock::builder().text("hi").build())
    }

    fn sample_ai_chunk() -> AIMessageChunk {
        AIMessageChunk::builder().content("").build()
    }

    fn sample_human_node() -> MessageNode {
        MessageNode {
            parent_id: None,
            message: agent_chain_core::messages::AnyMessage::HumanMessage(
                HumanMessage::builder().content("hi").build(),
            ),
            children: vec![],
            sibling_index: 0,
            depth: 0,
        }
    }

    fn sample_descriptor() -> WireToolDescriptor {
        WireToolDescriptor {
            definition: agent_chain_core::tools::ToolDefinition {
                name: "browser::youtube::get_current_timestamp".into(),
                description: "Return the user's current playback position.".into(),
                parameters: json!({"type": "object"}),
            },
            output_schema: json!({"type": "object"}),
            timeout_ms: 2_000,
            source: ToolSource::Bridge {
                app_kind: "browser".into(),
            },
            required_contexts: vec!["youtube::watch_page".into()],
            requires_user_approval: false,
        }
    }

    #[test]
    fn chat_client_message_serializes_send_with_tag() {
        let m = ChatClientMessage::Send(ChatSendRequest {
            content_blocks: vec![sample_text_block()],
            parent_message_id: None,
            asset_chips_json: None,
            activity_id: None,
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
    fn chat_server_message_round_trips_each_legacy_variant() {
        let cases = vec![
            ChatServerMessage::ConfirmedHumanMessage {
                message: sample_human_node(),
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

    // ------------------------------------------------------------------
    // New tool-routing variants
    // ------------------------------------------------------------------

    #[test]
    fn capability_update_round_trips() {
        let m = ChatClientMessage::CapabilityUpdate(CapabilityUpdatePayload {
            tools: vec![sample_descriptor()],
            contexts: vec![WireActiveContext {
                key: "youtube::watch_page".into(),
                activated_at: DateTime::parse_from_rfc3339("2026-01-15T12:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                data: json!({"video_id": "abc123"}),
            }],
        });
        let s = serde_json::to_string(&m).unwrap();
        let back: ChatClientMessage = serde_json::from_str(&s).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn capability_update_golden_json() {
        // Lock the exact wire shape — any future shift (field reorder,
        // tag rename, default-on-encode change) must update this golden
        // and is reviewable in the diff. The newtype `CapabilityUpdatePayload`
        // is inlined into the externally-tagged enum so the JSON object's
        // top-level fields are `type`, `tools`, and `contexts` — same as the
        // pre-struct shape.
        let m = ChatClientMessage::CapabilityUpdate(CapabilityUpdatePayload {
            tools: vec![WireToolDescriptor {
                definition: agent_chain_core::tools::ToolDefinition {
                    name: "browser::youtube::get_current_timestamp".into(),
                    description: "x".into(),
                    parameters: json!({}),
                },
                output_schema: json!({}),
                timeout_ms: 2_000,
                source: ToolSource::Bridge {
                    app_kind: "browser".into(),
                },
                required_contexts: vec!["youtube::watch_page".into()],
                requires_user_approval: false,
            }],
            contexts: vec![WireActiveContext {
                key: "youtube::watch_page".into(),
                activated_at: DateTime::parse_from_rfc3339("2026-01-15T12:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                data: json!({"video_id": "abc123"}),
            }],
        });
        let s = serde_json::to_string(&m).unwrap();
        assert_eq!(
            s,
            r#"{"type":"capability_update","tools":[{"name":"browser::youtube::get_current_timestamp","description":"x","parameters":{},"output_schema":{},"timeout_ms":2000,"source":{"kind":"bridge","app_kind":"browser"},"required_contexts":["youtube::watch_page"],"requires_user_approval":false}],"contexts":[{"key":"youtube::watch_page","activated_at":"2026-01-15T12:00:00Z","data":{"video_id":"abc123"}}]}"#
        );
    }

    #[test]
    fn tool_response_ok_round_trips() {
        let m = ChatClientMessage::ToolResponse {
            call_id: 42,
            result: Ok(json!({"timestamp_seconds": 12.5})),
        };
        let s = serde_json::to_string(&m).unwrap();
        let back: ChatClientMessage = serde_json::from_str(&s).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn tool_response_err_round_trips() {
        let m = ChatClientMessage::ToolResponse {
            call_id: 7,
            result: Err(ToolErrorWire::Timeout),
        };
        let s = serde_json::to_string(&m).unwrap();
        let back: ChatClientMessage = serde_json::from_str(&s).unwrap();
        assert_eq!(m, back);
    }

    /// Pin the default `Result` serde repr. The spec calls for serde's
    /// default tagged form (`{"Ok": ...}` / `{"Err": ...}`); if anyone
    /// ever slips a `#[serde(rename_all = "...")]` or a custom `serialize_with`
    /// in here, this test will fire before the shift hits the wire.
    #[test]
    fn tool_response_result_uses_default_serde_repr() {
        let ok = ChatClientMessage::ToolResponse {
            call_id: 1,
            result: Ok(json!({"x": 1})),
        };
        let s = serde_json::to_string(&ok).unwrap();
        assert_eq!(
            s,
            r#"{"type":"tool_response","call_id":1,"result":{"Ok":{"x":1}}}"#
        );

        let err = ChatClientMessage::ToolResponse {
            call_id: 2,
            result: Err(ToolErrorWire::Cancelled),
        };
        let s = serde_json::to_string(&err).unwrap();
        assert_eq!(
            s,
            r#"{"type":"tool_response","call_id":2,"result":{"Err":{"kind":"cancelled"}}}"#
        );
    }

    #[test]
    fn tool_request_round_trips() {
        let m = ChatServerMessage::ToolRequest {
            call_id: 99,
            descriptor: sample_descriptor(),
            arguments: json!({}),
        };
        let s = serde_json::to_string(&m).unwrap();
        let back: ChatServerMessage = serde_json::from_str(&s).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn tool_request_golden_json() {
        let m = ChatServerMessage::ToolRequest {
            call_id: 99,
            descriptor: WireToolDescriptor {
                definition: agent_chain_core::tools::ToolDefinition {
                    name: "browser::youtube::get_current_timestamp".into(),
                    description: "x".into(),
                    parameters: json!({}),
                },
                output_schema: json!({}),
                timeout_ms: 2_000,
                source: ToolSource::Bridge {
                    app_kind: "browser".into(),
                },
                required_contexts: vec!["youtube::watch_page".into()],
                requires_user_approval: false,
            },
            arguments: json!({}),
        };
        let s = serde_json::to_string(&m).unwrap();
        assert_eq!(
            s,
            r#"{"type":"tool_request","call_id":99,"descriptor":{"name":"browser::youtube::get_current_timestamp","description":"x","parameters":{},"output_schema":{},"timeout_ms":2000,"source":{"kind":"bridge","app_kind":"browser"},"required_contexts":["youtube::watch_page"],"requires_user_approval":false},"arguments":{}}"#
        );
    }

    #[test]
    fn tool_cancel_round_trips() {
        let m = ChatServerMessage::ToolCancel { call_id: 99 };
        let s = serde_json::to_string(&m).unwrap();
        assert_eq!(s, r#"{"type":"tool_cancel","call_id":99}"#);
        let back: ChatServerMessage = serde_json::from_str(&s).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn capability_update_decodes_with_empty_lists() {
        let m: ChatClientMessage =
            serde_json::from_str(r#"{"type":"capability_update","tools":[],"contexts":[]}"#)
                .unwrap();
        match m {
            ChatClientMessage::CapabilityUpdate(payload) => {
                assert!(payload.tools.is_empty());
                assert!(payload.contexts.is_empty());
            }
            other => panic!("expected CapabilityUpdate, got {other:?}"),
        }
    }

    /// Both fields carry `#[serde(default)]`, so a client that predates the
    /// addition of either list still parses cleanly with the missing field
    /// initialised to an empty `Vec`.
    #[test]
    fn capability_update_decodes_with_missing_fields() {
        let m: ChatClientMessage = serde_json::from_str(r#"{"type":"capability_update"}"#).unwrap();
        match m {
            ChatClientMessage::CapabilityUpdate(payload) => {
                assert!(payload.tools.is_empty());
                assert!(payload.contexts.is_empty());
            }
            other => panic!("expected CapabilityUpdate, got {other:?}"),
        }
    }

    #[test]
    fn unknown_client_message_type_is_rejected() {
        // The new variants don't change the behavior for unknown tags.
        let res: Result<ChatClientMessage, _> =
            serde_json::from_str(r#"{"type":"never_heard_of_it"}"#);
        assert!(res.is_err());
    }

    /// Confirms that adding tool variants didn't change how `Value`-typed
    /// arguments deserialize when they happen to look like our tagged shapes.
    #[test]
    fn arguments_with_type_field_decode_as_value() {
        let m = ChatServerMessage::ToolRequest {
            call_id: 1,
            descriptor: sample_descriptor(),
            arguments: json!({"type": "something_else"}),
        };
        let s = serde_json::to_string(&m).unwrap();
        let back: ChatServerMessage = serde_json::from_str(&s).unwrap();
        assert_eq!(m, back);
        // Sanity: the inner Value isn't being eaten by the outer tag.
        match back {
            ChatServerMessage::ToolRequest { arguments, .. } => {
                assert_eq!(
                    arguments,
                    Value::Object({
                        let mut m = serde_json::Map::new();
                        m.insert("type".to_string(), Value::String("something_else".into()));
                        m
                    })
                );
            }
            other => panic!("expected ToolRequest, got {other:?}"),
        }
    }
}
