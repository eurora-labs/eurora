//! Tests for ToolMessage and tool-related functions.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/test_tool.py`

use agent_chain_core::messages::{
    ToolMessage, ToolMessageChunk, default_tool_chunk_parser, default_tool_parser,
    invalid_tool_call, tool_call, tool_call_chunk,
};
use serde_json::json;
use uuid::Uuid;

// ============================================================================
// TestToolMessage
// ============================================================================

#[test]
fn test_init_basic() {
    let msg = ToolMessage::new("Result: 42", "call-123");
    assert_eq!(msg.content(), "Result: 42");
    assert_eq!(msg.tool_call_id(), "call-123");
    assert_eq!(msg.message_type(), "tool");
    assert_eq!(msg.status(), "success");
}

#[test]
fn test_init_with_name() {
    let msg = ToolMessage::new("Result", "call-123").with_name("calculator");
    assert_eq!(msg.name(), Some("calculator".to_string()));
}

#[test]
fn test_init_with_id() {
    let msg = ToolMessage::with_id("msg-123", "Result", "call-123");
    assert_eq!(msg.id(), Some("msg-123".to_string()));
}

#[test]
fn test_init_with_artifact() {
    let artifact = json!({"image": "base64_data", "metadata": {"width": 100}});
    let msg = ToolMessage::new("Image generated", "call-123").with_artifact(artifact.clone());
    assert_eq!(msg.artifact(), Some(&artifact));
}

#[test]
fn test_init_with_status_success() {
    let msg = ToolMessage::new("Result", "call-123").with_status("success");
    assert_eq!(msg.status(), "success");
}

#[test]
fn test_init_with_status_error() {
    let msg = ToolMessage::new("Error: Division by zero", "call-123").with_status("error");
    assert_eq!(msg.status(), "error");
}

#[test]
fn test_tool_call_id_coerced_to_string() {
    // UUID type
    let uuid = Uuid::parse_str("12345678-1234-5678-1234-567812345678").unwrap();
    let msg1 = ToolMessage::new("Result", uuid.to_string());
    assert!(!msg1.tool_call_id().is_empty());

    // Integer type (convert to string)
    let msg2 = ToolMessage::new("Result", "12345");
    assert_eq!(msg2.tool_call_id(), "12345");

    // Float type (convert to string)
    let msg3 = ToolMessage::new("Result", "123.45");
    assert_eq!(msg3.tool_call_id(), "123.45");
}

#[test]
fn test_type_is_tool() {
    let msg = ToolMessage::new("Test", "call-123");
    assert_eq!(msg.message_type(), "tool");
}

#[test]
fn test_serialization_roundtrip() {
    let artifact = json!({"data": "value"});

    let msg = ToolMessage::with_id("msg-123", "Result: 42", "call-123")
        .with_name("calculator")
        .with_artifact(artifact.clone())
        .with_status("success");

    let serialized = serde_json::to_value(&msg).unwrap();
    assert_eq!(serialized.get("type").unwrap().as_str().unwrap(), "tool");

    let deserialized: ToolMessage = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content(), "Result: 42");
    assert_eq!(deserialized.tool_call_id(), "call-123");
    assert_eq!(deserialized.name(), Some("calculator".to_string()));
    assert_eq!(deserialized.id(), Some("msg-123".to_string()));
    assert_eq!(deserialized.artifact(), Some(&artifact));
    assert_eq!(deserialized.status(), "success");
}

#[test]
fn test_text_property() {
    let msg = ToolMessage::new("Hello world", "call-123");
    assert_eq!(msg.text(), "Hello world");
}

// ============================================================================
// TestToolMessageChunk
// ============================================================================

#[test]
fn test_chunk_init_basic() {
    let chunk = ToolMessageChunk::new("Result", "call-123");
    assert_eq!(chunk.content(), "Result");
    assert_eq!(chunk.tool_call_id(), "call-123");
    assert_eq!(chunk.message_type(), "ToolMessageChunk");
}

