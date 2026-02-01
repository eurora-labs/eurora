//! Tests for ChatMessage and ChatMessageChunk.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/test_chat.py`

use agent_chain_core::messages::{ChatMessage, ChatMessageChunk};

// ============================================================================
// TestChatMessage
// ============================================================================

#[test]
fn test_init_basic() {
    let msg = ChatMessage::builder().content("Hello").role("user").build();
    assert_eq!(msg.content, "Hello");
    assert_eq!(msg.role, "user");
    assert_eq!(msg.message_type(), "chat");
}

#[test]
fn test_init_with_name() {
    let msg = ChatMessage::builder()
        .content("Hello")
        .role("assistant")
        .name("bot".to_string())
        .build();
    assert_eq!(msg.name, Some("bot".to_string()));
    assert_eq!(msg.role, "assistant");
}

#[test]
fn test_init_with_id() {
    let msg = ChatMessage::builder()
        .id("msg-123".to_string())
        .content("Hello")
        .role("user")
        .build();
    assert_eq!(msg.id, Some("msg-123".to_string()));
}

#[test]
fn test_init_with_additional_kwargs() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("custom".to_string(), serde_json::json!("value"));

    let msg = ChatMessage::builder()
        .content("Hello")
        .role("user")
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
    response_metadata.insert("model".to_string(), serde_json::json!("custom"));

    let msg = ChatMessage::builder()
        .content("Hello")
        .role("system")
        .response_metadata(response_metadata)
        .build();
    assert_eq!(
        msg.response_metadata.get("model").unwrap(),
        &serde_json::json!("custom")
    );
}

#[test]
fn test_different_roles() {
    let roles = vec!["user", "assistant", "system", "admin", "custom_role"];
    for role in roles {
        let msg = ChatMessage::builder().content("Test").role(role).build();
        assert_eq!(msg.role, role);
    }
}

#[test]
fn test_type_is_chat() {
    let msg = ChatMessage::builder().content("Test").role("user").build();
    assert_eq!(msg.message_type(), "chat");
}

#[test]
fn test_serialization_roundtrip() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("priority".to_string(), serde_json::json!("high"));

    let msg = ChatMessage::builder()
        .id("chat-123".to_string())
        .content("Hello")
        .role("moderator")
        .name("mod1".to_string())
        .additional_kwargs(additional_kwargs)
        .build();

    let serialized = serde_json::to_value(&msg).unwrap();
    assert_eq!(serialized.get("type").unwrap().as_str().unwrap(), "chat");

    let deserialized: ChatMessage = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content, "Hello");
    assert_eq!(deserialized.role, "moderator");
    assert_eq!(deserialized.name, Some("mod1".to_string()));
    assert_eq!(deserialized.id, Some("chat-123".to_string()));
    assert_eq!(
        deserialized.additional_kwargs.get("priority").unwrap(),
        &serde_json::json!("high")
    );
}

#[test]
fn test_content_property() {
    let msg = ChatMessage::builder()
        .content("Hello world")
        .role("user")
        .build();
    assert_eq!(msg.content, "Hello world");
}

// ============================================================================
// TestChatMessageChunk
// ============================================================================

#[test]
fn test_chunk_init_basic() {
    let chunk = ChatMessageChunk::builder()
        .content("Hello")
        .role("user")
        .build();
    assert_eq!(chunk.content, "Hello");
    assert_eq!(chunk.role, "user");
    assert_eq!(chunk.message_type(), "ChatMessageChunk");
}

#[test]
fn test_chunk_type_is_chat_message_chunk() {
    let chunk = ChatMessageChunk::builder()
        .content("Test")
        .role("user")
        .build();
    assert_eq!(chunk.message_type(), "ChatMessageChunk");
}

#[test]
fn test_chunk_add_same_role_chunks() {
    let chunk1 = ChatMessageChunk::builder()
        .id("1".to_string())
        .content("Hello")
        .role("user")
        .build();
    let chunk2 = ChatMessageChunk::builder()
        .content(" world")
        .role("user")
        .build();
    let result = chunk1 + chunk2;
    assert_eq!(result.content, "Hello world");
    assert_eq!(result.role, "user");
    assert_eq!(result.id, Some("1".to_string()));
}

#[test]
#[should_panic(expected = "Cannot concatenate")]
fn test_chunk_add_different_role_chunks_raises_error() {
    let chunk1 = ChatMessageChunk::builder()
        .content("Hello")
        .role("user")
        .build();
    let chunk2 = ChatMessageChunk::builder()
        .content(" world")
        .role("assistant")
        .build();
    let _result = chunk1 + chunk2;
}

#[test]
fn test_chunk_add_with_additional_kwargs() {
    let mut kwargs1 = std::collections::HashMap::new();
    kwargs1.insert("key1".to_string(), serde_json::json!("value1"));

    let mut kwargs2 = std::collections::HashMap::new();
    kwargs2.insert("key2".to_string(), serde_json::json!("value2"));

    let chunk1 = ChatMessageChunk::builder()
        .content("Hello")
        .role("user")
        .additional_kwargs(kwargs1)
        .build();
    let chunk2 = ChatMessageChunk::builder()
        .content(" world")
        .role("user")
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

    let chunk1 = ChatMessageChunk::builder()
        .content("Hello")
        .role("user")
        .response_metadata(meta1)
        .build();
    let chunk2 = ChatMessageChunk::builder()
        .content(" world")
        .role("user")
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
    let chunk1 = ChatMessageChunk::builder()
        .id("original-id".to_string())
        .content("Hello")
        .role("user")
        .build();
    let chunk2 = ChatMessageChunk::builder()
        .id("other-id".to_string())
        .content(" world")
        .role("user")
        .build();
    let result = chunk1 + chunk2;
    assert_eq!(result.id, Some("original-id".to_string()));
}

#[test]
fn test_chunk_serialization_roundtrip() {
    let chunk = ChatMessageChunk::builder()
        .id("chunk-123".to_string())
        .content("Hello")
        .role("moderator")
        .build();

    let serialized = serde_json::to_value(&chunk).unwrap();
    assert_eq!(
        serialized.get("type").unwrap().as_str().unwrap(),
        "ChatMessageChunk"
    );

    let deserialized: ChatMessageChunk = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content, "Hello");
    assert_eq!(deserialized.role, "moderator");
    assert_eq!(deserialized.id, Some("chunk-123".to_string()));
}

#[test]
fn test_chunk_multiple_additions() {
    let chunk1 = ChatMessageChunk::builder()
        .content("a")
        .role("user")
        .build();
    let chunk2 = ChatMessageChunk::builder()
        .content("b")
        .role("user")
        .build();
    let chunk3 = ChatMessageChunk::builder()
        .content("c")
        .role("user")
        .build();
    let result = chunk1 + chunk2 + chunk3;
    assert_eq!(result.content, "abc");
    assert_eq!(result.role, "user");
}

#[test]
fn test_chunk_empty_content() {
    let chunk1 = ChatMessageChunk::builder()
        .content("Hello")
        .role("user")
        .build();
    let chunk2 = ChatMessageChunk::builder().content("").role("user").build();
    let result = chunk1 + chunk2;
    assert_eq!(result.content, "Hello");
}

#[test]
fn test_chunk_content_property() {
    let chunk = ChatMessageChunk::builder()
        .content("Hello world")
        .role("user")
        .build();
    assert_eq!(chunk.content, "Hello world");
}
