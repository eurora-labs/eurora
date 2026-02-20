//! Tests for AI message types.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/test_ai.py`

use agent_chain_core::messages::{
    AIMessage, AIMessageChunk, ChunkPosition, InputTokenDetails, OutputTokenDetails, UsageMetadata,
    add_ai_message_chunks, add_usage, invalid_tool_call, subtract_usage, tool_call,
    tool_call_chunk,
};
use serde_json::json;

#[test]
fn test_serdes_message() {
    let msg = AIMessage::builder()
        .content("")
        .tool_calls(vec![tool_call(
            "foo",
            json!({"bar": 1}),
            Some("baz".to_string()),
        )])
        .invalid_tool_calls(vec![invalid_tool_call(
            Some("foobad".to_string()),
            Some("blah".to_string()),
            Some("booz".to_string()),
            Some("bad".to_string()),
        )])
        .build();

    let serialized = serde_json::to_value(&msg).unwrap();

    let tool_calls = serialized.get("tool_calls").unwrap().as_array().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].get("name").unwrap().as_str().unwrap(), "foo");

    let invalid_tool_calls = serialized
        .get("invalid_tool_calls")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(invalid_tool_calls.len(), 1);
    assert_eq!(
        invalid_tool_calls[0].get("name").unwrap().as_str().unwrap(),
        "foobad"
    );
    assert_eq!(
        invalid_tool_calls[0]
            .get("error")
            .unwrap()
            .as_str()
            .unwrap(),
        "bad"
    );

    let deserialized: AIMessage = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.tool_calls.len(), 1);
    assert_eq!(deserialized.tool_calls[0].name, "foo");
    assert_eq!(deserialized.invalid_tool_calls.len(), 1);
    assert_eq!(
        deserialized.invalid_tool_calls[0].name,
        Some("foobad".to_string())
    );
}

#[test]
fn test_serdes_message_chunk() {
    let chunk = AIMessageChunk::builder()
        .content("")
        .tool_call_chunks(vec![
            tool_call_chunk(
                Some("foo".to_string()),
                Some(r#"{"bar": 1}"#.to_string()),
                Some("baz".to_string()),
                Some(0),
            ),
            tool_call_chunk(
                Some("foobad".to_string()),
                Some("blah".to_string()),
                Some("booz".to_string()),
                Some(1),
            ),
        ])
        .build();

    let serialized = serde_json::to_value(&chunk).unwrap();

    let tool_call_chunks = serialized
        .get("tool_call_chunks")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(tool_call_chunks.len(), 2);
    assert_eq!(
        tool_call_chunks[0].get("name").unwrap().as_str().unwrap(),
        "foo"
    );
    assert_eq!(
        tool_call_chunks[0].get("index").unwrap().as_i64().unwrap(),
        0
    );
    assert_eq!(
        tool_call_chunks[1].get("name").unwrap().as_str().unwrap(),
        "foobad"
    );
    assert_eq!(
        tool_call_chunks[1].get("index").unwrap().as_i64().unwrap(),
        1
    );

    let deserialized: AIMessageChunk = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.tool_call_chunks.len(), 2);
    assert_eq!(
        deserialized.tool_call_chunks[0].name,
        Some("foo".to_string())
    );
}

#[test]
fn test_add_usage_both_none() {
    let result = add_usage(None, None);
    assert_eq!(result.input_tokens, 0);
    assert_eq!(result.output_tokens, 0);
    assert_eq!(result.total_tokens, 0);
}

#[test]
fn test_add_usage_one_none() {
    let usage = UsageMetadata::new(10, 20);
    let result = add_usage(Some(&usage), None);
    assert_eq!(result.input_tokens, 10);
    assert_eq!(result.output_tokens, 20);
    assert_eq!(result.total_tokens, 30);
}

#[test]
fn test_add_usage_both_present() {
    let usage1 = UsageMetadata::new(10, 20);
    let usage2 = UsageMetadata::new(5, 10);
    let result = add_usage(Some(&usage1), Some(&usage2));
    assert_eq!(result.input_tokens, 15);
    assert_eq!(result.output_tokens, 30);
    assert_eq!(result.total_tokens, 45);
}

