use agent_chain_core::messages::{
    ToolMessage, ToolMessageChunk, ToolOutputMixin, ToolStatus, default_tool_chunk_parser,
    default_tool_parser, invalid_tool_call, tool_call, tool_call_chunk,
};
use serde_json::json;
use uuid::Uuid;

#[test]
fn test_init_basic() {
    let msg = ToolMessage::builder()
        .content("Result: 42")
        .tool_call_id("call-123")
        .build();
    assert_eq!(msg.content, "Result: 42");
    assert_eq!(msg.tool_call_id, "call-123");
    assert_eq!(msg.message_type(), "tool");
    assert_eq!(msg.status, ToolStatus::Success);
}

#[test]
fn test_init_with_name() {
    let msg = ToolMessage::builder()
        .content("Result")
        .tool_call_id("call-123")
        .name("calculator".to_string())
        .build();
    assert_eq!(msg.name, Some("calculator".to_string()));
}

#[test]
fn test_init_with_id() {
    let msg = ToolMessage::builder()
        .id("msg-123".to_string())
        .content("Result")
        .tool_call_id("call-123")
        .build();
    assert_eq!(msg.id, Some("msg-123".to_string()));
}

#[test]
fn test_init_with_artifact() {
    let artifact = json!({"image": "base64_data", "metadata": {"width": 100}});
    let msg = ToolMessage::builder()
        .content("Image generated")
        .tool_call_id("call-123")
        .artifact(artifact.clone())
        .build();
    assert_eq!(msg.artifact, Some(artifact));
}

#[test]
fn test_init_with_status_success() {
    let msg = ToolMessage::builder()
        .content("Result")
        .tool_call_id("call-123")
        .status(ToolStatus::Success)
        .build();
    assert_eq!(msg.status, ToolStatus::Success);
}

#[test]
fn test_init_with_status_error() {
    let msg = ToolMessage::builder()
        .content("Error: Division by zero")
        .tool_call_id("call-123")
        .status(ToolStatus::Error)
        .build();
    assert_eq!(msg.status, ToolStatus::Error);
}

#[test]
fn test_tool_call_id_coerced_to_string() {
    let uuid = Uuid::parse_str("12345678-1234-5678-1234-567812345678").unwrap();
    let msg1 = ToolMessage::builder()
        .content("Result")
        .tool_call_id(uuid.to_string())
        .build();
    assert!(!msg1.tool_call_id.is_empty());

    let msg2 = ToolMessage::builder()
        .content("Result")
        .tool_call_id("12345")
        .build();
    assert_eq!(msg2.tool_call_id, "12345");

    let msg3 = ToolMessage::builder()
        .content("Result")
        .tool_call_id("123.45")
        .build();
    assert_eq!(msg3.tool_call_id, "123.45");
}

#[test]
fn test_type_is_tool() {
    let msg = ToolMessage::builder()
        .content("Test")
        .tool_call_id("call-123")
        .build();
    assert_eq!(msg.message_type(), "tool");
}

#[test]
fn test_serialization_roundtrip() {
    let artifact = json!({"data": "value"});

    let msg = ToolMessage::builder()
        .id("msg-123".to_string())
        .content("Result: 42")
        .tool_call_id("call-123")
        .name("calculator".to_string())
        .artifact(artifact.clone())
        .status(ToolStatus::Success)
        .build();

    let serialized = serde_json::to_value(&msg).unwrap();
    assert_eq!(serialized.get("type").unwrap().as_str().unwrap(), "tool");

    let deserialized: ToolMessage = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content, "Result: 42");
    assert_eq!(deserialized.tool_call_id, "call-123");
    assert_eq!(deserialized.name, Some("calculator".to_string()));
    assert_eq!(deserialized.id, Some("msg-123".to_string()));
    assert_eq!(deserialized.artifact, Some(artifact));
    assert_eq!(deserialized.status, ToolStatus::Success);
}

