//! Tests for SystemMessage and SystemMessageChunk.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/test_system.py`

use agent_chain_core::messages::{HumanMessageChunk, SystemMessage, SystemMessageChunk};

// ============================================================================
// TestSystemMessage
// ============================================================================

#[test]
fn test_init_basic() {
    let msg = SystemMessage::new("You are a helpful assistant.");
    assert_eq!(msg.content(), "You are a helpful assistant.");
    assert_eq!(msg.message_type(), "system");
}

#[test]
fn test_init_with_name() {
    let msg = SystemMessage::new("Instructions").with_name("system_prompt");
    assert_eq!(msg.name(), Some("system_prompt".to_string()));
}

#[test]
fn test_init_with_id() {
    let msg = SystemMessage::with_id("sys-123", "Instructions");
    assert_eq!(msg.id(), Some("sys-123".to_string()));
}

#[test]
fn test_init_with_additional_kwargs() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("priority".to_string(), serde_json::json!("high"));

    let msg = SystemMessage::new("Instructions").with_additional_kwargs(additional_kwargs);
    assert_eq!(
        msg.additional_kwargs().get("priority").unwrap(),
        &serde_json::json!("high")
    );
}

#[test]
fn test_init_with_response_metadata() {
    let mut response_metadata = std::collections::HashMap::new();
    response_metadata.insert("version".to_string(), serde_json::json!("1.0"));

    let msg = SystemMessage::new("Instructions").with_response_metadata(response_metadata);
    assert_eq!(
        msg.response_metadata().get("version").unwrap(),
        &serde_json::json!("1.0")
    );
}

#[test]
fn test_type_is_system() {
    let msg = SystemMessage::new("Test");
    assert_eq!(msg.message_type(), "system");
}

#[test]
fn test_serialization_roundtrip() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("version".to_string(), serde_json::json!("1.0"));

    let msg = SystemMessage::with_id("sys-123", "You are a helpful assistant.")
        .with_name("system_prompt")
        .with_additional_kwargs(additional_kwargs);

    let serialized = serde_json::to_value(&msg).unwrap();
    assert_eq!(serialized.get("type").unwrap().as_str().unwrap(), "system");

    let deserialized: SystemMessage = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content(), "You are a helpful assistant.");
    assert_eq!(deserialized.name(), Some("system_prompt".to_string()));
    assert_eq!(deserialized.id(), Some("sys-123".to_string()));
    assert_eq!(
        deserialized.additional_kwargs().get("version").unwrap(),
        &serde_json::json!("1.0")
    );
}

#[test]
fn test_text_property() {
    let msg = SystemMessage::new("Hello world");
    assert_eq!(msg.text(), "Hello world");
}

#[test]
fn test_empty_content() {
    let msg = SystemMessage::new("");
    assert_eq!(msg.content(), "");
    assert_eq!(msg.text(), "");
}

#[test]
fn test_developer_role_via_additional_kwargs() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert(
        "__openai_role__".to_string(),
        serde_json::json!("developer"),
    );

    let msg =
        SystemMessage::new("Developer instructions").with_additional_kwargs(additional_kwargs);
    assert_eq!(
        msg.additional_kwargs().get("__openai_role__").unwrap(),
        &serde_json::json!("developer")
    );
}

// ============================================================================
// TestSystemMessageChunk
// ============================================================================

#[test]
fn test_chunk_init_basic() {
    let chunk = SystemMessageChunk::new("Instructions");
    assert_eq!(chunk.content(), "Instructions");
    assert_eq!(chunk.message_type(), "SystemMessageChunk");
}

#[test]
fn test_chunk_type_is_system_message_chunk() {
    let chunk = SystemMessageChunk::new("Test");
    assert_eq!(chunk.message_type(), "SystemMessageChunk");
}

