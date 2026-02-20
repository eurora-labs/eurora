use agent_chain_core::messages::{AIMessage, HumanMessage};
use agent_chain_core::outputs::{
    ChatGeneration, ChatGenerationChunk, Generation, merge_chat_generation_chunks,
};
use serde_json::json;
use std::collections::HashMap;

mod chat_generation_tests {
    use super::*;

    #[test]
    fn test_msg_with_text() {
        let msg = AIMessage::builder().content("foo").build();
        let chat_gen = ChatGeneration::new(msg.into());
        assert_eq!(chat_gen.text, "foo");
    }

    #[test]
    fn test_msg_no_text() {
        let msg = AIMessage::builder().content("").build();
        let chat_gen = ChatGeneration::new(msg.into());
        assert_eq!(chat_gen.text, "");
    }

    #[test]
    fn test_creation_with_string_content() {
        let msg = AIMessage::builder().content("Hello, world!").build();
        let chat_gen = ChatGeneration::new(msg.clone().into());
        assert_eq!(chat_gen.text, "Hello, world!");
        assert_eq!(chat_gen.message, msg.into());
        assert_eq!(chat_gen.generation_type, "ChatGeneration");
    }

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

    #[test]
    fn test_type_field_is_literal() {
        let msg = AIMessage::builder().content("test").build();
        let chat_gen = ChatGeneration::new(msg.into());
        assert_eq!(chat_gen.generation_type, "ChatGeneration");
    }
}

mod test_chat_generation_chunk {
    use super::*;

    #[test]
    fn test_creation() {
        let msg = AIMessage::builder().content("chunk").build();
        let chunk = ChatGenerationChunk::new(msg.into());
        assert_eq!(chunk.text, "chunk");
        assert_eq!(chunk.generation_type, "ChatGenerationChunk");
    }

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
        assert_eq!(info.get("shared"), Some(&json!("firstsecond")));
    }

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

    #[test]
    fn test_add_list_of_chunks() {
        let chunk1 = ChatGenerationChunk::new(AIMessage::builder().content("A").build().into());
        let chunk2 = ChatGenerationChunk::new(AIMessage::builder().content("B").build().into());
        let chunk3 = ChatGenerationChunk::new(AIMessage::builder().content("C").build().into());
        let result = merge_chat_generation_chunks(vec![chunk1, chunk2, chunk3]);
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.text, "ABC");
    }

    #[test]
    fn test_add_list_of_chunks_with_generation_info() {
        let chunk1 = ChatGenerationChunk::with_info(
            AIMessage::builder().content("A").build().into(),
            HashMap::from([("key1".to_string(), json!("value1"))]),
        );
        let chunk2 = ChatGenerationChunk::with_info(
            AIMessage::builder().content("B").build().into(),
            HashMap::from([("key2".to_string(), json!("value2"))]),
        );
        let chunk3 = ChatGenerationChunk::with_info(
            AIMessage::builder().content("C").build().into(),
            HashMap::from([("key3".to_string(), json!("value3"))]),
        );
        let result = merge_chat_generation_chunks(vec![chunk1, chunk2, chunk3]);
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.text, "ABC");
        assert!(result.generation_info.is_some());
        let info = result.generation_info.unwrap();
        assert_eq!(info.get("key1"), Some(&json!("value1")));
        assert_eq!(info.get("key2"), Some(&json!("value2")));
        assert_eq!(info.get("key3"), Some(&json!("value3")));
    }

    #[test]
    fn test_add_empty_list() {
        let chunk = ChatGenerationChunk::new(AIMessage::builder().content("test").build().into());
        let result = merge_chat_generation_chunks(vec![chunk]);
        assert!(result.is_some());
        assert_eq!(result.unwrap().text, "test");
    }

    #[test]
    fn test_conversion_to_chat_generation() {
        let msg = AIMessage::builder().content("test").build();
        let chunk = ChatGenerationChunk::new(msg.into());
        let chat_gen: ChatGeneration = chunk.clone().into();
        assert_eq!(chat_gen.text, "test");
        assert_eq!(chat_gen.generation_type, "ChatGeneration");
        let converted_chunk: ChatGenerationChunk = chat_gen.into();
        assert_eq!(converted_chunk.text, "test");
    }

    #[test]
    fn test_type_field_is_literal() {
        let msg = AIMessage::builder().content("test").build();
        let chunk = ChatGenerationChunk::new(msg.into());
        assert_eq!(chunk.generation_type, "ChatGenerationChunk");
    }
}