#[test]
fn test_text_property() {
    let msg = ToolMessage::builder()
        .content("Hello world")
        .tool_call_id("call-123")
        .build();
    assert_eq!(msg.text(), "Hello world");
}

#[test]
fn test_chunk_init_basic() {
    let chunk = ToolMessageChunk::builder()
        .content("Result")
        .tool_call_id("call-123")
        .build();
    assert_eq!(chunk.content, "Result");
    assert_eq!(chunk.tool_call_id, "call-123");
    assert_eq!(chunk.message_type(), "ToolMessageChunk");
}

#[test]
fn test_chunk_type_is_tool_message_chunk() {
    let chunk = ToolMessageChunk::builder()
        .content("Test")
        .tool_call_id("call-123")
        .build();
    assert_eq!(chunk.message_type(), "ToolMessageChunk");
}

#[test]
fn test_chunk_add_same_tool_call_id_chunks() {
    let chunk1 = ToolMessageChunk::builder()
        .id("1".to_string())
        .content("Hello")
        .tool_call_id("call-123")
        .build();
    let chunk2 = ToolMessageChunk::builder()
        .content(" world")
        .tool_call_id("call-123")
        .build();
    let result = chunk1 + chunk2;
    assert_eq!(result.content, "Hello world");
    assert_eq!(result.tool_call_id, "call-123");
    assert_eq!(result.id, Some("1".to_string()));
}

#[test]
#[should_panic(expected = "Cannot concatenate")]
fn test_chunk_add_different_tool_call_id_raises_error() {
    let chunk1 = ToolMessageChunk::builder()
        .content("Hello")
        .tool_call_id("call-123")
        .build();
    let chunk2 = ToolMessageChunk::builder()
        .content(" world")
        .tool_call_id("call-456")
        .build();
    let _result = chunk1 + chunk2;
}

#[test]
fn test_chunk_add_with_artifact() {
    let artifact1 = json!({"data": "value1"});
    let artifact2 = json!({"more": "value2"});

    let chunk1 = ToolMessageChunk::builder()
        .content("Part 1")
        .tool_call_id("call-123")
        .artifact(artifact1)
        .build();
    let chunk2 = ToolMessageChunk::builder()
        .content(" Part 2")
        .tool_call_id("call-123")
        .artifact(artifact2)
        .build();

    let result = chunk1 + chunk2;
    assert!(result.artifact.is_some());
}

#[test]
fn test_chunk_add_with_different_status() {
    let chunk1 = ToolMessageChunk::builder()
        .content("Part 1")
        .tool_call_id("call-123")
        .status(ToolStatus::Success)
        .build();
    let chunk2 = ToolMessageChunk::builder()
        .content(" Part 2")
        .tool_call_id("call-123")
        .status(ToolStatus::Error)
        .build();
    let result = chunk1 + chunk2;
    assert_eq!(result.status, ToolStatus::Error);
}

#[test]
fn test_chunk_add_with_response_metadata() {
    let mut meta1 = std::collections::HashMap::new();
    meta1.insert("meta1".to_string(), serde_json::json!("data1"));

    let mut meta2 = std::collections::HashMap::new();
    meta2.insert("meta2".to_string(), serde_json::json!("data2"));

    let chunk1 = ToolMessageChunk::builder()
        .content("Hello")
        .tool_call_id("call-123")
        .response_metadata(meta1)
        .build();
    let chunk2 = ToolMessageChunk::builder()
        .content(" world")
        .tool_call_id("call-123")
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
fn test_chunk_serialization_roundtrip() {
    let chunk = ToolMessageChunk::builder()
        .id("chunk-123".to_string())
        .content("Result")
        .tool_call_id("call-123")
        .build();

    let serialized = serde_json::to_value(&chunk).unwrap();
    assert_eq!(
        serialized.get("type").unwrap().as_str().unwrap(),
        "ToolMessageChunk"
    );

    let deserialized: ToolMessageChunk = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content, "Result");
    assert_eq!(deserialized.tool_call_id, "call-123");
    assert_eq!(deserialized.id, Some("chunk-123".to_string()));
}

