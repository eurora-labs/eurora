//! Client-side YouTube adapter bound to `euro-bridge`.
//!
//! Each method on [`YoutubeBridgeImpl`] translates one
//! [`YoutubeAdapter`] call into a [`BridgeService::send_request`] round
//! trip, targeting the browser process identified by the frozen
//! [`BrowserOrigin`] from the per-turn snapshot. The browser extension
//! satisfies the request via the matching action constant
//! ([`YOUTUBE_GET_CURRENT_TIMESTAMP`], …) and the response payload is
//! decoded into the typed return.
//!
//! Errors are funnelled through [`map_bridge_err`] so transport
//! failures, timeouts, and remote errors land in the right
//! [`ToolError`] variant without leaking bridge-protocol types into the
//! framework.

use euro_bridge::BridgeService;
use euro_bridge_protocol::BridgeError;
use eurora_tools::{BrowserOrigin, Empty, ToolError};
use eurora_tools_youtube::{CapturedFrame, CurrentTimestamp, Transcript, YoutubeAdapter};
use serde::de::DeserializeOwned;
use serde_json::json;
use std::borrow::Cow;

/// Bridge action emitted for `browser::youtube::get_current_timestamp`.
pub const YOUTUBE_GET_CURRENT_TIMESTAMP: &str = "YOUTUBE_GET_CURRENT_TIMESTAMP";
/// Bridge action emitted for `browser::youtube::get_transcript`.
pub const YOUTUBE_GET_TRANSCRIPT: &str = "YOUTUBE_GET_TRANSCRIPT";
/// Bridge action emitted for `browser::youtube::get_current_frame`.
pub const YOUTUBE_GET_CURRENT_FRAME: &str = "YOUTUBE_GET_CURRENT_FRAME";

/// `ErrorFrame.code` returned by the browser extension when the target
/// tab is unreachable (closed, content script missing). Modelled on HTTP
/// `410 Gone` — the resource the call was pinned to is no longer there
/// and the desktop must not retry.
const CLIENT_CODE_TAB_GONE: u32 = 410;

const TIMESTAMP_TOOL: &str = "browser::youtube::get_current_timestamp";
const TRANSCRIPT_TOOL: &str = "browser::youtube::get_transcript";
const FRAME_TOOL: &str = "browser::youtube::get_current_frame";

/// Wrapper that fulfils every [`YoutubeAdapter`] method by hitting the
/// browser process registered with [`BridgeService`].
pub struct YoutubeBridgeImpl {
    bridge: &'static BridgeService,
}

impl YoutubeBridgeImpl {
    pub fn new(bridge: &'static BridgeService) -> Self {
        Self { bridge }
    }

    async fn call_action<T>(
        &self,
        target: &BrowserOrigin,
        action: &'static str,
        tool: &'static str,
    ) -> Result<T, ToolError>
    where
        T: DeserializeOwned,
    {
        let payload = serde_json::to_string(&json!({ "tab_id": target.tab_id }))
            .map_err(ToolError::encode)?;
        let response = self
            .bridge
            .send_request(target.process_id, action, Some(payload))
            .await
            .map_err(|err| map_bridge_err(tool, err))?;
        decode_payload(tool, response.payload)
    }
}

impl YoutubeAdapter for YoutubeBridgeImpl {
    async fn get_current_timestamp(
        &self,
        target: &BrowserOrigin,
        _args: Empty,
    ) -> Result<CurrentTimestamp, ToolError> {
        self.call_action(target, YOUTUBE_GET_CURRENT_TIMESTAMP, TIMESTAMP_TOOL)
            .await
    }

    async fn get_transcript(
        &self,
        target: &BrowserOrigin,
        _args: Empty,
    ) -> Result<Transcript, ToolError> {
        self.call_action(target, YOUTUBE_GET_TRANSCRIPT, TRANSCRIPT_TOOL)
            .await
    }

    async fn get_current_frame(
        &self,
        target: &BrowserOrigin,
        _args: Empty,
    ) -> Result<CapturedFrame, ToolError> {
        self.call_action(target, YOUTUBE_GET_CURRENT_FRAME, FRAME_TOOL)
            .await
    }
}