#[test]
fn test_chunk_add_two_chunks() {
    let chunk1 = SystemMessageChunk::with_id("1", "Hello");
    let chunk2 = SystemMessageChunk::new(" world");
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

    let chunk1 = SystemMessageChunk::new("Hello").with_additional_kwargs(kwargs1);
    let chunk2 = SystemMessageChunk::new(" world").with_additional_kwargs(kwargs2);

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

    let chunk1 = SystemMessageChunk::new("Hello").with_response_metadata(meta1);
    let chunk2 = SystemMessageChunk::new(" world").with_response_metadata(meta2);

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
    let chunk1 = SystemMessageChunk::with_id("original-id", "Hello");
    let chunk2 = SystemMessageChunk::with_id("other-id", " world");
    let result = chunk1 + chunk2;
    assert_eq!(result.id(), Some("original-id".to_string()));
}

#[test]
fn test_chunk_serialization_roundtrip() {
    let chunk = SystemMessageChunk::with_id("chunk-123", "Instructions").with_name("sys_prompt");

    let serialized = serde_json::to_value(&chunk).unwrap();
    assert_eq!(
        serialized.get("type").unwrap().as_str().unwrap(),
        "SystemMessageChunk"
    );

    let deserialized: SystemMessageChunk = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content(), "Instructions");
    assert_eq!(deserialized.name(), Some("sys_prompt".to_string()));
    assert_eq!(deserialized.id(), Some("chunk-123".to_string()));
}

#[test]
fn test_chunk_multiple_additions() {
    let chunk1 = SystemMessageChunk::new("a");
    let chunk2 = SystemMessageChunk::new("b");
    let chunk3 = SystemMessageChunk::new("c");
    let result = chunk1 + chunk2 + chunk3;
    assert_eq!(result.content(), "abc");
}

#[test]
fn test_chunk_empty_content() {
    let chunk1 = SystemMessageChunk::new("Hello");
    let chunk2 = SystemMessageChunk::new("");
    let result = chunk1 + chunk2;
    assert_eq!(result.content(), "Hello");
}

#[test]
fn test_chunk_add_different_chunk_type() {
    // In Rust, we can't directly add different chunk types together like in Python.
    // Instead, we test that we can convert chunks to messages and work with them.
    let chunk1 = SystemMessageChunk::with_id("1", "Hello");
    let chunk2 = HumanMessageChunk::new(" world");

    // Convert to messages and verify content
    let msg1 = chunk1.to_message();
    let msg2: agent_chain_core::messages::HumanMessage = chunk2.into();

    // Verify both messages have their content
    assert_eq!(msg1.content(), "Hello");
    assert_eq!(msg2.content(), " world");

    // We can concatenate content strings manually
    let combined_content = format!("{}{}", msg1.content(), msg2.content());
    assert_eq!(combined_content, "Hello world");
}

#[test]
fn test_chunk_text_property() {
    let chunk = SystemMessageChunk::new("Hello world");
    assert_eq!(chunk.text(), "Hello world");
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

    let msg =
        SystemMessage::new("Developer instructions").with_additional_kwargs(additional_kwargs);

    let serialized = serde_json::to_value(&msg).unwrap();
    let deserialized: SystemMessage = serde_json::from_value(serialized).unwrap();

    assert_eq!(
        deserialized
            .additional_kwargs()
            .get("__openai_role__")
            .unwrap(),
        &serde_json::json!("developer")
    );
}

#[test]
fn test_multiple_system_messages_with_different_roles() {
    let system_msg = SystemMessage::new("System instructions");

    let mut dev_kwargs = std::collections::HashMap::new();
    dev_kwargs.insert(
        "__openai_role__".to_string(),
        serde_json::json!("developer"),
    );
    let developer_msg =
        SystemMessage::new("Developer instructions").with_additional_kwargs(dev_kwargs);

    assert!(
        !system_msg
            .additional_kwargs()
            .contains_key("__openai_role__")
    );
    assert_eq!(
        developer_msg
            .additional_kwargs()
            .get("__openai_role__")
            .unwrap(),
        &serde_json::json!("developer")
    );
}