#[test]
fn test_basic_tool_call() {
    let tc = tool_call(
        "test_tool",
        json!({"param": "value"}),
        Some("call-123".to_string()),
    );
    assert_eq!(tc.name, "test_tool");
    assert_eq!(tc.args, json!({"param": "value"}));
    assert_eq!(tc.id, Some("call-123".to_string()));
}

#[test]
fn test_tool_call_with_none_id() {
    let tc = tool_call("test_tool", json!({}), None);
    assert_eq!(tc.id, None);
}

#[test]
fn test_tool_call_with_empty_args() {
    let tc = tool_call("test_tool", json!({}), Some("call-123".to_string()));
    assert_eq!(tc.args, json!({}));
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
    assert_eq!(tc.args, complex_args);
}

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

    assert_eq!(tool_calls[0].name, "calculator");
    assert_eq!(
        tool_calls[0].args,
        json!({"operation": "add", "a": 1, "b": 2})
    );
    assert_eq!(tool_calls[0].id, Some("call-1".to_string()));

    assert_eq!(tool_calls[1].name, "search");
    assert_eq!(tool_calls[1].args, json!({"query": "weather"}));
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
    assert_eq!(tool_calls[0].args, json!({}));
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

    assert_eq!(tool_calls[0].name, "valid_tool");
    assert_eq!(invalid_calls[0].name, Some("invalid_tool".to_string()));
}

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

#[test]
fn test_tool_message_is_tool_output_mixin() {
    let msg = ToolMessage::builder()
        .content("Result")
        .tool_call_id("call-123")
        .build();
    fn assert_tool_output(_: &impl ToolOutputMixin) {}
    assert_tool_output(&msg);
}

#[test]
fn test_merge_status_success_plus_success() {
    let chunk1 = ToolMessageChunk::builder()
        .content("")
        .tool_call_id("call-1")
        .status(ToolStatus::Success)
        .build();
    let chunk2 = ToolMessageChunk::builder()
        .content("")
        .tool_call_id("call-1")
        .status(ToolStatus::Success)
        .build();
    let result = chunk1 + chunk2;
    assert_eq!(result.status, ToolStatus::Success);
}

#[test]
fn test_merge_status_error_plus_success() {
    let chunk1 = ToolMessageChunk::builder()
        .content("")
        .tool_call_id("call-1")
        .status(ToolStatus::Error)
        .build();
    let chunk2 = ToolMessageChunk::builder()
        .content("")
        .tool_call_id("call-1")
        .status(ToolStatus::Success)
        .build();
    let result = chunk1 + chunk2;
    assert_eq!(result.status, ToolStatus::Error);
}

#[test]
fn test_merge_status_success_plus_error() {
    let chunk1 = ToolMessageChunk::builder()
        .content("")
        .tool_call_id("call-1")
        .status(ToolStatus::Success)
        .build();
    let chunk2 = ToolMessageChunk::builder()
        .content("")
        .tool_call_id("call-1")
        .status(ToolStatus::Error)
        .build();
    let result = chunk1 + chunk2;
    assert_eq!(result.status, ToolStatus::Error);
}

#[test]
fn test_merge_status_error_plus_error() {
    let chunk1 = ToolMessageChunk::builder()
        .content("")
        .tool_call_id("call-1")
        .status(ToolStatus::Error)
        .build();
    let chunk2 = ToolMessageChunk::builder()
        .content("")
        .tool_call_id("call-1")
        .status(ToolStatus::Error)
        .build();
    let result = chunk1 + chunk2;
    assert_eq!(result.status, ToolStatus::Error);
}

#[test]
fn test_empty_string_content() {
    let msg = ToolMessage::builder()
        .content("")
        .tool_call_id("call-400")
        .build();
    assert_eq!(msg.content, "");
    assert_eq!(msg.tool_call_id, "call-400");
    assert_eq!(msg.status, ToolStatus::Success);
    assert_eq!(msg.message_type(), "tool");
}

