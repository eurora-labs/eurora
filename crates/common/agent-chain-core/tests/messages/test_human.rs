//! Tests for HumanMessage and HumanMessageChunk.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/test_human.py`

use agent_chain_core::messages::{HumanMessage, HumanMessageChunk};

// ============================================================================
// TestHumanMessage
// ============================================================================

#[test]
fn test_init_basic() {
    let msg = HumanMessage::builder()
        .content("Hello, how are you?")
        .build();
    assert_eq!(msg.content.as_text(), "Hello, how are you?");
    assert_eq!(msg.message_type(), "human");
}

#[test]
fn test_init_with_name() {
    let msg = HumanMessage::builder()
        .content("Hello")
        .name("user1".to_string())
        .build();
    assert_eq!(msg.name, Some("user1".to_string()));
}

#[test]
fn test_init_with_id() {
    let msg = HumanMessage::builder()
        .content("Hello")
        .id("msg-123".to_string())
        .build();
    assert_eq!(msg.id, Some("msg-123".to_string()));
}

#[test]
fn test_init_with_additional_kwargs() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("custom".to_string(), serde_json::json!("value"));

    let msg = HumanMessage::builder()
        .content("Hello")
        .additional_kwargs(additional_kwargs)
        .build();
    assert_eq!(
        msg.additional_kwargs.get("custom").unwrap(),
        &serde_json::json!("value")
    );
}

#[test]
fn test_init_with_response_metadata() {
    let mut response_metadata = std::collections::HashMap::new();
    response_metadata.insert("source".to_string(), serde_json::json!("web"));

    let msg = HumanMessage::builder()
        .content("Hello")
        .response_metadata(response_metadata)
        .build();
    assert_eq!(
        msg.response_metadata.get("source").unwrap(),
        &serde_json::json!("web")
    );
}

#[test]
fn test_type_is_human() {
    let msg = HumanMessage::builder().content("Test").build();
    assert_eq!(msg.message_type(), "human");
}

#[test]
fn test_serialization_roundtrip() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("custom".to_string(), serde_json::json!("value"));

    let msg = HumanMessage::builder()
        .content("Hello")
        .id("msg-123".to_string())
        .name("user1".to_string())
        .additional_kwargs(additional_kwargs)
        .build();

    let serialized = serde_json::to_value(&msg).unwrap();
    assert_eq!(serialized.get("type").unwrap().as_str().unwrap(), "human");

    let deserialized: HumanMessage = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content.as_text(), "Hello");
    assert_eq!(deserialized.name, Some("user1".to_string()));
    assert_eq!(deserialized.id, Some("msg-123".to_string()));
    assert_eq!(
        deserialized.additional_kwargs.get("custom").unwrap(),
        &serde_json::json!("value")
    );
}

#[test]
fn test_text_property() {
    let msg = HumanMessage::builder().content("Hello world").build();
    assert_eq!(msg.content.as_text(), "Hello world");
}

#[test]
fn test_empty_content() {
    let msg = HumanMessage::builder().content("").build();
    assert_eq!(msg.content.as_text(), "");
}

// ============================================================================
// TestHumanMessageChunk
// ============================================================================

#[test]
fn test_chunk_init_basic() {
    let chunk = HumanMessageChunk::builder().content("Hello").build();
    assert_eq!(chunk.content.as_text(), "Hello");
    assert_eq!(chunk.message_type(), "HumanMessageChunk");
}

#[test]
fn test_chunk_type_is_human_message_chunk() {
    let chunk = HumanMessageChunk::builder().content("Test").build();
    assert_eq!(chunk.message_type(), "HumanMessageChunk");
}

#[test]
fn test_chunk_add_two_chunks() {
    let chunk1 = HumanMessageChunk::builder()
        .content("Hello")
        .id("1".to_string())
        .build();
    let chunk2 = HumanMessageChunk::builder().content(" world").build();
    let result = chunk1 + chunk2;
    assert_eq!(result.content.as_text(), "Hello world");
    assert_eq!(result.id, Some("1".to_string()));
}