#[test]
fn test_add_usage_with_details() {
    let usage1 = UsageMetadata {
        input_tokens: 10,
        output_tokens: 20,
        total_tokens: 30,
        input_token_details: Some(InputTokenDetails {
            audio: Some(5),
            cache_creation: None,
            cache_read: None,
            ..Default::default()
        }),
        output_token_details: Some(OutputTokenDetails {
            audio: None,
            reasoning: Some(10),
            ..Default::default()
        }),
    };
    let usage2 = UsageMetadata {
        input_tokens: 5,
        output_tokens: 10,
        total_tokens: 15,
        input_token_details: Some(InputTokenDetails {
            audio: Some(3),
            cache_creation: None,
            cache_read: None,
            ..Default::default()
        }),
        output_token_details: Some(OutputTokenDetails {
            audio: None,
            reasoning: Some(5),
            ..Default::default()
        }),
    };
    let result = add_usage(Some(&usage1), Some(&usage2));

    assert_eq!(result.input_token_details.as_ref().unwrap().audio, Some(8));
    assert_eq!(
        result.output_token_details.as_ref().unwrap().reasoning,
        Some(15)
    );
}

#[test]
fn test_subtract_usage_both_none() {
    let result = subtract_usage(None, None);
    assert_eq!(result.input_tokens, 0);
    assert_eq!(result.output_tokens, 0);
    assert_eq!(result.total_tokens, 0);
}

#[test]
fn test_subtract_usage_one_none() {
    let usage = UsageMetadata::new(10, 20);
    let result = subtract_usage(Some(&usage), None);
    assert_eq!(result.input_tokens, 10);
    assert_eq!(result.output_tokens, 20);
    assert_eq!(result.total_tokens, 30);
}

#[test]
fn test_subtract_usage_both_present() {
    let usage1 = UsageMetadata::new(10, 20);
    let usage2 = UsageMetadata::new(5, 10);
    let result = subtract_usage(Some(&usage1), Some(&usage2));
    assert_eq!(result.input_tokens, 5);
    assert_eq!(result.output_tokens, 10);
    assert_eq!(result.total_tokens, 15);
}

#[test]
fn test_subtract_usage_with_negative_result() {
    let usage1 = UsageMetadata::new(5, 10);
    let usage2 = UsageMetadata::new(10, 20);
    let result = subtract_usage(Some(&usage1), Some(&usage2));
    assert_eq!(result.input_tokens, 0);
    assert_eq!(result.output_tokens, 0);
    assert_eq!(result.total_tokens, 0);
}

#[test]
fn test_add_ai_message_chunks_usage() {
    let chunk1 = AIMessageChunk::builder().content("").build();

    let chunk2 = AIMessageChunk::builder()
        .content("")
        .usage_metadata(UsageMetadata::new(2, 3))
        .build();

    let chunk3 = AIMessageChunk::builder()
        .content("")
        .usage_metadata(UsageMetadata {
            input_tokens: 2,
            output_tokens: 3,
            total_tokens: 5,
            input_token_details: Some(InputTokenDetails {
                audio: Some(1),
                cache_creation: None,
                cache_read: Some(1),
                ..Default::default()
            }),
            output_token_details: Some(OutputTokenDetails {
                audio: Some(1),
                reasoning: Some(2),
                ..Default::default()
            }),
        })
        .build();

    let combined = add_ai_message_chunks(chunk1, vec![chunk2, chunk3]);

    assert!(combined.usage_metadata.is_some());
    let usage = combined.usage_metadata.as_ref().unwrap();
    assert_eq!(usage.input_tokens, 4);
    assert_eq!(usage.output_tokens, 6);
    assert_eq!(usage.total_tokens, 10);
    assert_eq!(usage.input_token_details.as_ref().unwrap().audio, Some(1));
    assert_eq!(
        usage.input_token_details.as_ref().unwrap().cache_read,
        Some(1)
    );
    assert_eq!(usage.output_token_details.as_ref().unwrap().audio, Some(1));
    assert_eq!(
        usage.output_token_details.as_ref().unwrap().reasoning,
        Some(2)
    );
}