#[test]
fn test_serialization_roundtrip_with_artifact_and_error_status() {
    let artifact_data = json!({"raw_output": "traceback info", "exit_code": 1});
    let msg = ToolMessage::builder()
        .id("msg-600".to_string())
        .content("Tool execution failed")
        .tool_call_id("call-600")
        .name("failing_tool".to_string())
        .artifact(artifact_data.clone())
        .status(ToolStatus::Error)
        .build();

    let serialized = serde_json::to_value(&msg).unwrap();
    assert_eq!(serialized.get("type").unwrap().as_str().unwrap(), "tool");

    let deserialized: ToolMessage = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content, "Tool execution failed");
    assert_eq!(deserialized.tool_call_id, "call-600");
    assert_eq!(deserialized.name, Some("failing_tool".to_string()));
    assert_eq!(deserialized.artifact, Some(artifact_data));
    assert_eq!(deserialized.status, ToolStatus::Error);
    assert_eq!(deserialized.id, Some("msg-600".to_string()));
}

#[test]
fn test_both_success_statuses_result_in_success() {
    let chunk1 = ToolMessageChunk::builder()
        .content("Part A")
        .tool_call_id("call-700")
        .status(ToolStatus::Success)
        .build();
    let chunk2 = ToolMessageChunk::builder()
        .content(" Part B")
        .tool_call_id("call-700")
        .status(ToolStatus::Success)
        .build();
    let result = chunk1 + chunk2;
    assert_eq!(result.status, ToolStatus::Success);
    assert_eq!(result.content, "Part A Part B");
}

#[test]
fn test_tool_call_id_preserved_from_first_chunk() {
    let chunk1 = ToolMessageChunk::builder()
        .id("chunk-first".to_string())
        .content("Hello")
        .tool_call_id("call-800")
        .build();
    let chunk2 = ToolMessageChunk::builder()
        .id("chunk-second".to_string())
        .content(" World")
        .tool_call_id("call-800")
        .build();
    let result = chunk1 + chunk2;
    assert_eq!(result.tool_call_id, "call-800");
    assert_eq!(result.id, Some("chunk-first".to_string()));
}

#[test]
fn test_tool_call_with_no_id_field() {
    let raw_calls = vec![json!({
        "function": {
            "name": "lookup",
            "arguments": r#"{"term": "python"}"#,
        },
    })];
    let (tool_calls, invalid_calls) = default_tool_parser(&raw_calls);
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(invalid_calls.len(), 0);
    assert_eq!(tool_calls[0].name, "lookup");
    assert_eq!(tool_calls[0].args, json!({"term": "python"}));
    assert_eq!(tool_calls[0].id, None);
}

#[test]
fn test_empty_function_args_string() {
    let raw_calls = vec![json!({
        "id": "call-a",
        "function": {
            "name": "no_args",
            "arguments": "",
        },
    })];
    let (tool_calls, invalid_calls) = default_tool_parser(&raw_calls);
    assert_eq!(tool_calls.len(), 0);
    assert_eq!(invalid_calls.len(), 1);
    assert_eq!(invalid_calls[0].name, Some("no_args".to_string()));
    assert_eq!(invalid_calls[0].args, Some("".to_string()));
    assert_eq!(invalid_calls[0].id, Some("call-a".to_string()));
}

#[test]
fn test_null_function_args() {
    let raw_calls = vec![json!({
        "id": "call-b",
        "function": {
            "name": "null_args_tool",
            "arguments": "null",
        },
    })];
    let (tool_calls, invalid_calls) = default_tool_parser(&raw_calls);
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(invalid_calls.len(), 0);
    assert_eq!(tool_calls[0].name, "null_args_tool");
    assert_eq!(tool_calls[0].args, json!({}));
    assert_eq!(tool_calls[0].id, Some("call-b".to_string()));
}

