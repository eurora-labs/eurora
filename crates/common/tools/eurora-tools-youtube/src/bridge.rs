//! Bridge-backed [`YoutubeAdapter`] implementation.
//!
//! Each method on [`YoutubeBridgeImpl`] translates one
//! [`YoutubeAdapter`] call into a
//! [`euro_bridge::BridgeService::send_request`] round trip, targeting the
//! browser process identified by the frozen [`BrowserOrigin`] from the
//! per-turn snapshot. The browser extension satisfies the request via the
//! matching action constant ([`YOUTUBE_GET_CURRENT_TIMESTAMP`], …) and
//! the response payload is decoded into the typed return.
//!
//! Payload framing, response decoding, and transport-error mapping all
//! flow through the shared [`eurora_tools::bridge`] helpers so every
//! adapter speaks the same wire shape and surfaces identical
//! [`ToolError`] semantics.
//!
//! This module is gated behind the crate's `bridge` feature so non-desktop
//! consumers (the agent loop, codegen utilities) don't pull
//! [`euro_bridge`] and its transitive dependencies.

use euro_bridge::BridgeService;
use eurora_tools::bridge::{build_payload, decode_payload, map_bridge_err};
use eurora_tools::{BrowserOrigin, Empty, ToolError};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::adapter::YoutubeAdapter;
use crate::types::{CapturedFrame, CurrentTimestamp, Transcript};

/// Bridge action emitted for `browser_youtube_get_current_timestamp`.
pub const YOUTUBE_GET_CURRENT_TIMESTAMP: &str = "YOUTUBE_GET_CURRENT_TIMESTAMP";
/// Bridge action emitted for `browser_youtube_get_transcript`.
pub const YOUTUBE_GET_TRANSCRIPT: &str = "YOUTUBE_GET_TRANSCRIPT";
/// Bridge action emitted for `browser_youtube_get_current_frame`.
pub const YOUTUBE_GET_CURRENT_FRAME: &str = "YOUTUBE_GET_CURRENT_FRAME";

const TIMESTAMP_TOOL: &str = "browser_youtube_get_current_timestamp";
const TRANSCRIPT_TOOL: &str = "browser_youtube_get_transcript";
const FRAME_TOOL: &str = "browser_youtube_get_current_frame";

/// Wrapper that fulfils every [`YoutubeAdapter`] method by hitting the
/// browser process registered with [`BridgeService`].
///
/// Constructed once per process from
/// [`BridgeService::get_or_init`](euro_bridge::BridgeService::get_or_init).
/// The `'static` reference matches that initializer's return type so the
/// struct is cheaply `Clone`-able and trivially sharable across threads.
pub struct YoutubeBridgeImpl {
    bridge: &'static BridgeService,
}

impl YoutubeBridgeImpl {
    pub fn new(bridge: &'static BridgeService) -> Self {
        Self { bridge }
    }

    async fn call_action<A, T>(
        &self,
        target: &BrowserOrigin,
        action: &'static str,
        tool: &'static str,
        args: &A,
    ) -> Result<T, ToolError>
    where
        A: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let payload = build_payload(target, args)?;
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
        args: Empty,
    ) -> Result<CurrentTimestamp, ToolError> {
        self.call_action(target, YOUTUBE_GET_CURRENT_TIMESTAMP, TIMESTAMP_TOOL, &args)
            .await
    }

    async fn get_transcript(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<Transcript, ToolError> {
        self.call_action(target, YOUTUBE_GET_TRANSCRIPT, TRANSCRIPT_TOOL, &args)
            .await
    }

    async fn get_current_frame(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<CapturedFrame, ToolError> {
        self.call_action(target, YOUTUBE_GET_CURRENT_FRAME, FRAME_TOOL, &args)
            .await
    }
}