#[test]
fn test_chunk_type_is_tool_message_chunk() {
    let chunk = ToolMessageChunk::new("Test", "call-123");
    assert_eq!(chunk.message_type(), "ToolMessageChunk");
}

#[test]
fn test_chunk_add_same_tool_call_id_chunks() {
    let chunk1 = ToolMessageChunk::with_id("1", "Hello", "call-123");
    let chunk2 = ToolMessageChunk::new(" world", "call-123");
    let result = chunk1 + chunk2;
    assert_eq!(result.content(), "Hello world");
    assert_eq!(result.tool_call_id(), "call-123");
    assert_eq!(result.id(), Some("1".to_string()));
}

#[test]
#[should_panic(expected = "Cannot concatenate")]
fn test_chunk_add_different_tool_call_id_raises_error() {
    let chunk1 = ToolMessageChunk::new("Hello", "call-123");
    let chunk2 = ToolMessageChunk::new(" world", "call-456");
    let _result = chunk1 + chunk2;
}

#[test]
fn test_chunk_add_with_artifact() {
    let artifact1 = json!({"data": "value1"});
    let artifact2 = json!({"more": "value2"});

    let chunk1 = ToolMessageChunk::new("Part 1", "call-123").with_artifact(artifact1);
    let chunk2 = ToolMessageChunk::new(" Part 2", "call-123").with_artifact(artifact2);

    let result = chunk1 + chunk2;
    // Artifacts are merged
    assert!(result.artifact().is_some());
}

#[test]
fn test_chunk_add_with_different_status() {
    let chunk1 = ToolMessageChunk::new("Part 1", "call-123").with_status("success");
    let chunk2 = ToolMessageChunk::new(" Part 2", "call-123").with_status("error");
    let result = chunk1 + chunk2;
    // Error status takes precedence
    assert_eq!(result.status(), "error");
}

#[test]
fn test_chunk_add_with_response_metadata() {
    let mut meta1 = std::collections::HashMap::new();
    meta1.insert("meta1".to_string(), serde_json::json!("data1"));

    let mut meta2 = std::collections::HashMap::new();
    meta2.insert("meta2".to_string(), serde_json::json!("data2"));

    let chunk1 = ToolMessageChunk::new("Hello", "call-123").with_response_metadata(meta1);
    let chunk2 = ToolMessageChunk::new(" world", "call-123").with_response_metadata(meta2);

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
fn test_chunk_serialization_roundtrip() {
    let chunk = ToolMessageChunk::with_id("chunk-123", "Result", "call-123");

    let serialized = serde_json::to_value(&chunk).unwrap();
    assert_eq!(
        serialized.get("type").unwrap().as_str().unwrap(),
        "ToolMessageChunk"
    );

    let deserialized: ToolMessageChunk = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content(), "Result");
    assert_eq!(deserialized.tool_call_id(), "call-123");
    assert_eq!(deserialized.id(), Some("chunk-123".to_string()));
}

// ============================================================================
// TestToolCallFactory
// ============================================================================

#[test]
fn test_basic_tool_call() {
    let tc = tool_call(
        "test_tool",
        json!({"param": "value"}),
        Some("call-123".to_string()),
    );
    assert_eq!(tc.name(), "test_tool");
    assert_eq!(tc.args(), &json!({"param": "value"}));
    assert_eq!(tc.id(), Some("call-123".to_string()));
}

#[test]
fn test_tool_call_with_none_id() {
    let tc = tool_call("test_tool", json!({}), None);
    assert_eq!(tc.id(), None);
}

#[test]
fn test_tool_call_with_empty_args() {
    let tc = tool_call("test_tool", json!({}), Some("call-123".to_string()));
    assert_eq!(tc.args(), &json!({}));
}

#[test]
fn test_tool_call_with_complex_args() {
    let complex_args = json!({
        "string": "value",
        "number": 42,
        "nested": {"key": "value"},
        "list": [1, 2, 3],
    });
    let tc = tool_call(
        "test_tool",
        complex_args.clone(),
        Some("call-123".to_string()),
    );
    assert_eq!(tc.args(), &complex_args);
}

