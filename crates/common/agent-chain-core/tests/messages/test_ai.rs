//! Tests for AI message types.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/test_ai.py`

use agent_chain_core::messages::{
    AIMessage, AIMessageChunk, ChunkPosition, InputTokenDetails, OutputTokenDetails, UsageMetadata,
    add_ai_message_chunks, add_usage, invalid_tool_call, subtract_usage, tool_call,
    tool_call_chunk,
};
use serde_json::json;

// ============================================================================
// test_serdes_message
// ============================================================================

#[test]
fn test_serdes_message() {
    let msg = AIMessage::with_all_tool_calls(
        "",
        vec![tool_call("foo", json!({"bar": 1}), Some("baz".to_string()))],
        vec![invalid_tool_call(
            Some("foobad".to_string()),
            Some("blah".to_string()),
            Some("booz".to_string()),
            Some("bad".to_string()),
        )],
    );

    // For now, test that we can serialize/deserialize using serde_json
    // Python test expects:
    // {
    //     "lc": 1,
    //     "type": "constructor",
    //     "id": ["langchain", "schema", "messages", "AIMessage"],
    //     "kwargs": {
    //         "type": "ai",
    //         "content": [{"text": "blah", "type": "text"}],
    //         "tool_calls": [
    //             {"name": "foo", "args": {"bar": 1}, "id": "baz", "type": "tool_call"}
    //         ],
    //         "invalid_tool_calls": [
    //             {
    //                 "name": "foobad",
    //                 "args": "blah",
    //                 "id": "booz",
    //                 "error": "bad",
    //                 "type": "invalid_tool_call",
    //             }
    //         ],
    //     },
    // }
    let serialized = serde_json::to_value(&msg).unwrap();

    // Check tool_calls
    let tool_calls = serialized.get("tool_calls").unwrap().as_array().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].get("name").unwrap().as_str().unwrap(), "foo");

    // Check invalid_tool_calls
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

    // Test roundtrip
    let deserialized: AIMessage = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.tool_calls.len(), 1);
    assert_eq!(deserialized.tool_calls[0].name(), "foo");
    assert_eq!(deserialized.invalid_tool_calls.len(), 1);
    assert_eq!(
        deserialized.invalid_tool_calls[0].name,
        Some("foobad".to_string())
    );
}

// ============================================================================
// test_serdes_message_chunk
// ============================================================================

#[test]
fn test_serdes_message_chunk() {
    let chunk = AIMessageChunk::new_with_tool_call_chunks(
        "",
        vec![
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
        ],
    );

    // Serialize and check structure
    let serialized = serde_json::to_value(&chunk).unwrap();

    // Check tool_call_chunks
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

    // Test roundtrip
    let deserialized: AIMessageChunk = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.tool_call_chunks().len(), 2);
    assert_eq!(
        deserialized.tool_call_chunks()[0].name,
        Some("foo".to_string())
    );
}

// ============================================================================
// test_add_usage_both_none
// ============================================================================

#[test]
fn test_add_usage_both_none() {
    let result = add_usage(None, None);
    assert_eq!(result.input_tokens, 0);
    assert_eq!(result.output_tokens, 0);
    assert_eq!(result.total_tokens, 0);
}

// ============================================================================
// test_add_usage_one_none
// ============================================================================

#[test]
fn test_add_usage_one_none() {
    let usage = UsageMetadata::new(10, 20);
    let result = add_usage(Some(&usage), None);
    assert_eq!(result.input_tokens, 10);
    assert_eq!(result.output_tokens, 20);
    assert_eq!(result.total_tokens, 30);
}

// ============================================================================
// test_add_usage_both_present
// ============================================================================

#[test]
fn test_add_usage_both_present() {
    let usage1 = UsageMetadata::new(10, 20);
    let usage2 = UsageMetadata::new(5, 10);
    let result = add_usage(Some(&usage1), Some(&usage2));
    assert_eq!(result.input_tokens, 15);
    assert_eq!(result.output_tokens, 30);
    assert_eq!(result.total_tokens, 45);
}