/// Decode a bridge response payload into the adapter's return type.
///
/// A missing payload is treated as a structured decode error rather
/// than `Adapter` so the LLM-side surface clearly attributes the
/// failure to the wire layer, not the bridge implementation.
fn decode_payload<T: DeserializeOwned>(
    tool: &'static str,
    payload: Option<String>,
) -> Result<T, ToolError> {
    let raw = payload.ok_or_else(|| ToolError::Decode {
        message: format!("tool `{tool}` returned an empty payload").into(),
        source: None,
    })?;
    serde_json::from_str(&raw).map_err(ToolError::decode)
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
fn map_bridge_err(tool: &'static str, err: BridgeError) -> ToolError {
    match err {
        BridgeError::NotFound { .. } => ToolError::ContextUnavailable {
            tool: Cow::Borrowed(tool),
            reason: Cow::Borrowed("browser bridge client disconnected"),
        },
        BridgeError::Timeout => ToolError::Timeout,
        BridgeError::ChannelClosed => ToolError::Transport(Cow::Borrowed("bridge channel closed")),
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
            details: details.and_then(|s| serde_json::from_str(&s).ok()),
        },
        other => ToolError::Transport(other.to_string().into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn map_bridge_err_not_found_to_context_unavailable() {
        match map_bridge_err(TIMESTAMP_TOOL, BridgeError::NotFound { app_pid: 42 }) {
            ToolError::ContextUnavailable { tool, reason } => {
                assert_eq!(tool, TIMESTAMP_TOOL);
                assert!(reason.contains("disconnected"));
            }
            other => panic!("expected ContextUnavailable, got {other:?}"),
        }
    }

    #[test]
    fn map_bridge_err_timeout_maps_directly() {
        assert!(matches!(
            map_bridge_err(TRANSCRIPT_TOOL, BridgeError::Timeout),
            ToolError::Timeout
        ));
    }

    #[test]
    fn map_bridge_err_channel_closed_is_transport() {
        match map_bridge_err(FRAME_TOOL, BridgeError::ChannelClosed) {
            ToolError::Transport(msg) => assert!(msg.contains("channel closed")),
            other => panic!("expected Transport, got {other:?}"),
        }
    }

    #[test]
    fn map_bridge_err_client_to_remote_with_decoded_details() {
        let details = serde_json::to_string(&json!({"hint": "video offline"})).unwrap();
        match map_bridge_err(
            TIMESTAMP_TOOL,
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
                assert_eq!(details, Some(json!({"hint": "video offline"})));
            }
            other => panic!("expected Remote, got {other:?}"),
        }
    }

    #[test]
    fn map_bridge_err_client_with_garbled_details_drops_details() {
        match map_bridge_err(
            TIMESTAMP_TOOL,
            BridgeError::Client {
                code: 0,
                message: "weird".into(),
                details: Some("not valid json".into()),
            },
        ) {
            ToolError::Remote { details, .. } => assert!(details.is_none()),
            other => panic!("expected Remote, got {other:?}"),
        }
    }

    #[test]
    fn map_bridge_err_client_code_410_is_context_unavailable() {
        match map_bridge_err(
            TRANSCRIPT_TOOL,
            BridgeError::Client {
                code: 410,
                message: "tab unreachable".into(),
                details: None,
            },
        ) {
            ToolError::ContextUnavailable { tool, reason } => {
                assert_eq!(tool, TRANSCRIPT_TOOL);
                assert!(reason.contains("gone"));
            }
            other => panic!("expected ContextUnavailable, got {other:?}"),
        }
    }

    #[test]
    fn map_bridge_err_client_preserves_non_410_code() {
        match map_bridge_err(
            FRAME_TOOL,
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

    #[test]
    fn decode_payload_missing_is_decode_error_with_no_source() {
        let err: ToolError = decode_payload::<CurrentTimestamp>(TIMESTAMP_TOOL, None).unwrap_err();
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
        let err: ToolError =
            decode_payload::<CurrentTimestamp>(TIMESTAMP_TOOL, Some("{not json".into()))
                .unwrap_err();
        match err {
            ToolError::Decode { source, .. } => assert!(source.is_some()),
            other => panic!("expected Decode, got {other:?}"),
        }
    }

    #[test]
    fn decode_payload_happy_path_round_trips_current_timestamp() {
        let raw = serde_json::to_string(&CurrentTimestamp {
            video_id: "abc123".into(),
            timestamp_seconds: 12.5,
            duration_seconds: 240.0,
            playing: true,
        })
        .unwrap();
        let decoded: CurrentTimestamp = decode_payload(TIMESTAMP_TOOL, Some(raw)).expect("decode");
        assert_eq!(decoded.video_id, "abc123");
        assert!(decoded.playing);
    }
}
