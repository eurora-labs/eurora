use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::chat_generation::{ChatGeneration, ChatGenerationChunk};
use super::generation::{Generation, GenerationChunk};
use super::run_info::RunInfo;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum GenerationType {
    Generation(Generation),
    GenerationChunk(GenerationChunk),
    ChatGeneration(ChatGeneration),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResult {
    pub generations: Vec<Vec<GenerationType>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_output: Option<HashMap<String, Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<Vec<RunInfo>>,

    #[serde(rename = "type", default = "default_llm_result_type")]
    pub result_type: String,
}

fn default_llm_result_type() -> String {
    "LLMResult".to_string()
}

impl LLMResult {
    pub fn new(generations: Vec<Vec<GenerationType>>) -> Self {
        Self {
            generations,
            llm_output: None,
            run: None,
            result_type: "LLMResult".to_string(),
        }
    }

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

    pub fn flatten(&self) -> Vec<LLMResult> {
        let mut llm_results = Vec::new();

        for (i, gen_list) in self.generations.iter().enumerate() {
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

        assert!(flattened[0].llm_output.is_some());
        let first_output = flattened[0].llm_output.as_ref().unwrap();
        assert_eq!(
            first_output.get("token_usage"),
            Some(&json!({"total": 100}))
        );

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
