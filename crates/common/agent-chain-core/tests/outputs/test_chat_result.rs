//! Unit tests for ChatResult class.
//!
//! Ported from `langchain/libs/core/tests/unit_tests/outputs/test_chat_result.py`

use agent_chain_core::messages::AIMessage;
use agent_chain_core::outputs::{ChatGeneration, ChatResult};
use serde_json::json;
use std::collections::HashMap;

/// Test suite for ChatResult class.
mod chat_result_tests {
    use super::*;

    /// Test creating ChatResult with a single generation.
    #[test]
    fn test_creation_with_single_generation() {
        let msg = AIMessage::builder().content("Hello").build();
        let chat_gen = ChatGeneration::new(msg.into());
        let result = ChatResult::new(vec![chat_gen.clone()]);
        assert_eq!(result.generations.len(), 1);
        assert_eq!(result.generations[0], chat_gen);
        assert!(result.llm_output.is_none());
    }

    /// Test creating ChatResult with multiple generations.
    #[test]
    fn test_creation_with_multiple_generations() {
        let gen1 = ChatGeneration::new(AIMessage::builder().content("Response 1").build().into());
        let gen2 = ChatGeneration::new(AIMessage::builder().content("Response 2").build().into());
        let gen3 = ChatGeneration::new(AIMessage::builder().content("Response 3").build().into());
        let result = ChatResult::new(vec![gen1.clone(), gen2.clone(), gen3.clone()]);
        assert_eq!(result.generations.len(), 3);
        assert_eq!(result.generations[0], gen1);
        assert_eq!(result.generations[1], gen2);
        assert_eq!(result.generations[2], gen3);
    }

    /// Test creating ChatResult with llm_output.
    #[test]
    fn test_creation_with_llm_output() {
        let msg = AIMessage::builder().content("Test").build();
        let chat_gen = ChatGeneration::new(msg.into());
        let mut llm_output = HashMap::new();
        llm_output.insert(
            "token_usage".to_string(),
            json!({"prompt_tokens": 10, "completion_tokens": 20}),
        );
        llm_output.insert("model_name".to_string(), json!("gpt-4"));
        let result = ChatResult::with_llm_output(vec![chat_gen], llm_output.clone());
        assert_eq!(result.llm_output, Some(llm_output));
        assert_eq!(
            result.llm_output.as_ref().unwrap().get("token_usage"),
            Some(&json!({"prompt_tokens": 10, "completion_tokens": 20}))
        );
        assert_eq!(
            result.llm_output.as_ref().unwrap().get("model_name"),
            Some(&json!("gpt-4"))
        );
    }

    /// Test creating ChatResult with empty llm_output dict.
    #[test]
    fn test_creation_with_empty_llm_output() {
        let msg = AIMessage::builder().content("Test").build();
        let chat_gen = ChatGeneration::new(msg.into());
        let result = ChatResult::with_llm_output(vec![chat_gen], HashMap::new());
        assert_eq!(result.llm_output, Some(HashMap::new()));
    }

    /// Test that llm_output defaults to None when not provided.
    #[test]
    fn test_llm_output_defaults_to_none() {
        let msg = AIMessage::builder().content("Test").build();
        let chat_gen = ChatGeneration::new(msg.into());
        let result = ChatResult::new(vec![chat_gen]);
        assert!(result.llm_output.is_none());
    }

    /// Test ChatResult with generations that have generation_info.
    #[test]
    fn test_generations_with_generation_info() {
        let mut gen_info1 = HashMap::new();
        gen_info1.insert("finish_reason".to_string(), json!("stop"));
        let gen1 = ChatGeneration::with_info(
            AIMessage::builder().content("Response 1").build().into(),
            gen_info1.clone(),
        );

        let mut gen_info2 = HashMap::new();
        gen_info2.insert("finish_reason".to_string(), json!("length"));
        let gen2 = ChatGeneration::with_info(
            AIMessage::builder().content("Response 2").build().into(),
            gen_info2.clone(),
        );

        let result = ChatResult::new(vec![gen1, gen2]);
        assert_eq!(
            result.generations[0]
                .generation_info
                .as_ref()
                .unwrap()
                .get("finish_reason"),
            Some(&json!("stop"))
        );
        assert_eq!(
            result.generations[1]
                .generation_info
                .as_ref()
                .unwrap()
                .get("finish_reason"),
            Some(&json!("length"))
        );
    }

    /// Test creating ChatResult with empty generations list.
    #[test]
    fn test_empty_generations_list() {
        let result = ChatResult::new(vec![]);
        assert_eq!(result.generations.len(), 0);
        assert!(result.llm_output.is_none());
    }

    /// Test that message attributes are preserved in generations.
    #[test]
    fn test_generations_preserve_message_attributes() {
        let mut msg = AIMessage::builder().content("Test response").build();
        msg.additional_kwargs
            .insert("function_call".to_string(), json!({"name": "test"}));
        let chat_gen = ChatGeneration::new(msg.clone().into());
        let result = ChatResult::new(vec![chat_gen]);
        assert_eq!(result.generations[0].text, "Test response");
        // Access the message from the generation
        if let agent_chain_core::BaseMessage::AI(ai_msg) = &result.generations[0].message {
            assert_eq!(
                ai_msg.additional_kwargs.get("function_call"),
                Some(&json!({"name": "test"}))
            );
        } else {
            panic!("Expected AIMessage");
        }
    }

    /// Test llm_output can contain various data types.
    #[test]
    fn test_llm_output_with_various_types() {
        let msg = AIMessage::builder().content("Test").build();
        let chat_gen = ChatGeneration::new(msg.into());
        let mut llm_output = HashMap::new();
        llm_output.insert("string_field".to_string(), json!("value"));
        llm_output.insert("int_field".to_string(), json!(42));
        llm_output.insert("float_field".to_string(), json!(2.71));
        llm_output.insert("bool_field".to_string(), json!(true));
        llm_output.insert("list_field".to_string(), json!([1, 2, 3]));
        llm_output.insert("nested_dict".to_string(), json!({"key": "value"}));

        let result = ChatResult::with_llm_output(vec![chat_gen], llm_output.clone());
        assert_eq!(result.llm_output, Some(llm_output));
        let output = result.llm_output.as_ref().unwrap();
        assert_eq!(output.get("string_field"), Some(&json!("value")));
        assert_eq!(output.get("int_field"), Some(&json!(42)));
        assert_eq!(
            output.get("nested_dict").unwrap().get("key"),
            Some(&json!("value"))
        );
    }

    /// Test ChatResult with multiple candidate generations for same prompt.
    #[test]
    fn test_multiple_candidate_generations() {
        // Simulates n>1 parameter in API calls
        let candidates = vec![
            ChatGeneration::new(AIMessage::builder().content("Candidate 1").build().into()),
            ChatGeneration::new(AIMessage::builder().content("Candidate 2").build().into()),
            ChatGeneration::new(AIMessage::builder().content("Candidate 3").build().into()),
        ];
        let result = ChatResult::new(candidates);
        assert_eq!(result.generations.len(), 3);
        for (i, chat_gen) in result.generations.iter().enumerate() {
            assert_eq!(chat_gen.text, format!("Candidate {}", i + 1));
        }
    }
}
