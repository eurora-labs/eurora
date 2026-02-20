use agent_chain_core::messages::block_translators::openai::{
    OpenAiApi, convert_to_openai_data_block,
};
use agent_chain_core::messages::{
    AIMessage, AIMessageChunk, Annotation, AudioContentBlock, BlockIndex, ContentBlock,
    FileContentBlock, HumanMessage, ImageContentBlock, MessageContent, NonStandardContentBlock,
    ReasoningContentBlock, ServerToolCall, ServerToolResult, ServerToolStatus, TextContentBlock,
    ToolCallBlock, tool_call, tool_call_chunk,
};
use serde_json::json;
use std::collections::HashMap;

fn content_blocks_equal_ignore_id(actual: &[ContentBlock], expected: &[ContentBlock]) -> bool {
    if actual.len() != expected.len() {
        return false;
    }

    for (actual_block, expected_block) in actual.iter().zip(expected.iter()) {
        let actual_without_id = remove_id_from_block(actual_block);
        let expected_without_id = remove_id_from_block(expected_block);

        if actual_without_id != expected_without_id {
            return false;
        }
    }

    true
}

fn remove_id_from_block(block: &ContentBlock) -> ContentBlock {
    match block {
        ContentBlock::Text(text_block) => ContentBlock::Text(TextContentBlock {
            id: None,
            ..text_block.clone()
        }),
        ContentBlock::Image(image_block) => ContentBlock::Image(ImageContentBlock {
            id: None,
            ..image_block.clone()
        }),
        ContentBlock::Audio(audio_block) => ContentBlock::Audio(AudioContentBlock {
            id: None,
            ..audio_block.clone()
        }),
        ContentBlock::File(file_block) => ContentBlock::File(FileContentBlock {
            id: None,
            ..file_block.clone()
        }),
        ContentBlock::Reasoning(reasoning_block) => {
            ContentBlock::Reasoning(ReasoningContentBlock {
                id: None,
                ..reasoning_block.clone()
            })
        }
        ContentBlock::ToolCall(tool_call_block) => ContentBlock::ToolCall(ToolCallBlock {
            id: None,
            ..tool_call_block.clone()
        }),
        ContentBlock::ServerToolCall(server_tool_call) => {
            ContentBlock::ServerToolCall(ServerToolCall {
                id: server_tool_call.id.clone(), // Keep server tool call id as it's part of the data
                ..server_tool_call.clone()
            })
        }
        ContentBlock::ServerToolResult(server_tool_result) => {
            ContentBlock::ServerToolResult(ServerToolResult {
                id: None,
                ..server_tool_result.clone()
            })
        }
        ContentBlock::NonStandard(non_standard) => {
            ContentBlock::NonStandard(NonStandardContentBlock {
                id: None,
                ..non_standard.clone()
            })
        }
        other => other.clone(),
    }
}

