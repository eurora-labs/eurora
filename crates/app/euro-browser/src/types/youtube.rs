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
#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeYoutubeAsset {
    pub url: String,
    pub title: String,
    pub transcript: Vec<TranscriptEntry>,
    pub current_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeYoutubeSnapshot {
    pub current_time: f32,
    pub video_frame_base64: String,
    pub video_frame_width: i32,
    pub video_frame_height: i32,
}
