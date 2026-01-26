//! Tests for HumanMessage and HumanMessageChunk.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/test_human.py`

use agent_chain_core::messages::{HumanMessage, HumanMessageChunk};

// ============================================================================
// TestHumanMessage
// ============================================================================

#[test]
fn test_init_basic() {
    let msg = HumanMessage::new("Hello, how are you?");
    assert_eq!(msg.content(), "Hello, how are you?");
    assert_eq!(msg.message_type(), "human");
}

#[test]
fn test_init_with_name() {
    let msg = HumanMessage::new("Hello").with_name("user1");
    assert_eq!(msg.name(), Some("user1".to_string()));
}

#[test]
fn test_init_with_id() {
    let msg = HumanMessage::with_id("msg-123", "Hello");
    assert_eq!(msg.id(), Some("msg-123".to_string()));
}

#[test]
fn test_init_with_additional_kwargs() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("custom".to_string(), serde_json::json!("value"));

    let msg = HumanMessage::new("Hello").with_additional_kwargs(additional_kwargs);
    assert_eq!(
        msg.additional_kwargs().get("custom").unwrap(),
        &serde_json::json!("value")
    );
}

#[test]
fn test_init_with_response_metadata() {
    let mut response_metadata = std::collections::HashMap::new();
    response_metadata.insert("source".to_string(), serde_json::json!("web"));

    let msg = HumanMessage::new("Hello").with_response_metadata(response_metadata);
    assert_eq!(
        msg.response_metadata().get("source").unwrap(),
        &serde_json::json!("web")
    );
}

#[test]
fn test_type_is_human() {
    let msg = HumanMessage::new("Test");
    assert_eq!(msg.message_type(), "human");
}

#[test]
fn test_serialization_roundtrip() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("custom".to_string(), serde_json::json!("value"));

    let msg = HumanMessage::with_id("msg-123", "Hello")
        .with_name("user1")
        .with_additional_kwargs(additional_kwargs);

    let serialized = serde_json::to_value(&msg).unwrap();
    assert_eq!(serialized.get("type").unwrap().as_str().unwrap(), "human");

    let deserialized: HumanMessage = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content(), "Hello");
    assert_eq!(deserialized.name(), Some("user1".to_string()));
    assert_eq!(deserialized.id(), Some("msg-123".to_string()));
    assert_eq!(
        deserialized.additional_kwargs().get("custom").unwrap(),
        &serde_json::json!("value")
    );
}

#[test]
fn test_text_property() {
    let msg = HumanMessage::new("Hello world");
    assert_eq!(msg.text(), "Hello world");
}

#[test]
fn test_empty_content() {
    let msg = HumanMessage::new("");
    assert_eq!(msg.content(), "");
    assert_eq!(msg.text(), "");
}

// ============================================================================
// TestHumanMessageChunk
// ============================================================================

#[test]
fn test_chunk_init_basic() {
    let chunk = HumanMessageChunk::new("Hello");
    assert_eq!(chunk.content(), "Hello");
    assert_eq!(chunk.message_type(), "HumanMessageChunk");
}

#[test]
fn test_chunk_type_is_human_message_chunk() {
    let chunk = HumanMessageChunk::new("Test");
    assert_eq!(chunk.message_type(), "HumanMessageChunk");
}

#[test]
fn test_chunk_add_two_chunks() {
    let chunk1 = HumanMessageChunk::with_id("1", "Hello");
    let chunk2 = HumanMessageChunk::new(" world");
    let result = chunk1 + chunk2;
    assert_eq!(result.content(), "Hello world");
    assert_eq!(result.id(), Some("1".to_string()));
}

#[test]
fn test_chunk_add_with_additional_kwargs() {
    let mut kwargs1 = std::collections::HashMap::new();
    kwargs1.insert("key1".to_string(), serde_json::json!("value1"));

    let mut kwargs2 = std::collections::HashMap::new();
    kwargs2.insert("key2".to_string(), serde_json::json!("value2"));

    let chunk1 = HumanMessageChunk::new("Hello").with_additional_kwargs(kwargs1);
    let chunk2 = HumanMessageChunk::new(" world").with_additional_kwargs(kwargs2);

    let result = chunk1 + chunk2;
    assert_eq!(
        result.additional_kwargs().get("key1").unwrap(),
        &serde_json::json!("value1")
    );
    assert_eq!(
        result.additional_kwargs().get("key2").unwrap(),
        &serde_json::json!("value2")
    );
}

#[test]
fn test_chunk_add_with_response_metadata() {
    let mut meta1 = std::collections::HashMap::new();
    meta1.insert("meta1".to_string(), serde_json::json!("data1"));

    let mut meta2 = std::collections::HashMap::new();
    meta2.insert("meta2".to_string(), serde_json::json!("data2"));

    let chunk1 = HumanMessageChunk::new("Hello").with_response_metadata(meta1);
    let chunk2 = HumanMessageChunk::new(" world").with_response_metadata(meta2);

    let result = chunk1 + chunk2;
    assert_eq!(
        result.response_metadata().get("meta1").unwrap(),
        &serde_json::json!("data1")
    );
    assert_eq!(
        result.response_metadata().get("meta2").unwrap(),
        &serde_json::json!("data2")
    );
}

#[test]
fn test_chunk_add_preserves_id() {
    let chunk1 = HumanMessageChunk::with_id("original-id", "Hello");
    let chunk2 = HumanMessageChunk::with_id("other-id", " world");
    let result = chunk1 + chunk2;
    assert_eq!(result.id(), Some("original-id".to_string()));
}

#[test]
fn test_chunk_serialization_roundtrip() {
    let chunk = HumanMessageChunk::with_id("chunk-123", "Hello").with_name("user1");

    let serialized = serde_json::to_value(&chunk).unwrap();
    assert_eq!(
        serialized.get("type").unwrap().as_str().unwrap(),
        "HumanMessageChunk"
    );

    let deserialized: HumanMessageChunk = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content(), "Hello");
    assert_eq!(deserialized.name(), Some("user1".to_string()));
    assert_eq!(deserialized.id(), Some("chunk-123".to_string()));
}

#[test]
fn test_chunk_multiple_additions() {
    let chunk1 = HumanMessageChunk::new("a");
    let chunk2 = HumanMessageChunk::new("b");
    let chunk3 = HumanMessageChunk::new("c");
    let result = chunk1 + chunk2 + chunk3;
    assert_eq!(result.content(), "abc");
}

#[test]
fn test_chunk_empty_content() {
    let chunk1 = HumanMessageChunk::new("Hello");
    let chunk2 = HumanMessageChunk::new("");
    let result = chunk1 + chunk2;
    assert_eq!(result.content(), "Hello");
}

#[test]
fn test_chunk_text_property() {
    let chunk = HumanMessageChunk::new("Hello world");
    assert_eq!(chunk.text(), "Hello world");
}

#[test]
fn test_chunk_to_message() {
    let chunk = HumanMessageChunk::with_id("chunk-1", "Hello!");
    let message: HumanMessage = chunk.clone().into();
    assert_eq!(message.content(), "Hello!");
    assert_eq!(message.id(), Some("chunk-1".to_string()));
}

#[test]
fn test_chunk_sum() {
    let chunks = vec![
        HumanMessageChunk::new("Hello "),
        HumanMessageChunk::new("beautiful "),
        HumanMessageChunk::new("world!"),
    ];
    let result: HumanMessageChunk = chunks.into_iter().sum();
    assert_eq!(result.content(), "Hello beautiful world!");
}