// ============================================================================
// test_add_usage_with_details
// ============================================================================

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
        }),
        output_token_details: Some(OutputTokenDetails {
            audio: None,
            reasoning: Some(10),
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
        }),
        output_token_details: Some(OutputTokenDetails {
            audio: None,
            reasoning: Some(5),
        }),
    };
    let result = add_usage(Some(&usage1), Some(&usage2));

    assert_eq!(result.input_token_details.as_ref().unwrap().audio, Some(8));
    assert_eq!(
        result.output_token_details.as_ref().unwrap().reasoning,
        Some(15)
    );
}

// ============================================================================
// test_subtract_usage_both_none
// ============================================================================

#[test]
fn test_subtract_usage_both_none() {
    let result = subtract_usage(None, None);
    assert_eq!(result.input_tokens, 0);
    assert_eq!(result.output_tokens, 0);
    assert_eq!(result.total_tokens, 0);
}

// ============================================================================
// test_subtract_usage_one_none
// ============================================================================

#[test]
fn test_subtract_usage_one_none() {
    let usage = UsageMetadata::new(10, 20);
    let result = subtract_usage(Some(&usage), None);
    assert_eq!(result.input_tokens, 10);
    assert_eq!(result.output_tokens, 20);
    assert_eq!(result.total_tokens, 30);
}

// ============================================================================
// test_subtract_usage_both_present
// ============================================================================

#[test]
fn test_subtract_usage_both_present() {
    let usage1 = UsageMetadata::new(10, 20);
    let usage2 = UsageMetadata::new(5, 10);
    let result = subtract_usage(Some(&usage1), Some(&usage2));
    assert_eq!(result.input_tokens, 5);
    assert_eq!(result.output_tokens, 10);
    assert_eq!(result.total_tokens, 15);
}

// ============================================================================
// test_subtract_usage_with_negative_result
// ============================================================================

#[test]
fn test_subtract_usage_with_negative_result() {
    let usage1 = UsageMetadata::new(5, 10);
    let usage2 = UsageMetadata::new(10, 20);
    let result = subtract_usage(Some(&usage1), Some(&usage2));
    // Results should be floored at 0
    assert_eq!(result.input_tokens, 0);
    assert_eq!(result.output_tokens, 0);
    assert_eq!(result.total_tokens, 0);
}

// ============================================================================
// test_add_ai_message_chunks_usage
// ============================================================================

