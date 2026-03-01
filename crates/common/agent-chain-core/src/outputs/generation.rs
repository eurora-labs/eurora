use bon::bon;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::ops::Add;

use crate::utils::merge::merge_dicts;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Generation {
    pub text: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_info: Option<HashMap<String, Value>>,

    #[serde(rename = "type", default = "default_generation_type")]
    pub generation_type: String,
}

fn default_generation_type() -> String {
    "Generation".to_string()
}

#[bon]
impl Generation {
    #[builder]
    pub fn new(text: impl Into<String>, generation_info: Option<HashMap<String, Value>>) -> Self {
        Self {
            text: text.into(),
            generation_info,
            generation_type: "Generation".to_string(),
        }
    }

    pub fn is_lc_serializable() -> bool {
        true
    }

    pub fn get_lc_namespace() -> Vec<&'static str> {
        vec!["langchain", "schema", "output"]
    }
}

impl Default for Generation {
    fn default() -> Self {
        Self {
            text: String::new(),
            generation_info: None,
            generation_type: "Generation".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenerationChunk {
    pub text: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_info: Option<HashMap<String, Value>>,

    #[serde(rename = "type", default = "default_generation_type")]
    pub generation_type: String,
}

#[bon]
impl GenerationChunk {
    #[builder]
    pub fn new(text: impl Into<String>, generation_info: Option<HashMap<String, Value>>) -> Self {
        Self {
            text: text.into(),
            generation_info,
            generation_type: "Generation".to_string(),
        }
    }
}

impl Default for GenerationChunk {
    fn default() -> Self {
        Self {
            text: String::new(),
            generation_info: None,
            generation_type: "Generation".to_string(),
        }
    }
}

impl Add for GenerationChunk {
    type Output = GenerationChunk;

    fn add(self, other: GenerationChunk) -> Self::Output {
        let generation_info = match (self.generation_info, other.generation_info) {
            (Some(left), Some(right)) => {
                let left_value =
                    serde_json::to_value(&left).unwrap_or(Value::Object(Default::default()));
                let right_value =
                    serde_json::to_value(&right).unwrap_or(Value::Object(Default::default()));
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
        };

        GenerationChunk {
            text: self.text + &other.text,
            generation_info,
            generation_type: "Generation".to_string(),
        }
    }
}

impl From<Generation> for GenerationChunk {
    fn from(generation: Generation) -> Self {
        GenerationChunk {
            text: generation.text,
            generation_info: generation.generation_info,
            generation_type: "Generation".to_string(),
        }
    }
}

impl From<GenerationChunk> for Generation {
    fn from(chunk: GenerationChunk) -> Self {
        Generation {
            text: chunk.text,
            generation_info: chunk.generation_info,
            generation_type: "Generation".to_string(),
        }
    }
}

pub fn merge_generation_chunks(chunks: Vec<GenerationChunk>) -> Option<GenerationChunk> {
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
    use serde_json::json;

    #[test]
    fn test_generation_new() {
        let generation = Generation::builder().text("Hello, world!").build();
        assert_eq!(generation.text, "Hello, world!");
        assert!(generation.generation_info.is_none());
        assert_eq!(generation.generation_type, "Generation");
    }

    #[test]
    fn test_generation_with_info() {
        let mut info = HashMap::new();
        info.insert("finish_reason".to_string(), json!("stop"));
        let generation = Generation::builder()
            .text("Hello")
            .generation_info(info.clone())
            .build();
        assert_eq!(generation.text, "Hello");
        assert_eq!(generation.generation_info, Some(info));
    }

    #[test]
    fn test_generation_chunk_add() {
        let chunk1 = GenerationChunk::builder().text("Hello, ").build();
        let chunk2 = GenerationChunk::builder().text("world!").build();
        let result = chunk1 + chunk2;
        assert_eq!(result.text, "Hello, world!");
    }

    #[test]
    fn test_generation_chunk_add_with_info() {
        let mut info1 = HashMap::new();
        info1.insert("a".to_string(), json!(1));
        let chunk1 = GenerationChunk::builder()
            .text("Hello, ")
            .generation_info(info1)
            .build();

        let mut info2 = HashMap::new();
        info2.insert("b".to_string(), json!(2));
        let chunk2 = GenerationChunk::builder()
            .text("world!")
            .generation_info(info2)
            .build();

        let result = chunk1 + chunk2;
        assert_eq!(result.text, "Hello, world!");

        let info = result.generation_info.unwrap();
        assert_eq!(info.get("a"), Some(&json!(1)));
        assert_eq!(info.get("b"), Some(&json!(2)));
    }

    #[test]
    fn test_generation_serialization() {
        let generation = Generation::builder().text("test").build();
        let json = serde_json::to_string(&generation).unwrap();
        assert!(json.contains("\"type\":\"Generation\""));

        let deserialized: Generation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.text, "test");
    }

    #[test]
    fn test_merge_generation_chunks_empty() {
        assert_eq!(merge_generation_chunks(vec![]), None);
    }

    #[test]
    fn test_merge_generation_chunks_single() {
        let chunk = GenerationChunk::builder().text("Hello").build();
        let result = merge_generation_chunks(vec![chunk.clone()]);
        assert_eq!(result, Some(chunk));
    }

    #[test]
    fn test_merge_generation_chunks_multiple() {
        let chunks = vec![
            GenerationChunk::builder().text("Hello").build(),
            GenerationChunk::builder().text(", ").build(),
            GenerationChunk::builder().text("world!").build(),
        ];
        let result = merge_generation_chunks(chunks).unwrap();
        assert_eq!(result.text, "Hello, world!");
    }

    #[test]
    fn test_merge_generation_chunks_with_info() {
        let mut info1 = HashMap::new();
        info1.insert("key1".to_string(), json!("val1"));
        let mut info2 = HashMap::new();
        info2.insert("key2".to_string(), json!("val2"));

        let chunks = vec![
            GenerationChunk::builder()
                .text("a")
                .generation_info(info1)
                .build(),
            GenerationChunk::builder()
                .text("b")
                .generation_info(info2)
                .build(),
        ];
        let result = merge_generation_chunks(chunks).unwrap();
        assert_eq!(result.text, "ab");
        let info = result.generation_info.unwrap();
        assert_eq!(info.get("key1"), Some(&json!("val1")));
        assert_eq!(info.get("key2"), Some(&json!("val2")));
    }
}
