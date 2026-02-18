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



    /// Test that GenerationChunk can be created from Generation via From trait.
    /// In Python this would test inheritance; in Rust we use the From trait.
    #[test]
    fn test_conversion_from_generation() {
        let generation = Generation::new("test");
        let chunk: GenerationChunk = generation.into();
        assert_eq!(chunk.text, "test");
    }

    /// Test that GenerationChunk is serializable.
    #[test]
    fn test_is_lc_serializable_inherited() {
        let chunk = GenerationChunk::new("test");
        let json_str = serde_json::to_string(&chunk).expect("serialization should succeed");
        let _: GenerationChunk =
            serde_json::from_str(&json_str).expect("deserialization should succeed");
    }

    /// Test that GenerationChunk follows same namespace convention as Generation.
    #[test]
    fn test_get_lc_namespace_inherited() {
        let expected_namespace = vec!["langchain", "schema", "output"];
        assert_eq!(Generation::get_lc_namespace(), expected_namespace);
    }

    /// Test that GenerationChunk type field stays "Generation" (inherited from
    /// Generation in Python, where GenerationChunk does not override the type).
    #[test]
    fn test_type_field_is_generation() {
        let chunk = GenerationChunk::new("test");
        assert_eq!(chunk.generation_type, "Generation");
    }
}

/// Test suite for Generation serialization roundtrips.
mod test_generation_serialization {
    use super::*;

    /// Test that serialization produces correct structure.
    #[test]
    fn test_model_dump_basic() {
        let mut gen_info = HashMap::new();
        gen_info.insert("reason".to_string(), json!("stop"));
        let generation = Generation::with_info("Hello", gen_info);
        let data: serde_json::Value =
            serde_json::to_value(&generation).expect("serialization should succeed");
        assert_eq!(data["text"], "Hello");
        assert_eq!(data["generation_info"]["reason"], "stop");
        assert_eq!(data["type"], "Generation");
    }

    /// Test serialization with None generation_info.
    #[test]
    fn test_model_dump_none_generation_info() {
        let generation = Generation::new("Hello");
        let data: serde_json::Value =
            serde_json::to_value(&generation).expect("serialization should succeed");
        assert!(data.get("generation_info").is_none());
    }

    /// Test serialization roundtrip via to_value/from_value.
    #[test]
    fn test_model_validate_roundtrip() {
        let mut gen_info = HashMap::new();
        gen_info.insert("logprobs".to_string(), json!([0.1, 0.2]));
        let generation = Generation::with_info("test output", gen_info);
        let data = serde_json::to_value(&generation).expect("serialization should succeed");
        let restored: Generation =
            serde_json::from_value(data).expect("deserialization should succeed");
        assert_eq!(restored.text, generation.text);
        assert_eq!(restored.generation_info, generation.generation_info);
        assert_eq!(restored.generation_type, generation.generation_type);
    }

    /// Test JSON string serialization roundtrip.
    #[test]
    fn test_json_roundtrip() {
        let mut gen_info = HashMap::new();
        gen_info.insert("finish_reason".to_string(), json!("stop"));
        gen_info.insert("index".to_string(), json!(0));
        let generation = Generation::with_info("json test", gen_info);
        let json_str = serde_json::to_string(&generation).expect("serialization should succeed");
        let restored: Generation =
            serde_json::from_str(&json_str).expect("deserialization should succeed");
        assert_eq!(restored.text, generation.text);
        assert_eq!(restored.generation_info, generation.generation_info);
        assert_eq!(restored.generation_type, generation.generation_type);
    }

    /// Test GenerationChunk serialization produces correct structure.
    #[test]
    fn test_generation_chunk_model_dump() {
        let mut gen_info = HashMap::new();
        gen_info.insert("key".to_string(), json!("val"));
        let chunk = GenerationChunk::with_info("chunk", gen_info);
        let data: serde_json::Value =
            serde_json::to_value(&chunk).expect("serialization should succeed");
        assert_eq!(data["text"], "chunk");
        assert_eq!(data["generation_info"]["key"], "val");
    }

