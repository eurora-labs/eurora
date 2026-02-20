use agent_chain_core::messages::{AIMessage, AIMessageChunk};
use agent_chain_core::outputs::{ChatGeneration, ChatGenerationChunk, ChatResult};
use serde_json::json;
use std::collections::HashMap;

mod chat_result_tests {
    use super::*;

    #[test]
    fn test_creation_with_single_generation() {
        let msg = AIMessage::builder().content("Hello").build();
        let chat_gen = ChatGeneration::new(msg.into());
        let result = ChatResult::new(vec![chat_gen.clone()]);
        assert_eq!(result.generations.len(), 1);
        assert_eq!(result.generations[0], chat_gen);
        assert!(result.llm_output.is_none());
    }

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

    #[test]
    fn test_creation_with_empty_llm_output() {
        let msg = AIMessage::builder().content("Test").build();
        let chat_gen = ChatGeneration::new(msg.into());
        let result = ChatResult::with_llm_output(vec![chat_gen], HashMap::new());
        assert_eq!(result.llm_output, Some(HashMap::new()));
    }

    #[test]
    fn test_llm_output_defaults_to_none() {
        let msg = AIMessage::builder().content("Test").build();
        let chat_gen = ChatGeneration::new(msg.into());
        let result = ChatResult::new(vec![chat_gen]);
        assert!(result.llm_output.is_none());
    }

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

    #[test]
    fn test_empty_generations_list() {
        let result = ChatResult::new(vec![]);
        assert_eq!(result.generations.len(), 0);
        assert!(result.llm_output.is_none());
    }

    #[test]
    fn test_generations_preserve_message_attributes() {
        let mut msg = AIMessage::builder().content("Test response").build();
        msg.additional_kwargs
            .insert("function_call".to_string(), json!({"name": "test"}));
        let chat_gen = ChatGeneration::new(msg.clone().into());
        let result = ChatResult::new(vec![chat_gen]);
        assert_eq!(result.generations[0].text, "Test response");
        if let agent_chain_core::BaseMessage::AI(ai_msg) = &result.generations[0].message {
            assert_eq!(
                ai_msg.additional_kwargs.get("function_call"),
                Some(&json!({"name": "test"}))
            );
        } else {
            panic!("Expected AIMessage");
        }
    }

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

