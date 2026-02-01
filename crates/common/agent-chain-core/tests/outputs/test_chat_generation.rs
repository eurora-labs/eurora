//! Unit tests for ChatGeneration and ChatGenerationChunk classes.
//!
//! Ported from `langchain/libs/core/tests/unit_tests/outputs/test_chat_generation.py`
//!
//! Note: In the Python implementation, ChatGenerationChunk uses AIMessageChunk.
//! In the Rust implementation, ChatGenerationChunk uses BaseMessage (which can be AIMessage).
//! This is a language-specific API difference - the Rust API uses AIMessage for simplicity
//! while still supporting streaming chunk concatenation.

use agent_chain_core::messages::AIMessage;
use agent_chain_core::outputs::{
    ChatGeneration, ChatGenerationChunk, merge_chat_generation_chunks,
};
use serde_json::json;
use std::collections::HashMap;

/// Test suite for ChatGeneration class.
mod chat_generation_tests {
    use super::*;

    // Note: test_msg_with_text and test_msg_no_text are parametrized tests in Python
    // that test various content formats. In Rust, the AIMessage content is always a string,
    // so these tests are simplified. The Python tests for list content types are not
    // directly applicable since Rust uses a different message content model.

    /// Test that text is extracted correctly from string content.
    #[test]
    fn test_msg_with_text() {
        let msg = AIMessage::builder().content("foo").build();
        let chat_gen = ChatGeneration::new(msg.into());
        assert_eq!(chat_gen.text, "foo");
    }

    /// Test that empty message returns empty text.
    #[test]
    fn test_msg_no_text() {
        let msg = AIMessage::builder().content("").build();
        let chat_gen = ChatGeneration::new(msg.into());
        assert_eq!(chat_gen.text, "");
    }

    /// Test creating ChatGeneration with string content.
    #[test]
    fn test_creation_with_string_content() {
        let msg = AIMessage::builder().content("Hello, world!").build();
        let chat_gen = ChatGeneration::new(msg.clone().into());
        assert_eq!(chat_gen.text, "Hello, world!");
        assert_eq!(chat_gen.message, msg.into());
        assert_eq!(chat_gen.generation_type, "ChatGeneration");
    }

    /// Test creating ChatGeneration with generation_info.
    #[test]
    fn test_creation_with_generation_info() {
        let msg = AIMessage::builder().content("Test").build();
        let mut gen_info = HashMap::new();
        gen_info.insert("finish_reason".to_string(), json!("stop"));
        gen_info.insert("model".to_string(), json!("gpt-4"));
        let chat_gen = ChatGeneration::with_info(msg.into(), gen_info.clone());
        assert_eq!(chat_gen.text, "Test");
        assert_eq!(chat_gen.generation_info, Some(gen_info));
    }

    /// Test that type field is set correctly.
    #[test]
    fn test_type_field_is_literal() {
        let msg = AIMessage::builder().content("test").build();
        let chat_gen = ChatGeneration::new(msg.into());
        assert_eq!(chat_gen.generation_type, "ChatGeneration");
    }
}

/// Test suite for ChatGenerationChunk class.
///
/// Note: In Rust, ChatGenerationChunk uses AIMessage (not AIMessageChunk)
/// because the Rust API keeps things simpler while still supporting concatenation.
mod test_chat_generation_chunk {
    use super::*;

    /// Test creating a ChatGenerationChunk.
    #[test]
    fn test_creation() {
        let msg = AIMessage::builder().content("chunk").build();
        let chunk = ChatGenerationChunk::new(msg.into());
        assert_eq!(chunk.text, "chunk");
        assert_eq!(chunk.generation_type, "ChatGenerationChunk");
    }

    /// Test concatenating two ChatGenerationChunks.
    #[test]
    fn test_add_two_chunks() {
        let msg1 = AIMessage::builder().content("Hello, ").build();
        let msg2 = AIMessage::builder().content("world!").build();
        let chunk1 = ChatGenerationChunk::new(msg1.into());
        let chunk2 = ChatGenerationChunk::new(msg2.into());
        let result = chunk1 + chunk2;
        assert_eq!(result.text, "Hello, world!");
        assert!(result.generation_info.is_none());
    }

    /// Test concatenating chunks with generation_info.
    #[test]
    fn test_add_chunks_with_generation_info() {
        let msg1 = AIMessage::builder().content("Hello").build();
        let msg2 = AIMessage::builder().content(" world").build();
        let mut info1 = HashMap::new();
        info1.insert("key1".to_string(), json!("value1"));
        info1.insert("shared".to_string(), json!("first"));
        let chunk1 = ChatGenerationChunk::with_info(msg1.into(), info1);

        let mut info2 = HashMap::new();
        info2.insert("key2".to_string(), json!("value2"));
        info2.insert("shared".to_string(), json!("second"));
        let chunk2 = ChatGenerationChunk::with_info(msg2.into(), info2);

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
        let msg1 = AIMessage::builder().content("Hello").build();
        let msg2 = AIMessage::builder().content(" world").build();
        let mut info = HashMap::new();
        info.insert("key".to_string(), json!("value"));
        let chunk1 = ChatGenerationChunk::with_info(msg1.into(), info.clone());
        let chunk2 = ChatGenerationChunk::new(msg2.into());
        let result = chunk1 + chunk2;
        assert_eq!(result.text, "Hello world");
        assert_eq!(result.generation_info, Some(info));
    }

