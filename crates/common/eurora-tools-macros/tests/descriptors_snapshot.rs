//! Insta snapshot of the macro-emitted descriptor table.
//!
//! Runs the macro on a small, representative adapter and snapshots the
//! resulting `Vec<WireToolDescriptor>` (after `to_wire()`). The snapshot
//! pins the runtime data — tool names, descriptions, schemas, timeouts,
//! sources, contexts, approval flags — which is what downstream
//! consumers depend on. Regenerate with `INSTA_UPDATE=auto cargo test
//! -p eurora-tools-macros --test descriptors_snapshot`.

use eurora_tools::{BrowserOrigin, ToolError, adapter};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Default)]
pub struct Empty {}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct CurrentTimestamp {
    pub video_id: String,
    pub current_time: f64,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Transcript {
    pub video_id: String,
    pub text: String,
}

/// Tools for the YouTube video the user is currently watching.
#[adapter(namespace = "browser_youtube", version = 1)]
pub trait YoutubeAdapter: Send + Sync {
    /// Return the user's current playback position.
    #[tool(
        timeout_ms = 2_000,
        source = "bridge(browser)",
        requires_context = "youtube::watch_page"
    )]
    async fn get_current_timestamp(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<CurrentTimestamp, ToolError>;

    /// Return the full transcript of the video the user is watching.
    #[tool(
        timeout_ms = 10_000,
        source = "bridge(browser)",
        requires_context = ["youtube::watch_page", "browser::active_tab"],
    )]
    async fn get_transcript(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<Transcript, ToolError>;
}

#[test]
fn descriptor_table_snapshot() {
    let wire: Vec<_> = YOUTUBE_DESCRIPTORS.iter().map(|d| d.to_wire()).collect();
    insta::assert_debug_snapshot!(wire);
}