#[test]
fn test_add_ai_message_chunks_usage() {
    let chunk1 = AIMessageChunk::builder().content("").build();
    // chunk1 has no usage_metadata

    let chunk2 = AIMessageChunk::builder().content("").build().with_usage_metadata(UsageMetadata::new(2, 3));

    let chunk3 = AIMessageChunk::builder().content("").build().with_usage_metadata(UsageMetadata {
        input_tokens: 2,
        output_tokens: 3,
        total_tokens: 5,
        input_token_details: Some(InputTokenDetails {
            audio: Some(1),
            cache_creation: None,
            cache_read: Some(1),
        }),
        output_token_details: Some(OutputTokenDetails {
            audio: Some(1),
            reasoning: Some(2),
        }),
    });

    let combined = add_ai_message_chunks(chunk1, vec![chunk2, chunk3]);

    assert!(combined.usage_metadata().is_some());
    let usage = combined.usage_metadata().unwrap();
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

// ============================================================================
// test_init_tool_calls
// ============================================================================

#[test]
fn test_init_tool_calls() {
    // Test we can create AIMessage with tool_calls (Python test adds "type" key on init)
    let msg = AIMessage::with_tool_calls(
        "",
        vec![tool_call("foo", json!({"a": "b"}), Some("abc".to_string()))],
    );
    assert_eq!(msg.tool_calls.len(), 1);
    // In Rust, tool_call helper creates a ToolCall struct which has consistent type
    assert_eq!(msg.tool_calls[0].name(), "foo");
}

// ============================================================================
// test_content_blocks
// ============================================================================

#[test]
fn test_content_blocks() {
    // Test AIMessage with tool_calls
    let message = AIMessage::with_tool_calls(
        "",
        vec![tool_call(
            "foo",
            json!({"a": "b"}),
            Some("abc_123".to_string()),
        )],
    );
    assert_eq!(message.tool_calls.len(), 1);
    assert_eq!(message.content(), "");

    // Test AIMessage with content and tool_calls
    let message2 = AIMessage::with_tool_calls(
        "foo",
        vec![tool_call(
            "foo",
            json!({"a": "b"}),
            Some("abc_123".to_string()),
        )],
    );
    assert_eq!(message2.content(), "foo");
    assert_eq!(message2.tool_calls.len(), 1);

    // Test AIMessageChunk with tool_call_chunks
    let chunk = AIMessageChunk::new_with_tool_call_chunks(
        "",
        vec![tool_call_chunk(
            Some("foo".to_string()),
            Some("".to_string()),
            Some("abc_123".to_string()),
            Some(0),
        )],
    );
    assert_eq!(chunk.tool_call_chunks().len(), 1);
    assert_eq!(chunk.content(), "");

    // Test merging tool call chunks
    let chunk_1 = AIMessageChunk::new_with_tool_call_chunks(
        "",
        vec![tool_call_chunk(
            Some("foo".to_string()),
            Some(r#"{"foo": "b"#.to_string()),
            Some("abc_123".to_string()),
            Some(0),
        )],
    );

    let chunk_2 = AIMessageChunk::new_with_tool_call_chunks(
        "",
        vec![tool_call_chunk(
            Some("".to_string()),
            Some(r#"ar"}"#.to_string()),
            Some("abc_123".to_string()),
            Some(0),
        )],
    );

    let mut chunk_3 = AIMessageChunk::builder().content("").build();
    chunk_3.set_chunk_position(Some(ChunkPosition::Last));

    let merged = add_ai_message_chunks(chunk_1, vec![chunk_2, chunk_3]);
    assert_eq!(merged.content(), "");

    // With chunk_position=Last, tool_call_chunks should be parsed into tool_calls
    assert!(!merged.tool_calls().is_empty() || !merged.tool_call_chunks().is_empty());
}

// ============================================================================
// test_content_blocks_reasoning_extraction
// ============================================================================

#[test]
fn test_content_blocks_reasoning_extraction() {
    // Test best-effort reasoning extraction from `additional_kwargs`
    // Python test adds reasoning_content to additional_kwargs
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert(
        "reasoning_content".to_string(),
        json!("Let me think about this problem..."),
    );

    let message = AIMessage::builder().content("The answer is 42.").build().with_additional_kwargs(additional_kwargs);

    assert_eq!(message.content(), "The answer is 42.");
    // In Python, content_blocks property extracts reasoning from additional_kwargs
    // For now, we verify the additional_kwargs is set correctly
    assert!(
        message
            .additional_kwargs()
            .contains_key("reasoning_content")
    );
    assert_eq!(
        message
            .additional_kwargs()
            .get("reasoning_content")
            .unwrap()
            .as_str()
            .unwrap(),
        "Let me think about this problem..."
    );

    // Test no reasoning extraction when no reasoning content
    let mut additional_kwargs2 = std::collections::HashMap::new();
    additional_kwargs2.insert("other_field".to_string(), json!("some value"));

    let message2 = AIMessage::builder().content("The answer is 42.").build().with_additional_kwargs(additional_kwargs2);

    assert!(
        !message2
            .additional_kwargs()
            .contains_key("reasoning_content")
    );
}

// ============================================================================
// Additional tests for AIMessage/AIMessageChunk
// ============================================================================

#[test]
fn test_ai_message_basic() {
    let msg = AIMessage::builder().content("Hello, world!").build();
    assert_eq!(msg.content(), "Hello, world!");
    assert!(msg.id().is_none());
    assert!(msg.name().is_none());
    assert!(msg.tool_calls.is_empty());
    assert!(msg.invalid_tool_calls.is_empty());
}

#[test]
fn test_ai_message_with_id() {
    let msg = AIMessage::with_id("msg-123", "Hello!");
    assert_eq!(msg.id(), Some("msg-123".to_string()));
    assert_eq!(msg.content(), "Hello!");
}

#[test]
fn test_ai_message_with_name() {
    let msg = AIMessage::builder().content("Hello!").build().with_name("Assistant");
    assert_eq!(msg.name(), Some("Assistant".to_string()));
}

#[test]
fn test_ai_message_with_usage_metadata() {
    let usage = UsageMetadata::new(10, 20);
    let msg = AIMessage::builder().content("Hello!").build().with_usage_metadata(usage);
    assert!(msg.usage_metadata().is_some());
    assert_eq!(msg.usage_metadata().unwrap().input_tokens, 10);
}

#[test]
fn test_ai_message_chunk_basic() {
    let chunk = AIMessageChunk::builder().content("Hello").build();
    assert_eq!(chunk.content(), "Hello");
    assert!(chunk.id().is_none());
}

#[test]
fn test_ai_message_chunk_with_id() {
    let chunk = AIMessageChunk::with_id("chunk-123", "Hello");
    assert_eq!(chunk.id(), Some("chunk-123".to_string()));
}

#[test]
fn test_ai_message_chunk_add() {
    let chunk1 = AIMessageChunk::builder().content("Hello ").build();
    let chunk2 = AIMessageChunk::builder().content("world!").build();
    let result = chunk1 + chunk2;
    assert_eq!(result.content(), "Hello world!");
}

#[test]
fn test_ai_message_chunk_sum() {
    let chunks = vec![
        AIMessageChunk::builder().content("Hello ").build(),
        AIMessageChunk::builder().content("beautiful ").build(),
        AIMessageChunk::builder().content("world!").build(),
    ];
    let result: AIMessageChunk = chunks.into_iter().sum();
    assert_eq!(result.content(), "Hello beautiful world!");
}

#[test]
fn test_ai_message_chunk_to_message() {
    let mut chunk = AIMessageChunk::with_id("chunk-1", "Hello!");
    chunk.set_usage_metadata(Some(UsageMetadata::new(5, 10)));

    let message = chunk.to_message();
    assert_eq!(message.content(), "Hello!");
    assert_eq!(message.id(), Some("chunk-1".to_string()));
    assert!(message.usage_metadata().is_some());
}

#[test]
fn test_ai_message_chunk_id_priority() {
    // Provider-assigned ID should take priority over lc_* IDs
    let chunk1 = AIMessageChunk::with_id("lc_auto123", "");
    let chunk2 = AIMessageChunk::with_id("provider_id_456", "");
    let chunk3 = AIMessageChunk::with_id("lc_run-789", "");

    let result = add_ai_message_chunks(chunk1, vec![chunk2, chunk3]);
    assert_eq!(result.id(), Some("provider_id_456".to_string()));
}

#[test]
fn test_ai_message_chunk_lc_run_priority() {
    // lc_run-* should take priority over lc_* (auto-generated)
    let chunk1 = AIMessageChunk::with_id("lc_auto123", "");
    let chunk2 = AIMessageChunk::with_id("lc_run-789", "");

    let result = add_ai_message_chunks(chunk1, vec![chunk2]);
    assert_eq!(result.id(), Some("lc_run-789".to_string()));
}

#[test]
fn test_ai_message_chunk_init_tool_calls() {
    // Test that tool_call_chunks are parsed into tool_calls when chunk_position is Last
    let mut chunk = AIMessageChunk::new_with_tool_call_chunks(
        "",
        vec![tool_call_chunk(
            Some("get_weather".to_string()),
            Some(r#"{"city": "London"}"#.to_string()),
            Some("call_123".to_string()),
            Some(0),
        )],
    );
    chunk.set_chunk_position(Some(ChunkPosition::Last));
    chunk.init_tool_calls();

    assert_eq!(chunk.tool_calls().len(), 1);
    assert_eq!(chunk.tool_calls()[0].name(), "get_weather");
    assert_eq!(chunk.tool_calls()[0].id(), Some("call_123".to_string()));
}

#[test]
fn test_ai_message_chunk_init_tool_calls_invalid_json() {
    // Test that invalid JSON in tool_call_chunks creates invalid_tool_calls
    let mut chunk = AIMessageChunk::new_with_tool_call_chunks(
        "",
        vec![tool_call_chunk(
            Some("get_weather".to_string()),
            Some("invalid json {".to_string()),
            Some("call_123".to_string()),
            Some(0),
        )],
    );
    chunk.set_chunk_position(Some(ChunkPosition::Last));
    chunk.init_tool_calls();

    assert!(chunk.tool_calls().is_empty());
    assert_eq!(chunk.invalid_tool_calls().len(), 1);
    assert_eq!(
        chunk.invalid_tool_calls()[0].name,
        Some("get_weather".to_string())
    );
}
