use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use specta::Type;

// message ProtoNativeArticleAsset {
//     string type = 1;
//     string content = 2;
//     string text_content = 3;
//     optional string selected_text = 4;

//     string title = 5;
//     string site_name = 6;
//     string language = 7;
//     string excerpt = 8;

//     int32 length = 9;
// }

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeYoutubeAsset {
    pub url: String,
    pub title: String,
    pub transcript: String,
    pub current_time: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeArticleAsset {
    pub content: String,
    pub text_content: String,
    pub selected_text: Option<String>,
    pub title: String,
    pub site_name: String,
    pub language: String,
    pub excerpt: String,
    pub length: i32,
}

#[enum_dispatch]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum NativeAsset {
    NativeYoutubeAsset,
    NativeArticleAsset,
}
