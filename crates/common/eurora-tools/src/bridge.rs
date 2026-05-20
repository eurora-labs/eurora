//! Shared helpers for bridge-backed adapter implementations.
//!
//! Every adapter crate that targets the browser bridge
//! (`eurora-tools-youtube`, `eurora-tools-web`, future site adapters)
//! translates a typed call into a [`euro_bridge::BridgeService::send_request`]
//! round trip with the same three concerns:
//!
//! 1. **Payload shape.** The browser extension expects a flat JSON object
//!    `{ tab_id, …args }` keyed on `tab_id`; the args' fields ride along
//!    at the top level so [`tab-rpc.ts::parseTabId`] can split the
//!    routing key from the content-script message in one read.
//!    [`build_payload`] enforces this shape from any adapter arg type.
//! 2. **Response decode.** A successful response carries a typed payload
//!    that needs to deserialize back into the adapter's return type; a
//!    missing payload is treated as a structured decode failure rather
//!    than an opaque transport error so the LLM-side surface attributes
//!    blame correctly. [`decode_payload`] handles both paths.
//! 3. **Error mapping.** A [`BridgeError`] from the transport layer has
//!    to be mapped onto the framework's [`ToolError`] without leaking
//!    bridge-protocol types into adapters. [`map_bridge_err`] is the
//!    one place every adapter funnels transport errors through, so a
//!    consistent policy applies — `NotFound` and the
//!    [`CLIENT_CODE_TAB_GONE`] reply both surface as
//!    [`ToolError::ContextUnavailable`]; `Timeout` maps directly; other
//!    client errors surface as [`ToolError::Remote`] with the extension-
//!    supplied numeric code preserved.
//!
//! All three helpers are bridge-protocol-aware and therefore live behind
//! this crate's `bridge` cargo feature; non-desktop consumers (the
//! agent loop, codegen utilities) don't pull
//! [`euro_bridge_protocol`].
//!
//! [`tab-rpc.ts::parseTabId`]: https://github.com/eurora-labs/eurora/blob/main/apps/browser/src/shared/background/tab-rpc.ts

use std::borrow::Cow;

use euro_bridge_protocol::{BridgeError, Payload};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;

use crate::{BrowserOrigin, ToolError};

/// `ErrorFrame.code` returned by the browser extension when the target
/// tab is unreachable (closed, content script missing). Modelled on HTTP
/// `410 Gone` — the resource the call was pinned to is no longer there
/// and the desktop must not retry.
pub const CLIENT_CODE_TAB_GONE: u32 = 410;

/// Build the `{ tab_id, …args }` payload the browser extension expects.
///
/// `args` must serialize to a JSON object. Every current adapter arg
/// type is a named-field struct (or [`crate::Empty`], which serializes
/// to `{}`) so the object cast holds by construction; if someone adds a
/// tuple-struct or enum arg later they get a structured
/// [`ToolError::Encode`] with a pointed message instead of a silently
/// mis-shaped payload landing in the extension.
///
/// `tab_id` is always written last so it overrides any field of the
/// same name on the arg type — adapters cannot accidentally shadow the
/// routing key with a payload field.
pub fn build_payload<A>(target: &BrowserOrigin, args: &A) -> Result<Payload, ToolError>
where
    A: Serialize + ?Sized,
{
    let mut value = serde_json::to_value(args).map_err(ToolError::encode)?;
    let obj = value.as_object_mut().ok_or_else(|| ToolError::Encode {
        message: Cow::Borrowed(
            "bridge tool args must serialize to a JSON object — fix the adapter type",
        ),
        source: None,
    })?;
    obj.insert("tab_id".into(), json!(target.tab_id));
    Payload::from_value(&value).map_err(ToolError::encode)
}

