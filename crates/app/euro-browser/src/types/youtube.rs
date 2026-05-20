use eurora_tools_youtube::TranscriptEntry;
use serde::{Deserialize, Serialize};
use specta::Type;

/// Activity-capture asset emitted by the browser extension's
/// `GENERATE_ASSETS` flow on YouTube watch pages.
///
/// The transcript reuses the canonical [`TranscriptEntry`] from
/// `eurora-tools-youtube`: extension, native-messaging host, activity
/// pipeline, and tool dispatchers all encode YouTube transcripts in
/// exactly one shape (`{start, duration, text}`).
///
/// Snapshots (the per-frame variant) now go through
/// [`eurora_tools_youtube::CapturedFrame`] directly — the legacy
/// `NativeYoutubeSnapshot` wrapper was dropped to eliminate the
/// duplicate field-name + precision drift (`video_frame_base64`/f32 vs
/// `image_base64`/f64) between the activity-capture and tool-call paths.
#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeYoutubeAsset {
    pub url: String,
    pub title: String,
    pub transcript: Vec<TranscriptEntry>,
    pub current_time: f64,
}
