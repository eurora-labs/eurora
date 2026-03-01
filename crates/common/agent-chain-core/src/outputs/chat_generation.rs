use bon::bon;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::ops::Add;

use crate::messages::BaseMessage;
use crate::utils::merge::merge_dicts;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatGeneration {
    #[serde(default)]
    pub text: String,

    pub message: BaseMessage,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_info: Option<HashMap<String, Value>>,

    #[serde(rename = "type", default = "default_chat_generation_type")]
    pub generation_type: String,
}

fn default_chat_generation_type() -> String {
    "ChatGeneration".to_string()
}

#[bon]
impl ChatGeneration {
    #[builder]
    pub fn new(
        message: BaseMessage,
        generation_info: Option<HashMap<String, Value>>,
    ) -> Self {
        let text = extract_text_from_message(&message);
        Self {
            text,
            message,
            generation_info,
            generation_type: "ChatGeneration".to_string(),
        }
    }

    pub fn is_lc_serializable() -> bool {
        true
    }

    pub fn get_lc_namespace() -> Vec<&'static str> {
        vec!["langchain", "schema", "output"]
    }
}

fn extract_text_from_message(message: &BaseMessage) -> String {
    let content = message.content();

    let blocks: Option<Vec<Value>> = match content {
        crate::messages::content::MessageContent::Parts(_) => Some(content.as_json_values()),
        crate::messages::content::MessageContent::Text(s) => serde_json::from_str(s).ok(),
    };

    if let Some(blocks) = blocks {
        for block in &blocks {
            if let Some(s) = block.as_str() {
                return s.to_string();
            }
            if let Some(obj) = block.as_object()
                && let Some(Value::String(text)) = obj.get("text")
            {
                return text.clone();
            }
        }
        return String::new();
    }

    content.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatGenerationChunk {
    #[serde(default)]
    pub text: String,

    pub message: BaseMessage,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_info: Option<HashMap<String, Value>>,

    #[serde(rename = "type", default = "default_chat_generation_chunk_type")]
    pub generation_type: String,
}

fn default_chat_generation_chunk_type() -> String {
    "ChatGenerationChunk".to_string()
}

#[bon]
impl ChatGenerationChunk {
    #[builder]
    pub fn new(
        message: BaseMessage,
        generation_info: Option<HashMap<String, Value>>,
    ) -> Self {
        let text = extract_text_from_message(&message);
        Self {
            text,
            message,
            generation_info,
            generation_type: "ChatGenerationChunk".to_string(),
        }
    }
}

impl Add for ChatGenerationChunk {
    type Output = ChatGenerationChunk;

    fn add(self, other: ChatGenerationChunk) -> Self::Output {
        let generation_info = merge_generation_info(self.generation_info, other.generation_info);

        let self_chunk = crate::messages::utils::msg_to_chunk(&self.message);
        let other_chunk = crate::messages::utils::msg_to_chunk(&other.message);
        let merged_chunk = self_chunk + other_chunk;
        let merged_message = crate::messages::utils::chunk_to_msg(&merged_chunk);
        let text = extract_text_from_message(&merged_message);

        ChatGenerationChunk {
            text,
            message: merged_message,
            generation_info,
            generation_type: "ChatGenerationChunk".to_string(),
        }
    }
}

fn merge_generation_info(
    left: Option<HashMap<String, Value>>,
    right: Option<HashMap<String, Value>>,
) -> Option<HashMap<String, Value>> {
    match (left, right) {
        (Some(left_map), Some(right_map)) => {
            let left_value =
                serde_json::to_value(&left_map).unwrap_or(Value::Object(Default::default()));
            let right_value =
                serde_json::to_value(&right_map).unwrap_or(Value::Object(Default::default()));
            match merge_dicts(left_value, vec![right_value]) {
                Ok(Value::Object(map)) => {
                    let result: HashMap<String, Value> = map.into_iter().collect();
                    if result.is_empty() {
                        None
                    } else {
                        Some(result)
                    }
                }
                _ => None,
            }
        }
        (Some(info), None) | (None, Some(info)) => Some(info),
        (None, None) => None,
    }
}

impl From<ChatGeneration> for ChatGenerationChunk {
    fn from(chat_gen: ChatGeneration) -> Self {
        ChatGenerationChunk {
            text: chat_gen.text,
            message: chat_gen.message,
            generation_info: chat_gen.generation_info,
            generation_type: "ChatGenerationChunk".to_string(),
        }
    }
}

impl From<ChatGenerationChunk> for ChatGeneration {
    fn from(chunk: ChatGenerationChunk) -> Self {
        ChatGeneration {
            text: chunk.text,
            message: chunk.message,
            generation_info: chunk.generation_info,
            generation_type: "ChatGeneration".to_string(),
        }
    }
}

pub fn merge_chat_generation_chunks(
    chunks: Vec<ChatGenerationChunk>,
) -> Option<ChatGenerationChunk> {
    if chunks.is_empty() {
        return None;
    }

    if chunks.len() == 1 {
        return chunks.into_iter().next();
    }

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
        let chat_gen = ChatGeneration::new(msg.into());
        assert_eq!(chat_gen.text, "Hello, world!");
        assert!(chat_gen.generation_info.is_none());
        assert_eq!(chat_gen.generation_type, "ChatGeneration");
    }

    #[test]
    fn test_chat_generation_with_info() {
        let msg = AIMessage::builder().content("Hello").build();
        let mut info = HashMap::new();
        info.insert("finish_reason".to_string(), json!("stop"));
        let chat_gen = ChatGeneration::with_info(msg.into(), info.clone());
        assert_eq!(chat_gen.text, "Hello");
        assert_eq!(chat_gen.generation_info, Some(info));
    }

    #[test]
    fn test_chat_generation_chunk_add() {
        let msg1 = AIMessage::builder().content("Hello, ").build();
        let msg2 = AIMessage::builder().content("world!").build();
        let chunk1 = ChatGenerationChunk::new(msg1.into());
        let chunk2 = ChatGenerationChunk::new(msg2.into());
        let result = chunk1 + chunk2;
        assert_eq!(result.text, "Hello, world!");
    }

    #[test]
    fn test_merge_chat_generation_chunks_empty() {
        let result = merge_chat_generation_chunks(vec![]);
        assert!(result.is_none());
    }

    #[test]
    fn test_merge_chat_generation_chunks_single() {
        let msg = AIMessage::builder().content("Hello").build();
        let chunk = ChatGenerationChunk::new(msg.into());
        let result = merge_chat_generation_chunks(vec![chunk.clone()]);
        assert!(result.is_some());
        assert_eq!(result.unwrap().text, "Hello");
    }

    #[test]
    fn test_merge_chat_generation_chunks_multiple() {
        let msg1 = AIMessage::builder().content("Hello, ").build();
        let msg2 = AIMessage::builder().content("world").build();
        let msg3 = AIMessage::builder().content("!").build();
        let chunks = vec![
            ChatGenerationChunk::new(msg1.into()),
            ChatGenerationChunk::new(msg2.into()),
            ChatGenerationChunk::new(msg3.into()),
        ];
        let result = merge_chat_generation_chunks(chunks);
        assert!(result.is_some());
        assert_eq!(result.unwrap().text, "Hello, world!");
    }
}