    #[test]
    fn test_multiple_candidate_generations() {
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

mod chat_result_serialization_tests {
    use super::*;

    #[test]
    fn test_model_dump_basic() {
        let chat_gen = ChatGeneration::new(AIMessage::builder().content("Hello").build().into());
        let result = ChatResult::new(vec![chat_gen]);
        let data = serde_json::to_value(&result).expect("serialization should succeed");
        assert!(data.get("generations").is_some());
        assert_eq!(
            data.get("generations").unwrap().as_array().unwrap().len(),
            1
        );
        assert!(data.get("llm_output").is_none());
    }

    #[test]
    fn test_model_dump_with_llm_output() {
        let chat_gen = ChatGeneration::new(AIMessage::builder().content("Hello").build().into());
        let mut llm_output = HashMap::new();
        llm_output.insert("model".to_string(), json!("gpt-4"));
        llm_output.insert("token_usage".to_string(), json!({"total": 50}));
        let result = ChatResult::with_llm_output(vec![chat_gen], llm_output);
        let data = serde_json::to_value(&result).expect("serialization should succeed");
        assert_eq!(data["llm_output"]["model"], json!("gpt-4"));
        assert_eq!(data["llm_output"]["token_usage"]["total"], json!(50));
    }

    #[test]
    fn test_json_roundtrip() {
        let mut generation_info = HashMap::new();
        generation_info.insert("finish_reason".to_string(), json!("stop"));
        let chat_gen = ChatGeneration::with_info(
            AIMessage::builder().content("test").build().into(),
            generation_info,
        );
        let mut llm_output = HashMap::new();
        llm_output.insert("model".to_string(), json!("gpt-4"));
        let result = ChatResult::with_llm_output(vec![chat_gen], llm_output);

        let json_str = serde_json::to_string(&result).expect("serialization should succeed");
        let restored: ChatResult =
            serde_json::from_str(&json_str).expect("deserialization should succeed");
        assert_eq!(restored.generations.len(), 1);
        assert_eq!(restored.generations[0].text, "test");
        let mut expected_output = HashMap::new();
        expected_output.insert("model".to_string(), json!("gpt-4"));
        assert_eq!(restored.llm_output, Some(expected_output));
    }

    #[test]
    fn test_model_validate_from_dict() {
        let chat_gen = ChatGeneration::new(AIMessage::builder().content("test").build().into());
        let mut llm_output = HashMap::new();
        llm_output.insert("key".to_string(), json!("val"));
        let result = ChatResult::with_llm_output(vec![chat_gen], llm_output);

        let data = serde_json::to_value(&result).expect("serialization should succeed");
        let restored: ChatResult =
            serde_json::from_value(data).expect("deserialization should succeed");
        assert_eq!(restored.generations.len(), result.generations.len());
        assert_eq!(restored.llm_output, result.llm_output);
    }
}

mod chat_result_equality_tests {
    use super::*;

    #[test]
    fn test_equality_same_content() {
        let chat_gen = ChatGeneration::new(AIMessage::builder().content("Hello").build().into());
        let mut llm_output = HashMap::new();
        llm_output.insert("model".to_string(), json!("gpt-4"));
        let result1 = ChatResult::with_llm_output(vec![chat_gen.clone()], llm_output.clone());
        let result2 = ChatResult::with_llm_output(vec![chat_gen], llm_output);
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_inequality_different_generations() {
        let chat_gen1 = ChatGeneration::new(AIMessage::builder().content("Hello").build().into());
        let chat_gen2 = ChatGeneration::new(AIMessage::builder().content("Goodbye").build().into());
        let result1 = ChatResult::new(vec![chat_gen1]);
        let result2 = ChatResult::new(vec![chat_gen2]);
        assert_ne!(result1, result2);
    }

    #[test]
    fn test_inequality_different_llm_output() {
        let chat_gen = ChatGeneration::new(AIMessage::builder().content("Hello").build().into());
        let mut output1 = HashMap::new();
        output1.insert("model".to_string(), json!("gpt-4"));
        let mut output2 = HashMap::new();
        output2.insert("model".to_string(), json!("gpt-3.5"));
        let result1 = ChatResult::with_llm_output(vec![chat_gen.clone()], output1);
        let result2 = ChatResult::with_llm_output(vec![chat_gen], output2);
        assert_ne!(result1, result2);
    }

    #[test]
    fn test_equality_both_none_llm_output() {
        let chat_gen = ChatGeneration::new(AIMessage::builder().content("Hello").build().into());
        let result1 = ChatResult::new(vec![chat_gen.clone()]);
        let result2 = ChatResult::new(vec![chat_gen]);
        assert_eq!(result1, result2);
    }
}

mod chat_result_model_behavior_tests {
    use super::*;

    #[test]
    fn test_with_chat_generation_chunk() {
        let chunk = ChatGenerationChunk::new(
            AIMessageChunk::builder()
                .content("chunk")
                .build()
                .to_message()
                .into(),
        );
        let chat_gen: ChatGeneration = chunk.into();
        let result = ChatResult::new(vec![chat_gen]);
        assert_eq!(result.generations.len(), 1);
        assert_eq!(result.generations[0].text, "chunk");
    }

    #[test]
    fn test_generations_ordering_preserved() {
        let generations: Vec<ChatGeneration> = (0..5)
            .map(|i| {
                ChatGeneration::new(
                    AIMessage::builder()
                        .content(format!("Response {i}"))
                        .build()
                        .into(),
                )
            })
            .collect();
        let result = ChatResult::new(generations);
        for (i, generation) in result.generations.iter().enumerate() {
            assert_eq!(generation.text, format!("Response {i}"));
        }
    }

    #[test]
    fn test_generations_with_mixed_content_types() {
        let gen_str = ChatGeneration::new(
            AIMessage::builder()
                .content("string content")
                .build()
                .into(),
        );
        let gen_list = ChatGeneration::new(
            AIMessage::with_content_list(vec![json!({"text": "list content", "type": "text"})])
                .into(),
        );
        let result = ChatResult::new(vec![gen_str, gen_list]);
        assert_eq!(result.generations[0].text, "string content");
        assert_eq!(result.generations[1].text, "list content");
    }
}