/// Decode a bridge response payload into the adapter's return type.
///
/// A missing payload is treated as a structured decode error rather
/// than [`ToolError::Adapter`] so the LLM-side surface clearly attributes
/// the failure to the wire layer, not the bridge implementation.
pub fn decode_payload<T>(tool: &'static str, payload: Option<Payload>) -> Result<T, ToolError>
where
    T: DeserializeOwned,
{
    payload
        .ok_or_else(|| ToolError::Decode {
            message: format!("tool `{tool}` returned an empty payload").into(),
            source: None,
        })?
        .deserialize()
        .map_err(ToolError::decode)
}

/// Translate a [`BridgeError`] into a tool-facing [`ToolError`].
///
/// `NotFound` is treated as a lost context — the browser bridge client
/// has disconnected and there's no point in retrying this turn.
/// `Timeout` maps directly. A [`CLIENT_CODE_TAB_GONE`] reply from the
/// extension (tab closed, content script missing) maps to
/// [`ToolError::ContextUnavailable`] — the call was pinned to a tab
/// that no longer exists, retrying within the turn cannot succeed.
/// Other `Client` errors surface as [`ToolError::Remote`] with the
/// extension-supplied code preserved; the optional `details` blob is
/// parsed back to JSON when possible. Anything else surfaces as a
/// transport failure with the rendered display message.
pub fn map_bridge_err(tool: &'static str, err: BridgeError) -> ToolError {
    match err {
        BridgeError::NotFound { .. } => ToolError::ContextUnavailable {
            tool: Cow::Borrowed(tool),
            reason: Cow::Borrowed("browser bridge client disconnected"),
        },
        BridgeError::Timeout => ToolError::Timeout,
        BridgeError::ChannelClosed => ToolError::Transport {
            message: Cow::Borrowed("bridge channel closed"),
        },
        BridgeError::Client {
            code: CLIENT_CODE_TAB_GONE,
            ..
        } => ToolError::ContextUnavailable {
            tool: Cow::Borrowed(tool),
            reason: Cow::Borrowed("browser tab is gone"),
        },
        BridgeError::Client {
            code,
            message,
            details,
        } => ToolError::Remote {
            code,
            message,
            details: details.and_then(|p| p.deserialize().ok()),
        },
        other => ToolError::Transport {
            message: other.to_string().into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BrowserOrigin, Empty, ToolError};
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    const TEST_TOOL: &str = "test::tool";

    fn origin() -> BrowserOrigin {
        BrowserOrigin {
            process_id: 1234,
            tab_id: 42,
            window_id: Some("win-7".into()),
            page_url: "https://example.com/".into(),
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct SampleReturn {
        ok: bool,
        value: u32,
    }

    #[derive(Debug, Serialize)]
    struct SampleArgs {
        selector: String,
        limit: u32,
    }

    // --- build_payload ----------------------------------------------------

    #[test]
    fn build_payload_empty_args_flattens_to_tab_id_only() {
        let payload = build_payload(&origin(), &Empty::default()).expect("build");
        let value: serde_json::Value = serde_json::from_str(payload.as_str()).unwrap();
        assert_eq!(value, json!({ "tab_id": 42 }));
    }

    #[test]
    fn build_payload_struct_args_merge_alongside_tab_id() {
        let payload = build_payload(
            &origin(),
            &SampleArgs {
                selector: "main p".into(),
                limit: 50,
            },
        )
        .expect("build");
        let value: serde_json::Value = serde_json::from_str(payload.as_str()).unwrap();
        assert_eq!(
            value,
            json!({
                "tab_id": 42,
                "selector": "main p",
                "limit": 50,
            })
        );
    }

    #[test]
    fn build_payload_tab_id_overrides_arg_field_of_same_name() {
        // If an adapter arg type ever ships a `tab_id` field by accident,
        // the routing key always wins.
        let payload =
            build_payload(&origin(), &json!({ "tab_id": 999, "selector": "main" })).expect("build");
        let value: serde_json::Value = serde_json::from_str(payload.as_str()).unwrap();
        assert_eq!(value["tab_id"], json!(42));
        assert_eq!(value["selector"], json!("main"));
    }

    #[test]
    fn build_payload_rejects_non_object_args() {
        // Tuple types (and bare scalars) serialize to JSON arrays / scalars,
        // which can't carry a `tab_id` field. Surface this as `Encode` so
        // future adapter authors get a clear diagnostic.
        let err = build_payload(&origin(), &(1u32, 2u32)).unwrap_err();
        match err {
            ToolError::Encode { message, source } => {
                assert!(message.contains("JSON object"));
                assert!(source.is_none());
            }
            other => panic!("expected Encode, got {other:?}"),
        }
    }

    // --- map_bridge_err ---------------------------------------------------

    #[test]
    fn map_bridge_err_not_found_to_context_unavailable() {
        match map_bridge_err(TEST_TOOL, BridgeError::NotFound { app_pid: 42 }) {
            ToolError::ContextUnavailable { tool, reason } => {
                assert_eq!(tool, TEST_TOOL);
                assert!(reason.contains("disconnected"));
            }
            other => panic!("expected ContextUnavailable, got {other:?}"),
        }
    }

    #[test]
    fn map_bridge_err_timeout_maps_directly() {
        assert!(matches!(
            map_bridge_err(TEST_TOOL, BridgeError::Timeout),
            ToolError::Timeout
        ));
    }

    #[test]
    fn map_bridge_err_channel_closed_is_transport() {
        match map_bridge_err(TEST_TOOL, BridgeError::ChannelClosed) {
            ToolError::Transport { message } => assert!(message.contains("channel closed")),
            other => panic!("expected Transport, got {other:?}"),
        }
    }

    #[test]
    fn map_bridge_err_client_to_remote_with_decoded_details() {
        let details = Payload::from_value(&json!({ "hint": "video offline" })).unwrap();
        match map_bridge_err(
            TEST_TOOL,
            BridgeError::Client {
                code: 500,
                message: "no captions".into(),
                details: Some(details),
            },
        ) {
            ToolError::Remote {
                code,
                message,
                details,
            } => {
                assert_eq!(code, 500);
                assert_eq!(message, "no captions");
                assert_eq!(details, Some(json!({ "hint": "video offline" })));
            }
            other => panic!("expected Remote, got {other:?}"),
        }
    }

    #[test]
    fn map_bridge_err_client_code_410_is_context_unavailable() {
        match map_bridge_err(
            TEST_TOOL,
            BridgeError::Client {
                code: CLIENT_CODE_TAB_GONE,
                message: "tab unreachable".into(),
                details: None,
            },
        ) {
            ToolError::ContextUnavailable { tool, reason } => {
                assert_eq!(tool, TEST_TOOL);
                assert!(reason.contains("gone"));
            }
            other => panic!("expected ContextUnavailable, got {other:?}"),
        }
    }

    #[test]
    fn map_bridge_err_client_preserves_non_410_code() {
        match map_bridge_err(
            TEST_TOOL,
            BridgeError::Client {
                code: 400,
                message: "bad tab_id".into(),
                details: None,
            },
        ) {
            ToolError::Remote { code, message, .. } => {
                assert_eq!(code, 400);
                assert_eq!(message, "bad tab_id");
            }
            other => panic!("expected Remote, got {other:?}"),
        }
    }

    // --- decode_payload ---------------------------------------------------

    #[test]
    fn decode_payload_missing_is_decode_error_with_no_source() {
        let err: ToolError = decode_payload::<SampleReturn>(TEST_TOOL, None).unwrap_err();
        match err {
            ToolError::Decode { source, message } => {
                assert!(source.is_none());
                assert!(message.contains("empty payload"));
            }
            other => panic!("expected Decode, got {other:?}"),
        }
    }

    #[test]
    fn decode_payload_malformed_json_preserves_serde_source() {
        let payload = Payload::from_value(&json!({ "unexpected": "shape" })).unwrap();
        let err: ToolError = decode_payload::<SampleReturn>(TEST_TOOL, Some(payload)).unwrap_err();
        match err {
            ToolError::Decode { source, .. } => assert!(source.is_some()),
            other => panic!("expected Decode, got {other:?}"),
        }
    }

    #[test]
    fn decode_payload_happy_path_round_trips() {
        let payload = Payload::from_value(&SampleReturn {
            ok: true,
            value: 99,
        })
        .unwrap();
        let decoded: SampleReturn = decode_payload(TEST_TOOL, Some(payload)).expect("decode");
        assert_eq!(
            decoded,
            SampleReturn {
                ok: true,
                value: 99,
            }
        );
    }
}
