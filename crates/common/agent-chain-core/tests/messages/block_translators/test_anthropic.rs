//! Tests for Anthropic block translator.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/block_translators/test_anthropic.py`
//!
//! These tests verify that Anthropic-specific content blocks are correctly translated
//! to the standard v1 content block format via the `content_blocks` property.
//!
//! NOTE: These tests use API methods that may not yet exist in the Rust implementation.
//! The tests are written to match the Python API exactly, serving as a specification
//! for what needs to be implemented.

use agent_chain_core::messages::{
    AIMessage,
    AIMessageChunk,
    Annotation,
    BlockIndex,
    ChunkPosition,
    ContentBlock,
    FileContentBlock,
    HumanMessage,
    ImageContentBlock,
    MessageContent,
    NonStandardContentBlock,
    PlainTextContentBlock,
    ReasoningContentBlock,
    ServerToolCall,
    ServerToolResult,
    ServerToolStatus,
    TextContentBlock,
    ToolCallBlock,
    ToolCallChunkBlock,
    tool_call_chunk,
};
use serde_json::json;
use std::collections::HashMap;


/// Test conversion of Anthropic AI message content to v1 format.
///
/// This test verifies that various Anthropic-specific content block types
/// (thinking, tool_use, server_tool_use, web_search_tool_result, etc.)
/// are correctly translated to standard v1 content blocks.
#[test]
fn test_convert_to_v1_from_anthropic() {
    let content = vec![
        json!({
            "type": "thinking",
            "thinking": "foo",
            "signature": "foo_signature"
        }),
        json!({
            "type": "text",
            "text": "Let's call a tool."
        }),
        json!({
            "type": "tool_use",
            "id": "abc_123",
            "name": "get_weather",
            "input": {"location": "San Francisco"}
        }),
        json!({
            "type": "tool_use",
            "id": "abc_234",
            "name": "get_weather_programmatic",
            "input": {"location": "Boston"},
            "caller": {
                "type": "code_execution_20250825",
                "tool_id": "srvtoolu_abc234"
            }
        }),
        json!({
            "type": "text",
            "text": "It's sunny.",
            "citations": [
                {
                    "type": "search_result_location",
                    "cited_text": "The weather is sunny.",
                    "source": "source_123",
                    "title": "Document Title",
                    "search_result_index": 1,
                    "start_block_index": 0,
                    "end_block_index": 2
                },
                {"bar": "baz"}
            ]
        }),
        json!({
            "type": "server_tool_use",
            "name": "web_search",
            "input": {"query": "web search query"},
            "id": "srvtoolu_abc123"
        }),
        json!({
            "type": "web_search_tool_result",
            "tool_use_id": "srvtoolu_abc123",
            "content": [
                {
                    "type": "web_search_result",
                    "title": "Page Title 1",
                    "url": "<page url 1>",
                    "page_age": "January 1, 2025",
                    "encrypted_content": "<encrypted content 1>"
                },
                {
                    "type": "web_search_result",
                    "title": "Page Title 2",
                    "url": "<page url 2>",
                    "page_age": "January 2, 2025",
                    "encrypted_content": "<encrypted content 2>"
                }
            ]
        }),
        json!({
            "type": "server_tool_use",
            "id": "srvtoolu_def456",
            "name": "code_execution",
            "input": {"code": "import numpy as np..."}
        }),
        json!({
            "type": "code_execution_tool_result",
            "tool_use_id": "srvtoolu_def456",
            "content": {
                "type": "code_execution_result",
                "stdout": "Mean: 5.5\nStandard deviation...",
                "stderr": "",
                "return_code": 0
            }
        }),
        json!({
            "type": "something_else",
            "foo": "bar"
        }),
    ];

    let mut response_metadata = HashMap::new();
    response_metadata.insert("model_provider".to_string(), json!("anthropic"));

    let content_str = serde_json::to_string(&content).unwrap_or_default();
    let message = AIMessage::builder()
        .content(content_str)
        .response_metadata(response_metadata)
        .build();

    let expected_content: Vec<ContentBlock> = vec![
        ContentBlock::Reasoning(ReasoningContentBlock {
            block_type: "reasoning".to_string(),
            id: None,
            reasoning: Some("foo".to_string()),
            index: None,
            extras: Some({
                let mut extras = HashMap::new();
                extras.insert("signature".to_string(), json!("foo_signature"));
                extras
            }),
        }),
        ContentBlock::Text(TextContentBlock {
            block_type: "text".to_string(),
            id: None,
            text: "Let's call a tool.".to_string(),
            annotations: None,
            index: None,
            extras: None,
        }),
        ContentBlock::ToolCall(ToolCallBlock {
            block_type: "tool_call".to_string(),
            id: Some("abc_123".to_string()),
            name: "get_weather".to_string(),
            args: {
                let mut args = HashMap::new();
                args.insert("location".to_string(), json!("San Francisco"));
                args
            },
            index: None,
            extras: None,
        }),
        ContentBlock::ToolCall(ToolCallBlock {
            block_type: "tool_call".to_string(),
            id: Some("abc_234".to_string()),
            name: "get_weather_programmatic".to_string(),
            args: {
                let mut args = HashMap::new();
                args.insert("location".to_string(), json!("Boston"));
                args
            },
            index: None,
            extras: Some({
                let mut extras = HashMap::new();
                extras.insert(
                    "caller".to_string(),
                    json!({
                        "type": "code_execution_20250825",
                        "tool_id": "srvtoolu_abc234"
                    }),
                );
                extras
            }),
        }),
        ContentBlock::Text(TextContentBlock {
            block_type: "text".to_string(),
            id: None,
            text: "It's sunny.".to_string(),
            annotations: Some(vec![
                Annotation::Citation {
                    id: None,
                    url: None,
                    title: Some("Document Title".to_string()),
                    start_index: None,
                    end_index: None,
                    cited_text: Some("The weather is sunny.".to_string()),
                    extras: Some({
                        let mut extras = HashMap::new();
                        extras.insert("source".to_string(), json!("source_123"));
                        extras.insert("search_result_index".to_string(), json!(1));
                        extras.insert("start_block_index".to_string(), json!(0));
                        extras.insert("end_block_index".to_string(), json!(2));
                        extras
                    }),
                },
                Annotation::NonStandardAnnotation {
                    id: None,
                    value: {
                        let mut value = HashMap::new();
                        value.insert("bar".to_string(), json!("baz"));
                        value
                    },
                },
            ]),
            index: None,
            extras: None,
        }),
        ContentBlock::ServerToolCall(ServerToolCall {
            block_type: "server_tool_call".to_string(),
            id: "srvtoolu_abc123".to_string(),
            name: "web_search".to_string(),
            args: {
                let mut args = HashMap::new();
                args.insert("query".to_string(), json!("web search query"));
                args
            },
            index: None,
            extras: None,
        }),
        ContentBlock::ServerToolResult(ServerToolResult {
            block_type: "server_tool_result".to_string(),
            id: None,
            tool_call_id: "srvtoolu_abc123".to_string(),
            status: ServerToolStatus::Success,
            output: Some(json!([
                {
                    "type": "web_search_result",
                    "title": "Page Title 1",
                    "url": "<page url 1>",
                    "page_age": "January 1, 2025",
                    "encrypted_content": "<encrypted content 1>"
                },
                {
                    "type": "web_search_result",
                    "title": "Page Title 2",
                    "url": "<page url 2>",
                    "page_age": "January 2, 2025",
                    "encrypted_content": "<encrypted content 2>"
                }
            ])),
            index: None,
            extras: Some({
                let mut extras = HashMap::new();
                extras.insert("block_type".to_string(), json!("web_search_tool_result"));
                extras
            }),
        }),
        ContentBlock::ServerToolCall(ServerToolCall {
            block_type: "server_tool_call".to_string(),
            id: "srvtoolu_def456".to_string(),
            name: "code_interpreter".to_string(),
            args: {
                let mut args = HashMap::new();
                args.insert("code".to_string(), json!("import numpy as np..."));
                args
            },
            index: None,
            extras: None,
        }),
        ContentBlock::ServerToolResult(ServerToolResult {
            block_type: "server_tool_result".to_string(),
            id: None,
            tool_call_id: "srvtoolu_def456".to_string(),
            status: ServerToolStatus::Success,
            output: Some(json!({
                "type": "code_execution_result",
                "return_code": 0,
                "stdout": "Mean: 5.5\nStandard deviation...",
                "stderr": ""
            })),
            index: None,
            extras: Some({
                let mut extras = HashMap::new();
                extras.insert(
                    "block_type".to_string(),
                    json!("code_execution_tool_result"),
                );
                extras
            }),
        }),
        ContentBlock::NonStandard(NonStandardContentBlock {
            block_type: "non_standard".to_string(),
            id: None,
            value: {
                let mut value = HashMap::new();
                value.insert("type".to_string(), json!("something_else"));
                value.insert("foo".to_string(), json!("bar"));
                value
            },
            index: None,
        }),
    ];

    let content_blocks = message.content_blocks();
    assert_eq!(content_blocks, expected_content);

    assert_eq!(message.content_list(), content);

    let mut response_metadata2 = HashMap::new();
    response_metadata2.insert("model_provider".to_string(), json!("anthropic"));

    let message2 = AIMessage::builder()
        .content("Hello")
        .response_metadata(response_metadata2)
        .build();

    let expected_content2 = vec![ContentBlock::Text(TextContentBlock::new("Hello"))];
    assert_eq!(message2.content_blocks(), expected_content2);
    assert_ne!(message2.content, "");
}