#[test]
fn test_init_tool_calls() {
    let msg = AIMessage::builder()
        .content("")
        .tool_calls(vec![tool_call(
            "foo",
            json!({"a": "b"}),
            Some("abc".to_string()),
        )])
        .build();
    assert_eq!(msg.tool_calls.len(), 1);
    assert_eq!(msg.tool_calls[0].name, "foo");
}

#[test]
fn test_content_blocks() {
    let message = AIMessage::builder()
        .content("")
        .tool_calls(vec![tool_call(
            "foo",
            json!({"a": "b"}),
            Some("abc_123".to_string()),
        )])
        .build();
    assert_eq!(message.tool_calls.len(), 1);
    assert_eq!(message.content, "");

    let message2 = AIMessage::builder()
        .content("foo")
        .tool_calls(vec![tool_call(
            "foo",
            json!({"a": "b"}),
            Some("abc_123".to_string()),
        )])
        .build();
    assert_eq!(message2.content, "foo");
    assert_eq!(message2.tool_calls.len(), 1);

    let chunk = AIMessageChunk::builder()
        .content("")
        .tool_call_chunks(vec![tool_call_chunk(
            Some("foo".to_string()),
            Some("".to_string()),
            Some("abc_123".to_string()),
            Some(0),
        )])
        .build();
    assert_eq!(chunk.tool_call_chunks.len(), 1);
    assert_eq!(chunk.content, "");

    let chunk_1 = AIMessageChunk::builder()
        .content("")
        .tool_call_chunks(vec![tool_call_chunk(
            Some("foo".to_string()),
            Some(r#"{"foo": "b"#.to_string()),
            Some("abc_123".to_string()),
            Some(0),
        )])
        .build();

    let chunk_2 = AIMessageChunk::builder()
        .content("")
        .tool_call_chunks(vec![tool_call_chunk(
            Some("".to_string()),
            Some(r#"ar"}"#.to_string()),
            Some("abc_123".to_string()),
            Some(0),
        )])
        .build();

    let mut chunk_3 = AIMessageChunk::builder().content("").build();
    chunk_3.set_chunk_position(Some(ChunkPosition::Last));

    let merged = add_ai_message_chunks(chunk_1, vec![chunk_2, chunk_3]);
    assert_eq!(merged.content, "");

    assert!(!merged.tool_calls.is_empty() || !merged.tool_call_chunks.is_empty());
}

#[test]
fn test_content_blocks_reasoning_extraction() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert(
        "reasoning_content".to_string(),
        json!("Let me think about this problem..."),
    );

    let message = AIMessage::builder()
        .content("The answer is 42.")
        .additional_kwargs(additional_kwargs)
        .build();

    assert_eq!(message.content, "The answer is 42.");
    assert!(message.additional_kwargs.contains_key("reasoning_content"));
    assert_eq!(
        message
            .additional_kwargs
            .get("reasoning_content")
            .unwrap()
            .as_str()
            .unwrap(),
        "Let me think about this problem..."
    );

    let mut additional_kwargs2 = std::collections::HashMap::new();
    additional_kwargs2.insert("other_field".to_string(), json!("some value"));

    let message2 = AIMessage::builder()
        .content("The answer is 42.")
        .additional_kwargs(additional_kwargs2)
        .build();

    assert!(!message2.additional_kwargs.contains_key("reasoning_content"));
}

#[test]
fn test_ai_message_basic() {
    let msg = AIMessage::builder().content("Hello, world!").build();
    assert_eq!(msg.content, "Hello, world!");
    assert!(msg.id.is_none());
    assert!(msg.name.is_none());
    assert!(msg.tool_calls.is_empty());
    assert!(msg.invalid_tool_calls.is_empty());
}

#[test]
fn test_ai_message_with_id() {
    let msg = AIMessage::builder()
        .content("Hello!")
        .id("msg-123".to_string())
        .build();
    assert_eq!(msg.id, Some("msg-123".to_string()));
    assert_eq!(msg.content, "Hello!");
}

#[test]
fn test_ai_message_with_name() {
    let msg = AIMessage::builder()
        .content("Hello!")
        .name("Assistant".to_string())
        .build();
    assert_eq!(msg.name, Some("Assistant".to_string()));
}

#[test]
fn test_ai_message_with_usage_metadata() {
    let usage = UsageMetadata::new(10, 20);
    let msg = AIMessage::builder()
        .content("Hello!")
        .usage_metadata(usage)
        .build();
    assert!(msg.usage_metadata.is_some());
    assert_eq!(msg.usage_metadata.as_ref().unwrap().input_tokens, 10);
}

#[test]
fn test_ai_message_chunk_basic() {
    let chunk = AIMessageChunk::builder().content("Hello").build();
    assert_eq!(chunk.content, "Hello");
    assert!(chunk.id.is_none());
}

#[test]
fn test_ai_message_chunk_with_id() {
    let chunk = AIMessageChunk::builder()
        .content("Hello")
        .id("chunk-123".to_string())
        .build();
    assert_eq!(chunk.id, Some("chunk-123".to_string()));
}

#[test]
fn test_ai_message_chunk_add() {
    let chunk1 = AIMessageChunk::builder().content("Hello ").build();
    let chunk2 = AIMessageChunk::builder().content("world!").build();
    let result = chunk1 + chunk2;
    assert_eq!(result.content, "Hello world!");
}

#[test]
fn test_ai_message_chunk_sum() {
    let chunks = vec![
        AIMessageChunk::builder().content("Hello ").build(),
        AIMessageChunk::builder().content("beautiful ").build(),
        AIMessageChunk::builder().content("world!").build(),
    ];
    let result: AIMessageChunk = chunks.into_iter().sum();
    assert_eq!(result.content, "Hello beautiful world!");
}

#[test]
fn test_ai_message_chunk_to_message() {
    let mut chunk = AIMessageChunk::builder()
        .content("Hello!")
        .id("chunk-1".to_string())
        .build();
    chunk.set_usage_metadata(Some(UsageMetadata::new(5, 10)));

    let message = chunk.to_message();
    assert_eq!(message.content, "Hello!");
    assert_eq!(message.id, Some("chunk-1".to_string()));
    assert!(message.usage_metadata.is_some());
}

#[test]
fn test_ai_message_chunk_id_priority() {
    let chunk1 = AIMessageChunk::builder()
        .content("")
        .id("lc_auto123".to_string())
        .build();
    let chunk2 = AIMessageChunk::builder()
        .content("")
        .id("provider_id_456".to_string())
        .build();
    let chunk3 = AIMessageChunk::builder()
        .content("")
        .id("lc_run-789".to_string())
        .build();

    let result = add_ai_message_chunks(chunk1, vec![chunk2, chunk3]);
    assert_eq!(result.id, Some("provider_id_456".to_string()));
}

#[test]
fn test_ai_message_chunk_lc_run_priority() {
    let chunk1 = AIMessageChunk::builder()
        .content("")
        .id("lc_auto123".to_string())
        .build();
    let chunk2 = AIMessageChunk::builder()
        .content("")
        .id("lc_run-789".to_string())
        .build();

    let result = add_ai_message_chunks(chunk1, vec![chunk2]);
    assert_eq!(result.id, Some("lc_run-789".to_string()));
}

#[test]
fn test_ai_message_chunk_init_tool_calls() {
    let mut chunk = AIMessageChunk::builder()
        .content("")
        .tool_call_chunks(vec![tool_call_chunk(
            Some("get_weather".to_string()),
            Some(r#"{"city": "London"}"#.to_string()),
            Some("call_123".to_string()),
            Some(0),
        )])
        .build();
    chunk.set_chunk_position(Some(ChunkPosition::Last));
    chunk.init_tool_calls();

    assert_eq!(chunk.tool_calls.len(), 1);
    assert_eq!(chunk.tool_calls[0].name, "get_weather");
    assert_eq!(chunk.tool_calls[0].id, Some("call_123".to_string()));
}

#[test]
fn test_ai_message_chunk_init_tool_calls_invalid_json() {
    let mut chunk = AIMessageChunk::builder()
        .content("")
        .tool_call_chunks(vec![tool_call_chunk(
            Some("get_weather".to_string()),
            Some("invalid json {".to_string()),
            Some("call_123".to_string()),
            Some(0),
        )])
        .build();
    chunk.set_chunk_position(Some(ChunkPosition::Last));
    chunk.init_tool_calls();

    assert!(chunk.tool_calls.is_empty());
    assert_eq!(chunk.invalid_tool_calls.len(), 1);
    assert_eq!(
        chunk.invalid_tool_calls[0].name,
        Some("get_weather".to_string())
    );
}

#[test]
fn test_ai_message_type_field() {
    let msg = AIMessage::builder().content("hello").build();
    assert_eq!(msg.message_type(), "ai");

    let msg_with_tools = AIMessage::builder()
        .content("")
        .tool_calls(vec![tool_call("t", json!({}), Some("1".to_string()))])
        .build();
    assert_eq!(msg_with_tools.message_type(), "ai");
}

#[test]
fn test_ai_message_pretty_repr_with_tool_calls() {
    let msg = AIMessage::builder()
        .content("Sure, let me call that tool.")
        .tool_calls(vec![tool_call(
            "get_weather",
            json!({"city": "SF"}),
            Some("call_1".to_string()),
        )])
        .build();
    let result = msg.pretty_repr(false);
    assert!(result.contains("Tool Calls:"));
    assert!(result.contains("get_weather (call_1)"));
    assert!(result.contains("Call ID: call_1"));
    assert!(result.contains("Args:"));
    assert!(result.contains("city"));
}

#[test]
fn test_ai_message_pretty_repr_with_invalid_tool_calls() {
    let msg = AIMessage::builder()
        .content("")
        .invalid_tool_calls(vec![invalid_tool_call(
            Some("broken".to_string()),
            Some("not json".to_string()),
            Some("call_bad".to_string()),
            Some("parse error".to_string()),
        )])
        .build();
    let result = msg.pretty_repr(false);
    assert!(result.contains("Invalid Tool Calls:"));
    assert!(result.contains("broken (call_bad)"));
    assert!(result.contains("Call ID: call_bad"));
    assert!(result.contains("Error: parse error"));
    assert!(result.contains("Args:"));
    assert!(result.contains("not json"));
}

#[test]
fn test_ai_message_pretty_repr_with_string_args() {
    let msg = AIMessage::builder()
        .content("")
        .invalid_tool_calls(vec![invalid_tool_call(
            Some("mytool".to_string()),
            Some("raw string args".to_string()),
            Some("id1".to_string()),
            None,
        )])
        .build();
    let result = msg.pretty_repr(false);
    assert!(result.contains("Invalid Tool Calls:"));
    assert!(result.contains("raw string args"));
}

#[test]
fn test_ai_message_init_with_usage_metadata() {
    let usage = UsageMetadata::new(10, 5);
    let msg = AIMessage::builder()
        .content("hello")
        .usage_metadata(usage)
        .build();
    assert!(msg.usage_metadata.is_some());
    let um = msg.usage_metadata.as_ref().unwrap();
    assert_eq!(um.input_tokens, 10);
    assert_eq!(um.output_tokens, 5);
    assert_eq!(um.total_tokens, 15);

    let msg_no_usage = AIMessage::builder().content("hi").build();
    assert!(msg_no_usage.usage_metadata.is_none());
}

#[test]
fn test_ai_message_serdes_with_usage_metadata() {
    let usage = UsageMetadata {
        input_tokens: 100,
        output_tokens: 50,
        total_tokens: 150,
        input_token_details: Some(InputTokenDetails {
            audio: None,
            cache_creation: None,
            cache_read: Some(20),
            ..Default::default()
        }),
        output_token_details: Some(OutputTokenDetails {
            audio: None,
            reasoning: Some(10),
            ..Default::default()
        }),
    };
    let msg = AIMessage::builder()
        .content("result")
        .usage_metadata(usage.clone())
        .build();

    let serialized = serde_json::to_value(&msg).unwrap();
    let loaded: AIMessage = serde_json::from_value(serialized).unwrap();

    assert_eq!(loaded.usage_metadata, Some(usage));
    assert_eq!(
        loaded
            .usage_metadata
            .as_ref()
            .unwrap()
            .input_token_details
            .as_ref()
            .unwrap()
            .cache_read,
        Some(20)
    );
    assert_eq!(
        loaded
            .usage_metadata
            .as_ref()
            .unwrap()
            .output_token_details
            .as_ref()
            .unwrap()
            .reasoning,
        Some(10)
    );
    assert_eq!(loaded.content, "result");
}

#[test]
fn test_backwards_compat_tool_calls_from_additional_kwargs() {
    use agent_chain_core::messages::backwards_compat_tool_calls;

    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert(
        "tool_calls".to_string(),
        json!([
            {
                "id": "call_abc",
                "function": {
                    "name": "search",
                    "arguments": "{\"query\": \"langchain\"}"
                },
                "type": "function"
            }
        ]),
    );

    let (tool_calls, invalid_tool_calls, _) =
        backwards_compat_tool_calls(&additional_kwargs, false);

    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].name, "search");
    assert_eq!(tool_calls[0].args, json!({"query": "langchain"}));
    assert_eq!(tool_calls[0].id, Some("call_abc".to_string()));
    assert_eq!(tool_calls[0].call_type, Some("tool_call".to_string()));
    assert!(invalid_tool_calls.is_empty());
}

