use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeYoutubeAsset {
    pub url: String,
    pub title: String,
    pub transcript: String,
    pub current_time: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeYoutubeSnapshot {
    pub current_time: f32,
    pub video_frame_base64: String,
    pub video_frame_width: i32,
    pub video_frame_height: i32,
}
