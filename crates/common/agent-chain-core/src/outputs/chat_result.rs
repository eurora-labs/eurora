use bon::bon;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::chat_generation::ChatGeneration;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ChatResult {
    pub generations: Vec<ChatGeneration>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_output: Option<HashMap<String, Value>>,
}

#[bon]
impl ChatResult {
    #[builder]
    pub fn new(
        generations: Vec<ChatGeneration>,
        llm_output: Option<HashMap<String, Value>>,
    ) -> Self {
        Self {
            generations,
            llm_output,
        }
    }

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
        let msg = AIMessage::builder().content("Hello").build();
        let chat_gen = ChatGeneration::new(msg.into());
        let result = ChatResult::new(vec![chat_gen]);
        assert_eq!(result.generations.len(), 1);
        assert!(result.llm_output.is_none());
    }

    #[test]
    fn test_chat_result_with_llm_output() {
        let msg = AIMessage::builder().content("Hello").build();
        let chat_gen = ChatGeneration::new(msg.into());
        let mut output = HashMap::new();
        output.insert("model".to_string(), json!("gpt-4"));
        let result = ChatResult::with_llm_output(vec![chat_gen], output.clone());
        assert_eq!(result.generations.len(), 1);
        assert_eq!(result.llm_output, Some(output));
    }

    #[test]
    fn test_chat_result_from_generation() {
        let msg = AIMessage::builder().content("Hello").build();
        let chat_gen = ChatGeneration::new(msg.into());
        let result = ChatResult::from_generation(chat_gen);
        assert_eq!(result.generations.len(), 1);
    }

    #[test]
    fn test_chat_result_serialization() {
        let msg = AIMessage::builder().content("test").build();
        let chat_gen = ChatGeneration::new(msg.into());
        let result = ChatResult::new(vec![chat_gen]);
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ChatResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.generations.len(), 1);
    }
}