#[test]
fn test_backwards_compat_invalid_json_becomes_invalid_tool_calls() {
    use agent_chain_core::messages::backwards_compat_tool_calls;

    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert(
        "tool_calls".to_string(),
        json!([
            {
                "id": "call_xyz",
                "function": {
                    "name": "broken_tool",
                    "arguments": "this is not json{{{"
                },
                "type": "function"
            }
        ]),
    );

    let (tool_calls, invalid_tool_calls, _) =
        backwards_compat_tool_calls(&additional_kwargs, false);

    assert!(tool_calls.is_empty());
    assert_eq!(invalid_tool_calls.len(), 1);
    assert_eq!(invalid_tool_calls[0].name, Some("broken_tool".to_string()));
    assert_eq!(
        invalid_tool_calls[0].args,
        Some("this is not json{{{".to_string())
    );
    assert_eq!(invalid_tool_calls[0].id, Some("call_xyz".to_string()));
    assert_eq!(
        invalid_tool_calls[0].call_type,
        Some("invalid_tool_call".to_string())
    );
}

#[test]
fn test_ai_message_chunk_type_field() {
    let chunk = AIMessageChunk::builder().content("hi").build();
    let serialized = serde_json::to_value(&chunk).unwrap();
    assert_eq!(
        serialized.get("type").unwrap().as_str().unwrap(),
        "AIMessageChunk"
    );
}