#[test]
fn test_convert_to_v1_from_responses() {
    let content = vec![
        json!({"type": "reasoning", "id": "abc123", "summary": []}),
        json!({
            "type": "reasoning",
            "id": "abc234",
            "summary": [
                {"type": "summary_text", "text": "foo bar"},
                {"type": "summary_text", "text": "baz"}
            ]
        }),
        json!({
            "type": "function_call",
            "call_id": "call_123",
            "name": "get_weather",
            "arguments": r#"{"location": "San Francisco"}"#
        }),
        json!({
            "type": "function_call",
            "call_id": "call_234",
            "name": "get_weather_2",
            "arguments": r#"{"location": "New York"}"#,
            "id": "fc_123"
        }),
        json!({"type": "text", "text": "Hello "}),
        json!({
            "type": "text",
            "text": "world",
            "annotations": [
                {"type": "url_citation", "url": "https://example.com"},
                {
                    "type": "file_citation",
                    "filename": "my doc",
                    "index": 1,
                    "file_id": "file_123"
                },
                {"bar": "baz"}
            ]
        }),
        json!({"type": "image_generation_call", "id": "ig_123", "result": "..."}),
        json!({
            "type": "file_search_call",
            "id": "fs_123",
            "queries": ["query for file search"],
            "results": [{"file_id": "file-123"}],
            "status": "completed"
        }),
        json!({"type": "something_else", "foo": "bar"}),
    ];

    let mut response_metadata = HashMap::new();
    response_metadata.insert("model_provider".to_string(), json!("openai"));

    let tool_calls = vec![
        tool_call(
            "get_weather".to_string(),
            json!({"location": "San Francisco"}),
            Some("call_123".to_string()),
        ),
        tool_call(
            "get_weather_2".to_string(),
            json!({"location": "New York"}),
            Some("call_234".to_string()),
        ),
    ];

    let content_str = serde_json::to_string(&content).unwrap_or_default();
    let message = AIMessage::builder()
        .content(content_str)
        .response_metadata(response_metadata)
        .tool_calls(tool_calls)
        .build();

    let expected_content: Vec<ContentBlock> = vec![
        ContentBlock::Reasoning(ReasoningContentBlock {
            block_type: "reasoning".to_string(),
            id: Some("abc123".to_string()),
            reasoning: None,
            index: None,
            extras: None,
        }),
        ContentBlock::Reasoning(ReasoningContentBlock {
            block_type: "reasoning".to_string(),
            id: Some("abc234".to_string()),
            reasoning: Some("foo bar".to_string()),
            index: None,
            extras: None,
        }),
        ContentBlock::Reasoning(ReasoningContentBlock {
            block_type: "reasoning".to_string(),
            id: Some("abc234".to_string()),
            reasoning: Some("baz".to_string()),
            index: None,
            extras: None,
        }),
        ContentBlock::ToolCall(ToolCallBlock {
            block_type: "tool_call".to_string(),
            id: Some("call_123".to_string()),
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
            id: Some("call_234".to_string()),
            name: "get_weather_2".to_string(),
            args: {
                let mut args = HashMap::new();
                args.insert("location".to_string(), json!("New York"));
                args
            },
            index: None,
            extras: Some({
                let mut extras = HashMap::new();
                extras.insert("item_id".to_string(), json!("fc_123"));
                extras
            }),
        }),
        ContentBlock::Text(TextContentBlock {
            block_type: "text".to_string(),
            id: None,
            text: "Hello ".to_string(),
            annotations: None,
            index: None,
            extras: None,
        }),
        ContentBlock::Text(TextContentBlock {
            block_type: "text".to_string(),
            id: None,
            text: "world".to_string(),
            annotations: Some(vec![
                Annotation::Citation {
                    id: None,
                    url: Some("https://example.com".to_string()),
                    title: None,
                    start_index: None,
                    end_index: None,
                    cited_text: None,
                    extras: None,
                },
                Annotation::Citation {
                    id: None,
                    url: None,
                    title: Some("my doc".to_string()),
                    start_index: None,
                    end_index: None,
                    cited_text: None,
                    extras: Some({
                        let mut extras = HashMap::new();
                        extras.insert("file_id".to_string(), json!("file_123"));
                        extras.insert("index".to_string(), json!(1));
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
        ContentBlock::Image(ImageContentBlock {
            block_type: "image".to_string(),
            id: Some("ig_123".to_string()),
            file_id: None,
            mime_type: None,
            index: None,
            url: None,
            base64: Some("...".to_string()),
            extras: None,
        }),
        ContentBlock::ServerToolCall(ServerToolCall {
            block_type: "server_tool_call".to_string(),
            id: "fs_123".to_string(),
            name: "file_search".to_string(),
            args: {
                let mut args = HashMap::new();
                args.insert("queries".to_string(), json!(["query for file search"]));
                args
            },
            index: None,
            extras: None,
        }),
        ContentBlock::ServerToolResult(ServerToolResult {
            block_type: "server_tool_result".to_string(),
            id: None,
            tool_call_id: "fs_123".to_string(),
            status: ServerToolStatus::Success,
            output: Some(json!([{"file_id": "file-123"}])),
            index: None,
            extras: None,
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

    assert_ne!(
        message.content_list(),
        expected_content
            .iter()
            .map(|b| json!(b))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_convert_to_v1_from_responses_chunk() {
    let mut response_metadata = HashMap::new();
    response_metadata.insert("model_provider".to_string(), json!("openai"));

    let chunks = vec![
        {
            let content = serde_json::to_string(&vec![
                json!({"type": "reasoning", "id": "abc123", "summary": [], "index": 0}),
            ])
            .unwrap_or_default();
            AIMessageChunk::builder()
                .content(content)
                .response_metadata(response_metadata.clone())
                .build()
        },
        {
            let content = serde_json::to_string(&vec![json!({
                "type": "reasoning",
                "id": "abc234",
                "summary": [
                    {"type": "summary_text", "text": "foo ", "index": 0}
                ],
                "index": 1
            })])
            .unwrap_or_default();
            AIMessageChunk::builder()
                .content(content)
                .response_metadata(response_metadata.clone())
                .build()
        },
        {
            let content = serde_json::to_string(&vec![json!({
                "type": "reasoning",
                "id": "abc234",
                "summary": [
                    {"type": "summary_text", "text": "bar", "index": 0}
                ],
                "index": 1
            })])
            .unwrap_or_default();
            AIMessageChunk::builder()
                .content(content)
                .response_metadata(response_metadata.clone())
                .build()
        },
        {
            let content = serde_json::to_string(&vec![json!({
                "type": "reasoning",
                "id": "abc234",
                "summary": [
                    {"type": "summary_text", "text": "baz", "index": 1}
                ],
                "index": 1
            })])
            .unwrap_or_default();
            AIMessageChunk::builder()
                .content(content)
                .response_metadata(response_metadata.clone())
                .build()
        },
    ];

    let expected_chunks = [
        {
            let content = serde_json::to_string(&vec![
                json!({"type": "reasoning", "id": "abc123", "index": "lc_rs_305f30"}),
            ])
            .unwrap_or_default();
            AIMessageChunk::builder()
                .content(content)
                .response_metadata(response_metadata.clone())
                .build()
        },
        {
            let content = serde_json::to_string(&vec![json!({
                "type": "reasoning",
                "id": "abc234",
                "reasoning": "foo ",
                "index": "lc_rs_315f30"
            })])
            .unwrap_or_default();
            AIMessageChunk::builder()
                .content(content)
                .response_metadata(response_metadata.clone())
                .build()
        },
        {
            let content = serde_json::to_string(&vec![json!({
                "type": "reasoning",
                "id": "abc234",
                "reasoning": "bar",
                "index": "lc_rs_315f30"
            })])
            .unwrap_or_default();
            AIMessageChunk::builder()
                .content(content)
                .response_metadata(response_metadata.clone())
                .build()
        },
        {
            let content = serde_json::to_string(&vec![json!({
                "type": "reasoning",
                "id": "abc234",
                "reasoning": "baz",
                "index": "lc_rs_315f31"
            })])
            .unwrap_or_default();
            AIMessageChunk::builder()
                .content(content)
                .response_metadata(response_metadata.clone())
                .build()
        },
    ];

    for (chunk, expected) in chunks.iter().zip(expected_chunks.iter()) {
        assert_eq!(chunk.content_blocks(), expected.content_blocks());
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
        json!({"type": "reasoning", "id": "abc123", "summary": [], "index": 0}),
        json!({
            "type": "reasoning",
            "id": "abc234",
            "summary": [
                {"type": "summary_text", "text": "foo bar", "index": 0},
                {"type": "summary_text", "text": "baz", "index": 1}
            ],
            "index": 1
        }),
    ];
    assert_eq!(full.content_list(), expected_merged_content);

    let expected_merged_content_blocks = vec![
        ContentBlock::Reasoning(ReasoningContentBlock {
            block_type: "reasoning".to_string(),
            id: Some("abc123".to_string()),
            reasoning: None,
            index: Some(BlockIndex::Str("lc_rs_305f30".to_string())),
            extras: None,
        }),
        ContentBlock::Reasoning(ReasoningContentBlock {
            block_type: "reasoning".to_string(),
            id: Some("abc234".to_string()),
            reasoning: Some("foo bar".to_string()),
            index: Some(BlockIndex::Str("lc_rs_315f30".to_string())),
            extras: None,
        }),
        ContentBlock::Reasoning(ReasoningContentBlock {
            block_type: "reasoning".to_string(),
            id: Some("abc234".to_string()),
            reasoning: Some("baz".to_string()),
            index: Some(BlockIndex::Str("lc_rs_315f31".to_string())),
            extras: None,
        }),
    ];
    assert_eq!(full.content_blocks(), expected_merged_content_blocks);
}

#[test]
fn test_convert_to_v1_from_openai_input() {
    let content = [
        json!({"type": "text", "text": "Hello"}),
        json!({
            "type": "image_url",
            "image_url": {"url": "https://example.com/image.png"}
        }),
        json!({
            "type": "image_url",
            "image_url": {"url": "data:image/jpeg;base64,/9j/4AAQSkZJRg..."}
        }),
        json!({
            "type": "input_audio",
            "input_audio": {
                "format": "wav",
                "data": "<base64 string>"
            }
        }),
        json!({
            "type": "file",
            "file": {
                "filename": "draconomicon.pdf",
                "file_data": "data:application/pdf;base64,<base64 string>"
            }
        }),
        json!({
            "type": "file",
            "file": {"file_id": "<file id>"}
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
        ContentBlock::Text(TextContentBlock::new("Hello")),
        ContentBlock::Image(ImageContentBlock {
            block_type: "image".to_string(),
            id: None,
            file_id: None,
            mime_type: None,
            index: None,
            url: Some("https://example.com/image.png".to_string()),
            base64: None,
            extras: None,
        }),
        ContentBlock::Image(ImageContentBlock {
            block_type: "image".to_string(),
            id: None,
            file_id: None,
            mime_type: Some("image/jpeg".to_string()),
            index: None,
            url: None,
            base64: Some("/9j/4AAQSkZJRg...".to_string()),
            extras: None,
        }),
        ContentBlock::Audio(AudioContentBlock {
            block_type: "audio".to_string(),
            id: None,
            file_id: None,
            mime_type: Some("audio/wav".to_string()),
            index: None,
            url: None,
            base64: Some("<base64 string>".to_string()),
            extras: None,
        }),
        ContentBlock::File(FileContentBlock {
            block_type: "file".to_string(),
            id: None,
            file_id: None,
            mime_type: Some("application/pdf".to_string()),
            index: None,
            url: None,
            base64: Some("<base64 string>".to_string()),
            extras: Some({
                let mut extras = HashMap::new();
                extras.insert("filename".to_string(), json!("draconomicon.pdf"));
                extras
            }),
        }),
        ContentBlock::File(FileContentBlock {
            block_type: "file".to_string(),
            id: None,
            file_id: Some("<file id>".to_string()),
            mime_type: None,
            index: None,
            url: None,
            base64: None,
            extras: None,
        }),
    ];

    assert!(content_blocks_equal_ignore_id(
        &message.content_blocks(),
        &expected
    ));
}

#[test]
fn test_compat_responses_v03() {
    let content = vec![json!({
        "type": "text",
        "text": "Hello, world!",
        "annotations": [{"type": "foo"}]
    })];

    let mut additional_kwargs = HashMap::new();
    additional_kwargs.insert(
        "reasoning".to_string(),
        json!({
            "type": "reasoning",
            "id": "rs_123",
            "summary": [
                {"type": "summary_text", "text": "summary 1"},
                {"type": "summary_text", "text": "summary 2"}
            ]
        }),
    );
    additional_kwargs.insert(
        "tool_outputs".to_string(),
        json!([{
            "type": "web_search_call",
            "id": "websearch_123",
            "status": "completed"
        }]),
    );
    additional_kwargs.insert("refusal".to_string(), json!("I cannot assist with that."));
    additional_kwargs.insert(
        "__openai_function_call_ids__".to_string(),
        json!({"call_abc": "fc_abc"}),
    );

    let mut response_metadata = HashMap::new();
    response_metadata.insert("id".to_string(), json!("resp_123"));
    response_metadata.insert("model_provider".to_string(), json!("openai"));

    let tool_calls = vec![tool_call(
        "my_tool".to_string(),
        json!({"x": 3}),
        Some("call_abc".to_string()),
    )];

    let content_str = serde_json::to_string(&content).unwrap_or_default();
    let message = AIMessage::builder()
        .content(content_str)
        .additional_kwargs(additional_kwargs)
        .response_metadata(response_metadata)
        .tool_calls(tool_calls)
        .id("msg_123".to_string())
        .build();

    let expected_content: Vec<ContentBlock> = vec![
        ContentBlock::Reasoning(ReasoningContentBlock {
            block_type: "reasoning".to_string(),
            id: Some("rs_123".to_string()),
            reasoning: Some("summary 1".to_string()),
            index: None,
            extras: None,
        }),
        ContentBlock::Reasoning(ReasoningContentBlock {
            block_type: "reasoning".to_string(),
            id: Some("rs_123".to_string()),
            reasoning: Some("summary 2".to_string()),
            index: None,
            extras: None,
        }),
        ContentBlock::Text(TextContentBlock {
            block_type: "text".to_string(),
            id: Some("msg_123".to_string()),
            text: "Hello, world!".to_string(),
            annotations: Some(vec![Annotation::NonStandardAnnotation {
                id: None,
                value: {
                    let mut value = HashMap::new();
                    value.insert("type".to_string(), json!("foo"));
                    value
                },
            }]),
            index: None,
            extras: None,
        }),
        ContentBlock::NonStandard(NonStandardContentBlock {
            block_type: "non_standard".to_string(),
            id: None,
            value: {
                let mut value = HashMap::new();
                value.insert("type".to_string(), json!("refusal"));
                value.insert("refusal".to_string(), json!("I cannot assist with that."));
                value
            },
            index: None,
        }),
        ContentBlock::ToolCall(ToolCallBlock {
            block_type: "tool_call".to_string(),
            id: Some("call_abc".to_string()),
            name: "my_tool".to_string(),
            args: {
                let mut args = HashMap::new();
                args.insert("x".to_string(), json!(3));
                args
            },
            index: None,
            extras: Some({
                let mut extras = HashMap::new();
                extras.insert("item_id".to_string(), json!("fc_abc"));
                extras
            }),
        }),
        ContentBlock::ServerToolCall(ServerToolCall {
            block_type: "server_tool_call".to_string(),
            id: "websearch_123".to_string(),
            name: "web_search".to_string(),
            args: HashMap::new(),
            index: None,
            extras: None,
        }),
        ContentBlock::ServerToolResult(ServerToolResult {
            block_type: "server_tool_result".to_string(),
            id: None,
            tool_call_id: "websearch_123".to_string(),
            status: ServerToolStatus::Success,
            output: None,
            index: None,
            extras: None,
        }),
    ];

    assert_eq!(message.content_blocks(), expected_content);

    let mut additional_kwargs_chunk1 = HashMap::new();
    additional_kwargs_chunk1.insert(
        "__openai_function_call_ids__".to_string(),
        json!({"call_abc": "fc_abc"}),
    );

    let mut response_metadata_chunk = HashMap::new();
    response_metadata_chunk.insert("model_provider".to_string(), json!("openai"));

    let chunk_1 = AIMessageChunk::builder()
        .content("[]")
        .additional_kwargs(additional_kwargs_chunk1)
        .tool_call_chunks(vec![tool_call_chunk(
            Some("my_tool".to_string()),
            Some("".to_string()),
            Some("call_abc".to_string()),
            Some(0),
        )])
        .response_metadata(response_metadata_chunk.clone())
        .build();

    let expected_chunk1_content = vec![ContentBlock::ToolCallChunk(
        agent_chain_core::messages::ToolCallChunkBlock {
            block_type: "tool_call_chunk".to_string(),
            id: Some("call_abc".to_string()),
            name: Some("my_tool".to_string()),
            args: Some("".to_string()),
            index: Some(BlockIndex::Int(0)),
            extras: Some({
                let mut extras = HashMap::new();
                extras.insert("item_id".to_string(), json!("fc_abc"));
                extras
            }),
        },
    )];
    assert_eq!(chunk_1.content_blocks(), expected_chunk1_content);

    let mut additional_kwargs_chunk2 = HashMap::new();
    additional_kwargs_chunk2.insert("__openai_function_call_ids__".to_string(), json!({}));

    let chunk_2 = AIMessageChunk::builder()
        .content("[]")
        .additional_kwargs(additional_kwargs_chunk2)
        .tool_call_chunks(vec![tool_call_chunk(
            None,
            Some("{".to_string()),
            None,
            Some(0),
        )])
        .build();

    let expected_chunk2_content = vec![ContentBlock::ToolCallChunk(
        agent_chain_core::messages::ToolCallChunkBlock {
            block_type: "tool_call_chunk".to_string(),
            id: None,
            name: None,
            args: Some("{".to_string()),
            index: Some(BlockIndex::Int(0)),
            extras: None,
        },
    )];
    assert_eq!(chunk_2.content_blocks(), expected_chunk2_content);

    let merged_chunk = chunk_1 + chunk_2;
    let expected_merged_content = vec![ContentBlock::ToolCallChunk(
        agent_chain_core::messages::ToolCallChunkBlock {
            block_type: "tool_call_chunk".to_string(),
            id: Some("call_abc".to_string()),
            name: Some("my_tool".to_string()),
            args: Some("{".to_string()),
            index: Some(BlockIndex::Int(0)),
            extras: Some({
                let mut extras = HashMap::new();
                extras.insert("item_id".to_string(), json!("fc_abc"));
                extras
            }),
        },
    )];
    assert_eq!(merged_chunk.content_blocks(), expected_merged_content);

    let mut additional_kwargs_reasoning1 = HashMap::new();
    additional_kwargs_reasoning1.insert(
        "reasoning".to_string(),
        json!({"id": "rs_abc", "summary": [], "type": "reasoning"}),
    );

    let reasoning_chunk_1 = AIMessageChunk::builder()
        .content("[]")
        .additional_kwargs(additional_kwargs_reasoning1)
        .response_metadata(response_metadata_chunk.clone())
        .build();

    let expected_reasoning1_content = vec![ContentBlock::Reasoning(ReasoningContentBlock {
        block_type: "reasoning".to_string(),
        id: Some("rs_abc".to_string()),
        reasoning: None,
        index: None,
        extras: None,
    })];
    assert_eq!(
        reasoning_chunk_1.content_blocks(),
        expected_reasoning1_content
    );

    let mut additional_kwargs_reasoning2 = HashMap::new();
    additional_kwargs_reasoning2.insert(
        "reasoning".to_string(),
        json!({
            "summary": [
                {"index": 0, "type": "summary_text", "text": "reasoning text"}
            ]
        }),
    );

    let reasoning_chunk_2 = AIMessageChunk::builder()
        .content("[]")
        .additional_kwargs(additional_kwargs_reasoning2)
        .response_metadata(response_metadata_chunk.clone())
        .build();

    let expected_reasoning2_content = vec![ContentBlock::Reasoning(ReasoningContentBlock {
        block_type: "reasoning".to_string(),
        id: None,
        reasoning: Some("reasoning text".to_string()),
        index: None,
        extras: None,
    })];
    assert_eq!(
        reasoning_chunk_2.content_blocks(),
        expected_reasoning2_content
    );

    let merged_reasoning = reasoning_chunk_1 + reasoning_chunk_2;
    let expected_merged_reasoning = vec![ContentBlock::Reasoning(ReasoningContentBlock {
        block_type: "reasoning".to_string(),
        id: Some("rs_abc".to_string()),
        reasoning: Some("reasoning text".to_string()),
        index: None,
        extras: None,
    })];
    assert_eq!(merged_reasoning.content_blocks(), expected_merged_reasoning);
}

#[test]
fn test_convert_to_openai_data_block() {
    let block = json!({
        "type": "image",
        "url": "https://example.com/test.png"
    });
    let expected = json!({
        "type": "image_url",
        "image_url": {"url": "https://example.com/test.png"}
    });
    let result = convert_to_openai_data_block(&block, OpenAiApi::ChatCompletions).unwrap();
    assert_eq!(result, expected);

    let block = json!({
        "type": "image",
        "base64": "<base64 string>",
        "mime_type": "image/png"
    });
    let expected = json!({
        "type": "image_url",
        "image_url": {"url": "data:image/png;base64,<base64 string>"}
    });
    let result = convert_to_openai_data_block(&block, OpenAiApi::ChatCompletions).unwrap();
    assert_eq!(result, expected);

    let block = json!({
        "type": "file",
        "url": "https://example.com/test.pdf"
    });
    let result = convert_to_openai_data_block(&block, OpenAiApi::ChatCompletions);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("does not support"));

    let block = json!({
        "type": "file",
        "base64": "<base64 string>",
        "mime_type": "application/pdf",
        "filename": "test.pdf"
    });
    let expected = json!({
        "type": "file",
        "file": {
            "file_data": "data:application/pdf;base64,<base64 string>",
            "filename": "test.pdf"
        }
    });
    let result = convert_to_openai_data_block(&block, OpenAiApi::ChatCompletions).unwrap();
    assert_eq!(result, expected);

    let block = json!({
        "type": "file",
        "file_id": "file-abc123"
    });
    let expected = json!({"type": "file", "file": {"file_id": "file-abc123"}});
    let result = convert_to_openai_data_block(&block, OpenAiApi::ChatCompletions).unwrap();
    assert_eq!(result, expected);

    let block = json!({
        "type": "audio",
        "base64": "<base64 string>",
        "mime_type": "audio/wav"
    });
    let expected = json!({
        "type": "input_audio",
        "input_audio": {"data": "<base64 string>", "format": "wav"}
    });
    let result = convert_to_openai_data_block(&block, OpenAiApi::ChatCompletions).unwrap();
    assert_eq!(result, expected);

    let block = json!({
        "type": "image",
        "url": "https://example.com/test.png"
    });
    let expected = json!({"type": "input_image", "image_url": "https://example.com/test.png"});
    let result = convert_to_openai_data_block(&block, OpenAiApi::Responses).unwrap();
    assert_eq!(result, expected);

    let block = json!({
        "type": "image",
        "base64": "<base64 string>",
        "mime_type": "image/png"
    });
    let expected = json!({
        "type": "input_image",
        "image_url": "data:image/png;base64,<base64 string>"
    });
    let result = convert_to_openai_data_block(&block, OpenAiApi::Responses).unwrap();
    assert_eq!(result, expected);

    let block = json!({
        "type": "file",
        "url": "https://example.com/test.pdf"
    });
    let expected = json!({"type": "input_file", "file_url": "https://example.com/test.pdf"});
    let result = convert_to_openai_data_block(&block, OpenAiApi::Responses).unwrap();
    assert_eq!(result, expected);

    let block = json!({
        "type": "file",
        "base64": "<base64 string>",
        "mime_type": "application/pdf",
        "filename": "test.pdf"
    });
    let expected = json!({
        "type": "input_file",
        "file_data": "data:application/pdf;base64,<base64 string>",
        "filename": "test.pdf"
    });
    let result = convert_to_openai_data_block(&block, OpenAiApi::Responses).unwrap();
    assert_eq!(result, expected);

    let block = json!({
        "type": "file",
        "file_id": "file-abc123"
    });
    let expected = json!({"type": "input_file", "file_id": "file-abc123"});
    let result = convert_to_openai_data_block(&block, OpenAiApi::Responses).unwrap();
    assert_eq!(result, expected);
}
