//! Tests for RemoveMessage.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/test_modifier.py`

use agent_chain_core::messages::{AIMessage, BaseMessage, HumanMessage, RemoveMessage};

// ============================================================================
// TestRemoveMessage
// ============================================================================

#[test]
fn test_init_basic() {
    let msg = RemoveMessage::new("msg-to-remove");
    assert_eq!(msg.id(), Some("msg-to-remove".to_string()));
    assert_eq!(msg.message_type(), "remove");
    assert_eq!(msg.content(), "");
}

#[test]
fn test_type_is_remove() {
    let msg = RemoveMessage::new("msg-123");
    assert_eq!(msg.message_type(), "remove");
}

#[test]
fn test_content_is_empty_string() {
    let msg = RemoveMessage::new("msg-123");
    assert_eq!(msg.content(), "");
}

#[test]
fn test_serialization_roundtrip() {
    let msg = RemoveMessage::new("msg-to-remove");

    let serialized = serde_json::to_value(&msg).unwrap();
    assert_eq!(serialized.get("type").unwrap().as_str().unwrap(), "remove");

    let deserialized: RemoveMessage = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.id(), Some("msg-to-remove".to_string()));
    assert_eq!(deserialized.content(), "");
}

#[test]
fn test_with_name() {
    let msg = RemoveMessage::new("msg-123").with_name("delete-marker");
    assert_eq!(msg.name(), Some("delete-marker".to_string()));
    assert_eq!(msg.id(), Some("msg-123".to_string()));
}

#[test]
fn test_with_additional_kwargs() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("reason".to_string(), serde_json::json!("outdated"));

    let msg = RemoveMessage::new("msg-123").with_additional_kwargs(additional_kwargs);
    assert_eq!(
        msg.additional_kwargs().get("reason").unwrap(),
        &serde_json::json!("outdated")
    );
}

#[test]
fn test_with_response_metadata() {
    let mut response_metadata = std::collections::HashMap::new();
    response_metadata.insert("deleted_at".to_string(), serde_json::json!("2024-01-01"));

    let msg = RemoveMessage::new("msg-123").with_response_metadata(response_metadata);
    assert_eq!(
        msg.response_metadata().get("deleted_at").unwrap(),
        &serde_json::json!("2024-01-01")
    );
}

#[test]
fn test_text_property_is_empty() {
    let msg = RemoveMessage::new("msg-123");
    assert_eq!(msg.text(), "");
}

#[test]
fn test_multiple_remove_messages() {
    let msg1 = RemoveMessage::new("msg-1");
    let msg2 = RemoveMessage::new("msg-2");
    let msg3 = RemoveMessage::new("msg-3");

    assert_eq!(msg1.id(), Some("msg-1".to_string()));
    assert_eq!(msg2.id(), Some("msg-2".to_string()));
    assert_eq!(msg3.id(), Some("msg-3".to_string()));

    // All should have empty content
    assert_eq!(msg1.content(), "");
    assert_eq!(msg2.content(), "");
    assert_eq!(msg3.content(), "");
}

// ============================================================================
// TestRemoveMessageUseCases
// ============================================================================

#[test]
fn test_remove_message_in_list() {
    let messages = [
        BaseMessage::Human(HumanMessage::with_id("human-1", "Hello")),
        BaseMessage::AI(
            AIMessage::builder()
                .content("Hi there!")
                .id("ai-1".to_string())
                .build(),
        ),
        BaseMessage::Remove(RemoveMessage::new("human-1")),
    ];

    assert_eq!(messages.len(), 3);
    assert!(matches!(messages[2], BaseMessage::Remove(_)));
    assert_eq!(messages[2].id(), Some("human-1".to_string()));
}

#[test]
fn test_remove_message_serialization_in_list() {
    let messages = [
        BaseMessage::Human(HumanMessage::with_id("human-1", "Hello")),
        BaseMessage::Remove(RemoveMessage::new("human-1")),
    ];

    // Serialize both messages
    let serialized: Vec<serde_json::Value> = messages
        .iter()
        .map(|m| serde_json::to_value(m).unwrap())
        .collect();

    assert_eq!(serialized.len(), 2);
    assert_eq!(
        serialized[0].get("type").unwrap().as_str().unwrap(),
        "human"
    );
    assert_eq!(
        serialized[1].get("type").unwrap().as_str().unwrap(),
        "remove"
    );

    // Deserialize both messages
    let deserialized: Vec<BaseMessage> = serialized
        .into_iter()
        .map(|s| serde_json::from_value(s).unwrap())
        .collect();

    assert!(matches!(deserialized[0], BaseMessage::Human(_)));
    assert!(matches!(deserialized[1], BaseMessage::Remove(_)));
    assert_eq!(deserialized[1].id(), Some("human-1".to_string()));
}

#[test]
fn test_remove_message_does_not_modify_content() {
    let msg = RemoveMessage::new("msg-123");

    // Content should always be empty string
    assert_eq!(msg.content(), "");
}
