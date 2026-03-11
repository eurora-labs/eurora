use serde::{Deserialize, Serialize};
use specta::Type;

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
pub struct NativeArticleSnapshot {
    pub highlighted_text: Option<String>,
}
