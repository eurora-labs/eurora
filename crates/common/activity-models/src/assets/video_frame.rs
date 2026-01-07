use super::AssetFunctionality;
use agent_chain_core::{BaseMessage, HumanMessage};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Transcript line for YouTube videos
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptLine {
    pub text: String,
    pub start: f32,
    pub duration: f32,
}

/// YouTube video asset with transcript and metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VideoTranscript {
    pub title: String,
    pub transcript: Vec<TranscriptLine>,
}

impl VideoTranscript {
    /// Get all transcript text as a single string
    pub fn flatten_transcript(&self) -> String {
        self.transcript
            .iter()
            .map(|line| line.text.clone())
            .collect::<Vec<String>>()
            .join(" ")
    }
}

impl AssetFunctionality for VideoTranscript {
    /// Construct a message for LLM interaction
    fn construct_messages(&self) -> Vec<BaseMessage> {
        let content = format!(
            "I am watching a video titled '{}' and have a question about it. \
             Here's the transcript of the video: \n {}",
            self.title,
            self.flatten_transcript()
        );
        vec![HumanMessage::new(content).into()]
    }

    fn get_context_card(&self) -> Value {
        todo!()
    }
}