mod test_merge_chat_generation_chunks {
    use super::*;

    #[test]
    fn test_merge_empty_list() {
        let result = merge_chat_generation_chunks(vec![]);
        assert!(result.is_none());
    }

    #[test]
    fn test_merge_single_chunk() {
        let msg = AIMessage::builder().content("single").build();
        let chunk = ChatGenerationChunk::new(msg.into());
        let result = merge_chat_generation_chunks(vec![chunk.clone()]);
        assert!(result.is_some());
        assert_eq!(result.unwrap().text, "single");
    }

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

    #[test]
    fn test_merge_chunks_all_none_generation_info() {
        let chunks = vec![
            ChatGenerationChunk::new(AIMessage::builder().content("A").build().into()),
            ChatGenerationChunk::new(AIMessage::builder().content("B").build().into()),
            ChatGenerationChunk::new(AIMessage::builder().content("C").build().into()),
        ];
        let result = merge_chat_generation_chunks(chunks);
        assert!(result.is_some());
        let merged = result.unwrap();
        assert_eq!(merged.text, "ABC");
        assert!(merged.generation_info.is_none());
    }

    #[test]
    fn test_merge_chunks_returns_chat_generation_chunk_type() {
        let chunks = vec![
            ChatGenerationChunk::new(AIMessage::builder().content("A").build().into()),
            ChatGenerationChunk::new(AIMessage::builder().content("B").build().into()),
        ];
        let result = merge_chat_generation_chunks(chunks);
        assert!(result.is_some());
        let merged = result.unwrap();
        assert_eq!(merged.generation_type, "ChatGenerationChunk");
    }
}

mod test_chat_generation_inheritance {
    use super::*;

    #[test]
    fn test_chat_generation_shares_generation_interface() {
        let chat_gen = ChatGeneration::new(AIMessage::builder().content("test").build().into());
        let generation = Generation::new("test");
        assert_eq!(chat_gen.text, generation.text);
        assert_eq!(chat_gen.generation_info, generation.generation_info);
    }

    #[test]
    fn test_chat_generation_is_lc_serializable() {
        assert!(ChatGeneration::is_lc_serializable());
    }

    #[test]
    fn test_chat_generation_get_lc_namespace() {
        assert_eq!(
            ChatGeneration::get_lc_namespace(),
            vec!["langchain", "schema", "output"]
        );
    }

    #[test]
    fn test_chat_generation_chunk_shares_generation_interface() {
        let chunk = ChatGenerationChunk::new(AIMessage::builder().content("test").build().into());
        assert_eq!(chunk.text, "test");
        assert!(chunk.generation_info.is_none());

        let chat_gen: ChatGeneration = chunk.clone().into();
        assert_eq!(chat_gen.text, "test");

        assert_eq!(chunk.generation_type, "ChatGenerationChunk");
    }
}

mod test_chat_generation_text_extraction {
    use super::*;

    #[test]
    fn test_empty_string_content() {
        let chat_gen = ChatGeneration::new(AIMessage::builder().content("").build().into());
        assert_eq!(chat_gen.text, "");
    }

    #[test]
    fn test_text_derived_from_message_content() {
        let msg = AIMessage::builder().content("from_message").build();
        let chat_gen = ChatGeneration::new(msg.into());
        assert_eq!(chat_gen.text, "from_message");
    }

    #[test]
    fn test_with_human_message() {
        let msg = HumanMessage::builder().content("user input").build();
        let chat_gen = ChatGeneration::new(msg.into());
        assert_eq!(chat_gen.text, "user input");
    }
}

