use agent_chain_core::messages::AIMessage;
use agent_chain_core::outputs::{
    ChatGeneration, ChatGenerationChunk, Generation, GenerationChunk, GenerationType, LLMResult,
    RunInfo,
};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

mod llm_result_tests {
    use super::*;

    #[test]
    fn test_creation_with_single_prompt_single_generation() {
        let generation = Generation::new("Response");
        let result = LLMResult::new(vec![vec![generation.clone().into()]]);
        assert_eq!(result.generations.len(), 1);
        assert_eq!(result.generations[0].len(), 1);
        assert!(result.llm_output.is_none());
        assert!(result.run.is_none());
        assert_eq!(result.result_type, "LLMResult");
    }

    #[test]
    fn test_creation_with_multiple_prompts() {
        let gen1 = Generation::new("Response 1");
        let gen2 = Generation::new("Response 2");
        let gen3 = Generation::new("Response 3");
        let result = LLMResult::new(vec![
            vec![gen1.into()],
            vec![gen2.into()],
            vec![gen3.into()],
        ]);
        assert_eq!(result.generations.len(), 3);
        if let GenerationType::Generation(g) = &result.generations[0][0] {
            assert_eq!(g.text, "Response 1");
        }
        if let GenerationType::Generation(g) = &result.generations[1][0] {
            assert_eq!(g.text, "Response 2");
        }
        if let GenerationType::Generation(g) = &result.generations[2][0] {
            assert_eq!(g.text, "Response 3");
        }
    }

    #[test]
    fn test_creation_with_multiple_candidates() {
        let gen1 = Generation::new("Candidate 1");
        let gen2 = Generation::new("Candidate 2");
        let gen3 = Generation::new("Candidate 3");
        let result = LLMResult::new(vec![vec![gen1.into(), gen2.into(), gen3.into()]]);
        assert_eq!(result.generations.len(), 1);
        assert_eq!(result.generations[0].len(), 3);
        if let GenerationType::Generation(g) = &result.generations[0][0] {
            assert_eq!(g.text, "Candidate 1");
        }
        if let GenerationType::Generation(g) = &result.generations[0][1] {
            assert_eq!(g.text, "Candidate 2");
        }
        if let GenerationType::Generation(g) = &result.generations[0][2] {
            assert_eq!(g.text, "Candidate 3");
        }
    }

    #[test]
    fn test_creation_with_chat_generations() {
        let gen1 = ChatGeneration::new(
            AIMessage::builder()
                .content("Chat response 1")
                .build()
                .into(),
        );
        let gen2 = ChatGeneration::new(
            AIMessage::builder()
                .content("Chat response 2")
                .build()
                .into(),
        );
        let result = LLMResult::new(vec![vec![gen1.into()], vec![gen2.into()]]);
        assert_eq!(result.generations.len(), 2);
        if let GenerationType::ChatGeneration(cg) = &result.generations[0][0] {
            assert_eq!(cg.text, "Chat response 1");
        }
    }

    #[test]
    fn test_creation_with_generation_chunks() {
        let chunk1 = GenerationChunk::new("Chunk 1");
        let chunk2 =
            ChatGenerationChunk::new(AIMessage::builder().content("Chunk 2").build().into());
        let result = LLMResult::new(vec![vec![chunk1.into()], vec![chunk2.into()]]);
        assert_eq!(result.generations.len(), 2);
        if let GenerationType::GenerationChunk(gc) = &result.generations[0][0] {
            assert_eq!(gc.text, "Chunk 1");
        }
        if let GenerationType::ChatGenerationChunk(cgc) = &result.generations[1][0] {
            assert_eq!(cgc.text, "Chunk 2");
        }
    }

    #[test]
    fn test_creation_with_llm_output() {
        let generation = Generation::new("Response");
        let mut llm_output = HashMap::new();
        llm_output.insert(
            "token_usage".to_string(),
            json!({"prompt_tokens": 10, "completion_tokens": 20}),
        );
        llm_output.insert("model_name".to_string(), json!("gpt-4"));
        let result = LLMResult::with_llm_output(vec![vec![generation.into()]], llm_output.clone());
        assert_eq!(result.llm_output, Some(llm_output));
        assert_eq!(
            result
                .llm_output
                .as_ref()
                .unwrap()
                .get("token_usage")
                .unwrap()
                .get("prompt_tokens"),
            Some(&json!(10))
        );
    }