#[test]
fn test_ai_message_chunk_chunk_position_field() {
    let chunk = AIMessageChunk::builder()
        .content("done")
        .chunk_position(ChunkPosition::Last)
        .build();
    assert_eq!(chunk.chunk_position, Some(ChunkPosition::Last));

    let chunk_none = AIMessageChunk::builder().content("partial").build();
    assert!(chunk_none.chunk_position.is_none());
}

#[test]
fn test_init_tool_calls_populates_tool_call_chunks_from_tool_calls() {
    let mut chunk = AIMessageChunk::builder()
        .content("")
        .tool_calls(vec![tool_call(
            "my_tool",
            json!({"key": "val"}),
            Some("tc_1".to_string()),
        )])
        .build();

    chunk.init_tool_calls();

    assert_eq!(chunk.tool_call_chunks.len(), 1);
    assert_eq!(chunk.tool_call_chunks[0].name, Some("my_tool".to_string()));
    assert_eq!(
        chunk.tool_call_chunks[0].args,
        Some("{\"key\":\"val\"}".to_string())
    );
    assert_eq!(chunk.tool_call_chunks[0].id, Some("tc_1".to_string()));
    assert!(chunk.tool_call_chunks[0].index.is_none());
    assert_eq!(
        chunk.tool_call_chunks[0].chunk_type,
        Some("tool_call_chunk".to_string())
    );
}