#[test]
fn test_tool_calls_with_function_name_none() {
    let raw_calls = vec![json!({
        "id": null,
        "index": 0,
        "function": {
            "name": null,
            "arguments": r#""continued"}"#,
        },
    })];
    let chunks = default_tool_chunk_parser(&raw_calls);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].name, None);
    assert_eq!(chunks[0].args, Some(r#""continued"}"#.to_string()));
    assert_eq!(chunks[0].id, None);
    assert_eq!(chunks[0].index, Some(0));
}

#[test]
fn test_additional_kwargs_default_is_empty_dict() {
    let msg = ToolMessage::builder()
        .content("test")
        .tool_call_id("call-1100")
        .build();
    assert!(msg.additional_kwargs.is_empty());
}

#[test]
fn test_response_metadata_default_is_empty_dict() {
    let msg = ToolMessage::builder()
        .content("test")
        .tool_call_id("call-1200")
        .build();
    assert!(msg.response_metadata.is_empty());
}

#[test]
fn test_additional_kwargs_and_response_metadata_with_values() {
    let mut additional = std::collections::HashMap::new();
    additional.insert("custom".to_string(), json!("value"));
    let mut response_meta = std::collections::HashMap::new();
    response_meta.insert("meta".to_string(), json!("data"));

    let msg = ToolMessage::builder()
        .content("test")
        .tool_call_id("call-1301")
        .additional_kwargs(additional)
        .response_metadata(response_meta)
        .build();

    assert_eq!(
        msg.additional_kwargs.get("custom").unwrap(),
        &json!("value")
    );
    assert_eq!(msg.response_metadata.get("meta").unwrap(), &json!("data"));
}

#[test]
fn test_pretty_repr_includes_tool_name() {
    let msg = ToolMessage::builder()
        .content("42")
        .tool_call_id("call-1400")
        .name("calculator".to_string())
        .build();
    let result = msg.pretty_repr(false);
    assert!(result.contains("Tool Message"));
    assert!(result.contains("Name: calculator"));
    assert!(result.contains("42"));
}

#[test]
fn test_pretty_repr_without_name() {
    let msg = ToolMessage::builder()
        .content("result data")
        .tool_call_id("call-1500")
        .build();
    let result = msg.pretty_repr(false);
    assert!(result.contains("Tool Message"));
    assert!(!result.contains("Name:"));
    assert!(result.contains("result data"));
}

#[test]
fn test_pretty_repr_with_error_content() {
    let msg = ToolMessage::builder()
        .content("Error: division by zero")
        .tool_call_id("call-1600")
        .name("math_tool".to_string())
        .status(ToolStatus::Error)
        .build();
    let result = msg.pretty_repr(false);
    assert!(result.contains("Tool Message"));
    assert!(result.contains("Name: math_tool"));
    assert!(result.contains("Error: division by zero"));
}

#[test]
fn test_tool_call_structure() {
    let tc = tool_call(
        "test_tool",
        json!({"param": "value"}),
        Some("call-123".to_string()),
    );
    assert_eq!(tc.name, "test_tool");
    assert_eq!(tc.args["param"], "value");
    assert_eq!(tc.id, Some("call-123".to_string()));
}

#[test]
fn test_tool_call_with_type() {
    let tc = tool_call("test_tool", json!({}), Some("call-123".to_string()));
    assert_eq!(tc.call_type, Some("tool_call".to_string()));
}

#[test]
fn test_tool_call_chunk_structure() {
    let tc = tool_call_chunk(
        Some("test_tool".to_string()),
        Some(r#"{"key": "value"}"#.to_string()),
        Some("call-123".to_string()),
        Some(0),
    );
    assert_eq!(tc.name, Some("test_tool".to_string()));
    assert_eq!(tc.args, Some(r#"{"key": "value"}"#.to_string()));
    assert_eq!(tc.id, Some("call-123".to_string()));
    assert_eq!(tc.index, Some(0));
}

#[test]
fn test_tool_call_chunk_with_type() {
    let tc = tool_call_chunk(None, None, None, None);
    assert_eq!(tc.chunk_type, Some("tool_call_chunk".to_string()));
}