// ============================================================================
// TestToolCallChunkFactory
// ============================================================================

#[test]
fn test_basic_tool_call_chunk() {
    let tc = tool_call_chunk(
        Some("test_tool".to_string()),
        Some(r#"{"param": "value"}"#.to_string()),
        Some("call-123".to_string()),
        Some(0),
    );
    assert_eq!(tc.name, Some("test_tool".to_string()));
    assert_eq!(tc.args, Some(r#"{"param": "value"}"#.to_string()));
    assert_eq!(tc.id, Some("call-123".to_string()));
    assert_eq!(tc.index, Some(0));
}

#[test]
fn test_tool_call_chunk_with_none_values() {
    let tc = tool_call_chunk(None, None, None, None);
    assert_eq!(tc.name, None);
    assert_eq!(tc.args, None);
    assert_eq!(tc.id, None);
    assert_eq!(tc.index, None);
}

#[test]
fn test_tool_call_chunk_defaults() {
    let tc = tool_call_chunk(None, None, None, None);
    assert_eq!(tc.name, None);
    assert_eq!(tc.args, None);
    assert_eq!(tc.id, None);
    assert_eq!(tc.index, None);
}

#[test]
fn test_tool_call_chunk_partial_args() {
    let tc1 = tool_call_chunk(
        Some("test".to_string()),
        Some(r#"{"key":"#.to_string()),
        Some("123".to_string()),
        Some(0),
    );
    let tc2 = tool_call_chunk(None, Some(r#""value"}"#.to_string()), None, Some(0));
    assert_eq!(tc1.args, Some(r#"{"key":"#.to_string()));
    assert_eq!(tc2.args, Some(r#""value"}"#.to_string()));
}

// ============================================================================
// TestInvalidToolCallFactory
// ============================================================================

#[test]
fn test_basic_invalid_tool_call() {
    let itc = invalid_tool_call(
        Some("test_tool".to_string()),
        Some("invalid json".to_string()),
        Some("call-123".to_string()),
        Some("JSON parse error".to_string()),
    );
    assert_eq!(itc.name, Some("test_tool".to_string()));
    assert_eq!(itc.args, Some("invalid json".to_string()));
    assert_eq!(itc.id, Some("call-123".to_string()));
    assert_eq!(itc.error, Some("JSON parse error".to_string()));
}

#[test]
fn test_invalid_tool_call_with_none_values() {
    let itc = invalid_tool_call(None, None, None, None);
    assert_eq!(itc.name, None);
    assert_eq!(itc.args, None);
    assert_eq!(itc.id, None);
    assert_eq!(itc.error, None);
}

#[test]
fn test_invalid_tool_call_defaults() {
    let itc = invalid_tool_call(None, None, None, None);
    assert_eq!(itc.name, None);
    assert_eq!(itc.args, None);
    assert_eq!(itc.id, None);
    assert_eq!(itc.error, None);
}

// ============================================================================
// TestDefaultToolParser
// ============================================================================

#[test]
fn test_parse_valid_tool_calls() {
    let raw_calls = vec![
        json!({
            "id": "call-1",
            "function": {
                "name": "calculator",
                "arguments": r#"{"operation": "add", "a": 1, "b": 2}"#,
            },
        }),
        json!({
            "id": "call-2",
            "function": {
                "name": "search",
                "arguments": r#"{"query": "weather"}"#,
            },
        }),
    ];
    let (tool_calls, invalid_calls) = default_tool_parser(&raw_calls);

    assert_eq!(tool_calls.len(), 2);
    assert_eq!(invalid_calls.len(), 0);

    assert_eq!(tool_calls[0].name(), "calculator");
    assert_eq!(
        tool_calls[0].args(),
        &json!({"operation": "add", "a": 1, "b": 2})
    );
    assert_eq!(tool_calls[0].id(), Some("call-1".to_string()));

    assert_eq!(tool_calls[1].name(), "search");
    assert_eq!(tool_calls[1].args(), &json!({"query": "weather"}));
}

#[test]
fn test_parse_invalid_json_args() {
    let raw_calls = vec![json!({
        "id": "call-1",
        "function": {
            "name": "test_tool",
            "arguments": "not valid json",
        },
    })];
    let (tool_calls, invalid_calls) = default_tool_parser(&raw_calls);

    assert_eq!(tool_calls.len(), 0);
    assert_eq!(invalid_calls.len(), 1);

    assert_eq!(invalid_calls[0].name, Some("test_tool".to_string()));
    assert_eq!(invalid_calls[0].args, Some("not valid json".to_string()));
    assert_eq!(invalid_calls[0].id, Some("call-1".to_string()));
}

#[test]
fn test_parse_empty_args() {
    let raw_calls = vec![json!({
        "id": "call-1",
        "function": {
            "name": "no_args_tool",
            "arguments": "{}",
        },
    })];
    let (tool_calls, _invalid_calls) = default_tool_parser(&raw_calls);

    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].args(), &json!({}));
}

#[test]
fn test_parse_without_function_key() {
    let raw_calls = vec![json!({
        "id": "call-1",
        "other": "data",
    })];
    let (tool_calls, invalid_calls) = default_tool_parser(&raw_calls);

    assert_eq!(tool_calls.len(), 0);
    assert_eq!(invalid_calls.len(), 0);
}

#[test]
fn test_parse_empty_list() {
    let raw_calls: Vec<serde_json::Value> = vec![];
    let (tool_calls, invalid_calls) = default_tool_parser(&raw_calls);
    assert_eq!(tool_calls.len(), 0);
    assert_eq!(invalid_calls.len(), 0);
}

#[test]
fn test_parse_mixed_valid_and_invalid() {
    let raw_calls = vec![
        json!({
            "id": "call-1",
            "function": {
                "name": "valid_tool",
                "arguments": r#"{"key": "value"}"#,
            },
        }),
        json!({
            "id": "call-2",
            "function": {
                "name": "invalid_tool",
                "arguments": "broken json {",
            },
        }),
    ];
    let (tool_calls, invalid_calls) = default_tool_parser(&raw_calls);

    assert_eq!(tool_calls.len(), 1);
    assert_eq!(invalid_calls.len(), 1);

    assert_eq!(tool_calls[0].name(), "valid_tool");
    assert_eq!(invalid_calls[0].name, Some("invalid_tool".to_string()));
}

// ============================================================================
// TestDefaultToolChunkParser
// ============================================================================

#[test]
fn test_parse_tool_call_chunks() {
    let raw_calls = vec![
        json!({
            "id": "call-1",
            "index": 0,
            "function": {
                "name": "test_tool",
                "arguments": r#"{"key":"#,
            },
        }),
        json!({
            "id": "call-1",
            "index": 0,
            "function": {
                "name": null,
                "arguments": r#""value"}"#,
            },
        }),
    ];
    let chunks = default_tool_chunk_parser(&raw_calls);

    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].name, Some("test_tool".to_string()));
    assert_eq!(chunks[0].args, Some(r#"{"key":"#.to_string()));
    assert_eq!(chunks[0].index, Some(0));

    assert_eq!(chunks[1].args, Some(r#""value"}"#.to_string()));
}

#[test]
fn test_chunk_parse_without_function_key() {
    let raw_calls = vec![json!({
        "id": "call-1",
        "index": 0,
    })];
    let chunks = default_tool_chunk_parser(&raw_calls);

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].name, None);
    assert_eq!(chunks[0].args, None);
    assert_eq!(chunks[0].id, Some("call-1".to_string()));
    assert_eq!(chunks[0].index, Some(0));
}

#[test]
fn test_chunk_parse_empty_list() {
    let raw_calls: Vec<serde_json::Value> = vec![];
    let chunks = default_tool_chunk_parser(&raw_calls);
    assert_eq!(chunks.len(), 0);
}
