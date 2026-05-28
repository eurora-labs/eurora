//! Wire-side primitives for the unified tool-execution architecture.
//!
//! These types travel over the chat WebSocket inside `ChatClientMessage` and
//! `ChatServerMessage` frames. They are intentionally owned and serializable
//! so the server can pass descriptors through to the LLM and persist them
//! without depending on the client-side `eurora-tools` framework crate.
//!
//! The framework's in-process counterparts (`ToolDescriptor`, `Origin`,
//! `ToolError`) live in `eurora-tools` and convert to/from the wire shapes
//! defined here. See `plan.md` for the full architecture.

use agent_chain_core::tools::ToolDefinition;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "specta")]
use specta::Type;
#[cfg(feature = "specta")]
use specta_typescript::Unknown;

/// Where a tool runs. The server uses this to dispatch each call:
/// `ServerLocal` tools execute in-process on the backend; everything else
/// is routed back to the client over the chat WebSocket as a `ToolRequest`,
/// and from there the client picks the right destination (bridge, native
/// app, ACP session).
///
/// `#[non_exhaustive]` so adding new sources later is non-breaking for
/// downstream consumers that `match` on the enum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ToolSource {
    /// Routed via `euro-bridge` to a registered app of `app_kind` (in v1,
    /// `"browser"`).
    Bridge { app_kind: String },
    /// Runs in-process on the client (native Tauri tools).
    ClientLocal,
    /// Runs in-process on the backend (Firecrawl, describe-image).
    ServerLocal,
    /// Piped through an ACP session.
    Acp,
}

/// Wire-side counterpart to `eurora_tools::ToolError`.
///
/// The framework's `ToolError` carries non-serializable payloads
/// (`serde_json::Error`, boxed `dyn Error`); converting to `ToolErrorWire`
/// is lossy on purpose (the `Adapter` variant collapses to a single
/// `message: String`) so the wire shape stays serde-friendly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Error)]
#[cfg_attr(feature = "specta", derive(Type))]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ToolErrorWire {
    #[error("context unavailable for tool `{tool}`: {reason}")]
    ContextUnavailable { tool: String, reason: String },
    #[error("origin mismatch for tool `{tool}`: expected {expected}, got {got}")]
    OriginMismatch {
        tool: String,
        expected: String,
        got: String,
    },
    #[error("tool call timed out")]
    Timeout,
    #[error("tool call cancelled")]
    Cancelled,
    #[error("transport error: {message}")]
    Transport { message: String },
    #[error("remote error {code}: {message}")]
    Remote {
        code: u32,
        message: String,
        #[serde(default)]
        #[cfg_attr(feature = "specta", specta(type = Option<Unknown>))]
        details: Option<serde_json::Value>,
    },
    #[error("failed to decode tool payload: {message}")]
    Decode { message: String },
    #[error("failed to encode tool payload: {message}")]
    Encode { message: String },
    #[error("adapter error: {message}")]
    Adapter { message: String },
}

/// Wire-side descriptor for a tool. One per call, one per entry in the
/// per-turn `CapabilityUpdate.tools` list.
///
/// `WireToolDescriptor` is a [`ToolDefinition`] (name, description,
/// parameters schema) plus dispatch metadata (timeout, source, required
/// contexts, approval flag) and the tool's output schema. The framework
/// form (`eurora_tools::ToolDescriptor`) is the in-process
/// `&'static`-everywhere shape; this is its serializable counterpart, owned
/// and ready for transport. The server only ever sees this form.
///
/// The `definition` field is flattened on the wire so the JSON shape stays
/// flat (`{"name": ..., "description": ..., "parameters": ..., ...}`),
/// while in Rust the typed relationship `WireToolDescriptor IS-A
/// ToolDefinition + dispatch metadata` is explicit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct WireToolDescriptor {
    /// The LLM-facing identity of the tool: name, description, parameters
    /// schema. Reused by `agent-chain-core` providers when binding the
    /// tool to a chat model.
    #[serde(flatten)]
    pub definition: ToolDefinition,
    /// JSON Schema for the tool's success payload.
    #[cfg_attr(feature = "specta", specta(type = Unknown))]
    pub output_schema: serde_json::Value,
    /// Per-call timeout in milliseconds. Used by the server's bus to bound
    /// in-flight remote calls; the framework's `ToolDescriptor.timeout`
    /// converts to/from this field. `u32` ms (~49 days) is bounded well
    /// above any sensible per-tool budget and keeps the wire shape inside
    /// JS's safe integer range.
    pub timeout_ms: u32,
    /// Where the tool runs. The server uses this to pick a dispatch path.
    pub source: ToolSource,
    /// Context keys whose presence is required for this tool to be
    /// surfaced to the LLM in a given turn (e.g.
    /// `["youtube::watch_page"]`).
    #[serde(default)]
    pub required_contexts: Vec<String>,
    /// If true, the server must obtain explicit user approval before
    /// dispatching the call. Not enforced in v1 (all v1 tools are
    /// read-only) but declared so the protocol is stable.
    #[serde(default)]
    pub requires_user_approval: bool,
}

