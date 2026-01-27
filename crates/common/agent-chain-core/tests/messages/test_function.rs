//! Tests for FunctionMessage and FunctionMessageChunk.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/test_function.py`

use agent_chain_core::messages::{FunctionMessage, FunctionMessageChunk};

// ============================================================================
// TestFunctionMessage
// ============================================================================

#[test]
fn test_init_basic() {
    let msg = FunctionMessage::new("Result: 42", "calculator");
    assert_eq!(msg.content(), "Result: 42");
    assert_eq!(msg.name(), "calculator");
    assert_eq!(msg.message_type(), "function");
}

#[test]
fn test_init_with_id() {
    let msg = FunctionMessage::with_id("func-123", "Result", "func");
    assert_eq!(msg.id(), Some("func-123".to_string()));
}

#[test]
fn test_init_with_additional_kwargs() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("custom".to_string(), serde_json::json!("value"));

    let msg = FunctionMessage::new("Result", "func").with_additional_kwargs(additional_kwargs);
    assert_eq!(
        msg.additional_kwargs().get("custom").unwrap(),
        &serde_json::json!("value")
    );
}

#[test]
fn test_init_with_response_metadata() {
    let mut response_metadata = std::collections::HashMap::new();
    response_metadata.insert("model".to_string(), serde_json::json!("gpt-4"));

    let msg = FunctionMessage::new("Result", "func").with_response_metadata(response_metadata);
    assert_eq!(
        msg.response_metadata().get("model").unwrap(),
        &serde_json::json!("gpt-4")
    );
}

#[test]
fn test_type_is_function() {
    let msg = FunctionMessage::new("Result", "func");
    assert_eq!(msg.message_type(), "function");
}

#[test]
fn test_serialization_roundtrip() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("status".to_string(), serde_json::json!("success"));

    let msg = FunctionMessage::with_id("func-123", "Result: 42", "calculator")
        .with_additional_kwargs(additional_kwargs);

    let serialized = serde_json::to_value(&msg).unwrap();
    assert_eq!(
        serialized.get("type").unwrap().as_str().unwrap(),
        "function"
    );

    let deserialized: FunctionMessage = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content(), "Result: 42");
    assert_eq!(deserialized.name(), "calculator");
    assert_eq!(deserialized.id(), Some("func-123".to_string()));
    assert_eq!(
        deserialized.additional_kwargs().get("status").unwrap(),
        &serde_json::json!("success")
    );
}

#[test]
fn test_text_property() {
    let msg = FunctionMessage::new("Hello world", "func");
    assert_eq!(msg.text(), "Hello world");
}

// ============================================================================
// TestFunctionMessageChunk
// ============================================================================

#[test]
fn test_chunk_init_basic() {
    let chunk = FunctionMessageChunk::new("Result", "func");
    assert_eq!(chunk.content(), "Result");
    assert_eq!(chunk.name(), "func");
    assert_eq!(chunk.message_type(), "FunctionMessageChunk");
}

#[test]
fn test_chunk_type_is_function_message_chunk() {
    let chunk = FunctionMessageChunk::new("Result", "func");
    assert_eq!(chunk.message_type(), "FunctionMessageChunk");
}

#[test]
fn test_chunk_add_same_name_chunks() {
    let chunk1 = FunctionMessageChunk::with_id("1", "Hello", "func");
    let chunk2 = FunctionMessageChunk::new(" world", "func");
    let result = chunk1 + chunk2;
    assert_eq!(result.content(), "Hello world");
    assert_eq!(result.name(), "func");
    assert_eq!(result.id(), Some("1".to_string()));
}

#[test]
#[should_panic(expected = "Cannot concatenate")]
fn test_chunk_add_different_name_chunks_raises_error() {
    let chunk1 = FunctionMessageChunk::new("Hello", "func1");
    let chunk2 = FunctionMessageChunk::new(" world", "func2");
    let _result = chunk1 + chunk2;
}

#[test]
fn test_chunk_add_with_additional_kwargs() {
    let mut kwargs1 = std::collections::HashMap::new();
    kwargs1.insert("key1".to_string(), serde_json::json!("value1"));

    let mut kwargs2 = std::collections::HashMap::new();
    kwargs2.insert("key2".to_string(), serde_json::json!("value2"));

    let chunk1 = FunctionMessageChunk::new("Hello", "func").with_additional_kwargs(kwargs1);
    let chunk2 = FunctionMessageChunk::new(" world", "func").with_additional_kwargs(kwargs2);

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

    let chunk1 = FunctionMessageChunk::new("Hello", "func").with_response_metadata(meta1);
    let chunk2 = FunctionMessageChunk::new(" world", "func").with_response_metadata(meta2);

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
    let chunk1 = FunctionMessageChunk::with_id("original-id", "Hello", "func");
    let chunk2 = FunctionMessageChunk::with_id("other-id", " world", "func");
    let result = chunk1 + chunk2;
    assert_eq!(result.id(), Some("original-id".to_string()));
}

#[test]
fn test_chunk_serialization_roundtrip() {
    let chunk = FunctionMessageChunk::with_id("chunk-123", "Result", "calculator");

    let serialized = serde_json::to_value(&chunk).unwrap();
    assert_eq!(
        serialized.get("type").unwrap().as_str().unwrap(),
        "FunctionMessageChunk"
    );

    let deserialized: FunctionMessageChunk = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content(), "Result");
    assert_eq!(deserialized.name(), "calculator");
    assert_eq!(deserialized.id(), Some("chunk-123".to_string()));
}

#[test]
fn test_chunk_multiple_additions() {
    let chunk1 = FunctionMessageChunk::new("a", "func");
    let chunk2 = FunctionMessageChunk::new("b", "func");
    let chunk3 = FunctionMessageChunk::new("c", "func");
    let result = chunk1 + chunk2 + chunk3;
    assert_eq!(result.content(), "abc");
    assert_eq!(result.name(), "func");
}

#[test]
fn test_chunk_empty_content() {
    let chunk1 = FunctionMessageChunk::new("Hello", "func");
    let chunk2 = FunctionMessageChunk::new("", "func");
    let result = chunk1 + chunk2;
    assert_eq!(result.content(), "Hello");
}

#[test]
fn test_chunk_text_property() {
    let chunk = FunctionMessageChunk::new("Hello world", "func");
    assert_eq!(chunk.text(), "Hello world");
}

// ============================================================================
// TestFunctionMessageDeprecation
// ============================================================================

#[test]
fn test_function_message_vs_tool_message() {
    use agent_chain_core::messages::ToolMessage;

    let func_msg = FunctionMessage::new("Result", "func");
    let tool_msg = ToolMessage::new("Result", "call-123");

    // FunctionMessage has name field
    assert_eq!(func_msg.name(), "func");
    // ToolMessage has tool_call_id field
    assert_eq!(tool_msg.tool_call_id(), "call-123");
}

#[test]
fn test_function_message_still_serializable() {
    let msg = FunctionMessage::new("test", "test_func");
    let serialized = serde_json::to_value(&msg).unwrap();
    let deserialized: FunctionMessage = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content(), "test");
    assert_eq!(deserialized.name(), "test_func");
}
