//! Adapter mixing multiple methods and multi-context tools.

use eurora_tools::{BrowserOrigin, ToolError, adapter};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Default)]
pub struct Empty {}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Transcript {
    pub text: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
}

/// Tools for the YouTube video the user is currently watching.
#[adapter(namespace = "browser::youtube")]
pub trait YoutubeAdapter: Send + Sync {
    /// Return the transcript of the video.
    #[tool(
        timeout_ms = 10_000,
        source = "bridge(browser)",
        requires_context = "youtube::watch_page",
    )]
    async fn get_transcript(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<Transcript, ToolError>;

    /// Capture the currently-visible video frame as a base64 PNG.
    #[tool(
        timeout_ms = 5_000,
        source = "bridge(browser)",
        requires_context = ["youtube::watch_page", "browser::active_tab"],
    )]
    async fn get_current_frame(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<CapturedFrame, ToolError>;
}

fn main() {
    assert_eq!(YOUTUBE_DESCRIPTORS.len(), 2);
}
