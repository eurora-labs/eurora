//! Tests for ChatMessage and ChatMessageChunk.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/test_chat.py`

use agent_chain_core::messages::{ChatMessage, ChatMessageChunk};

// ============================================================================
// TestChatMessage
// ============================================================================

#[test]
fn test_init_basic() {
    let msg = ChatMessage::new("Hello", "user");
    assert_eq!(msg.content(), "Hello");
    assert_eq!(msg.role(), "user");
    assert_eq!(msg.message_type(), "chat");
}

#[test]
fn test_init_with_name() {
    let msg = ChatMessage::new("Hello", "assistant").with_name("bot");
    assert_eq!(msg.name(), Some("bot".to_string()));
    assert_eq!(msg.role(), "assistant");
}

#[test]
fn test_init_with_id() {
    let msg = ChatMessage::with_id("msg-123", "Hello", "user");
    assert_eq!(msg.id(), Some("msg-123".to_string()));
}

#[test]
fn test_init_with_additional_kwargs() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("custom".to_string(), serde_json::json!("value"));

    let msg = ChatMessage::new("Hello", "user").with_additional_kwargs(additional_kwargs);
    assert_eq!(
        msg.additional_kwargs().get("custom").unwrap(),
        &serde_json::json!("value")
    );
}

#[test]
fn test_init_with_response_metadata() {
    let mut response_metadata = std::collections::HashMap::new();
    response_metadata.insert("model".to_string(), serde_json::json!("custom"));

    let msg = ChatMessage::new("Hello", "system").with_response_metadata(response_metadata);
    assert_eq!(
        msg.response_metadata().get("model").unwrap(),
        &serde_json::json!("custom")
    );
}

#[test]
fn test_different_roles() {
    let roles = vec!["user", "assistant", "system", "admin", "custom_role"];
    for role in roles {
        let msg = ChatMessage::new("Test", role);
        assert_eq!(msg.role(), role);
    }
}

#[test]
fn test_type_is_chat() {
    let msg = ChatMessage::new("Test", "user");
    assert_eq!(msg.message_type(), "chat");
}

#[test]
fn test_serialization_roundtrip() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("priority".to_string(), serde_json::json!("high"));

    let msg = ChatMessage::with_id("chat-123", "Hello", "moderator")
        .with_name("mod1")
        .with_additional_kwargs(additional_kwargs);

    let serialized = serde_json::to_value(&msg).unwrap();
    assert_eq!(serialized.get("type").unwrap().as_str().unwrap(), "chat");

    let deserialized: ChatMessage = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content(), "Hello");
    assert_eq!(deserialized.role(), "moderator");
    assert_eq!(deserialized.name(), Some("mod1".to_string()));
    assert_eq!(deserialized.id(), Some("chat-123".to_string()));
    assert_eq!(
        deserialized.additional_kwargs().get("priority").unwrap(),
        &serde_json::json!("high")
    );
}

#[test]
fn test_text_property() {
    let msg = ChatMessage::new("Hello world", "user");
    assert_eq!(msg.text(), "Hello world");
}

// ============================================================================
// TestChatMessageChunk
// ============================================================================

#[test]
fn test_chunk_init_basic() {
    let chunk = ChatMessageChunk::new("Hello", "user");
    assert_eq!(chunk.content(), "Hello");
    assert_eq!(chunk.role(), "user");
    assert_eq!(chunk.message_type(), "ChatMessageChunk");
}

#[test]
fn test_chunk_type_is_chat_message_chunk() {
    let chunk = ChatMessageChunk::new("Test", "user");
    assert_eq!(chunk.message_type(), "ChatMessageChunk");
}

#[test]
fn test_chunk_add_same_role_chunks() {
    let chunk1 = ChatMessageChunk::with_id("1", "Hello", "user");
    let chunk2 = ChatMessageChunk::new(" world", "user");
    let result = chunk1 + chunk2;
    assert_eq!(result.content(), "Hello world");
    assert_eq!(result.role(), "user");
    assert_eq!(result.id(), Some("1".to_string()));
}

#[test]
#[should_panic(expected = "Cannot concatenate")]
fn test_chunk_add_different_role_chunks_raises_error() {
    let chunk1 = ChatMessageChunk::new("Hello", "user");
    let chunk2 = ChatMessageChunk::new(" world", "assistant");
    let _result = chunk1 + chunk2;
}

#[test]
fn test_chunk_add_with_additional_kwargs() {
    let mut kwargs1 = std::collections::HashMap::new();
    kwargs1.insert("key1".to_string(), serde_json::json!("value1"));

    let mut kwargs2 = std::collections::HashMap::new();
    kwargs2.insert("key2".to_string(), serde_json::json!("value2"));

    let chunk1 = ChatMessageChunk::new("Hello", "user").with_additional_kwargs(kwargs1);
    let chunk2 = ChatMessageChunk::new(" world", "user").with_additional_kwargs(kwargs2);

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

    let chunk1 = ChatMessageChunk::new("Hello", "user").with_response_metadata(meta1);
    let chunk2 = ChatMessageChunk::new(" world", "user").with_response_metadata(meta2);

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
    let chunk1 = ChatMessageChunk::with_id("original-id", "Hello", "user");
    let chunk2 = ChatMessageChunk::with_id("other-id", " world", "user");
    let result = chunk1 + chunk2;
    assert_eq!(result.id(), Some("original-id".to_string()));
}

#[test]
fn test_chunk_serialization_roundtrip() {
    let chunk = ChatMessageChunk::with_id("chunk-123", "Hello", "moderator");

    let serialized = serde_json::to_value(&chunk).unwrap();
    assert_eq!(
        serialized.get("type").unwrap().as_str().unwrap(),
        "ChatMessageChunk"
    );

    let deserialized: ChatMessageChunk = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content(), "Hello");
    assert_eq!(deserialized.role(), "moderator");
    assert_eq!(deserialized.id(), Some("chunk-123".to_string()));
}

#[test]
fn test_chunk_multiple_additions() {
    let chunk1 = ChatMessageChunk::new("a", "user");
    let chunk2 = ChatMessageChunk::new("b", "user");
    let chunk3 = ChatMessageChunk::new("c", "user");
    let result = chunk1 + chunk2 + chunk3;
    assert_eq!(result.content(), "abc");
    assert_eq!(result.role(), "user");
}

#[test]
fn test_chunk_empty_content() {
    let chunk1 = ChatMessageChunk::new("Hello", "user");
    let chunk2 = ChatMessageChunk::new("", "user");
    let result = chunk1 + chunk2;
    assert_eq!(result.content(), "Hello");
}

#[test]
fn test_chunk_text_property() {
    let chunk = ChatMessageChunk::new("Hello world", "user");
    assert_eq!(chunk.text(), "Hello world");
}
