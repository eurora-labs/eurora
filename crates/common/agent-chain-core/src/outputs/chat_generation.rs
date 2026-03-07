use bon::bon;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::ops::Add;

use crate::load::Serializable;
use crate::messages::AnyMessage;

pub const CHAT_GENERATION_TYPE: &str = "ChatGeneration";
pub const CHAT_GENERATION_CHUNK_TYPE: &str = "ChatGenerationChunk";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatGeneration {
    pub message: AnyMessage,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_info: Option<HashMap<String, Value>>,

    #[serde(rename = "type", default = "default_chat_generation_type")]
    pub generation_type: String,
}

fn default_chat_generation_type() -> String {
    CHAT_GENERATION_TYPE.to_string()
}

#[bon]
impl ChatGeneration {
    #[builder]
    pub fn new(message: AnyMessage, generation_info: Option<HashMap<String, Value>>) -> Self {
        Self {
            message,
            generation_info,
            generation_type: CHAT_GENERATION_TYPE.to_string(),
        }
    }
}

impl Serializable for ChatGeneration {
    fn is_lc_serializable() -> bool {
        true
    }

    fn get_lc_namespace() -> Vec<String> {
        vec!["langchain".into(), "schema".into(), "output".into()]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatGenerationChunk {
    pub message: AnyMessage,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_info: Option<HashMap<String, Value>>,

    #[serde(rename = "type", default = "default_chat_generation_chunk_type")]
    pub generation_type: String,
}

fn default_chat_generation_chunk_type() -> String {
    CHAT_GENERATION_CHUNK_TYPE.to_string()
}

#[bon]
impl ChatGenerationChunk {
    #[builder]
    pub fn new(message: AnyMessage, generation_info: Option<HashMap<String, Value>>) -> Self {
        Self {
            message,
            generation_info,
            generation_type: CHAT_GENERATION_CHUNK_TYPE.to_string(),
        }
    }
}

impl Serializable for ChatGenerationChunk {
    fn is_lc_serializable() -> bool {
        true
    }

    fn get_lc_namespace() -> Vec<String> {
        vec!["langchain".into(), "schema".into(), "output".into()]
    }
}

impl Add for ChatGenerationChunk {
    type Output = ChatGenerationChunk;

    fn add(self, other: ChatGenerationChunk) -> Self::Output {
        let generation_info =
            super::merge_generation_info(self.generation_info, other.generation_info);

        let self_chunk = crate::messages::utils::msg_to_chunk(&self.message);
        let other_chunk = crate::messages::utils::msg_to_chunk(&other.message);
        let merged_chunk = self_chunk + other_chunk;
        let merged_message = crate::messages::utils::chunk_to_msg(&merged_chunk);

        ChatGenerationChunk {
            message: merged_message,
            generation_info,
            generation_type: CHAT_GENERATION_CHUNK_TYPE.to_string(),
        }
    }
}

impl From<ChatGeneration> for ChatGenerationChunk {
    fn from(chat_gen: ChatGeneration) -> Self {
        ChatGenerationChunk {
            message: chat_gen.message,
            generation_info: chat_gen.generation_info,
            generation_type: CHAT_GENERATION_CHUNK_TYPE.to_string(),
        }
    }
}

impl From<ChatGenerationChunk> for ChatGeneration {
    fn from(chunk: ChatGenerationChunk) -> Self {
        ChatGeneration {
            message: chunk.message,
            generation_info: chunk.generation_info,
            generation_type: CHAT_GENERATION_TYPE.to_string(),
        }
    }
}

pub fn merge_chat_generation_chunks(
    chunks: Vec<ChatGenerationChunk>,
) -> Option<ChatGenerationChunk> {
    let mut iter = chunks.into_iter();
    let first = iter.next()?;
    Some(iter.fold(first, |acc, chunk| acc + chunk))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::AIMessage;
    use serde_json::json;

    #[test]
    fn test_chat_generation_new() {
        let msg = AIMessage::builder().content("Hello, world!").build();
        let chat_gen = ChatGeneration::builder().message(msg.into()).build();
        assert_eq!(chat_gen.message.text(), "Hello, world!");
        assert!(chat_gen.generation_info.is_none());
        assert_eq!(chat_gen.generation_type, CHAT_GENERATION_TYPE);
    }

    #[test]
    fn test_chat_generation_with_info() {
        let msg = AIMessage::builder().content("Hello").build();
        let mut info = HashMap::new();
        info.insert("finish_reason".to_string(), json!("stop"));
        let chat_gen = ChatGeneration::builder()
            .message(msg.into())
            .generation_info(info.clone())
            .build();
        assert_eq!(chat_gen.message.text(), "Hello");
        assert_eq!(chat_gen.generation_info, Some(info));
    }

    #[test]
    fn test_chat_generation_chunk_add() {
        let msg1 = AIMessage::builder().content("Hello, ").build();
        let msg2 = AIMessage::builder().content("world!").build();
        let chunk1 = ChatGenerationChunk::builder().message(msg1.into()).build();
        let chunk2 = ChatGenerationChunk::builder().message(msg2.into()).build();
        let result = chunk1 + chunk2;
        assert_eq!(result.message.text(), "Hello, world!");
    }

    #[test]
    fn test_merge_chat_generation_chunks_empty() {
        let result = merge_chat_generation_chunks(vec![]);
        assert!(result.is_none());
    }

    #[test]
    fn test_merge_chat_generation_chunks_single() {
        let msg = AIMessage::builder().content("Hello").build();
        let chunk = ChatGenerationChunk::builder().message(msg.into()).build();
        let result = merge_chat_generation_chunks(vec![chunk.clone()]);
        assert!(result.is_some());
        assert_eq!(result.unwrap().message.text(), "Hello");
    }

    #[test]
    fn test_merge_chat_generation_chunks_multiple() {
        let msg1 = AIMessage::builder().content("Hello, ").build();
        let msg2 = AIMessage::builder().content("world").build();
        let msg3 = AIMessage::builder().content("!").build();
        let chunks = vec![
            ChatGenerationChunk::builder().message(msg1.into()).build(),
            ChatGenerationChunk::builder().message(msg2.into()).build(),
            ChatGenerationChunk::builder().message(msg3.into()).build(),
        ];
        let result = merge_chat_generation_chunks(chunks);
        assert!(result.is_some());
        assert_eq!(result.unwrap().message.text(), "Hello, world!");
    }
}
