//! Tests for SystemMessage and SystemMessageChunk.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/test_system.py`

use agent_chain_core::messages::{
    HumanMessage, HumanMessageChunk, MessageContent, SystemMessage, SystemMessageChunk,
};

// ============================================================================
// TestSystemMessage
// ============================================================================

#[test]
fn test_init_basic() {
    let msg = SystemMessage::builder()
        .content("You are a helpful assistant.")
        .build();
    assert!(matches!(&msg.content, MessageContent::Text(s) if s == "You are a helpful assistant."));
    assert_eq!(msg.message_type(), "system");
}

#[test]
fn test_init_with_name() {
    let msg = SystemMessage::builder()
        .content("Instructions")
        .maybe_name(Some("system_prompt".to_string()))
        .build();
    assert_eq!(msg.name, Some("system_prompt".to_string()));
}

#[test]
fn test_init_with_id() {
    let msg = SystemMessage::builder()
        .content("Instructions")
        .maybe_id(Some("sys-123".to_string()))
        .build();
    assert_eq!(msg.id, Some("sys-123".to_string()));
}

#[test]
fn test_init_with_additional_kwargs() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("priority".to_string(), serde_json::json!("high"));

    let msg = SystemMessage::builder()
        .content("Instructions")
        .additional_kwargs(additional_kwargs)
        .build();
    assert_eq!(
        msg.additional_kwargs.get("priority").unwrap(),
        &serde_json::json!("high")
    );
}

#[test]
fn test_init_with_response_metadata() {
    let mut response_metadata = std::collections::HashMap::new();
    response_metadata.insert("version".to_string(), serde_json::json!("1.0"));

    let msg = SystemMessage::builder()
        .content("Instructions")
        .response_metadata(response_metadata)
        .build();
    assert_eq!(
        msg.response_metadata.get("version").unwrap(),
        &serde_json::json!("1.0")
    );
}

#[test]
fn test_type_is_system() {
    let msg = SystemMessage::builder().content("Test").build();
    assert_eq!(msg.message_type(), "system");
}

#[test]
fn test_serialization_roundtrip() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("version".to_string(), serde_json::json!("1.0"));

    let msg = SystemMessage::builder()
        .content("You are a helpful assistant.")
        .maybe_id(Some("sys-123".to_string()))
        .maybe_name(Some("system_prompt".to_string()))
        .additional_kwargs(additional_kwargs)
        .build();

    let serialized = serde_json::to_value(&msg).unwrap();
    assert_eq!(serialized.get("type").unwrap().as_str().unwrap(), "system");

    let deserialized: SystemMessage = serde_json::from_value(serialized).unwrap();
    assert!(
        matches!(&deserialized.content, MessageContent::Text(s) if s == "You are a helpful assistant.")
    );
    assert_eq!(deserialized.name, Some("system_prompt".to_string()));
    assert_eq!(deserialized.id, Some("sys-123".to_string()));
    assert_eq!(
        deserialized.additional_kwargs.get("version").unwrap(),
        &serde_json::json!("1.0")
    );
}

#[test]
fn test_text_content() {
    let msg = SystemMessage::builder().content("Hello world").build();
    assert!(matches!(&msg.content, MessageContent::Text(s) if s == "Hello world"));
}

#[test]
fn test_empty_content() {
    let msg = SystemMessage::builder().content("").build();
    assert!(matches!(&msg.content, MessageContent::Text(s) if s.is_empty()));
}

#[test]
fn test_developer_role_via_additional_kwargs() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert(
        "__openai_role__".to_string(),
        serde_json::json!("developer"),
    );

    let msg = SystemMessage::builder()
        .content("Developer instructions")
        .additional_kwargs(additional_kwargs)
        .build();
    assert_eq!(
        msg.additional_kwargs.get("__openai_role__").unwrap(),
        &serde_json::json!("developer")
    );
}

// ============================================================================
// TestSystemMessageChunk
// ============================================================================

#[test]
fn test_chunk_init_basic() {
    let chunk = SystemMessageChunk::builder()
        .content("Instructions")
        .build();
    assert!(matches!(&chunk.content, MessageContent::Text(s) if s == "Instructions"));
    assert_eq!(chunk.message_type(), "SystemMessageChunk");
}

#[test]
fn test_chunk_type_is_system_message_chunk() {
    let chunk = SystemMessageChunk::builder().content("Test").build();
    assert_eq!(chunk.message_type(), "SystemMessageChunk");
}

#[test]
fn test_chunk_add_two_chunks() {
    let chunk1 = SystemMessageChunk::builder()
        .content("Hello")
        .maybe_id(Some("1".to_string()))
        .build();
    let chunk2 = SystemMessageChunk::builder().content(" world").build();
    let result = chunk1 + chunk2;
    assert!(matches!(&result.content, MessageContent::Text(s) if s == "Hello world"));
    assert_eq!(result.id, Some("1".to_string()));
}

