use agent_chain_core::messages::{ContentBlocks, PlainTextContentBlock};
use async_trait::async_trait;
use euro_native_messaging::NativeYoutubeAsset;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    ActivityResult,
    error::ActivityError,
    storage::SaveableAsset,
    types::{AssetFunctionality, ContextChip},
};

const YOUTUBE_EXTENSION_ID: &str = "7c7b59bb-d44d-431a-9f4d-64240172e092";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptLine {
    pub text: String,
    pub start: f32,
    pub duration: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct YoutubeAsset {
    pub id: String,
    pub url: String,
    pub title: String,
    pub transcript: Vec<TranscriptLine>,
    pub current_time: f32,
}

impl YoutubeAsset {
    pub fn new(
        id: String,
        url: String,
        title: String,
        transcript: Vec<TranscriptLine>,
        current_time: f32,
    ) -> Self {
        Self {
            id,
            url,
            title,
            transcript,
            current_time,
        }
    }

    pub fn try_from(asset: NativeYoutubeAsset) -> Result<Self, ActivityError> {
        let transcript = serde_json::from_str::<Vec<TranscriptLine>>(&asset.transcript)
            .map_err(ActivityError::from)?;

        Ok(YoutubeAsset {
            id: uuid::Uuid::new_v4().to_string(),
            url: asset.url,
            title: asset.title,
            transcript,
            current_time: asset.current_time,
        })
    }
}

impl AssetFunctionality for YoutubeAsset {
    fn get_name(&self) -> &str {
        &self.title
    }

    fn get_icon(&self) -> Option<&str> {
        Some("youtube")
    }

    fn construct_messages(&self) -> ContentBlocks {
        let asset_json = serde_json::to_string(&self).unwrap_or_default();

        let extras = HashMap::from([(
            "asset_id".to_string(),
            serde_json::json!(YOUTUBE_EXTENSION_ID),
        )]);

        let block = PlainTextContentBlock::builder()
            .context(format!(
                "Transcript of the YouTube video titled: '{}'",
                self.title
            ))
            .title(format!("{}.json", self.title))
            .mime_type("application/json".to_string())
            .text(asset_json)
            .extras(extras)
            .build();

        vec![block.into()].into()
    }

    fn get_context_chip(&self) -> Option<ContextChip> {
        let title: String = self.title.chars().take(50).collect();

        Some(ContextChip {
            id: self.id.clone(),
            name: title,
            extension_id: YOUTUBE_EXTENSION_ID.to_string(),
            attrs: HashMap::new(),
            icon: None,
            position: Some(0),
        })
    }

    fn get_id(&self) -> &str {
        &self.id
    }
}

#[async_trait]
impl SaveableAsset for YoutubeAsset {
    fn get_asset_type(&self) -> &'static str {
        "YoutubeAsset"
    }

    async fn serialize_content(&self) -> ActivityResult<Vec<u8>> {
        let json = serde_json::to_vec(self)?;
        Ok(json)
    }

    fn get_unique_id(&self) -> String {
        self.id.clone()
    }

    fn get_display_name(&self) -> String {
        self.title.clone()
    }
}

impl From<NativeYoutubeAsset> for YoutubeAsset {
    fn from(asset: NativeYoutubeAsset) -> Self {
        Self::try_from(asset).expect("Failed to convert NativeYoutubeAsset to YoutubeAsset")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_chain_core::messages::ContentBlock;

    #[test]
    fn test_youtube_asset_creation() {
        let transcript = vec![
            TranscriptLine {
                text: "Hello world".to_string(),
                start: 0.0,
                duration: 2.0,
            },
            TranscriptLine {
                text: "This is a test".to_string(),
                start: 2.0,
                duration: 3.0,
            },
        ];

        let asset = YoutubeAsset::new(
            "test-id".to_string(),
            "https://youtube.com/watch?v=test".to_string(),
            "Test Video".to_string(),
            transcript,
            1.5,
        );

        assert_eq!(asset.id, "test-id");
        assert_eq!(asset.title, "Test Video");
        assert_eq!(asset.current_time, 1.5);
        assert_eq!(asset.transcript.len(), 2);
    }

    #[test]
    fn test_context_chip() {
        let asset = YoutubeAsset::new(
            "test-id".to_string(),
            "https://youtube.com/watch?v=test".to_string(),
            "Test V".to_string(),
            vec![],
            0.0,
        );

        let chip = asset.get_context_chip().unwrap();
        assert_eq!(chip.id, "test-id");
        assert_eq!(chip.name, "Test V");
        assert_eq!(chip.extension_id, "7c7b59bb-d44d-431a-9f4d-64240172e092");
    }

    #[test]
    fn trait_methods_work() {
        use crate::types::AssetFunctionality;
        let asset = YoutubeAsset::new(
            "test-id".to_string(),
            "https://youtube.com/watch?v=test".to_string(),
            "Test Video".to_string(),
            vec![],
            0.0,
        );
        let blocks = AssetFunctionality::construct_messages(&asset);
        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0], ContentBlock::PlainText(_)));
        let chip = AssetFunctionality::get_context_chip(&asset);
        assert!(chip.is_some());
    }
}
