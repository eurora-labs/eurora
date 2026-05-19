//! Argument and return types for the YouTube adapter.
//!
//! Every type derives `Serialize + Deserialize + JsonSchema` so the
//! `#[adapter]` macro can emit input/output schemas for the descriptor
//! table and so the dispatcher can encode/decode payloads at runtime.
//!
//! Empty-args methods take [`eurora_tools::Empty`] rather than a
//! YouTube-local type so every empty-args tool across adapters shares
//! one input schema.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Current playback state of the active YouTube video.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CurrentTimestamp {
    /// The YouTube video ID — the `v=` parameter from the watch URL.
    pub video_id: String,
    /// Playback position in seconds, possibly fractional.
    pub timestamp_seconds: f64,
    /// Total video length in seconds, possibly fractional.
    pub duration_seconds: f64,
    /// `true` when the video is playing, `false` when paused.
    pub playing: bool,
}

/// One cue from a YouTube transcript.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TranscriptEntry {
    /// Start time of the cue in seconds, relative to the video start.
    pub start_seconds: f64,
    /// How long the cue is on screen, in seconds.
    pub duration_seconds: f64,
    /// Cue text as YouTube serves it (HTML-escaped, single-language).
    pub text: String,
}

/// Full transcript of a YouTube video.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Transcript {
    /// The YouTube video ID the transcript belongs to.
    pub video_id: String,
    /// Caption language as a BCP-47 tag (`"en"`, `"en-US"`, `"de"`, …).
    pub language: String,
    /// Ordered cues; empty for videos with auto-captions disabled.
    pub entries: Vec<TranscriptEntry>,
}

/// A single captured frame from the active YouTube video.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CapturedFrame {
    /// The YouTube video ID the frame was captured from.
    pub video_id: String,
    /// Playback position at which the frame was captured, in seconds.
    pub timestamp_seconds: f64,
    /// Decoded frame width in pixels.
    pub width: u32,
    /// Decoded frame height in pixels.
    pub height: u32,
    /// PNG bytes, base64-encoded (standard alphabet, padded).
    pub image_base64: String,
}