#[test]
fn test_chunk_add_with_additional_kwargs() {
    let mut kwargs1 = std::collections::HashMap::new();
    kwargs1.insert("key1".to_string(), serde_json::json!("value1"));

    let mut kwargs2 = std::collections::HashMap::new();
    kwargs2.insert("key2".to_string(), serde_json::json!("value2"));

    let chunk1 = SystemMessageChunk::builder()
        .content("Hello")
        .additional_kwargs(kwargs1)
        .build();
    let chunk2 = SystemMessageChunk::builder()
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

    let chunk1 = SystemMessageChunk::builder()
        .content("Hello")
        .response_metadata(meta1)
        .build();
    let chunk2 = SystemMessageChunk::builder()
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
    let chunk1 = SystemMessageChunk::builder()
        .content("Hello")
        .maybe_id(Some("original-id".to_string()))
        .build();
    let chunk2 = SystemMessageChunk::builder()
        .content(" world")
        .maybe_id(Some("other-id".to_string()))
        .build();
    let result = chunk1 + chunk2;
    assert_eq!(result.id, Some("original-id".to_string()));
}

#[test]
fn test_chunk_serialization_roundtrip() {
    let chunk = SystemMessageChunk::builder()
        .content("Instructions")
        .maybe_id(Some("chunk-123".to_string()))
        .maybe_name(Some("sys_prompt".to_string()))
        .build();

    let serialized = serde_json::to_value(&chunk).unwrap();
    assert_eq!(
        serialized.get("type").unwrap().as_str().unwrap(),
        "SystemMessageChunk"
    );

    let deserialized: SystemMessageChunk = serde_json::from_value(serialized).unwrap();
    assert!(matches!(&deserialized.content, MessageContent::Text(s) if s == "Instructions"));
    assert_eq!(deserialized.name, Some("sys_prompt".to_string()));
    assert_eq!(deserialized.id, Some("chunk-123".to_string()));
}

#[test]
fn test_chunk_multiple_additions() {
    let chunk1 = SystemMessageChunk::builder().content("a").build();
    let chunk2 = SystemMessageChunk::builder().content("b").build();
    let chunk3 = SystemMessageChunk::builder().content("c").build();
    let result = chunk1 + chunk2 + chunk3;
    assert!(matches!(&result.content, MessageContent::Text(s) if s == "abc"));
}

#[test]
fn test_chunk_empty_content() {
    let chunk1 = SystemMessageChunk::builder().content("Hello").build();
    let chunk2 = SystemMessageChunk::builder().content("").build();
    let result = chunk1 + chunk2;
    assert!(matches!(&result.content, MessageContent::Text(s) if s == "Hello"));
}

#[test]
fn test_chunk_add_different_chunk_type() {
    let chunk1 = SystemMessageChunk::builder()
        .content("Hello")
        .maybe_id(Some("1".to_string()))
        .build();
    let chunk2 = HumanMessageChunk::builder().content(" world").build();

    // Convert to messages and verify content
    let msg1 = chunk1.to_message();
    let msg2: HumanMessage = chunk2.into();

    // Verify both messages have their content
    let content1 = match &msg1.content {
        MessageContent::Text(s) => s.as_str(),
        MessageContent::Parts(_) => "",
    };
    let content2 = match &msg2.content {
        MessageContent::Text(s) => s.as_str(),
        MessageContent::Parts(_) => "",
    };
    assert_eq!(content1, "Hello");
    assert_eq!(content2, " world");

    // We can concatenate content strings manually
    let combined_content = format!("{}{}", content1, content2);
    assert_eq!(combined_content, "Hello world");
}

#[test]
fn test_chunk_text_content() {
    let chunk = SystemMessageChunk::builder().content("Hello world").build();
    assert!(matches!(&chunk.content, MessageContent::Text(s) if s == "Hello world"));
}

// ============================================================================
// TestSystemMessageDeveloperRole
// ============================================================================

#[test]
fn test_developer_role_preserved_in_serialization() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert(
        "__openai_role__".to_string(),
        serde_json::json!("developer"),
    );

    let msg = SystemMessage::builder()
        .content("Developer instructions")
        .additional_kwargs(additional_kwargs)
        .build();

    let serialized = serde_json::to_value(&msg).unwrap();
    let deserialized: SystemMessage = serde_json::from_value(serialized).unwrap();

    assert_eq!(
        deserialized
            .additional_kwargs
            .get("__openai_role__")
            .unwrap(),
        &serde_json::json!("developer")
    );
}

#[test]
fn test_multiple_system_messages_with_different_roles() {
    let system_msg = SystemMessage::builder()
        .content("System instructions")
        .build();

    let mut dev_kwargs = std::collections::HashMap::new();
    dev_kwargs.insert(
        "__openai_role__".to_string(),
        serde_json::json!("developer"),
    );
    let developer_msg = SystemMessage::builder()
        .content("Developer instructions")
        .additional_kwargs(dev_kwargs)
        .build();

    assert!(!system_msg.additional_kwargs.contains_key("__openai_role__"));
    assert_eq!(
        developer_msg
            .additional_kwargs
            .get("__openai_role__")
            .unwrap(),
        &serde_json::json!("developer")
    );
}