#[test]
fn test_init_tool_calls_populates_tool_call_chunks_from_invalid_tool_calls() {
    let mut chunk = AIMessageChunk::builder()
        .content("")
        .invalid_tool_calls(vec![invalid_tool_call(
            Some("bad_tool".to_string()),
            Some("bad args".to_string()),
            Some("itc_1".to_string()),
            Some("fail".to_string()),
        )])
        .build();

    chunk.init_tool_calls();

    assert_eq!(chunk.tool_call_chunks.len(), 1);
    assert_eq!(chunk.tool_call_chunks[0].name, Some("bad_tool".to_string()));
    assert_eq!(chunk.tool_call_chunks[0].args, Some("bad args".to_string()));
    assert_eq!(chunk.tool_call_chunks[0].id, Some("itc_1".to_string()));
    assert!(chunk.tool_call_chunks[0].index.is_none());
    assert_eq!(
        chunk.tool_call_chunks[0].chunk_type,
        Some("tool_call_chunk".to_string())
    );
}

#[test]
fn test_ai_message_chunk_add_with_list_of_chunks() {
    let base = AIMessageChunk::builder().content("Hello").build();
    let others = vec![
        AIMessageChunk::builder().content(" world").build(),
        AIMessageChunk::builder().content("!").build(),
    ];
    let result = add_ai_message_chunks(base, others);
    assert_eq!(result.content, "Hello world!");
}

