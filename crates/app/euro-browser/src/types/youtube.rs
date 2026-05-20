use serde::{Deserialize, Serialize};
use specta::Type;

/// One cue from a YouTube transcript as the browser extension emits it.
///
/// Field names mirror YouTube's caption format (`start` and `duration`
/// in seconds) — the extension's transcript parser already exposes them
/// in this shape, so we type the wire payload to match rather than
/// inventing a separate vocabulary at the host boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default, Type)]
pub struct NativeTranscriptLine {
    pub text: String,
    pub start: f64,
    pub duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeYoutubeAsset {
    pub url: String,
    pub title: String,
    pub transcript: Vec<NativeTranscriptLine>,
    pub current_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeYoutubeSnapshot {
    pub current_time: f32,
    pub video_frame_base64: String,
    pub video_frame_width: i32,
    pub video_frame_height: i32,
}