impl WireToolDescriptor {
    /// Shortcut for the most common field access — the tool's name lives
    /// inside the flattened [`ToolDefinition`].
    pub fn name(&self) -> &str {
        &self.definition.name
    }
}

/// Wire-side projection of an active context, sent in `CapabilityUpdate`.
///
/// Carries only the parts the server needs: the key (used to template the
/// system message), the activation timestamp, and the opaque per-key data
/// blob (surfaced to the LLM). Routing information — which window, which
/// tab, which session — is intentionally absent: that's `Origin`, and it
/// stays client-side.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
pub struct WireActiveContext {
    /// Stable namespaced key, e.g. `"youtube::watch_page"`.
    pub key: String,
    /// When the client activated this context.
    pub activated_at: DateTime<Utc>,
    /// Opaque per-key payload. Shape is determined by `key`; the server's
    /// per-key formatter renders it into the system message.
    #[cfg_attr(feature = "specta", specta(type = Unknown))]
    pub data: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tool_source_bridge_round_trips() {
        let src = ToolSource::Bridge {
            app_kind: "browser".into(),
        };
        let s = serde_json::to_string(&src).unwrap();
        assert_eq!(s, r#"{"kind":"bridge","app_kind":"browser"}"#);
        let back: ToolSource = serde_json::from_str(&s).unwrap();
        assert_eq!(src, back);
    }

    #[test]
    fn tool_source_unit_variants_round_trip() {
        for (src, expected) in [
            (ToolSource::ClientLocal, r#"{"kind":"client_local"}"#),
            (ToolSource::ServerLocal, r#"{"kind":"server_local"}"#),
            (ToolSource::Acp, r#"{"kind":"acp"}"#),
        ] {
            let s = serde_json::to_string(&src).unwrap();
            assert_eq!(s, expected, "encoding for {src:?}");
            let back: ToolSource = serde_json::from_str(&s).unwrap();
            assert_eq!(src, back);
        }
    }

    #[test]
    fn tool_error_wire_all_variants_round_trip() {
        let cases = vec![
            ToolErrorWire::ContextUnavailable {
                tool: "browser_youtube_get_transcript".into(),
                reason: "no active youtube tab".into(),
            },
            ToolErrorWire::OriginMismatch {
                tool: "browser_youtube_get_transcript".into(),
                expected: "Browser".into(),
                got: "Focused".into(),
            },
            ToolErrorWire::Timeout,
            ToolErrorWire::Cancelled,
            ToolErrorWire::Transport {
                message: "ws closed".into(),
            },
            ToolErrorWire::Remote {
                code: 404,
                message: "not found".into(),
                details: Some(json!({"hint": "video not playing"})),
            },
            ToolErrorWire::Remote {
                code: 500,
                message: "boom".into(),
                details: None,
            },
            ToolErrorWire::Decode {
                message: "expected number".into(),
            },
            ToolErrorWire::Encode {
                message: "non-utf8 string".into(),
            },
            ToolErrorWire::Adapter {
                message: "youtube API rate-limited".into(),
            },
        ];
        for case in cases {
            let s = serde_json::to_string(&case).unwrap();
            let back: ToolErrorWire = serde_json::from_str(&s).unwrap();
            assert_eq!(case, back, "round trip for {case:?}");
        }
    }

    #[test]
    fn tool_error_wire_uses_kind_tag() {
        let s = serde_json::to_string(&ToolErrorWire::Timeout).unwrap();
        assert_eq!(s, r#"{"kind":"timeout"}"#);
    }

    #[test]
    fn tool_error_wire_remote_omits_null_details_when_present() {
        let s = serde_json::to_string(&ToolErrorWire::Remote {
            code: 1,
            message: "x".into(),
            details: None,
        })
        .unwrap();
        // `Option`'s default serialization keeps the field as `null`; the
        // `#[serde(default)]` only affects deserialize. That's fine — pin
        // the exact shape so future serde tweaks are reviewable.
        assert_eq!(
            s,
            r#"{"kind":"remote","code":1,"message":"x","details":null}"#
        );
    }

    #[test]
    fn tool_error_wire_remote_decodes_with_missing_details() {
        let back: ToolErrorWire =
            serde_json::from_str(r#"{"kind":"remote","code":1,"message":"x"}"#).unwrap();
        match back {
            ToolErrorWire::Remote { details, .. } => assert!(details.is_none()),
            other => panic!("expected Remote, got {other:?}"),
        }
    }

    #[test]
    fn tool_error_wire_implements_std_error() {
        // Compile-time check: trait bound is satisfied.
        fn assert_error<E: std::error::Error>() {}
        assert_error::<ToolErrorWire>();

        let err = ToolErrorWire::Timeout;
        assert_eq!(err.to_string(), "tool call timed out");
    }

    fn sample_descriptor() -> WireToolDescriptor {
        WireToolDescriptor {
            definition: ToolDefinition {
                name: "browser_youtube_get_current_timestamp".into(),
                description: "Return the user's current playback position.".into(),
                parameters: json!({"type": "object"}),
            },
            output_schema: json!({"type": "object", "properties": {"timestamp_seconds": {"type": "number"}}}),
            timeout_ms: 2_000,
            source: ToolSource::Bridge {
                app_kind: "browser".into(),
            },
            required_contexts: vec!["youtube::watch_page".into()],
            requires_user_approval: false,
        }
    }

    #[test]
    fn wire_tool_descriptor_round_trips() {
        let d = sample_descriptor();
        let s = serde_json::to_string(&d).unwrap();
        let back: WireToolDescriptor = serde_json::from_str(&s).unwrap();
        assert_eq!(d, back);
    }

    #[test]
    fn wire_tool_descriptor_flattens_definition_on_the_wire() {
        // The flattened ToolDefinition fields sit at the top level of the
        // JSON object — no nested `definition` envelope. Pin this so a
        // future #[serde(flatten)] mistake breaks the test, not the wire.
        let s = serde_json::to_string(&sample_descriptor()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
        let obj = parsed.as_object().expect("wire shape is an object");
        assert!(obj.contains_key("name"));
        assert!(obj.contains_key("description"));
        assert!(obj.contains_key("parameters"));
        assert!(!obj.contains_key("definition"));
        assert!(!obj.contains_key("input_schema"));
    }

    #[test]
    fn wire_tool_descriptor_decodes_with_missing_optional_fields() {
        // Forward-compat: clients that predate the addition of
        // `required_contexts` / `requires_user_approval` should still parse.
        let json = r#"{
            "name": "browser_youtube_get_current_timestamp",
            "description": "x",
            "parameters": {},
            "output_schema": {},
            "timeout_ms": 2000,
            "source": {"kind": "bridge", "app_kind": "browser"}
        }"#;
        let back: WireToolDescriptor = serde_json::from_str(json).unwrap();
        assert!(back.required_contexts.is_empty());
        assert!(!back.requires_user_approval);
    }

    #[test]
    fn wire_tool_descriptor_name_helper_returns_definition_name() {
        let d = sample_descriptor();
        assert_eq!(d.name(), "browser_youtube_get_current_timestamp");
    }

    #[test]
    fn wire_active_context_round_trips() {
        let ctx = WireActiveContext {
            key: "youtube::watch_page".into(),
            activated_at: DateTime::parse_from_rfc3339("2026-01-15T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            data: json!({
                "video_id": "abc123",
                "title": "Example Video",
            }),
        };
        let s = serde_json::to_string(&ctx).unwrap();
        let back: WireActiveContext = serde_json::from_str(&s).unwrap();
        assert_eq!(ctx, back);
    }
}
