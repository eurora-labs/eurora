//! The YouTube adapter trait.
//!
//! The `#[adapter]` macro expands this module into:
//!
//! - [`YOUTUBE_DESCRIPTORS`] — the static descriptor table consumed by
//!   the server-side agent loop via `WireToolDescriptor`.
//! - [`YoutubeDispatcher<T>`] — the runtime dispatcher that decodes
//!   `IncomingCall::arguments`, validates the [`Origin`](eurora_tools::Origin)
//!   variant, awaits the user-written adapter, and re-encodes the result.
//! - A Send-bounded [`YoutubeAdapter`] trait plus a non-Send
//!   [`YoutubeAdapterLocal`] variant produced by `trait_variant::make`.
//!   Production code should `impl YoutubeAdapter for …`; the `Local`
//!   variant comes for free via the blanket impl and is convenient when
//!   stubbing single-threaded tests.
//!
//! The first paragraph of each method's rustdoc is extracted by the
//! macro as the tool description sent to the LLM, so the wording here
//! is part of the runtime surface — keep it user-facing.

use eurora_tools::{BrowserOrigin, Empty, ToolError, adapter};

use crate::types::{CapturedFrame, CurrentTimestamp, Transcript};

/// Tools for the YouTube video the user is currently watching.
#[adapter(namespace = "browser::youtube", version = 1)]
pub trait YoutubeAdapter: Send + Sync {
    /// Return the user's current playback position in the active YouTube
    /// video, along with the video duration and play/pause state.
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
    /// Comes from YouTube's caption API; videos without captions fail
    /// with a structured error.
    #[tool(
        timeout_ms = 10_000,
        source = "bridge(browser)",
        requires_context = "youtube::watch_page"
    )]
    async fn get_transcript(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<Transcript, ToolError>;

    /// Capture the current visible video frame as a PNG, base64-encoded.
    #[tool(
        timeout_ms = 5_000,
        source = "bridge(browser)",
        requires_context = "youtube::watch_page"
    )]
    async fn get_current_frame(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<CapturedFrame, ToolError>;
}
