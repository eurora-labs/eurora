//! LLMResult class.
//!
//! This module contains the `LLMResult` type which is a container
//! for results of an LLM call.
//! Mirrors `langchain_core.outputs.llm_result`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::chat_generation::{ChatGeneration, ChatGenerationChunk};
use super::generation::{Generation, GenerationChunk};
use super::run_info::RunInfo;

/// Enum representing different types of generations.
///
/// This allows LLMResult to hold different generation types.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum GenerationType {
    /// A standard text generation.
    Generation(Generation),
    /// A text generation chunk.
    GenerationChunk(GenerationChunk),
    /// A chat generation.
    ChatGeneration(ChatGeneration),
    /// A chat generation chunk.
    ChatGenerationChunk(ChatGenerationChunk),
}

impl From<Generation> for GenerationType {
    fn from(generation: Generation) -> Self {
        GenerationType::Generation(generation)
    }
}

impl From<GenerationChunk> for GenerationType {
    fn from(generation: GenerationChunk) -> Self {
        GenerationType::GenerationChunk(generation)
    }
}

impl From<ChatGeneration> for GenerationType {
    fn from(generation: ChatGeneration) -> Self {
        GenerationType::ChatGeneration(generation)
    }
}

impl From<ChatGenerationChunk> for GenerationType {
    fn from(generation: ChatGenerationChunk) -> Self {
        GenerationType::ChatGenerationChunk(generation)
    }
}

/// A container for results of an LLM call.
///
/// Both chat models and LLMs generate an LLMResult object. This object contains the
/// generated outputs and any additional information that the model provider wants to
/// return.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResult {
    /// Generated outputs.
    ///
    /// The first dimension of the list represents completions for different input prompts.
    ///
    /// The second dimension of the list represents different candidate generations for a
    /// given prompt.
    ///
    /// - When returned from **an LLM**, the type is `list[list[Generation]]`.
    /// - When returned from a **chat model**, the type is `list[list[ChatGeneration]]`.
    ///
    /// ChatGeneration is a subclass of Generation that has a field for a structured chat
    /// message.
    pub generations: Vec<Vec<GenerationType>>,

    /// For arbitrary LLM provider specific output.
    ///
    /// This dictionary is a free-form dictionary that can contain any information that the
    /// provider wants to return. It is not standardized and is provider-specific.
    ///
    /// Users should generally avoid relying on this field and instead rely on accessing
    /// relevant information from standardized fields present in AIMessage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_output: Option<HashMap<String, Value>>,

    /// List of metadata info for model call for each input.
    ///
    /// See `RunInfo` for details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<Vec<RunInfo>>,

    /// Type is used exclusively for serialization purposes.
    #[serde(rename = "type", default = "default_llm_result_type")]
    pub result_type: String,
}

fn default_llm_result_type() -> String {
    "LLMResult".to_string()
}

impl LLMResult {
    /// Create a new LLMResult with the given generations.
    pub fn new(generations: Vec<Vec<GenerationType>>) -> Self {
        Self {
            generations,
            llm_output: None,
            run: None,
            result_type: "LLMResult".to_string(),
        }
    }

    /// Create a new LLMResult with generations and LLM output.
    pub fn with_llm_output(
        generations: Vec<Vec<GenerationType>>,
        llm_output: HashMap<String, Value>,
    ) -> Self {
        Self {
            generations,
            llm_output: Some(llm_output),
            run: None,
            result_type: "LLMResult".to_string(),
        }
    }

    /// Flatten generations into a single list.
    ///
    /// Unpack list\[list\[Generation\]\] -> list\[LLMResult\] where each returned LLMResult
    /// contains only a single Generation. If token usage information is available,
    /// it is kept only for the LLMResult corresponding to the top-choice
    /// Generation, to avoid over-counting of token usage downstream.
    ///
    /// Returns a list of LLMResults where each returned LLMResult contains a single
    /// Generation.
    pub fn flatten(&self) -> Vec<LLMResult> {
        let mut llm_results = Vec::new();

        for (i, gen_list) in self.generations.iter().enumerate() {
            // Avoid double counting tokens in OpenAICallback
            if i == 0 {
                llm_results.push(LLMResult {
                    generations: vec![gen_list.clone()],
                    llm_output: self.llm_output.clone(),
                    run: None,
                    result_type: "LLMResult".to_string(),
                });
            } else {
                let llm_output = if let Some(ref output) = self.llm_output {
                    let mut cloned = output.clone();
                    cloned.insert("token_usage".to_string(), Value::Object(Default::default()));
                    Some(cloned)
                } else {
                    None
                };
                llm_results.push(LLMResult {
                    generations: vec![gen_list.clone()],
                    llm_output,
                    run: None,
                    result_type: "LLMResult".to_string(),
                });
            }
        }

        llm_results
    }
}

impl PartialEq for LLMResult {
    fn eq(&self, other: &Self) -> bool {
        self.generations == other.generations
            && self.llm_output == other.llm_output
            && self.result_type == other.result_type
    }
}

impl Default for LLMResult {
    fn default() -> Self {
        Self {
            generations: Vec::new(),
            llm_output: None,
            run: None,
            result_type: "LLMResult".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::AIMessage;
    use serde_json::json;

    #[test]
    fn test_llm_result_new() {
        let generation = Generation::new("Hello");
        let result = LLMResult::new(vec![vec![generation.into()]]);
        assert_eq!(result.generations.len(), 1);
        assert_eq!(result.generations[0].len(), 1);
        assert!(result.llm_output.is_none());
    }

    #[test]
    fn test_llm_result_with_chat_generation() {
        let msg = AIMessage::builder().content("Hello").build();
        let chat_gen = ChatGeneration::new(msg.into());
        let result = LLMResult::new(vec![vec![chat_gen.into()]]);
        assert_eq!(result.generations.len(), 1);
    }

    #[test]
    fn test_llm_result_flatten() {
        let generation1 = Generation::new("First");
        let generation2 = Generation::new("Second");
        let mut output = HashMap::new();
        output.insert("token_usage".to_string(), json!({"total": 100}));
        let result = LLMResult::with_llm_output(
            vec![vec![generation1.into()], vec![generation2.into()]],
            output,
        );

        let flattened = result.flatten();
        assert_eq!(flattened.len(), 2);

        // First result should have the original llm_output
        assert!(flattened[0].llm_output.is_some());
        let first_output = flattened[0].llm_output.as_ref().unwrap();
        assert_eq!(
            first_output.get("token_usage"),
            Some(&json!({"total": 100}))
        );

        // Second result should have empty token_usage
        assert!(flattened[1].llm_output.is_some());
        let second_output = flattened[1].llm_output.as_ref().unwrap();
        assert_eq!(second_output.get("token_usage"), Some(&json!({})));
    }

    #[test]
    fn test_llm_result_equality() {
        let generation1 = Generation::new("Hello");
        let generation2 = Generation::new("Hello");
        let result1 = LLMResult::new(vec![vec![generation1.into()]]);
        let result2 = LLMResult::new(vec![vec![generation2.into()]]);
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_llm_result_serialization() {
        let generation = Generation::new("test");
        let result = LLMResult::new(vec![vec![generation.into()]]);
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"type\":\"LLMResult\""));
    }
}