    #[test]
    fn test_creation_with_run_info() {
        let generation = Generation::new("Response");
        let run_id = Uuid::new_v4();
        let run_info = RunInfo::new(run_id);
        let mut result = LLMResult::new(vec![vec![generation.into()]]);
        result.run = Some(vec![run_info.clone()]);
        assert!(result.run.is_some());
        assert_eq!(result.run.as_ref().unwrap().len(), 1);
        assert_eq!(result.run.as_ref().unwrap()[0].run_id, run_id);
    }

    #[test]
    fn test_creation_with_multiple_run_infos() {
        let gen1 = Generation::new("Response 1");
        let gen2 = Generation::new("Response 2");
        let run_id1 = Uuid::new_v4();
        let run_id2 = Uuid::new_v4();
        let run_info1 = RunInfo::new(run_id1);
        let run_info2 = RunInfo::new(run_id2);
        let mut result = LLMResult::new(vec![vec![gen1.into()], vec![gen2.into()]]);
        result.run = Some(vec![run_info1, run_info2]);
        assert!(result.run.is_some());
        assert_eq!(result.run.as_ref().unwrap().len(), 2);
        assert_eq!(result.run.as_ref().unwrap()[0].run_id, run_id1);
        assert_eq!(result.run.as_ref().unwrap()[1].run_id, run_id2);
    }

    #[test]
    fn test_flatten_single_prompt_single_generation() {
        let generation = Generation::new("Response");
        let result = LLMResult::new(vec![vec![generation.clone().into()]]);
        let flattened = result.flatten();
        assert_eq!(flattened.len(), 1);
        assert_eq!(flattened[0].generations.len(), 1);
    }

    #[test]
    fn test_flatten_multiple_prompts() {
        let gen1 = Generation::new("Response 1");
        let gen2 = Generation::new("Response 2");
        let gen3 = Generation::new("Response 3");
        let result = LLMResult::new(vec![
            vec![gen1.clone().into()],
            vec![gen2.clone().into()],
            vec![gen3.clone().into()],
        ]);
        let flattened = result.flatten();
        assert_eq!(flattened.len(), 3);
        for f in &flattened {
            assert_eq!(f.generations.len(), 1);
        }
    }

    #[test]
    fn test_flatten_preserves_llm_output_for_first() {
        let gen1 = Generation::new("Response 1");
        let gen2 = Generation::new("Response 2");
        let mut llm_output = HashMap::new();
        llm_output.insert("token_usage".to_string(), json!({"total": 100}));
        llm_output.insert("model".to_string(), json!("gpt-4"));
        let result = LLMResult::with_llm_output(
            vec![vec![gen1.into()], vec![gen2.into()]],
            llm_output.clone(),
        );
        let flattened = result.flatten();
        assert_eq!(flattened[0].llm_output, Some(llm_output));
        assert_eq!(
            flattened[0]
                .llm_output
                .as_ref()
                .unwrap()
                .get("token_usage")
                .unwrap()
                .get("total"),
            Some(&json!(100))
        );
    }

    #[test]
    fn test_flatten_clears_token_usage_for_subsequent() {
        let gen1 = Generation::new("Response 1");
        let gen2 = Generation::new("Response 2");
        let mut llm_output = HashMap::new();
        llm_output.insert("token_usage".to_string(), json!({"total": 100}));
        llm_output.insert("model".to_string(), json!("gpt-4"));
        let result =
            LLMResult::with_llm_output(vec![vec![gen1.into()], vec![gen2.into()]], llm_output);
        let flattened = result.flatten();
        assert!(flattened[1].llm_output.is_some());
        assert_eq!(
            flattened[1].llm_output.as_ref().unwrap().get("token_usage"),
            Some(&json!({}))
        );
        assert_eq!(
            flattened[1].llm_output.as_ref().unwrap().get("model"),
            Some(&json!("gpt-4"))
        );
    }

    #[test]
    fn test_flatten_handles_none_llm_output() {
        let gen1 = Generation::new("Response 1");
        let gen2 = Generation::new("Response 2");
        let result = LLMResult::new(vec![vec![gen1.into()], vec![gen2.into()]]);
        let flattened = result.flatten();
        assert!(flattened[0].llm_output.is_none());
        assert!(flattened[1].llm_output.is_none());
    }

    #[test]
    fn test_flatten_with_multiple_candidates() {
        let gen1 = Generation::new("Candidate 1");
        let gen2 = Generation::new("Candidate 2");
        let result = LLMResult::new(vec![vec![gen1.into(), gen2.into()]]);
        let flattened = result.flatten();
        assert_eq!(flattened.len(), 1);
        assert_eq!(flattened[0].generations[0].len(), 2);
    }

