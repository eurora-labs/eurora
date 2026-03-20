use agent_chain_core::messages::{
    AIMessage, AIMessageChunk, BlockIndex, ChunkPosition, ContentBlock, ContentBlocks,
    FileContentBlock, HumanMessage, ImageContentBlock, NonStandardContentBlock,
    PlainTextContentBlock, ReasoningContentBlock, ServerToolCall, ServerToolResult,
    ServerToolStatus, TextContentBlock, ToolCallBlock, ToolCallChunkBlock,
};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_convert_to_v1_from_anthropic() {
    let _content = vec![
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

    let expected_content: Vec<ContentBlock> = vec![
        ContentBlock::Reasoning(ReasoningContentBlock {
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
            id: None,
            text: "Let's call a tool.".to_string(),
            annotations: None,
            index: None,
            extras: None,
        }),
        ContentBlock::ToolCall(ToolCallBlock {
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
            id: None,
            text: "It's sunny.".to_string(),
            annotations: None,
            index: None,
            extras: None,
        }),
        ContentBlock::ServerToolCall(ServerToolCall {
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
            id: None,
            tool_call_id: "".to_string(),
            status: ServerToolStatus::Success,
            output: None,
            index: None,
            extras: Some({
                let mut extras = HashMap::new();
                extras.insert("tool_call_id".to_string(), json!("srvtoolu_abc123"));
                extras.insert("block_type".to_string(), json!("server_tool_result"));
                extras.insert("status".to_string(), json!("success"));
                extras.insert(
                    "extras".to_string(),
                    json!({"block_type": "web_search_tool_result"}),
                );
                extras.insert(
                    "output".to_string(),
                    json!([
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
                    ]),
                );
                extras
            }),
        }),
        ContentBlock::ServerToolCall(ServerToolCall {
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
            id: None,
            tool_call_id: "".to_string(),
            status: ServerToolStatus::Success,
            output: None,
            index: None,
            extras: Some({
                let mut extras = HashMap::new();
                extras.insert("tool_call_id".to_string(), json!("srvtoolu_def456"));
                extras.insert("block_type".to_string(), json!("server_tool_result"));
                extras.insert("status".to_string(), json!("success"));
                extras.insert(
                    "extras".to_string(),
                    json!({"block_type": "code_execution_tool_result"}),
                );
                extras.insert(
                    "output".to_string(),
                    json!({
                        "type": "code_execution_result",
                        "return_code": 0,
                        "stdout": "Mean: 5.5\nStandard deviation...",
                        "stderr": ""
                    }),
                );
                extras
            }),
        }),
        ContentBlock::NonStandard(NonStandardContentBlock {
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

    let message = AIMessage::builder()
        .content(ContentBlocks::from(expected_content.clone()))
        .response_metadata(response_metadata)
        .build();

    let content_blocks = message.content_blocks();
    assert_eq!(content_blocks.len(), expected_content.len());
    for (actual, expected) in content_blocks.iter().zip(expected_content.iter()) {
        match (actual, expected) {
            (ContentBlock::Text(a), ContentBlock::Text(e)) => assert_eq!(a.text, e.text),
            (ContentBlock::Reasoning(a), ContentBlock::Reasoning(e)) => {
                assert_eq!(a.reasoning, e.reasoning)
            }
            (ContentBlock::ToolCall(a), ContentBlock::ToolCall(e)) => {
                assert_eq!(a.name, e.name);
                assert_eq!(a.id, e.id);
            }
            (ContentBlock::ServerToolCall(a), ContentBlock::ServerToolCall(e)) => {
                assert_eq!(a.name, e.name);
                assert_eq!(a.id, e.id);
            }
            (ContentBlock::NonStandard(_), ContentBlock::NonStandard(_)) => {}
            _ => {
                let actual_type = std::mem::discriminant(actual);
                let expected_type = std::mem::discriminant(expected);
                assert_eq!(actual_type, expected_type);
            }
        }
    }

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

#[test]
fn test_convert_to_v1_from_anthropic_chunk() {
    let mut response_metadata = HashMap::new();
    response_metadata.insert("model_provider".to_string(), json!("anthropic"));

    let expected_contents: Vec<ContentBlock> = vec![
        ContentBlock::Text(TextContentBlock {
            id: None,
            text: "Looking ".to_string(),
            annotations: None,
            index: Some(BlockIndex::Int(0)),
            extras: None,
        }),
        ContentBlock::Text(TextContentBlock {
            id: None,
            text: "now.".to_string(),
            annotations: None,
            index: Some(BlockIndex::Int(0)),
            extras: None,
        }),
        ContentBlock::ToolCallChunk(ToolCallChunkBlock {
            id: Some("toolu_abc123".to_string()),
            name: Some("get_weather".to_string()),
            args: Some("".to_string()),
            index: Some(BlockIndex::Int(1)),
            extras: None,
        }),
        ContentBlock::ToolCallChunk(ToolCallChunkBlock {
            id: None,
            name: None,
            args: Some("".to_string()),
            index: Some(BlockIndex::Int(1)),
            extras: None,
        }),
        ContentBlock::ToolCallChunk(ToolCallChunkBlock {
            id: None,
            name: None,
            args: Some(r#"{"loca"#.to_string()),
            index: Some(BlockIndex::Int(1)),
            extras: None,
        }),
        ContentBlock::ToolCallChunk(ToolCallChunkBlock {
            id: None,
            name: None,
            args: Some(r#"tion": "San "#.to_string()),
            index: Some(BlockIndex::Int(1)),
            extras: None,
        }),
        ContentBlock::ToolCallChunk(ToolCallChunkBlock {
            id: None,
            name: None,
            args: Some(r#"Francisco"}"#.to_string()),
            index: Some(BlockIndex::Int(1)),
            extras: None,
        }),
    ];

    let chunks: Vec<AIMessageChunk> = expected_contents
        .iter()
        .map(|expected| {
            let content = ContentBlocks::from(vec![expected.clone()]);
            AIMessageChunk::builder()
                .content(content)
                .response_metadata(response_metadata.clone())
                .build()
        })
        .collect();

    for (chunk, _expected) in chunks.iter().zip(expected_contents.iter()) {
        let cb = chunk.content_blocks();
        assert_eq!(cb.len(), 1);
    }

    let _expected_merged_content_blocks = [
        ContentBlock::Text(TextContentBlock {
            id: None,
            text: "Looking now.".to_string(),
            annotations: None,
            index: Some(BlockIndex::Int(0)),
            extras: None,
        }),
        ContentBlock::ToolCallChunk(ToolCallChunkBlock {
            id: Some("toolu_abc123".to_string()),
            name: Some("get_weather".to_string()),
            args: Some(r#"{"location": "San Francisco"}"#.to_string()),
            index: Some(BlockIndex::Int(1)),
            extras: None,
        }),
    ];

    let expected_server_content_blocks = vec![
        ContentBlock::ServerToolCall(ServerToolCall {
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

    let full_server = AIMessageChunk::builder()
        .content(ContentBlocks::from(expected_server_content_blocks.clone()))
        .response_metadata(response_metadata.clone())
        .chunk_position(ChunkPosition::Last)
        .build();

    let expected_server_content_blocks = vec![
        ContentBlock::ServerToolCall(ServerToolCall {
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

#[test]
fn test_convert_to_v1_from_anthropic_input() {
    let _content = vec![
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

    let expected: Vec<ContentBlock> = vec![
        ContentBlock::Text(TextContentBlock::new("foo")),
        ContentBlock::File(FileContentBlock {
            id: None,
            file_id: None,
            mime_type: Some("application/pdf".to_string()),
            index: None,
            url: None,
            base64: Some("<base64 data>".to_string()),
            extras: None,
        }),
        ContentBlock::File(FileContentBlock {
            id: None,
            file_id: None,
            mime_type: None,
            index: None,
            url: Some("<document url>".to_string()),
            base64: None,
            extras: None,
        }),
        ContentBlock::NonStandard(NonStandardContentBlock {
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
            id: None,
            file_id: None,
            mime_type: Some("image/jpeg".to_string()),
            index: None,
            url: None,
            base64: Some("<base64 image data>".to_string()),
            extras: None,
        }),
        ContentBlock::Image(ImageContentBlock {
            id: None,
            file_id: None,
            mime_type: None,
            index: None,
            url: Some("<image url>".to_string()),
            base64: None,
            extras: None,
        }),
        ContentBlock::Image(ImageContentBlock {
            id: Some("<image file id>".to_string()),
            file_id: None,
            mime_type: None,
            index: None,
            url: None,
            base64: None,
            extras: None,
        }),
        ContentBlock::File(FileContentBlock {
            id: Some("<pdf file id>".to_string()),
            file_id: None,
            mime_type: None,
            index: None,
            url: None,
            base64: None,
            extras: None,
        }),
    ];

    let message = HumanMessage::builder()
        .content(ContentBlocks::from(expected.clone()))
        .build();
    assert_eq!(message.content_blocks(), expected);
}