mod test_chat_generation_serialization {
    use super::*;

    #[test]
    fn test_serialize_basic() {
        let chat_gen = ChatGeneration::new(AIMessage::builder().content("Hello").build().into());
        let data: serde_json::Value =
            serde_json::to_value(&chat_gen).expect("serialization failed");
        assert_eq!(data["text"], "Hello");
        assert_eq!(data["type"], "ChatGeneration");
        assert!(data.get("message").is_some());
    }

    #[test]
    fn test_serialize_with_generation_info() {
        let chat_gen = ChatGeneration::with_info(
            AIMessage::builder().content("test").build().into(),
            HashMap::from([("finish_reason".to_string(), json!("stop"))]),
        );
        let data: serde_json::Value =
            serde_json::to_value(&chat_gen).expect("serialization failed");
        assert_eq!(data["generation_info"]["finish_reason"], "stop");
    }

    #[test]
    fn test_chat_generation_chunk_serialize() {
        let chunk = ChatGenerationChunk::with_info(
            AIMessage::builder().content("chunk").build().into(),
            HashMap::from([("key".to_string(), json!("val"))]),
        );
        let data: serde_json::Value = serde_json::to_value(&chunk).expect("serialization failed");
        assert_eq!(data["text"], "chunk");
        assert_eq!(data["type"], "ChatGenerationChunk");
        assert_eq!(data["generation_info"]["key"], "val");
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let chat_gen = ChatGeneration::with_info(
            AIMessage::builder().content("roundtrip").build().into(),
            HashMap::from([("model".to_string(), json!("gpt-4"))]),
        );
        let json_str = serde_json::to_string(&chat_gen).expect("serialization failed");
        let deserialized: ChatGeneration =
            serde_json::from_str(&json_str).expect("deserialization failed");
        assert_eq!(deserialized.text, "roundtrip");
        assert_eq!(deserialized.generation_type, "ChatGeneration");
        assert!(deserialized.generation_info.is_some());
        assert_eq!(
            deserialized.generation_info.unwrap().get("model"),
            Some(&json!("gpt-4"))
        );
    }

    #[test]
    fn test_chunk_serialize_deserialize_roundtrip() {
        let chunk = ChatGenerationChunk::with_info(
            AIMessage::builder().content("chunk_rt").build().into(),
            HashMap::from([("key".to_string(), json!("val"))]),
        );
        let json_str = serde_json::to_string(&chunk).expect("serialization failed");
        let deserialized: ChatGenerationChunk =
            serde_json::from_str(&json_str).expect("deserialization failed");
        assert_eq!(deserialized.text, "chunk_rt");
        assert_eq!(deserialized.generation_type, "ChatGenerationChunk");
        assert_eq!(
            deserialized.generation_info.unwrap().get("key"),
            Some(&json!("val"))
        );
    }
}

mod test_chat_generation_chunk_merging_edge_cases {
    use super::*;

