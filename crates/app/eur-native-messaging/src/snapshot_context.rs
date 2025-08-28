use anyhow::{Context, Result};
use base64::prelude::*;
pub use eur_proto::{
    ipc::{ProtoArticleSnapshot, ProtoTwitterSnapshot, ProtoYoutubeSnapshot},
    native_messaging::{ProtoNativeTwitterSnapshot, ProtoNativeYoutubeSnapshot},
    shared::ProtoImage,
};
use tracing::info;

pub struct ArticleSnapshot(pub ProtoArticleSnapshot);

pub struct NativeArticleSnapshot(pub ArticleSnapshot);

impl From<&serde_json::Map<String, serde_json::Value>> for NativeArticleSnapshot {
    fn from(obj: &serde_json::Map<String, serde_json::Value>) -> Self {
        info!("NativeArticleSnapshot::from obj: {:?}", obj);
        let highlighted_content = obj
            .get("highlightedText")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        NativeArticleSnapshot(ArticleSnapshot(ProtoArticleSnapshot {
            highlighted_content,
        }))
    }
}

impl TryFrom<&NativeArticleSnapshot> for ArticleSnapshot {
    type Error = anyhow::Error;

    fn try_from(obj: &NativeArticleSnapshot) -> Result<Self> {
        Ok(ArticleSnapshot(ProtoArticleSnapshot {
            highlighted_content: obj.0.0.highlighted_content.clone(),
        }))
    }
}

// Wrapper type for ProtoYoutubeSnapshot
pub struct YoutubeSnapshot(pub ProtoYoutubeSnapshot);

// Wrapper type for ProtoNativeYoutubeSnapshot
pub struct NativeYoutubeSnapshot(pub ProtoNativeYoutubeSnapshot);

impl From<&serde_json::Map<String, serde_json::Value>> for NativeYoutubeSnapshot {
    fn from(obj: &serde_json::Map<String, serde_json::Value>) -> Self {
        info!("NativeYoutubeSnapshot::from obj: {:?}", obj);
        NativeYoutubeSnapshot(ProtoNativeYoutubeSnapshot {
            r#type: obj.get("type").unwrap().as_str().unwrap().to_string(),
            current_time: obj.get("currentTime").unwrap().as_f64().unwrap() as f32,
            video_frame_base64: obj
                .get("videoFrameBase64")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
            video_frame_width: obj.get("videoFrameWidth").unwrap().as_i64().unwrap() as i32,
            video_frame_height: obj.get("videoFrameHeight").unwrap().as_i64().unwrap() as i32,
            video_frame_format: obj.get("videoFrameFormat").unwrap().as_i64().unwrap() as i32,
        })
    }
}

impl TryFrom<&NativeYoutubeSnapshot> for YoutubeSnapshot {
    type Error = anyhow::Error;

    fn try_from(obj: &NativeYoutubeSnapshot) -> Result<Self> {
        let video_frame_data = BASE64_STANDARD
            .decode(obj.0.video_frame_base64.as_str())
            .with_context(|| {
                format!(
                    "Failed to decode base64 video frame data in snapshot: '{}'",
                    obj.0
                        .video_frame_base64
                        .chars()
                        .take(50)
                        .collect::<String>()
                )
            })?;

        Ok(YoutubeSnapshot(ProtoYoutubeSnapshot {
            current_time: obj.0.current_time,
            video_frame: Some(ProtoImage {
                data: video_frame_data,
                width: obj.0.video_frame_width,
                height: obj.0.video_frame_height,
                format: obj.0.video_frame_format,
            }),
        }))
    }
}

// Twitter snapshot types - similar to article snapshots for now
pub struct TwitterSnapshot(pub ProtoTwitterSnapshot);

pub struct NativeTwitterSnapshot(pub ProtoNativeTwitterSnapshot);

impl From<&serde_json::Map<String, serde_json::Value>> for NativeTwitterSnapshot {
    fn from(obj: &serde_json::Map<String, serde_json::Value>) -> Self {
        info!("NativeTwitterSnapshot::from obj: {:?}", obj);
        NativeTwitterSnapshot(ProtoNativeTwitterSnapshot {
            r#type: obj.get("type").unwrap().as_str().unwrap().to_string(),
            tweets: obj.get("tweets").unwrap().as_str().unwrap().to_string(),
            timestamp: obj.get("timestamp").unwrap().as_str().unwrap().to_string(),
        })
    }
}

impl TryFrom<&NativeTwitterSnapshot> for TwitterSnapshot {
    type Error = anyhow::Error;

    fn try_from(obj: &NativeTwitterSnapshot) -> Result<Self> {
        Ok(TwitterSnapshot(ProtoTwitterSnapshot {
            tweets: serde_json::from_str(&obj.0.tweets).unwrap_or_default(),
            timestamp: obj.0.timestamp.clone(),
        }))
    }
}
