//! Tests for base message functionality.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/test_base.py`

use agent_chain_core::messages::{
    AIMessage, BaseMessage, HumanMessage, HumanMessageChunk, SystemMessage,
    SystemMessageChunk, merge_content, message_to_dict, messages_to_dict,
};
use serde_json::json;

// ============================================================================
// TestTextAccessor - Tests for text property behavior
// ============================================================================

#[test]
fn test_text_property_string_content() {
    let msg = HumanMessage::new("Hello, world!");
    assert_eq!(msg.text(), "Hello, world!");
}

#[test]
fn test_text_property_empty_content() {
    let msg = HumanMessage::new("");
    assert_eq!(msg.text(), "");
}

// ============================================================================
// TestMergeContent - Tests for merge_content function
// ============================================================================

#[test]
fn test_merge_two_strings() {
    let result = merge_content("Hello", " world");
    assert_eq!(result, "Hello world");
}

#[test]
fn test_merge_multiple_strings() {
    let mut result = merge_content("a", "b");
    result = merge_content(&result, "c");
    result = merge_content(&result, "d");
    assert_eq!(result, "abcd");
}

#[test]
fn test_merge_empty_string_first() {
    let result = merge_content("", "Hello");
    assert_eq!(result, "Hello");
}

#[test]
fn test_merge_empty_string_second() {
    let result = merge_content("Hello", "");
    assert_eq!(result, "Hello");
}

// ============================================================================
// TestMessageToDict - Tests for message_to_dict and messages_to_dict
// ============================================================================

#[test]
fn test_message_to_dict_human_message() {
    let msg = HumanMessage::with_id("msg1", "Hello").with_name("user1");
    let result = message_to_dict(&BaseMessage::Human(msg));
    assert_eq!(result.get("type").unwrap().as_str().unwrap(), "human");
    assert_eq!(
        result
            .get("data")
            .unwrap()
            .get("content")
            .unwrap()
            .as_str()
            .unwrap(),
        "Hello"
    );
    assert_eq!(
        result
            .get("data")
            .unwrap()
            .get("name")
            .unwrap()
            .as_str()
            .unwrap(),
        "user1"
    );
    assert_eq!(
        result
            .get("data")
            .unwrap()
            .get("id")
            .unwrap()
            .as_str()
            .unwrap(),
        "msg1"
    );
}

#[test]
fn test_message_to_dict_ai_message() {
    let msg = AIMessage::with_id("ai1", "Hi there");
    let result = message_to_dict(&BaseMessage::AI(msg));
    assert_eq!(result.get("type").unwrap().as_str().unwrap(), "ai");
    assert_eq!(
        result
            .get("data")
            .unwrap()
            .get("content")
            .unwrap()
            .as_str()
            .unwrap(),
        "Hi there"
    );
    assert_eq!(
        result
            .get("data")
            .unwrap()
            .get("id")
            .unwrap()
            .as_str()
            .unwrap(),
        "ai1"
    );
}

#[test]
fn test_message_to_dict_system_message() {
    let msg = SystemMessage::new("You are a helpful assistant");
    let result = message_to_dict(&BaseMessage::System(msg));
    assert_eq!(result.get("type").unwrap().as_str().unwrap(), "system");
    assert_eq!(
        result
            .get("data")
            .unwrap()
            .get("content")
            .unwrap()
            .as_str()
            .unwrap(),
        "You are a helpful assistant"
    );
}

#[test]
fn test_message_to_dict_with_additional_kwargs() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert(
        "function_call".to_string(),
        json!({"name": "test", "arguments": "{}"}),
    );

    let msg = AIMessage::new("Hello").with_additional_kwargs(additional_kwargs);
    let result = message_to_dict(&BaseMessage::AI(msg));
    assert_eq!(
        result
            .get("data")
            .unwrap()
            .get("additional_kwargs")
            .unwrap()
            .get("function_call")
            .unwrap()
            .get("name")
            .unwrap()
            .as_str()
            .unwrap(),
        "test"
    );
}

#[test]
fn test_messages_to_dict_multiple_messages() {
    let messages = vec![
        BaseMessage::System(SystemMessage::new("System")),
        BaseMessage::Human(HumanMessage::new("Hello")),
        BaseMessage::AI(AIMessage::new("Hi")),
    ];
    let result = messages_to_dict(&messages);
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].get("type").unwrap().as_str().unwrap(), "system");
    assert_eq!(result[1].get("type").unwrap().as_str().unwrap(), "human");
    assert_eq!(result[2].get("type").unwrap().as_str().unwrap(), "ai");
}

