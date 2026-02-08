//! Generation output schema.
//!
//! This module contains the `Generation` and `GenerationChunk` types
//! which represent text generation outputs from LLMs.
//! Mirrors `langchain_core.outputs.generation`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::ops::Add;

use crate::utils::merge::merge_dicts;

/// A single text generation output.
///
/// Generation represents the response from an "old-fashioned" LLM (string-in,
/// string-out) that generates regular text (not chat messages).
///
/// This model is used internally by chat model and will eventually
/// be mapped to a more general `LLMResult` object, and then projected into
/// an `AIMessage` object.
///
/// LangChain users working with chat models will usually access information via
/// `AIMessage` (returned from runnable interfaces) or `LLMResult` (available
/// via callbacks). Please refer to `AIMessage` and `LLMResult` for more information.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Generation {
    /// Generated text output.
    pub text: String,

    /// Raw response from the provider.
    ///
    /// May include things like the reason for finishing or token log probabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_info: Option<HashMap<String, Value>>,

    /// Type is used exclusively for serialization purposes.
    ///
    /// Set to "Generation" for this class.
    #[serde(rename = "type", default = "default_generation_type")]
    pub generation_type: String,
}

fn default_generation_type() -> String {
    "Generation".to_string()
}

impl Generation {
    /// Create a new Generation with the given text.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            generation_info: None,
            generation_type: "Generation".to_string(),
        }
    }

    /// Create a new Generation with text and generation info.
    pub fn with_info(text: impl Into<String>, generation_info: HashMap<String, Value>) -> Self {
        Self {
            text: text.into(),
            generation_info: Some(generation_info),
            generation_type: "Generation".to_string(),
        }
    }

    /// Returns `true` as this class is serializable.
    pub fn is_lc_serializable() -> bool {
        true
    }

    /// Get the namespace of the LangChain object.
    ///
    /// Returns `["langchain", "schema", "output"]`
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

/// `GenerationChunk`, which can be concatenated with other Generation chunks.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GenerationChunk {
    /// Generated text output.
    pub text: String,

    /// Raw response from the provider.
    ///
    /// May include things like the reason for finishing or token log probabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_info: Option<HashMap<String, Value>>,

    /// Type is used exclusively for serialization purposes.
    ///
    /// Set to "Generation" for this class (inherited from Generation in Python).
    #[serde(rename = "type", default = "default_generation_type")]
    pub generation_type: String,
}

impl GenerationChunk {
    /// Create a new GenerationChunk with the given text.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            generation_info: None,
            generation_type: "Generation".to_string(),
        }
    }

    /// Create a new GenerationChunk with text and generation info.
    pub fn with_info(text: impl Into<String>, generation_info: HashMap<String, Value>) -> Self {
        Self {
            text: text.into(),
            generation_info: Some(generation_info),
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

    /// Concatenate two `GenerationChunk`s.
    ///
    /// Returns a new `GenerationChunk` concatenated from self and other.
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_generation_new() {
        let generation = Generation::new("Hello, world!");
        assert_eq!(generation.text, "Hello, world!");
        assert!(generation.generation_info.is_none());
        assert_eq!(generation.generation_type, "Generation");
    }

    #[test]
    fn test_generation_with_info() {
        let mut info = HashMap::new();
        info.insert("finish_reason".to_string(), json!("stop"));
        let generation = Generation::with_info("Hello", info.clone());
        assert_eq!(generation.text, "Hello");
        assert_eq!(generation.generation_info, Some(info));
    }

    #[test]
    fn test_generation_chunk_add() {
        let chunk1 = GenerationChunk::new("Hello, ");
        let chunk2 = GenerationChunk::new("world!");
        let result = chunk1 + chunk2;
        assert_eq!(result.text, "Hello, world!");
    }

    #[test]
    fn test_generation_chunk_add_with_info() {
        let mut info1 = HashMap::new();
        info1.insert("a".to_string(), json!(1));
        let chunk1 = GenerationChunk::with_info("Hello, ", info1);

        let mut info2 = HashMap::new();
        info2.insert("b".to_string(), json!(2));
        let chunk2 = GenerationChunk::with_info("world!", info2);

        let result = chunk1 + chunk2;
        assert_eq!(result.text, "Hello, world!");

        let info = result.generation_info.unwrap();
        assert_eq!(info.get("a"), Some(&json!(1)));
        assert_eq!(info.get("b"), Some(&json!(2)));
    }

    #[test]
    fn test_generation_serialization() {
        let generation = Generation::new("test");
        let json = serde_json::to_string(&generation).unwrap();
        assert!(json.contains("\"type\":\"Generation\""));

        let deserialized: Generation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.text, "test");
    }
}
