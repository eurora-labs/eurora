//! Bridge-backed [`YoutubeAdapter`] implementation.
//!
//! Each method on [`YoutubeBridgeImpl`] translates one
//! [`YoutubeAdapter`] call into a single
//! [`BridgeClient::call_action`](crate::BridgeClient::call_action)
//! round trip, targeting the browser process identified by the frozen
//! [`BrowserOrigin`] from the per-turn snapshot. The browser extension
//! satisfies the request via the matching action constant
//! ([`YOUTUBE_GET_CURRENT_TIMESTAMP`], …) and the response payload is
//! decoded into the typed return.

use euro_bridge::BridgeService;
use eurora_tools::{BrowserOrigin, Empty, ToolError};
use eurora_tools_browser::youtube::{CapturedFrame, CurrentTimestamp, Transcript, YoutubeAdapter};

use crate::BridgeClient;

/// Bridge action emitted for `browser_youtube_get_current_timestamp`.
pub const YOUTUBE_GET_CURRENT_TIMESTAMP: &str = "YOUTUBE_GET_CURRENT_TIMESTAMP";
/// Bridge action emitted for `browser_youtube_get_transcript`.
pub const YOUTUBE_GET_TRANSCRIPT: &str = "YOUTUBE_GET_TRANSCRIPT";
/// Bridge action emitted for `browser_youtube_get_current_frame`.
pub const YOUTUBE_GET_CURRENT_FRAME: &str = "YOUTUBE_GET_CURRENT_FRAME";

const TIMESTAMP_TOOL: &str = "browser_youtube_get_current_timestamp";
const TRANSCRIPT_TOOL: &str = "browser_youtube_get_transcript";
const FRAME_TOOL: &str = "browser_youtube_get_current_frame";

/// Fulfils every [`YoutubeAdapter`] method by hitting the browser
/// process registered with the underlying [`BridgeService`].
pub struct YoutubeBridgeImpl {
    client: BridgeClient,
}

impl YoutubeBridgeImpl {
    /// Bind to the process-wide [`BridgeService`] singleton.
    pub const fn new(bridge: &'static BridgeService) -> Self {
        Self {
            client: BridgeClient::new(bridge),
        }
    }

    /// Bind to a pre-constructed [`BridgeClient`] — convenient when the
    /// desktop wiring builds one client and shares it across every
    /// bridge-backed adapter on the same `BridgeService`.
    pub const fn with_client(client: BridgeClient) -> Self {
        Self { client }
    }
}

impl YoutubeAdapter for YoutubeBridgeImpl {
    async fn get_current_timestamp(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<CurrentTimestamp, ToolError> {
        self.client
            .call_action(target, YOUTUBE_GET_CURRENT_TIMESTAMP, TIMESTAMP_TOOL, &args)
            .await
    }

    async fn get_transcript(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<Transcript, ToolError> {
        self.client
            .call_action(target, YOUTUBE_GET_TRANSCRIPT, TRANSCRIPT_TOOL, &args)
            .await
    }

    async fn get_current_frame(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<CapturedFrame, ToolError> {
        self.client
            .call_action(target, YOUTUBE_GET_CURRENT_FRAME, FRAME_TOOL, &args)
            .await
    }
}
