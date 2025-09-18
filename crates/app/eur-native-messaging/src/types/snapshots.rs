use serde::{Deserialize, Serialize};
use specta::Type;

use crate::types::NativeTwitterTweet;

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeYoutubeSnapshot {
    pub current_time: f32,
    pub video_frame_base64: String,
    pub video_frame_width: i32,
    pub video_frame_height: i32,
    // pub video_frame_format: ProtoImageFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeArticleSnapshot {
    pub highlighted_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeTwitterSnapshot {
    pub tweets: Vec<NativeTwitterTweet>,
    pub timestamp: String,
}