    #[test]
    fn test_merge_list_with_mixed_none_generation_info() {
        let chunk1 = ChatGenerationChunk::with_info(
            AIMessage::builder().content("A").build().into(),
            HashMap::from([("k1".to_string(), json!("v1"))]),
        );
        let chunk2 = ChatGenerationChunk::new(AIMessage::builder().content("B").build().into());
        let chunk3 = ChatGenerationChunk::with_info(
            AIMessage::builder().content("C").build().into(),
            HashMap::from([("k3".to_string(), json!("v3"))]),
        );
        let result = merge_chat_generation_chunks(vec![chunk1, chunk2, chunk3]);
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.text, "ABC");
        assert!(result.generation_info.is_some());
        let info = result.generation_info.unwrap();
        assert_eq!(info.get("k1"), Some(&json!("v1")));
        assert_eq!(info.get("k3"), Some(&json!("v3")));
    }

    #[test]
    fn test_merge_list_all_none_generation_info() {
        let chunk1 = ChatGenerationChunk::new(AIMessage::builder().content("A").build().into());
        let chunk2 = ChatGenerationChunk::new(AIMessage::builder().content("B").build().into());
        let chunk3 = ChatGenerationChunk::new(AIMessage::builder().content("C").build().into());
        let result = merge_chat_generation_chunks(vec![chunk1, chunk2, chunk3]);
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.text, "ABC");
        assert!(result.generation_info.is_none());
    }

    #[test]
    fn test_add_returns_correct_type() {
        let chunk1 = ChatGenerationChunk::new(AIMessage::builder().content("A").build().into());
        let chunk2 = ChatGenerationChunk::new(AIMessage::builder().content("B").build().into());
        let result = chunk1 + chunk2;
        assert_eq!(result.generation_type, "ChatGenerationChunk");
    }

    #[test]
    fn test_merge_list_returns_correct_type() {
        let chunk1 = ChatGenerationChunk::new(AIMessage::builder().content("A").build().into());
        let chunk2 = ChatGenerationChunk::new(AIMessage::builder().content("B").build().into());
        let result = merge_chat_generation_chunks(vec![chunk1, chunk2]);
        assert!(result.is_some());
        assert_eq!(result.unwrap().generation_type, "ChatGenerationChunk");
    }

    #[test]
    fn test_merge_generation_info_with_nested_dicts() {
        let chunk1 = ChatGenerationChunk::with_info(
            AIMessage::builder().content("A").build().into(),
            HashMap::from([("meta".to_string(), json!({"key1": "val1"}))]),
        );
        let chunk2 = ChatGenerationChunk::with_info(
            AIMessage::builder().content("B").build().into(),
            HashMap::from([("meta".to_string(), json!({"key2": "val2"}))]),
        );
        let result = chunk1 + chunk2;
        assert!(result.generation_info.is_some());
        let info = result.generation_info.unwrap();
        let meta = info.get("meta").expect("meta key should exist");
        assert_eq!(meta["key1"], "val1");
        assert_eq!(meta["key2"], "val2");
    }

    #[test]
    fn test_merge_generation_info_with_int_values() {
        let chunk1 = ChatGenerationChunk::with_info(
            AIMessage::builder().content("A").build().into(),
            HashMap::from([("tokens".to_string(), json!(10))]),
        );
        let chunk2 = ChatGenerationChunk::with_info(
            AIMessage::builder().content("B").build().into(),
            HashMap::from([("tokens".to_string(), json!(20))]),
        );
        let result = chunk1 + chunk2;
        assert!(result.generation_info.is_some());
        let info = result.generation_info.unwrap();
        assert_eq!(info.get("tokens"), Some(&json!(30)));
    }

    #[test]
    fn test_sequential_add_chunks() {
        let c1 = ChatGenerationChunk::with_info(
            AIMessage::builder().content("A").build().into(),
            HashMap::from([("k1".to_string(), json!("v1"))]),
        );
        let c2 = ChatGenerationChunk::with_info(
            AIMessage::builder().content("B").build().into(),
            HashMap::from([("k2".to_string(), json!("v2"))]),
        );
        let c3 = ChatGenerationChunk::with_info(
            AIMessage::builder().content("C").build().into(),
            HashMap::from([("k3".to_string(), json!("v3"))]),
        );
        let result = c1 + c2 + c3;
        assert_eq!(result.text, "ABC");
        assert!(result.generation_info.is_some());
        let info = result.generation_info.unwrap();
        assert_eq!(info.get("k1"), Some(&json!("v1")));
        assert_eq!(info.get("k2"), Some(&json!("v2")));
        assert_eq!(info.get("k3"), Some(&json!("v3")));
    }

    #[test]
    fn test_add_empty_content_chunks() {
        let chunk1 = ChatGenerationChunk::new(AIMessage::builder().content("").build().into());
        let chunk2 = ChatGenerationChunk::new(AIMessage::builder().content("").build().into());
        let result = chunk1 + chunk2;
        assert_eq!(result.text, "");
    }
}