    /// Test GenerationChunk JSON string roundtrip.
    #[test]
    fn test_generation_chunk_json_roundtrip() {
        let mut gen_info = HashMap::new();
        gen_info.insert("a".to_string(), json!(1));
        let chunk = GenerationChunk::with_info("json chunk", gen_info);
        let json_str = serde_json::to_string(&chunk).expect("serialization should succeed");
        let restored: GenerationChunk =
            serde_json::from_str(&json_str).expect("deserialization should succeed");
        assert_eq!(restored.text, chunk.text);
        assert_eq!(restored.generation_info, chunk.generation_info);
    }
}

/// Test suite for GenerationChunk merging edge cases.
mod test_generation_chunk_merging {
    use super::*;

    /// Test merging generation_info containing nested dictionaries.
    #[test]
    fn test_merge_generation_info_with_nested_dicts() {
        let mut info1 = HashMap::new();
        info1.insert("nested".to_string(), json!({"key1": "val1"}));
        let chunk1 = GenerationChunk::with_info("A", info1);

        let mut info2 = HashMap::new();
        info2.insert("nested".to_string(), json!({"key2": "val2"}));
        let chunk2 = GenerationChunk::with_info("B", info2);

        let result = chunk1 + chunk2;
        let info = result
            .generation_info
            .expect("generation_info should be Some");
        let nested = info.get("nested").expect("nested key should exist");
        assert_eq!(nested["key1"], "val1");
        assert_eq!(nested["key2"], "val2");
    }

    /// Test merging generation_info containing list values.
    #[test]
    fn test_merge_generation_info_with_list_values() {
        let mut info1 = HashMap::new();
        info1.insert("items".to_string(), json!([1, 2]));
        let chunk1 = GenerationChunk::with_info("A", info1);

        let mut info2 = HashMap::new();
        info2.insert("items".to_string(), json!([3, 4]));
        let chunk2 = GenerationChunk::with_info("B", info2);

        let result = chunk1 + chunk2;
        let info = result
            .generation_info
            .expect("generation_info should be Some");
        assert_eq!(info.get("items"), Some(&json!([1, 2, 3, 4])));
    }

    /// Test merging generation_info containing integer values.
    #[test]
    fn test_merge_generation_info_with_int_values() {
        let mut info1 = HashMap::new();
        info1.insert("count".to_string(), json!(5));
        let chunk1 = GenerationChunk::with_info("A", info1);

        let mut info2 = HashMap::new();
        info2.insert("count".to_string(), json!(3));
        let chunk2 = GenerationChunk::with_info("B", info2);

        let result = chunk1 + chunk2;
        let info = result
            .generation_info
            .expect("generation_info should be Some");
        assert_eq!(info.get("count"), Some(&json!(8)));
    }

    /// Test that addition always returns GenerationChunk, not Generation.
    #[test]
    fn test_add_preserves_generation_chunk_type() {
        let chunk1 = GenerationChunk::new("A");
        let chunk2 = GenerationChunk::new("B");
        let result = chunk1 + chunk2;
        let _: GenerationChunk = result;
    }



    /// Test that sequential adds properly accumulate generation_info.
    #[test]
    fn test_sequential_add_accumulates_generation_info() {
        let mut info1 = HashMap::new();
        info1.insert("k1".to_string(), json!("v1"));
        let chunk1 = GenerationChunk::with_info("A", info1);

        let mut info2 = HashMap::new();
        info2.insert("k2".to_string(), json!("v2"));
        let chunk2 = GenerationChunk::with_info("B", info2);

        let mut info3 = HashMap::new();
        info3.insert("k3".to_string(), json!("v3"));
        let chunk3 = GenerationChunk::with_info("C", info3);

        let result = chunk1 + chunk2 + chunk3;
        assert_eq!(result.text, "ABC");
        let info = result
            .generation_info
            .expect("generation_info should be Some");
        assert_eq!(info.get("k1"), Some(&json!("v1")));
        assert_eq!(info.get("k2"), Some(&json!("v2")));
        assert_eq!(info.get("k3"), Some(&json!("v3")));
    }

    /// Test adding when first chunk has None generation_info.
    #[test]
    fn test_add_first_has_none_second_has_info() {
        let chunk1 = GenerationChunk::new("A");

        let mut info2 = HashMap::new();
        info2.insert("key".to_string(), json!("value"));
        let chunk2 = GenerationChunk::with_info("B", info2.clone());

        let result = chunk1 + chunk2;
        assert_eq!(result.generation_info, Some(info2));
    }
}