#[test]
fn test_messages_to_dict_empty_list() {
    let messages: Vec<BaseMessage> = vec![];
    let result = messages_to_dict(&messages);
    assert!(result.is_empty());
}

// ============================================================================
// TestBaseMessageChunkAdd - Tests for chunk addition
// ============================================================================

#[test]
fn test_add_human_message_chunks() {
    let chunk1 = HumanMessageChunk::with_id("1", "Hello");
    let chunk2 = HumanMessageChunk::new(" world");
    let result = chunk1 + chunk2;
    assert_eq!(result.content(), "Hello world");
    assert_eq!(result.id(), Some("1".to_string()));
}

#[test]
fn test_add_system_message_chunks() {
    let chunk1 = SystemMessageChunk::new("You are");
    let chunk2 = SystemMessageChunk::new(" helpful");
    let result = chunk1 + chunk2;
    assert_eq!(result.content(), "You are helpful");
}

#[test]
fn test_add_chunks_with_additional_kwargs() {
    let mut kwargs1 = std::collections::HashMap::new();
    kwargs1.insert("key1".to_string(), json!("value1"));

    let mut kwargs2 = std::collections::HashMap::new();
    kwargs2.insert("key2".to_string(), json!("value2"));

    let chunk1 = HumanMessageChunk::new("Hello").with_additional_kwargs(kwargs1);
    let chunk2 = HumanMessageChunk::new(" world").with_additional_kwargs(kwargs2);

    let result = chunk1 + chunk2;
    assert_eq!(
        result.additional_kwargs().get("key1").unwrap(),
        &json!("value1")
    );
    assert_eq!(
        result.additional_kwargs().get("key2").unwrap(),
        &json!("value2")
    );
}

#[test]
fn test_add_chunks_with_response_metadata() {
    let mut meta1 = std::collections::HashMap::new();
    meta1.insert("meta1".to_string(), json!("data1"));

    let mut meta2 = std::collections::HashMap::new();
    meta2.insert("meta2".to_string(), json!("data2"));

    let chunk1 = HumanMessageChunk::new("Hello").with_response_metadata(meta1);
    let chunk2 = HumanMessageChunk::new(" world").with_response_metadata(meta2);

    let result = chunk1 + chunk2;
    assert_eq!(
        result.response_metadata().get("meta1").unwrap(),
        &json!("data1")
    );
    assert_eq!(
        result.response_metadata().get("meta2").unwrap(),
        &json!("data2")
    );
}

// ============================================================================
// TestBaseMessageInit - Tests for message initialization
// ============================================================================

#[test]
fn test_init_with_string_content() {
    let msg = HumanMessage::new("Hello world");
    assert_eq!(msg.content(), "Hello world");
}

#[test]
fn test_init_with_id() {
    let msg = HumanMessage::with_id("msg-123", "Hello");
    assert_eq!(msg.id(), Some("msg-123".to_string()));
}

#[test]
fn test_init_with_name() {
    let msg = HumanMessage::new("Hello").with_name("user1");
    assert_eq!(msg.name(), Some("user1".to_string()));
}

#[test]
fn test_init_with_additional_kwargs() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("custom_key".to_string(), json!("custom_value"));

    let msg = HumanMessage::new("Hello").with_additional_kwargs(additional_kwargs);
    assert_eq!(
        msg.additional_kwargs().get("custom_key").unwrap(),
        &json!("custom_value")
    );
}

#[test]
fn test_init_with_response_metadata() {
    let mut response_metadata = std::collections::HashMap::new();
    response_metadata.insert("model".to_string(), json!("gpt-4"));
    response_metadata.insert("tokens".to_string(), json!(10));

    let msg = AIMessage::new("Hello").with_response_metadata(response_metadata);
    assert_eq!(
        msg.response_metadata().get("model").unwrap(),
        &json!("gpt-4")
    );
    assert_eq!(msg.response_metadata().get("tokens").unwrap(), &json!(10));
}

// ============================================================================
// TestBaseMessageSerialization
// ============================================================================

#[test]
fn test_message_types_have_consistent_types() {
    let human_msg = HumanMessage::new("Hello");
    let ai_msg = AIMessage::new("Hi");
    let system_msg = SystemMessage::new("You are helpful");

    assert_eq!(human_msg.message_type(), "human");
    assert_eq!(ai_msg.message_type(), "ai");
    assert_eq!(system_msg.message_type(), "system");
}