#[test]
fn test_chunk_add_with_additional_kwargs() {
    let mut kwargs1 = std::collections::HashMap::new();
    kwargs1.insert("key1".to_string(), serde_json::json!("value1"));

    let mut kwargs2 = std::collections::HashMap::new();
    kwargs2.insert("key2".to_string(), serde_json::json!("value2"));

    let chunk1 = HumanMessageChunk::builder()
        .content("Hello")
        .additional_kwargs(kwargs1)
        .build();
    let chunk2 = HumanMessageChunk::builder()
        .content(" world")
        .additional_kwargs(kwargs2)
        .build();

    let result = chunk1 + chunk2;
    assert_eq!(
        result.additional_kwargs.get("key1").unwrap(),
        &serde_json::json!("value1")
    );
    assert_eq!(
        result.additional_kwargs.get("key2").unwrap(),
        &serde_json::json!("value2")
    );
}

#[test]
fn test_chunk_add_with_response_metadata() {
    let mut meta1 = std::collections::HashMap::new();
    meta1.insert("meta1".to_string(), serde_json::json!("data1"));

    let mut meta2 = std::collections::HashMap::new();
    meta2.insert("meta2".to_string(), serde_json::json!("data2"));

    let chunk1 = HumanMessageChunk::builder()
        .content("Hello")
        .response_metadata(meta1)
        .build();
    let chunk2 = HumanMessageChunk::builder()
        .content(" world")
        .response_metadata(meta2)
        .build();

    let result = chunk1 + chunk2;
    assert_eq!(
        result.response_metadata.get("meta1").unwrap(),
        &serde_json::json!("data1")
    );
    assert_eq!(
        result.response_metadata.get("meta2").unwrap(),
        &serde_json::json!("data2")
    );
}

#[test]
fn test_chunk_add_preserves_id() {
    let chunk1 = HumanMessageChunk::builder()
        .content("Hello")
        .id("original-id".to_string())
        .build();
    let chunk2 = HumanMessageChunk::builder()
        .content(" world")
        .id("other-id".to_string())
        .build();
    let result = chunk1 + chunk2;
    assert_eq!(result.id, Some("original-id".to_string()));
}

#[test]
fn test_chunk_serialization_roundtrip() {
    let chunk = HumanMessageChunk::builder()
        .content("Hello")
        .id("chunk-123".to_string())
        .name("user1".to_string())
        .build();

    let serialized = serde_json::to_value(&chunk).unwrap();
    assert_eq!(
        serialized.get("type").unwrap().as_str().unwrap(),
        "HumanMessageChunk"
    );

    let deserialized: HumanMessageChunk = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content.as_text(), "Hello");
    assert_eq!(deserialized.name, Some("user1".to_string()));
    assert_eq!(deserialized.id, Some("chunk-123".to_string()));
}

#[test]
fn test_chunk_multiple_additions() {
    let chunk1 = HumanMessageChunk::builder().content("a").build();
    let chunk2 = HumanMessageChunk::builder().content("b").build();
    let chunk3 = HumanMessageChunk::builder().content("c").build();
    let result = chunk1 + chunk2 + chunk3;
    assert_eq!(result.content.as_text(), "abc");
}

#[test]
fn test_chunk_empty_content() {
    let chunk1 = HumanMessageChunk::builder().content("Hello").build();
    let chunk2 = HumanMessageChunk::builder().content("").build();
    let result = chunk1 + chunk2;
    assert_eq!(result.content.as_text(), "Hello");
}

#[test]
fn test_chunk_text_property() {
    let chunk = HumanMessageChunk::builder().content("Hello world").build();
    assert_eq!(chunk.content.as_text(), "Hello world");
}

#[test]
fn test_chunk_to_message() {
    let chunk = HumanMessageChunk::builder()
        .content("Hello!")
        .id("chunk-1".to_string())
        .build();
    let message: HumanMessage = chunk.clone().into();
    assert_eq!(message.content.as_text(), "Hello!");
    assert_eq!(message.id, Some("chunk-1".to_string()));
}

#[test]
fn test_chunk_sum() {
    let chunks = vec![
        HumanMessageChunk::builder().content("Hello ").build(),
        HumanMessageChunk::builder().content("beautiful ").build(),
        HumanMessageChunk::builder().content("world!").build(),
    ];
    let result: HumanMessageChunk = chunks.into_iter().sum();
    assert_eq!(result.content.as_text(), "Hello beautiful world!");
}
