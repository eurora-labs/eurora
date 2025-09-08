//! YouTube asset implementation

use crate::ActivityResult;
use crate::error::ActivityError;
use crate::storage::SaveableAsset;
use crate::types::{AssetFunctionality, ContextChip};
use async_trait::async_trait;
use eur_proto::ipc::ProtoYoutubeState;
use ferrous_llm_core::{Message, MessageContent, Role};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Transcript line for YouTube videos
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptLine {
    pub text: String,
    pub start: f32,
    pub duration: f32,
}

/// YouTube video asset with transcript and metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct YoutubeAsset {
    pub id: String,
    pub url: String,
    pub title: String,
    pub transcript: Vec<TranscriptLine>,
    pub current_time: f32,
}

impl YoutubeAsset {
    /// Create a new YouTube asset
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

    /// Try to create from protocol buffer state
    pub fn try_from(state: ProtoYoutubeState) -> Result<Self, ActivityError> {
        Ok(YoutubeAsset {
            id: uuid::Uuid::new_v4().to_string(),
            url: state.url,
            title: "YouTube Video".to_string(),
            transcript: state
                .transcript
                .into_iter()
                .map(|line| TranscriptLine {
                    text: line.text,
                    start: line.start,
                    duration: line.duration,
                })
                .collect(),
            current_time: state.current_time,
        })
    }

    /// Get transcript text at a specific time
    pub fn get_transcript_at_time(&self, time: f32) -> Option<&str> {
        self.transcript
            .iter()
            .find(|line| line.start <= time && time < line.start + line.duration)
            .map(|line| line.text.as_str())
    }

    /// Get all transcript text as a single string
    pub fn get_full_transcript(&self) -> String {
        self.transcript
            .iter()
            .map(|line| line.text.clone())
            .collect::<Vec<String>>()
            .join(" ")
    }
}

impl AssetFunctionality for YoutubeAsset {
    fn get_name(&self) -> &str {
        &self.title
    }

    fn get_icon(&self) -> Option<&str> {
        Some("youtube")
    }

    /// Construct a message for LLM interaction
    fn construct_message(&self) -> Message {
        Message {
            role: Role::User,
            content: MessageContent::Text(format!(
                "I am watching a YouTube video titled '{}' and have a question about it. \
                 Here's the transcript of the video: \n {}",
                self.title,
                self.transcript
                    .iter()
                    .map(|line| format!("{} ({}s)", line.text, line.start))
                    .collect::<Vec<_>>()
                    .join("\n")
            )),
        }
    }

    /// Get context chip for UI integration
    fn get_context_chip(&self) -> Option<ContextChip> {
        Some(ContextChip {
            id: self.id.clone(),
            name: "video".to_string(),
            extension_id: "7c7b59bb-d44d-431a-9f4d-64240172e092".to_string(),
            attrs: HashMap::new(),
            icon: None,
            position: Some(0),
        })
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

impl From<ProtoYoutubeState> for YoutubeAsset {
    fn from(state: ProtoYoutubeState) -> Self {
        Self::try_from(state).expect("Failed to convert ProtoYoutubeState to YoutubeAsset")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_transcript_at_time() {
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

        assert_eq!(asset.get_transcript_at_time(1.0), Some("Hello world"));
        assert_eq!(asset.get_transcript_at_time(3.0), Some("This is a test"));
        assert_eq!(asset.get_transcript_at_time(10.0), None);
    }

    #[test]
    fn test_full_transcript() {
        let transcript = vec![
            TranscriptLine {
                text: "Hello".to_string(),
                start: 0.0,
                duration: 1.0,
            },
            TranscriptLine {
                text: "world".to_string(),
                start: 1.0,
                duration: 1.0,
            },
        ];

        let asset = YoutubeAsset::new(
            "test-id".to_string(),
            "https://youtube.com/watch?v=test".to_string(),
            "Test Video".to_string(),
            transcript,
            0.0,
        );

        assert_eq!(asset.get_full_transcript(), "Hello world");
    }

    #[test]
    fn test_context_chip() {
        let asset = YoutubeAsset::new(
            "test-id".to_string(),
            "https://youtube.com/watch?v=test".to_string(),
            "Test Video".to_string(),
            vec![],
            0.0,
        );

        let chip = asset.get_context_chip().unwrap();
        assert_eq!(chip.id, "test-id");
        assert_eq!(chip.name, "video");
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
        let msg = AssetFunctionality::construct_message(&asset);
        let chip = AssetFunctionality::get_context_chip(&asset);
        assert!(matches!(msg.content, MessageContent::Text(_)));
        assert!(chip.is_some());
    }
}