    #[test]
    fn test_equality_same_generations_and_output() {
        let generation = Generation::new("Response");
        let mut llm_output = HashMap::new();
        llm_output.insert("model".to_string(), json!("gpt-4"));
        let result1 =
            LLMResult::with_llm_output(vec![vec![generation.clone().into()]], llm_output.clone());
        let result2 = LLMResult::with_llm_output(vec![vec![generation.into()]], llm_output);
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_equality_different_generations() {
        let gen1 = Generation::new("Response 1");
        let gen2 = Generation::new("Response 2");
        let result1 = LLMResult::new(vec![vec![gen1.into()]]);
        let result2 = LLMResult::new(vec![vec![gen2.into()]]);
        assert_ne!(result1, result2);
    }

    #[test]
    fn test_equality_different_llm_output() {
        let generation = Generation::new("Response");
        let mut llm_output1 = HashMap::new();
        llm_output1.insert("model".to_string(), json!("gpt-4"));
        let mut llm_output2 = HashMap::new();
        llm_output2.insert("model".to_string(), json!("gpt-3.5"));
        let result1 =
            LLMResult::with_llm_output(vec![vec![generation.clone().into()]], llm_output1);
        let result2 = LLMResult::with_llm_output(vec![vec![generation.into()]], llm_output2);
        assert_ne!(result1, result2);
    }

    #[test]
    fn test_equality_ignores_run_info() {
        let generation = Generation::new("Response");
        let run_id1 = Uuid::new_v4();
        let run_id2 = Uuid::new_v4();
        let mut result1 = LLMResult::new(vec![vec![generation.clone().into()]]);
        result1.run = Some(vec![RunInfo::new(run_id1)]);
        let mut result2 = LLMResult::new(vec![vec![generation.into()]]);
        result2.run = Some(vec![RunInfo::new(run_id2)]);
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_equality_with_none_llm_output() {
        let generation = Generation::new("Response");
        let result1 = LLMResult::new(vec![vec![generation.clone().into()]]);
        let result2 = LLMResult::new(vec![vec![generation.into()]]);
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_type_field_is_literal() {
        let generation = Generation::new("Response");
        let result = LLMResult::new(vec![vec![generation.into()]]);
        assert_eq!(result.result_type, "LLMResult");
    }

    #[test]
    fn test_empty_generations() {
        let result = LLMResult::new(vec![]);
        assert_eq!(result.generations.len(), 0);
        let flattened = result.flatten();
        assert_eq!(flattened.len(), 0);
    }

    #[test]
    fn test_mixed_generation_types() {
        let generation = Generation::new("Regular");
        let chat_gen = ChatGeneration::new(AIMessage::builder().content("Chat").build().into());
        let result = LLMResult::new(vec![vec![generation.into()], vec![chat_gen.into()]]);
        assert_eq!(result.generations.len(), 2);
        assert!(matches!(
            result.generations[0][0],
            GenerationType::Generation(_)
        ));
        assert!(matches!(
            result.generations[1][0],
            GenerationType::ChatGeneration(_)
        ));
    }

    #[test]
    fn test_complex_llm_output_structure() {
        let generation = Generation::new("Response");
        let mut llm_output = HashMap::new();
        llm_output.insert(
            "token_usage".to_string(),
            json!({
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30,
            }),
        );
        llm_output.insert("model_name".to_string(), json!("gpt-4"));
        llm_output.insert("system_fingerprint".to_string(), json!("fp_123"));
        llm_output.insert(
            "metadata".to_string(),
            json!({"temperature": 0.7, "top_p": 1.0}),
        );
        let result = LLMResult::with_llm_output(vec![vec![generation.into()]], llm_output.clone());
        assert_eq!(result.llm_output, Some(llm_output));
        assert_eq!(
            result
                .llm_output
                .as_ref()
                .unwrap()
                .get("metadata")
                .unwrap()
                .get("temperature"),
            Some(&json!(0.7))
        );
    }
}

mod llm_result_flatten_tests {
    use super::*;

    #[test]
    fn test_flatten_does_not_include_run_info() {
        let gen1 = Generation::new("R1");
        let gen2 = Generation::new("R2");
        let run_info = vec![RunInfo::new(Uuid::new_v4()), RunInfo::new(Uuid::new_v4())];
        let mut result = LLMResult::new(vec![vec![gen1.into()], vec![gen2.into()]]);
        result.run = Some(run_info);
        let flattened = result.flatten();
        for flat in &flattened {
            assert!(flat.run.is_none());
        }
    }

    #[test]
    fn test_flatten_clones_llm_output_for_subsequent() {
        let gen1 = Generation::new("R1");
        let gen2 = Generation::new("R2");
        let mut llm_output = HashMap::new();
        llm_output.insert("token_usage".to_string(), json!({"total": 100}));
        llm_output.insert("model".to_string(), json!("gpt-4"));
        let result =
            LLMResult::with_llm_output(vec![vec![gen1.into()], vec![gen2.into()]], llm_output);
        let mut flattened = result.flatten();
        flattened[1]
            .llm_output
            .as_mut()
            .unwrap()
            .insert("model".to_string(), json!("modified"));
        assert_eq!(
            result.llm_output.as_ref().unwrap().get("model"),
            Some(&json!("gpt-4"))
        );
        assert_eq!(
            flattened[0].llm_output.as_ref().unwrap().get("model"),
            Some(&json!("gpt-4"))
        );
    }

    #[test]
    fn test_flatten_preserves_all_candidates_in_gen_list() {
        let gen1a = Generation::new("1A");
        let gen1b = Generation::new("1B");
        let gen2a = Generation::new("2A");
        let result = LLMResult::new(vec![vec![gen1a.into(), gen1b.into()], vec![gen2a.into()]]);
        let flattened = result.flatten();
        assert_eq!(flattened.len(), 2);
        assert_eq!(flattened[0].generations[0].len(), 2);
        if let GenerationType::Generation(g) = &flattened[0].generations[0][0] {
            assert_eq!(g.text, "1A");
        }
        if let GenerationType::Generation(g) = &flattened[0].generations[0][1] {
            assert_eq!(g.text, "1B");
        }
        assert_eq!(flattened[1].generations[0].len(), 1);
        if let GenerationType::Generation(g) = &flattened[1].generations[0][0] {
            assert_eq!(g.text, "2A");
        }
    }

    #[test]
    fn test_flatten_with_chat_generations() {
        let gen1 = ChatGeneration::new(AIMessage::builder().content("Chat 1").build().into());
        let gen2 = ChatGeneration::new(AIMessage::builder().content("Chat 2").build().into());
        let mut llm_output = HashMap::new();
        llm_output.insert("token_usage".to_string(), json!({"total": 50}));
        let result =
            LLMResult::with_llm_output(vec![vec![gen1.into()], vec![gen2.into()]], llm_output);
        let flattened = result.flatten();
        assert_eq!(flattened.len(), 2);
        assert!(matches!(
            flattened[0].generations[0][0],
            GenerationType::ChatGeneration(_)
        ));
        if let GenerationType::ChatGeneration(cg) = &flattened[0].generations[0][0] {
            assert_eq!(cg.text, "Chat 1");
        }
        assert_eq!(
            flattened[0]
                .llm_output
                .as_ref()
                .unwrap()
                .get("token_usage")
                .unwrap()
                .get("total"),
            Some(&json!(50))
        );
        assert_eq!(
            flattened[1].llm_output.as_ref().unwrap().get("token_usage"),
            Some(&json!({}))
        );
    }

    #[test]
    fn test_flatten_single_generation_preserves_llm_output() {
        let generation = Generation::new("Only");
        let mut llm_output = HashMap::new();
        llm_output.insert("token_usage".to_string(), json!({"total": 10}));
        llm_output.insert("model".to_string(), json!("test"));
        let result = LLMResult::with_llm_output(vec![vec![generation.into()]], llm_output.clone());
        let flattened = result.flatten();
        assert_eq!(flattened.len(), 1);
        assert_eq!(flattened[0].llm_output, Some(llm_output));
    }

    #[test]
    fn test_flatten_many_prompts_token_usage_cleared() {
        let generations: Vec<Vec<GenerationType>> = (0..5)
            .map(|i| vec![Generation::new(format!("R{i}")).into()])
            .collect();
        let mut llm_output = HashMap::new();
        llm_output.insert("token_usage".to_string(), json!({"total": 200}));
        llm_output.insert("model".to_string(), json!("gpt-4"));
        let result = LLMResult::with_llm_output(generations, llm_output);
        let flattened = result.flatten();
        assert_eq!(flattened.len(), 5);
        assert_eq!(
            flattened[0]
                .llm_output
                .as_ref()
                .unwrap()
                .get("token_usage")
                .unwrap()
                .get("total"),
            Some(&json!(200))
        );
        for flat in &flattened[1..] {
            assert_eq!(
                flat.llm_output.as_ref().unwrap().get("token_usage"),
                Some(&json!({}))
            );
            assert_eq!(
                flat.llm_output.as_ref().unwrap().get("model"),
                Some(&json!("gpt-4"))
            );
        }
    }

    #[test]
    fn test_flatten_empty_generations() {
        let result = LLMResult::new(vec![]);
        let flattened = result.flatten();
        assert!(flattened.is_empty());
    }

    #[test]
    fn test_flatten_with_empty_llm_output_dict() {
        let gen1 = Generation::new("R1");
        let gen2 = Generation::new("R2");
        let result =
            LLMResult::with_llm_output(vec![vec![gen1.into()], vec![gen2.into()]], HashMap::new());
        let flattened = result.flatten();
        assert_eq!(flattened[0].llm_output, Some(HashMap::new()));
        assert!(flattened[1].llm_output.is_some());
        assert_eq!(
            flattened[1].llm_output.as_ref().unwrap().get("token_usage"),
            Some(&json!({}))
        );
    }
}

mod llm_result_equality_tests {
    use super::*;

    #[test]
    fn test_equality_empty_generations() {
        let result1 = LLMResult::new(vec![]);
        let result2 = LLMResult::new(vec![]);
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_equality_same_generations_different_run() {
        let generation = Generation::new("test");
        let mut result1 = LLMResult::new(vec![vec![generation.clone().into()]]);
        result1.run = Some(vec![RunInfo::new(Uuid::new_v4())]);
        let result2 = LLMResult::new(vec![vec![generation.into()]]);
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_inequality_none_vs_dict_llm_output() {
        let generation = Generation::new("Response");
        let result1 = LLMResult::new(vec![vec![generation.clone().into()]]);
        let result2 = LLMResult::with_llm_output(vec![vec![generation.into()]], HashMap::new());
        assert_ne!(result1, result2);
    }
}

mod llm_result_serialization_tests {
    use super::*;

    #[test]
    fn test_serialize_basic() {
        let generation = Generation::new("Response");
        let mut llm_output = HashMap::new();
        llm_output.insert("model".to_string(), json!("test"));
        let result = LLMResult::with_llm_output(vec![vec![generation.into()]], llm_output.clone());
        let data: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert!(data.get("generations").is_some());
        assert_eq!(data["llm_output"]["model"], json!("test"));
        assert_eq!(data["type"], json!("LLMResult"));
        assert!(data.get("run").is_none() || data["run"].is_null());
    }

    #[test]
    fn test_serialize_with_run_info() {
        let generation = Generation::new("Response");
        let run_id = Uuid::new_v4();
        let mut result = LLMResult::new(vec![vec![generation.into()]]);
        result.run = Some(vec![RunInfo::new(run_id)]);
        let data: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert!(data["run"].is_array());
        assert_eq!(data["run"].as_array().unwrap().len(), 1);
        assert_eq!(data["run"][0]["run_id"], json!(run_id.to_string()));
    }

    #[test]
    fn test_json_roundtrip() {
        let mut generation_info = HashMap::new();
        generation_info.insert("reason".to_string(), json!("stop"));
        let generation = Generation::with_info("test", generation_info);
        let mut llm_output = HashMap::new();
        llm_output.insert("model".to_string(), json!("gpt-4"));
        let result = LLMResult::with_llm_output(vec![vec![generation.into()]], llm_output.clone());
        let json_str = serde_json::to_string(&result).unwrap();
        let restored: LLMResult = serde_json::from_str(&json_str).unwrap();
        assert_eq!(restored.generations.len(), 1);
        if let GenerationType::Generation(g) = &restored.generations[0][0] {
            assert_eq!(g.text, "test");
        } else {
            panic!("Expected Generation variant");
        }
        assert_eq!(restored.llm_output, Some(llm_output));
        assert_eq!(restored.result_type, "LLMResult");
    }

    #[test]
    fn test_deserialize_from_value() {
        let generation = Generation::new("test");
        let mut llm_output = HashMap::new();
        llm_output.insert("key".to_string(), json!("val"));
        let result = LLMResult::with_llm_output(vec![vec![generation.into()]], llm_output.clone());
        let data = serde_json::to_value(&result).unwrap();
        let restored: LLMResult = serde_json::from_value(data).unwrap();
        assert_eq!(restored.llm_output, result.llm_output);
        assert_eq!(restored.generations.len(), result.generations.len());
    }
}
