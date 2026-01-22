//! Chat result schema.
//!
//! This module contains the `ChatResult` type which represents the result
//! of a chat model call with a single prompt.
//! Mirrors `langchain_core.outputs.chat_result`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::chat_generation::ChatGeneration;

/// Use to represent the result of a chat model call with a single prompt.
///
/// This container is used internally by some implementations of chat model,
/// it will eventually be mapped to a more general `LLMResult` object, and
/// then projected into an `AIMessage` object.
///
/// LangChain users working with chat models will usually access information via
/// `AIMessage` (returned from runnable interfaces) or `LLMResult` (available
/// via callbacks). Please refer to the `AIMessage` and `LLMResult` schema documentation
/// for more information.

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ChatResult {
    /// List of the chat generations.
    ///
    /// Generations is a list to allow for multiple candidate generations for a single
    /// input prompt.
    pub generations: Vec<ChatGeneration>,

    /// For arbitrary LLM provider specific output.
    ///
    /// This dictionary is a free-form dictionary that can contain any information that the
    /// provider wants to return. It is not standardized and is provider-specific.
    ///
    /// Users should generally avoid relying on this field and instead rely on
    /// accessing relevant information from standardized fields present in
    /// AIMessage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_output: Option<HashMap<String, Value>>,
}

impl ChatResult {
    /// Create a new ChatResult with the given generations.
    pub fn new(generations: Vec<ChatGeneration>) -> Self {
        Self {
            generations,
            llm_output: None,
        }
    }

    /// Create a new ChatResult with generations and LLM output.
    pub fn with_llm_output(
        generations: Vec<ChatGeneration>,
        llm_output: HashMap<String, Value>,
    ) -> Self {
        Self {
            generations,
            llm_output: Some(llm_output),
        }
    }

    /// Create a new ChatResult from a single generation.
    pub fn from_generation(generation: ChatGeneration) -> Self {
        Self {
            generations: vec![generation],
            llm_output: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::AIMessage;
    use serde_json::json;

    #[test]
    fn test_chat_result_new() {
        let msg = AIMessage::new("Hello");
        let chat_gen = ChatGeneration::new(msg.into());
        let result = ChatResult::new(vec![chat_gen]);
        assert_eq!(result.generations.len(), 1);
        assert!(result.llm_output.is_none());
    }

    #[test]
    fn test_chat_result_with_llm_output() {
        let msg = AIMessage::new("Hello");
        let chat_gen = ChatGeneration::new(msg.into());
        let mut output = HashMap::new();
        output.insert("model".to_string(), json!("gpt-4"));
        let result = ChatResult::with_llm_output(vec![chat_gen], output.clone());
        assert_eq!(result.generations.len(), 1);
        assert_eq!(result.llm_output, Some(output));
    }

    #[test]
    fn test_chat_result_from_generation() {
        let msg = AIMessage::new("Hello");
        let chat_gen = ChatGeneration::new(msg.into());
        let result = ChatResult::from_generation(chat_gen);
        assert_eq!(result.generations.len(), 1);
    }

    #[test]
    fn test_chat_result_serialization() {
        let msg = AIMessage::new("test");
        let chat_gen = ChatGeneration::new(msg.into());
        let result = ChatResult::new(vec![chat_gen]);
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ChatResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.generations.len(), 1);
    }
}