/// Test conversion of Anthropic AI message chunks to v1 format.
///
/// This test verifies that streaming chunks from Anthropic are correctly
/// translated to standard v1 content blocks, including proper handling
/// of tool_use chunks and input_json_delta.
#[test]
fn test_convert_to_v1_from_anthropic_chunk() {
    let mut response_metadata = HashMap::new();
    response_metadata.insert("model_provider".to_string(), json!("anthropic"));

    let chunks = vec![
        {
            let content = serde_json::to_string(&vec![
                json!({"text": "Looking ", "type": "text", "index": 0}),
            ])
            .unwrap_or_default();
            AIMessageChunk::builder()
                .content(content)
                .response_metadata(response_metadata.clone())
                .build()
        },
        {
            let content =
                serde_json::to_string(&vec![json!({"text": "now.", "type": "text", "index": 0})])
                    .unwrap_or_default();
            AIMessageChunk::builder()
                .content(content)
                .response_metadata(response_metadata.clone())
                .build()
        },
        {
            let content = serde_json::to_string(&vec![json!({
                "type": "tool_use",
                "name": "get_weather",
                "input": {},
                "id": "toolu_abc123",
                "index": 1
            })])
            .unwrap_or_default();
            AIMessageChunk::builder()
                .content(content)
                .tool_call_chunks(vec![tool_call_chunk(
                    Some("get_weather".to_string()),
                    Some("".to_string()),
                    Some("toolu_abc123".to_string()),
                    Some(1),
                )])
                .response_metadata(response_metadata.clone())
                .build()
        },
        {
            let content = serde_json::to_string(&vec![
                json!({"type": "input_json_delta", "partial_json": "", "index": 1}),
            ])
            .unwrap_or_default();
            AIMessageChunk::builder()
                .content(content)
                .tool_call_chunks(vec![tool_call_chunk(
                    None,
                    Some("".to_string()),
                    None,
                    Some(1),
                )])
                .response_metadata(response_metadata.clone())
                .build()
        },
        {
            let content = serde_json::to_string(&vec![
                json!({"type": "input_json_delta", "partial_json": r#"{"loca"#, "index": 1}),
            ])
            .unwrap_or_default();
            AIMessageChunk::builder()
                .content(content)
                .tool_call_chunks(vec![tool_call_chunk(
                    None,
                    Some(r#"{"loca"#.to_string()),
                    None,
                    Some(1),
                )])
                .response_metadata(response_metadata.clone())
                .build()
        },
        {
            let content = serde_json::to_string(&vec![
                json!({"type": "input_json_delta", "partial_json": r#"tion": "San "#, "index": 1}),
            ])
            .unwrap_or_default();
            AIMessageChunk::builder()
                .content(content)
                .tool_call_chunks(vec![tool_call_chunk(
                    None,
                    Some(r#"tion": "San "#.to_string()),
                    None,
                    Some(1),
                )])
                .response_metadata(response_metadata.clone())
                .build()
        },
        {
            let content = serde_json::to_string(&vec![
                json!({"type": "input_json_delta", "partial_json": r#"Francisco"}"#, "index": 1}),
            ])
            .unwrap_or_default();
            AIMessageChunk::builder()
                .content(content)
                .tool_call_chunks(vec![tool_call_chunk(
                    None,
                    Some(r#"Francisco"}"#.to_string()),
                    None,
                    Some(1),
                )])
                .response_metadata(response_metadata.clone())
                .build()
        },
    ];

    let expected_contents: Vec<ContentBlock> = vec![
        ContentBlock::Text(TextContentBlock {
            block_type: "text".to_string(),
            id: None,
            text: "Looking ".to_string(),
            annotations: None,
            index: Some(BlockIndex::Int(0)),
            extras: None,
        }),
        ContentBlock::Text(TextContentBlock {
            block_type: "text".to_string(),
            id: None,
            text: "now.".to_string(),
            annotations: None,
            index: Some(BlockIndex::Int(0)),
            extras: None,
        }),
        ContentBlock::ToolCallChunk(ToolCallChunkBlock {
            block_type: "tool_call_chunk".to_string(),
            id: Some("toolu_abc123".to_string()),
            name: Some("get_weather".to_string()),
            args: Some("".to_string()),
            index: Some(BlockIndex::Int(1)),
            extras: None,
        }),
        ContentBlock::ToolCallChunk(ToolCallChunkBlock {
            block_type: "tool_call_chunk".to_string(),
            id: None,
            name: None,
            args: Some("".to_string()),
            index: Some(BlockIndex::Int(1)),
            extras: None,
        }),
        ContentBlock::ToolCallChunk(ToolCallChunkBlock {
            block_type: "tool_call_chunk".to_string(),
            id: None,
            name: None,
            args: Some(r#"{"loca"#.to_string()),
            index: Some(BlockIndex::Int(1)),
            extras: None,
        }),
        ContentBlock::ToolCallChunk(ToolCallChunkBlock {
            block_type: "tool_call_chunk".to_string(),
            id: None,
            name: None,
            args: Some(r#"tion": "San "#.to_string()),
            index: Some(BlockIndex::Int(1)),
            extras: None,
        }),
        ContentBlock::ToolCallChunk(ToolCallChunkBlock {
            block_type: "tool_call_chunk".to_string(),
            id: None,
            name: None,
            args: Some(r#"Francisco"}"#.to_string()),
            index: Some(BlockIndex::Int(1)),
            extras: None,
        }),
    ];

    for (chunk, expected) in chunks.iter().zip(expected_contents.iter()) {
        assert_eq!(chunk.content_blocks(), vec![expected.clone()]);
    }

    let mut full: Option<AIMessageChunk> = None;
    for chunk in chunks {
        full = Some(match full {
            None => chunk,
            Some(f) => f + chunk,
        });
    }
    let full = full.unwrap();

    let expected_merged_content = vec![
        json!({"type": "text", "text": "Looking now.", "index": 0}),
        json!({
            "type": "tool_use",
            "name": "get_weather",
            "partial_json": r#"{"location": "San Francisco"}"#,
            "input": {},
            "id": "toolu_abc123",
            "index": 1
        }),
    ];
    assert_eq!(&full.content_list(), &expected_merged_content);

    let expected_merged_content_blocks = vec![
        ContentBlock::Text(TextContentBlock {
            block_type: "text".to_string(),
            id: None,
            text: "Looking now.".to_string(),
            annotations: None,
            index: Some(BlockIndex::Int(0)),
            extras: None,
        }),
        ContentBlock::ToolCallChunk(ToolCallChunkBlock {
            block_type: "tool_call_chunk".to_string(),
            id: Some("toolu_abc123".to_string()),
            name: Some("get_weather".to_string()),
            args: Some(r#"{"location": "San Francisco"}"#.to_string()),
            index: Some(BlockIndex::Int(1)),
            extras: None,
        }),
    ];
    assert_eq!(full.content_blocks(), expected_merged_content_blocks);

    let content = serde_json::to_string(&vec![
        json!({
            "id": "srvtoolu_abc123",
            "input": {},
            "name": "web_fetch",
            "type": "server_tool_use",
            "index": 0,
            "partial_json": r#"{"url": "https://docs.langchain.com"}"#
        }),
        json!({
            "id": "mcptoolu_abc123",
            "input": {},
            "name": "ask_question",
            "server_name": "<my server name>",
            "type": "mcp_tool_use",
            "index": 1,
            "partial_json": r#"{"repoName": "<my repo>", "question": "<my query>"}"#
        }),
    ])
    .unwrap_or_default();
    let full_server = AIMessageChunk::builder()
        .content(content)
        .response_metadata(response_metadata.clone())
        .chunk_position(ChunkPosition::Last)
        .build();

    let expected_server_content_blocks = vec![
        ContentBlock::ServerToolCall(ServerToolCall {
            block_type: "server_tool_call".to_string(),
            id: "srvtoolu_abc123".to_string(),
            name: "web_fetch".to_string(),
            args: {
                let mut args = HashMap::new();
                args.insert("url".to_string(), json!("https://docs.langchain.com"));
                args
            },
            index: Some(BlockIndex::Int(0)),
            extras: None,
        }),
        ContentBlock::ServerToolCall(ServerToolCall {
            block_type: "server_tool_call".to_string(),
            id: "mcptoolu_abc123".to_string(),
            name: "remote_mcp".to_string(),
            args: {
                let mut args = HashMap::new();
                args.insert("repoName".to_string(), json!("<my repo>"));
                args.insert("question".to_string(), json!("<my query>"));
                args
            },
            index: Some(BlockIndex::Int(1)),
            extras: Some({
                let mut extras = HashMap::new();
                extras.insert("tool_name".to_string(), json!("ask_question"));
                extras.insert("server_name".to_string(), json!("<my server name>"));
                extras
            }),
        }),
    ];
    assert_eq!(full_server.content_blocks(), expected_server_content_blocks);
}


/// Test conversion of Anthropic input content (HumanMessage) to v1 format.
///
/// This test verifies that Anthropic-specific input content blocks
/// (document, image with various source types) are correctly translated
/// to standard v1 content blocks.
#[test]
fn test_convert_to_v1_from_anthropic_input() {
    let content = vec![
        json!({"type": "text", "text": "foo"}),
        json!({
            "type": "document",
            "source": {
                "type": "base64",
                "data": "<base64 data>",
                "media_type": "application/pdf"
            }
        }),
        json!({
            "type": "document",
            "source": {
                "type": "url",
                "url": "<document url>"
            }
        }),
        json!({
            "type": "document",
            "source": {
                "type": "content",
                "content": [
                    {"type": "text", "text": "The grass is green"},
                    {"type": "text", "text": "The sky is blue"}
                ]
            },
            "citations": {"enabled": true}
        }),
        json!({
            "type": "document",
            "source": {
                "type": "text",
                "data": "<plain text data>",
                "media_type": "text/plain"
            }
        }),
        json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": "image/jpeg",
                "data": "<base64 image data>"
            }
        }),
        json!({
            "type": "image",
            "source": {
                "type": "url",
                "url": "<image url>"
            }
        }),
        json!({
            "type": "image",
            "source": {
                "type": "file",
                "file_id": "<image file id>"
            }
        }),
        json!({
            "type": "document",
            "source": {"type": "file", "file_id": "<pdf file id>"}
        }),
    ];

    let message = HumanMessage::builder()
        .content(MessageContent::Parts(
            content
                .iter()
                .map(|v| serde_json::from_value(v.clone()).unwrap())
                .collect(),
        ))
        .build();

    let expected: Vec<ContentBlock> = vec![
        ContentBlock::Text(TextContentBlock::new("foo")),
        ContentBlock::File(FileContentBlock {
            block_type: "file".to_string(),
            id: None,
            file_id: None,
            mime_type: Some("application/pdf".to_string()),
            index: None,
            url: None,
            base64: Some("<base64 data>".to_string()),
            extras: None,
        }),
        ContentBlock::File(FileContentBlock {
            block_type: "file".to_string(),
            id: None,
            file_id: None,
            mime_type: None,
            index: None,
            url: Some("<document url>".to_string()),
            base64: None,
            extras: None,
        }),
        ContentBlock::NonStandard(NonStandardContentBlock {
            block_type: "non_standard".to_string(),
            id: None,
            value: {
                let mut value = HashMap::new();
                value.insert("type".to_string(), json!("document"));
                value.insert(
                    "source".to_string(),
                    json!({
                        "type": "content",
                        "content": [
                            {"type": "text", "text": "The grass is green"},
                            {"type": "text", "text": "The sky is blue"}
                        ]
                    }),
                );
                value.insert("citations".to_string(), json!({"enabled": true}));
                value
            },
            index: None,
        }),
        ContentBlock::PlainText(PlainTextContentBlock {
            block_type: "text-plain".to_string(),
            id: None,
            file_id: None,
            mime_type: "text/plain".to_string(),
            index: None,
            url: None,
            base64: None,
            text: Some("<plain text data>".to_string()),
            title: None,
            context: None,
            extras: None,
        }),
        ContentBlock::Image(ImageContentBlock {
            block_type: "image".to_string(),
            id: None,
            file_id: None,
            mime_type: Some("image/jpeg".to_string()),
            index: None,
            url: None,
            base64: Some("<base64 image data>".to_string()),
            extras: None,
        }),
        ContentBlock::Image(ImageContentBlock {
            block_type: "image".to_string(),
            id: None,
            file_id: None,
            mime_type: None,
            index: None,
            url: Some("<image url>".to_string()),
            base64: None,
            extras: None,
        }),
        ContentBlock::Image(ImageContentBlock {
            block_type: "image".to_string(),
            id: Some("<image file id>".to_string()),
            file_id: None,
            mime_type: None,
            index: None,
            url: None,
            base64: None,
            extras: None,
        }),
        ContentBlock::File(FileContentBlock {
            block_type: "file".to_string(),
            id: Some("<pdf file id>".to_string()),
            file_id: None,
            mime_type: None,
            index: None,
            url: None,
            base64: None,
            extras: None,
        }),
    ];

    assert_eq!(message.content_blocks(), expected);
}