    /// Test concatenating chunks where both have None generation_info.
    #[test]
    fn test_add_chunks_both_none_generation_info() {
        let msg1 = AIMessage::builder().content("Hello").build();
        let msg2 = AIMessage::builder().content(" world").build();
        let chunk1 = ChatGenerationChunk::new(msg1.into());
        let chunk2 = ChatGenerationChunk::new(msg2.into());
        let result = chunk1 + chunk2;
        assert_eq!(result.text, "Hello world");
        assert!(result.generation_info.is_none());
    }

    // Note: test_add_list_of_chunks is not directly applicable in Rust.
    // Rust's Add trait only supports binary operations (a + b).
    // The equivalent functionality is provided by merge_chat_generation_chunks.

    /// Test concatenating empty list using merge function.
    #[test]
    fn test_merge_single_chunk_via_function() {
        let msg = AIMessage::builder().content("test").build();
        let chunk = ChatGenerationChunk::new(msg.into());
        // In Rust, we use merge_chat_generation_chunks for list merging
        let result = merge_chat_generation_chunks(vec![chunk.clone()]);
        assert!(result.is_some());
        assert_eq!(result.unwrap().text, "test");
    }

    // Note: test_add_with_invalid_type_raises_error is not applicable in Rust.
    // The type system prevents adding incompatible types at compile time.

    // Note: test_add_with_chat_generation_raises_error is not applicable in Rust.
    // In Rust, ChatGenerationChunk + ChatGeneration would be a compile error since
    // the Add trait is only implemented for ChatGenerationChunk + ChatGenerationChunk.

    /// Test that ChatGenerationChunk can be converted to/from ChatGeneration.
    /// This is the Rust equivalent of Python's inheritance test.
    #[test]
    fn test_conversion_to_chat_generation() {
        let msg = AIMessage::builder().content("test").build();
        let chunk = ChatGenerationChunk::new(msg.into());
        // Convert to ChatGeneration
        let chat_gen: ChatGeneration = chunk.clone().into();
        assert_eq!(chat_gen.text, "test");
        assert_eq!(chat_gen.generation_type, "ChatGeneration");
        // Convert back to chunk
        let converted_chunk: ChatGenerationChunk = chat_gen.into();
        assert_eq!(converted_chunk.text, "test");
    }

    /// Test that type field is set correctly.
    #[test]
    fn test_type_field_is_literal() {
        let msg = AIMessage::builder().content("test").build();
        let chunk = ChatGenerationChunk::new(msg.into());
        assert_eq!(chunk.generation_type, "ChatGenerationChunk");
    }
}

/// Test suite for merge_chat_generation_chunks function.
mod test_merge_chat_generation_chunks {
    use super::*;

    /// Test merging an empty list returns None.
    #[test]
    fn test_merge_empty_list() {
        let result = merge_chat_generation_chunks(vec![]);
        assert!(result.is_none());
    }

    /// Test merging a single chunk returns the chunk itself.
    #[test]
    fn test_merge_single_chunk() {
        let msg = AIMessage::builder().content("single").build();
        let chunk = ChatGenerationChunk::new(msg.into());
        let result = merge_chat_generation_chunks(vec![chunk.clone()]);
        assert!(result.is_some());
        // Note: In Python, the test checks `result is chunk`. In Rust, we check equality.
        // The merge function returns the same chunk for single-element lists.
        assert_eq!(result.unwrap().text, "single");
    }

    /// Test merging two chunks.
    #[test]
    fn test_merge_two_chunks() {
        let msg1 = AIMessage::builder().content("Hello ").build();
        let msg2 = AIMessage::builder().content("world").build();
        let chunk1 = ChatGenerationChunk::new(msg1.into());
        let chunk2 = ChatGenerationChunk::new(msg2.into());
        let result = merge_chat_generation_chunks(vec![chunk1, chunk2]);
        assert!(result.is_some());
        assert_eq!(result.unwrap().text, "Hello world");
    }

    /// Test merging multiple chunks.
    #[test]
    fn test_merge_multiple_chunks() {
        let chunks = vec![
            ChatGenerationChunk::new(AIMessage::builder().content("A").build().into()),
            ChatGenerationChunk::new(AIMessage::builder().content("B").build().into()),
            ChatGenerationChunk::new(AIMessage::builder().content("C").build().into()),
            ChatGenerationChunk::new(AIMessage::builder().content("D").build().into()),
        ];
        let result = merge_chat_generation_chunks(chunks);
        assert!(result.is_some());
        assert_eq!(result.unwrap().text, "ABCD");
    }

    /// Test merging chunks preserves and merges generation_info.
    #[test]
    fn test_merge_chunks_with_generation_info() {
        let msg1 = AIMessage::builder().content("A").build();
        let msg2 = AIMessage::builder().content("B").build();
        let mut info1 = HashMap::new();
        info1.insert("key1".to_string(), json!("value1"));
        let mut info2 = HashMap::new();
        info2.insert("key2".to_string(), json!("value2"));
        let chunks = vec![
            ChatGenerationChunk::with_info(msg1.into(), info1),
            ChatGenerationChunk::with_info(msg2.into(), info2),
        ];
        let result = merge_chat_generation_chunks(chunks);
        assert!(result.is_some());
        let merged = result.unwrap();
        assert!(merged.generation_info.is_some());
        let info = merged.generation_info.unwrap();
        assert_eq!(info.get("key1"), Some(&json!("value1")));
        assert_eq!(info.get("key2"), Some(&json!("value2")));
    }
}
