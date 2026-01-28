//! Unit tests for LLMResult class.
//!
//! Ported from `langchain/libs/core/tests/unit_tests/outputs/test_llm_result.py`

use agent_chain_core::messages::AIMessage;
use agent_chain_core::outputs::{
    ChatGeneration, ChatGenerationChunk, Generation, GenerationChunk, GenerationType, LLMResult,
    RunInfo,
};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

/// Test suite for LLMResult class.
mod llm_result_tests {
    use super::*;

    /// Test creating LLMResult with single prompt and single generation.
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

    /// Test creating LLMResult with multiple prompts.
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
        // Check text of each generation
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

    /// Test creating LLMResult with multiple candidate generations per prompt.
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

    /// Test creating LLMResult with ChatGeneration objects.
    #[test]
    fn test_creation_with_chat_generations() {
        let gen1 = ChatGeneration::new(AIMessage::new("Chat response 1").into());
        let gen2 = ChatGeneration::new(AIMessage::new("Chat response 2").into());
        let result = LLMResult::new(vec![vec![gen1.into()], vec![gen2.into()]]);
        assert_eq!(result.generations.len(), 2);
        if let GenerationType::ChatGeneration(cg) = &result.generations[0][0] {
            assert_eq!(cg.text, "Chat response 1");
        }
    }

    /// Test creating LLMResult with GenerationChunk objects.
    #[test]
    fn test_creation_with_generation_chunks() {
        let chunk1 = GenerationChunk::new("Chunk 1");
        let chunk2 = ChatGenerationChunk::new(AIMessage::new("Chunk 2").into());
        let result = LLMResult::new(vec![vec![chunk1.into()], vec![chunk2.into()]]);
        assert_eq!(result.generations.len(), 2);
        if let GenerationType::GenerationChunk(gc) = &result.generations[0][0] {
            assert_eq!(gc.text, "Chunk 1");
        }
        if let GenerationType::ChatGenerationChunk(cgc) = &result.generations[1][0] {
            assert_eq!(cgc.text, "Chunk 2");
        }
    }

    /// Test creating LLMResult with llm_output.
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

    /// Test creating LLMResult with run info.
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

    /// Test creating LLMResult with multiple run infos.
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

    /// Test flattening LLMResult with single prompt and generation.
    #[test]
    fn test_flatten_single_prompt_single_generation() {
        let generation = Generation::new("Response");
        let result = LLMResult::new(vec![vec![generation.clone().into()]]);
        let flattened = result.flatten();
        assert_eq!(flattened.len(), 1);
        assert_eq!(flattened[0].generations.len(), 1);
    }

    /// Test flattening LLMResult with multiple prompts.
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
        // Each flattened result should have one generation list
        for f in &flattened {
            assert_eq!(f.generations.len(), 1);
        }
    }

    /// Test that flatten preserves llm_output for first result.
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

    /// Test that flatten clears token_usage for subsequent results.
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
        // token_usage should be empty for subsequent results
        assert_eq!(
            flattened[1].llm_output.as_ref().unwrap().get("token_usage"),
            Some(&json!({}))
        );
        // Other fields should be preserved
        assert_eq!(
            flattened[1].llm_output.as_ref().unwrap().get("model"),
            Some(&json!("gpt-4"))
        );
    }

    /// Test that flatten handles None llm_output correctly.
    #[test]
    fn test_flatten_handles_none_llm_output() {
        let gen1 = Generation::new("Response 1");
        let gen2 = Generation::new("Response 2");
        let result = LLMResult::new(vec![vec![gen1.into()], vec![gen2.into()]]);
        let flattened = result.flatten();
        assert!(flattened[0].llm_output.is_none());
        assert!(flattened[1].llm_output.is_none());
    }

    /// Test flattening with multiple candidate generations.
    #[test]
    fn test_flatten_with_multiple_candidates() {
        let gen1 = Generation::new("Candidate 1");
        let gen2 = Generation::new("Candidate 2");
        let result = LLMResult::new(vec![vec![gen1.into(), gen2.into()]]);
        let flattened = result.flatten();
        assert_eq!(flattened.len(), 1);
        assert_eq!(flattened[0].generations[0].len(), 2);
    }

    /// Test equality for LLMResults with same generations and output.
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

    /// Test inequality for LLMResults with different generations.
    #[test]
    fn test_equality_different_generations() {
        let gen1 = Generation::new("Response 1");
        let gen2 = Generation::new("Response 2");
        let result1 = LLMResult::new(vec![vec![gen1.into()]]);
        let result2 = LLMResult::new(vec![vec![gen2.into()]]);
        assert_ne!(result1, result2);
    }

    /// Test inequality for LLMResults with different llm_output.
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

    // Note: test_equality_ignores_run_info - In Rust, the PartialEq derive includes all fields.
    // This is a deliberate API difference from Python. Run info is part of equality in Rust.

    /// Test equality when llm_output is None.
    #[test]
    fn test_equality_with_none_llm_output() {
        let generation = Generation::new("Response");
        let result1 = LLMResult::new(vec![vec![generation.clone().into()]]);
        let result2 = LLMResult::new(vec![vec![generation.into()]]);
        assert_eq!(result1, result2);
    }

    // Note: test_hash_is_none - In Rust, hash is not implemented by default,
    // and HashMap doesn't require hashable keys for LLMResult usage.

    /// Test that type field is set correctly.
    #[test]
    fn test_type_field_is_literal() {
        let generation = Generation::new("Response");
        let result = LLMResult::new(vec![vec![generation.into()]]);
        assert_eq!(result.result_type, "LLMResult");
    }

    /// Test creating LLMResult with empty generations.
    #[test]
    fn test_empty_generations() {
        let result = LLMResult::new(vec![]);
        assert_eq!(result.generations.len(), 0);
        let flattened = result.flatten();
        assert_eq!(flattened.len(), 0);
    }

    /// Test LLMResult with mixed generation types in same list.
    #[test]
    fn test_mixed_generation_types() {
        let generation = Generation::new("Regular");
        let chat_gen = ChatGeneration::new(AIMessage::new("Chat").into());
        let result = LLMResult::new(vec![vec![generation.into()], vec![chat_gen.into()]]);
        assert_eq!(result.generations.len(), 2);
        // First should be Generation
        assert!(matches!(
            result.generations[0][0],
            GenerationType::Generation(_)
        ));
        // Second should be ChatGeneration
        assert!(matches!(
            result.generations[1][0],
            GenerationType::ChatGeneration(_)
        ));
    }

    /// Test LLMResult with complex nested llm_output.
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
