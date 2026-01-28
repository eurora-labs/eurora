//! Unit tests for Generation and GenerationChunk classes.
//!
//! Ported from `langchain/libs/core/tests/unit_tests/outputs/test_generation.py`

use agent_chain_core::outputs::{Generation, GenerationChunk};
use serde_json::json;
use std::collections::HashMap;

/// Test suite for Generation class.
mod generation_tests {
    use super::*;

    /// Test creating a Generation with only text.
    #[test]
    fn test_creation_with_text_only() {
        let generation = Generation::new("Hello, world!");
        assert_eq!(generation.text, "Hello, world!");
        assert!(generation.generation_info.is_none());
        assert_eq!(generation.generation_type, "Generation");
    }

    /// Test creating a Generation with generation_info.
    #[test]
    fn test_creation_with_generation_info() {
        let mut gen_info = HashMap::new();
        gen_info.insert("finish_reason".to_string(), json!("stop"));
        gen_info.insert("logprobs".to_string(), json!(null));
        let generation = Generation::with_info("Test output", gen_info.clone());
        assert_eq!(generation.text, "Test output");
        assert_eq!(generation.generation_info, Some(gen_info));
        assert_eq!(generation.generation_type, "Generation");
    }

    /// Test creating a Generation with empty text.
    #[test]
    fn test_creation_with_empty_text() {
        let generation = Generation::new("");
        assert_eq!(generation.text, "");
        assert!(generation.generation_info.is_none());
    }

    /// Test that Generation is marked as serializable.
    #[test]
    fn test_is_lc_serializable() {
        assert!(Generation::is_lc_serializable());
    }

    /// Test that Generation returns correct namespace.
    #[test]
    fn test_get_lc_namespace() {
        let expected_namespace = vec!["langchain", "schema", "output"];
        assert_eq!(Generation::get_lc_namespace(), expected_namespace);
    }

    /// Test that type field is set correctly.
    #[test]
    fn test_type_field_is_literal() {
        let generation = Generation::new("test");
        assert_eq!(generation.generation_type, "Generation");
    }
}

/// Test suite for GenerationChunk class.
mod test_generation_chunk {
    use super::*;

    /// Test creating a GenerationChunk.
    #[test]
    fn test_creation() {
        let chunk = GenerationChunk::new("chunk");
        assert_eq!(chunk.text, "chunk");
        assert!(chunk.generation_info.is_none());
    }

    /// Test concatenating two GenerationChunks.
    #[test]
    fn test_add_two_chunks() {
        let chunk1 = GenerationChunk::new("Hello, ");
        let chunk2 = GenerationChunk::new("world!");
        let result = chunk1 + chunk2;
        assert_eq!(result.text, "Hello, world!");
        assert!(result.generation_info.is_none());
    }

    /// Test concatenating chunks with generation_info.
    #[test]
    fn test_add_chunks_with_generation_info() {
        let mut info1 = HashMap::new();
        info1.insert("key1".to_string(), json!("value1"));
        info1.insert("shared".to_string(), json!("first"));
        let chunk1 = GenerationChunk::with_info("Hello", info1);

        let mut info2 = HashMap::new();
        info2.insert("key2".to_string(), json!("value2"));
        info2.insert("shared".to_string(), json!("second"));
        let chunk2 = GenerationChunk::with_info(" world", info2);

        let result = chunk1 + chunk2;
        assert_eq!(result.text, "Hello world");
        assert!(result.generation_info.is_some());
        let info = result.generation_info.unwrap();
        assert_eq!(info.get("key1"), Some(&json!("value1")));
        assert_eq!(info.get("key2"), Some(&json!("value2")));
        // String values are concatenated in merge_dicts
        assert_eq!(info.get("shared"), Some(&json!("firstsecond")));
    }

    /// Test concatenating chunks where one has None generation_info.
    #[test]
    fn test_add_chunk_with_none_generation_info() {
        let mut info = HashMap::new();
        info.insert("key".to_string(), json!("value"));
        let chunk1 = GenerationChunk::with_info("Hello", info.clone());
        let chunk2 = GenerationChunk::new(" world");
        let result = chunk1 + chunk2;
        assert_eq!(result.text, "Hello world");
        assert_eq!(result.generation_info, Some(info));
    }

    /// Test concatenating chunks where both have None generation_info.
    #[test]
    fn test_add_chunks_both_none_generation_info() {
        let chunk1 = GenerationChunk::new("Hello");
        let chunk2 = GenerationChunk::new(" world");
        let result = chunk1 + chunk2;
        assert_eq!(result.text, "Hello world");
        assert!(result.generation_info.is_none());
    }

    /// Test concatenating empty chunks.
    #[test]
    fn test_add_empty_chunks() {
        let chunk1 = GenerationChunk::new("");
        let chunk2 = GenerationChunk::new("");
        let result = chunk1 + chunk2;
        assert_eq!(result.text, "");
    }

    /// Test concatenating multiple chunks in sequence.
    #[test]
    fn test_add_multiple_chunks_sequentially() {
        let chunk1 = GenerationChunk::new("A");
        let chunk2 = GenerationChunk::new("B");
        let chunk3 = GenerationChunk::new("C");
        let result = chunk1 + chunk2 + chunk3;
        assert_eq!(result.text, "ABC");
    }

    // Note: test_add_with_invalid_type_raises_error is not applicable in Rust.
    // The type system prevents adding incompatible types at compile time.

    // Note: test_add_with_generation_raises_error is not applicable in Rust.
    // In Rust, GenerationChunk + Generation would be a compile error since
    // the Add trait is only implemented for GenerationChunk + GenerationChunk.

    /// Test that GenerationChunk can be created from Generation via From trait.
    /// In Python this would test inheritance; in Rust we use the From trait.
    #[test]
    fn test_conversion_from_generation() {
        let generation = Generation::new("test");
        let chunk: GenerationChunk = generation.into();
        assert_eq!(chunk.text, "test");
    }

    /// Test that GenerationChunk is serializable.
    /// Note: In Python this tests inheritance of is_lc_serializable;
    /// in Rust, GenerationChunk derives Serialize/Deserialize.
    #[test]
    fn test_is_lc_serializable_inherited() {
        let chunk = GenerationChunk::new("test");
        let json = serde_json::to_string(&chunk).unwrap();
        let _: GenerationChunk = serde_json::from_str(&json).unwrap();
    }

    /// Test that GenerationChunk follows same namespace convention as Generation.
    /// Note: In Python this tests inheritance; in Rust we document the expected value.
    #[test]
    fn test_get_lc_namespace_inherited() {
        let expected_namespace = vec!["langchain", "schema", "output"];
        // GenerationChunk should follow the same convention as Generation
        // This test documents the expected namespace for serialization compatibility
        assert_eq!(Generation::get_lc_namespace(), expected_namespace);
    }
}