#[test]
fn test_add_ai_message_chunks_id_priority_full() {
    let provider_id = "chatcmpl-abc123";
    let run_id = "lc_run-some-uuid";
    let auto_id = "lc_auto-generated-uuid";

    let chunk_auto = AIMessageChunk::builder()
        .content("a")
        .id(auto_id.to_string())
        .build();
    let chunk_run = AIMessageChunk::builder()
        .content("b")
        .id(run_id.to_string())
        .build();
    let chunk_provider = AIMessageChunk::builder()
        .content("c")
        .id(provider_id.to_string())
        .build();

    let result = add_ai_message_chunks(chunk_auto.clone(), vec![chunk_run.clone(), chunk_provider]);
    assert_eq!(result.id.as_deref(), Some(provider_id));

    let result2 = add_ai_message_chunks(chunk_auto.clone(), vec![chunk_run]);
    assert_eq!(result2.id.as_deref(), Some(run_id));

    let chunk_auto2 = AIMessageChunk::builder()
        .content("d")
        .id("lc_other-uuid".to_string())
        .build();
    let result3 = add_ai_message_chunks(chunk_auto, vec![chunk_auto2]);
    assert_eq!(result3.id.as_deref(), Some(auto_id));
}

#[test]
fn test_add_ai_message_chunks_chunk_position_propagation() {
    let chunk1 = AIMessageChunk::builder().content("a").build();
    let chunk2 = AIMessageChunk::builder().content("b").build();
    let chunk3 = AIMessageChunk::builder()
        .content("c")
        .chunk_position(ChunkPosition::Last)
        .build();

    let result = add_ai_message_chunks(chunk1.clone(), vec![chunk2.clone(), chunk3]);
    assert_eq!(result.chunk_position, Some(ChunkPosition::Last));

    let result_no_last = add_ai_message_chunks(chunk1, vec![chunk2]);
    assert!(result_no_last.chunk_position.is_none());
}

