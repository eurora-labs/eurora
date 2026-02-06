use serde::{Deserialize, Serialize};
use specta::Type;

use crate::types::NativeTwitterTweet;

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeYoutubeAsset {
    pub url: String,
    pub title: String,
    pub transcript: String,
    pub current_time: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeArticleAsset {
    pub title: String,
    pub url: String,
    pub content: String,
    pub text_content: String,
    pub site_name: String,
    pub selected_text: Option<String>,
    pub language: String,
    pub excerpt: String,
    pub length: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeTwitterAsset {
    pub url: String,
    pub title: String,
    pub tweets: Vec<NativeTwitterTweet>,
    pub timestamp: String,
}