#[test]
fn test_subtract_usage_with_details() {
    let usage1 = UsageMetadata {
        input_tokens: 20,
        output_tokens: 30,
        total_tokens: 50,
        input_token_details: Some(InputTokenDetails {
            audio: Some(10),
            cache_creation: None,
            cache_read: Some(8),
            ..Default::default()
        }),
        output_token_details: Some(OutputTokenDetails {
            audio: Some(5),
            reasoning: Some(15),
            ..Default::default()
        }),
    };
    let usage2 = UsageMetadata {
        input_tokens: 5,
        output_tokens: 10,
        total_tokens: 15,
        input_token_details: Some(InputTokenDetails {
            audio: Some(3),
            cache_creation: None,
            cache_read: Some(2),
            ..Default::default()
        }),
        output_token_details: Some(OutputTokenDetails {
            audio: Some(2),
            reasoning: Some(5),
            ..Default::default()
        }),
    };
    let result = subtract_usage(Some(&usage1), Some(&usage2));
    assert_eq!(result.input_tokens, 15);
    assert_eq!(result.output_tokens, 20);
    assert_eq!(result.total_tokens, 35);
    assert_eq!(result.input_token_details.as_ref().unwrap().audio, Some(7));
    assert_eq!(
        result.input_token_details.as_ref().unwrap().cache_read,
        Some(6)
    );
    assert_eq!(result.output_token_details.as_ref().unwrap().audio, Some(3));
    assert_eq!(
        result.output_token_details.as_ref().unwrap().reasoning,
        Some(10)
    );
}

#[test]
fn test_subtract_usage_right_none_returns_left() {
    let usage = UsageMetadata {
        input_tokens: 10,
        output_tokens: 20,
        total_tokens: 30,
        input_token_details: Some(InputTokenDetails {
            audio: None,
            cache_creation: None,
            cache_read: Some(5),
            ..Default::default()
        }),
        output_token_details: None,
    };
    let result = subtract_usage(Some(&usage), None);
    assert_eq!(result, usage);
    assert_eq!(result.input_tokens, 10);
    assert_eq!(result.output_tokens, 20);
    assert_eq!(result.total_tokens, 30);
    assert_eq!(
        result.input_token_details.as_ref().unwrap().cache_read,
        Some(5)
    );
}

#[test]
fn test_ai_message_chunk_content_blocks_reasoning_from_additional_kwargs() {
    use agent_chain_core::messages::extract_reasoning_from_additional_kwargs;

    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert(
        "reasoning_content".to_string(),
        json!("I need to compute 3 + 4."),
    );

    let reasoning = extract_reasoning_from_additional_kwargs(&additional_kwargs);
    assert!(reasoning.is_some());

    let mut other_kwargs = std::collections::HashMap::new();
    other_kwargs.insert("something_else".to_string(), json!("value"));

    let no_reasoning = extract_reasoning_from_additional_kwargs(&other_kwargs);
    assert!(no_reasoning.is_none());
}

#[test]
fn test_ai_message_chunk_content_blocks_with_output_version_v1() {
    let content_list = json!([
        {"type": "text", "text": "hello"},
        {"type": "tool_call", "name": "foo", "args": {"a": 1}, "id": "tc1"}
    ]);

    let mut response_metadata = std::collections::HashMap::new();
    response_metadata.insert("output_version".to_string(), json!("v1"));

    let chunk = AIMessageChunk::builder()
        .content(serde_json::to_string(&content_list).unwrap())
        .response_metadata(response_metadata)
        .build();

    let blocks = chunk.content_blocks();
    assert_eq!(blocks.len(), 2);
    match &blocks[0] {
        agent_chain_core::messages::ContentBlock::Text(t) => {
            assert_eq!(t.text, "hello");
        }
        _ => panic!("Expected Text content block"),
    }
    match &blocks[1] {
        agent_chain_core::messages::ContentBlock::ToolCall(tc) => {
            assert_eq!(tc.name, "foo");
        }
        _ => panic!("Expected ToolCall content block"),
    }
}
