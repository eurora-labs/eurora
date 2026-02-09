//! Integration tests for agent_chain proto conversions.
//!
//! These tests verify that conversions between proto types and agent_chain_core types
//! are lossless and work correctly in both directions.

#[cfg(feature = "agent-chain")]
mod tests {
    use agent_chain_core::messages::*;
    use proto_gen::agent_chain::*;
    use std::collections::HashMap;

    #[test]
    fn test_usage_metadata_roundtrip() {
        let original = UsageMetadata {
            input_tokens: 100,
            output_tokens: 50,
            total_tokens: 150,
            input_token_details: Some(InputTokenDetails {
                audio: Some(10),
                cache_creation: Some(20),
                cache_read: Some(30),
            }),
            output_token_details: Some(OutputTokenDetails {
                audio: Some(5),
                reasoning: Some(15),
            }),
        };

        let proto: ProtoUsageMetadata = original.clone().into();
        let roundtrip: UsageMetadata = proto.into();

        assert_eq!(original, roundtrip);
    }

    #[test]
    fn test_tool_call_roundtrip() {
        let args = serde_json::json!({
            "city": "London",
            "units": "celsius"
        });

        let original = ToolCall::builder()
            .id("call_123".to_string())
            .name("get_weather")
            .args(args.clone())
            .build();

        let proto: ProtoToolCall = original.clone().into();
        let roundtrip: ToolCall = proto.into();

        assert_eq!(roundtrip.id, Some("call_123".to_string()));
        assert_eq!(roundtrip.name, "get_weather");
        assert_eq!(roundtrip.args, args);
    }

    #[test]
    fn test_human_message_roundtrip() {
        let original = HumanMessage::builder()
            .id("msg_123".to_string())
            .content("Hello, world!")
            .name("User".to_string())
            .build();

        let proto: ProtoHumanMessage = original.clone().into();
        let roundtrip: HumanMessage = proto.into();

        assert_eq!(roundtrip.id, Some("msg_123".to_string()));
        assert_eq!(roundtrip.content.as_text(), "Hello, world!");
        assert_eq!(roundtrip.name, Some("User".to_string()));
    }

    #[test]
    fn test_ai_message_roundtrip() {
        let tool_call = ToolCall::builder()
            .id("call_456".to_string())
            .name("search")
            .args(serde_json::json!({"query": "rust programming"}))
            .build();

        let original = AIMessage::builder()
            .id("msg_456".to_string())
            .content("Let me search for that.")
            .tool_calls(vec![tool_call])
            .usage_metadata(UsageMetadata::new(50, 25))
            .build();

        let proto: ProtoAiMessage = original.clone().into();
        let roundtrip: AIMessage = proto.into();

        assert_eq!(roundtrip.id, Some("msg_456".to_string()));
        assert_eq!(roundtrip.content, "Let me search for that.");
        assert_eq!(roundtrip.tool_calls.len(), 1);
        assert_eq!(roundtrip.tool_calls[0].name, "search");
        assert!(roundtrip.usage_metadata.is_some());
        assert_eq!(roundtrip.usage_metadata.unwrap().input_tokens, 50);
    }

    #[test]
    fn test_system_message_roundtrip() {
        let original = SystemMessage::builder()
            .id("msg_789".to_string())
            .content("You are a helpful assistant.")
            .build();

        let proto: ProtoSystemMessage = original.clone().into();
        let roundtrip: SystemMessage = proto.into();

        assert_eq!(roundtrip.id, Some("msg_789".to_string()));
        assert_eq!(roundtrip.content.as_text(), "You are a helpful assistant.");
    }

    #[test]
    fn test_tool_message_roundtrip() {
        let original = ToolMessage::builder()
            .id("msg_999".to_string())
            .content("Search results: ...")
            .tool_call_id("call_456")
            .name("search".to_string())
            .status(ToolStatus::Success)
            .build();

        let proto: ProtoToolMessage = original.clone().into();
        let roundtrip: ToolMessage = proto.into();

        assert_eq!(roundtrip.id, Some("msg_999".to_string()));
        assert_eq!(roundtrip.content, "Search results: ...");
        assert_eq!(roundtrip.tool_call_id, "call_456");
        assert_eq!(roundtrip.status, ToolStatus::Success);
    }

    #[test]
    fn test_base_message_roundtrip() {
        let human = HumanMessage::builder().content("Hello!").build();
        let original = BaseMessage::Human(human);

        let proto: ProtoBaseMessage = original.clone().into();
        let roundtrip: BaseMessage = proto.into();

        match roundtrip {
            BaseMessage::Human(msg) => {
                assert_eq!(msg.content.as_text(), "Hello!");
            }
            _ => panic!("Expected Human message"),
        }
    }

    #[test]
    fn test_ai_message_chunk_roundtrip() {
        let tool_chunk = ToolCallChunk {
            name: Some("search".to_string()),
            args: Some("{\"query\":".to_string()),
            id: Some("call_123".to_string()),
            index: Some(0),
            chunk_type: None,
        };

        let original = AIMessageChunk::builder()
            .content("Searching...")
            .tool_call_chunks(vec![tool_chunk])
            .usage_metadata(UsageMetadata::new(10, 5))
            .build();

        let proto: ProtoAiMessageChunk = original.clone().into();
        let roundtrip: AIMessageChunk = proto.into();

        assert_eq!(roundtrip.content, "Searching...");
        assert_eq!(roundtrip.tool_call_chunks.len(), 1);
        assert_eq!(
            roundtrip.tool_call_chunks[0].name,
            Some("search".to_string())
        );
        assert!(roundtrip.usage_metadata.is_some());
    }

    #[test]
    fn test_multimodal_human_message_roundtrip() {
        let parts = vec![
            ContentPart::Text {
                text: "What's in this image?".to_string(),
            },
            ContentPart::Image {
                source: ImageSource::Url {
                    url: "https://example.com/image.jpg".to_string(),
                },
                detail: Some(ImageDetail::High),
            },
        ];

        let original = HumanMessage::builder()
            .content(MessageContent::Parts(parts))
            .build();

        let proto: ProtoHumanMessage = original.clone().into();
        let roundtrip: HumanMessage = proto.into();

        match &roundtrip.content {
            MessageContent::Parts(parts) => {
                assert_eq!(parts.len(), 2);
                match &parts[0] {
                    ContentPart::Text { text } => assert_eq!(text, "What's in this image?"),
                    _ => panic!("Expected text part"),
                }
                match &parts[1] {
                    ContentPart::Image { source, detail } => {
                        match source {
                            ImageSource::Url { url } => {
                                assert_eq!(url, "https://example.com/image.jpg")
                            }
                            _ => panic!("Expected URL source"),
                        }
                        assert_eq!(detail, &Some(ImageDetail::High));
                    }
                    _ => panic!("Expected image part"),
                }
            }
            _ => panic!("Expected Parts content"),
        }
    }

    #[test]
    fn test_text_content_block_with_annotations() {
        let mut extras = HashMap::new();
        extras.insert(
            "source".to_string(),
            serde_json::Value::String("doc_123".to_string()),
        );

        let citation = Annotation::Citation {
            id: Some("cite_1".to_string()),
            url: Some("https://example.com/doc".to_string()),
            title: Some("Example Document".to_string()),
            start_index: Some(0),
            end_index: Some(10),
            cited_text: Some("The weather is sunny.".to_string()),
            extras: Some(extras),
        };

        let original = TextContentBlock {
            block_type: "text".to_string(),
            id: Some("block_1".to_string()),
            text: "It's sunny.".to_string(),
            annotations: Some(vec![citation]),
            index: None,
            extras: None,
        };

        let proto: ProtoTextContentBlock = original.clone().into();
        let roundtrip: TextContentBlock = proto.into();

        assert_eq!(roundtrip.text, "It's sunny.");
        assert!(roundtrip.annotations.is_some());
        let annotations = roundtrip.annotations.unwrap();
        assert_eq!(annotations.len(), 1);

        match &annotations[0] {
            Annotation::Citation {
                url,
                title,
                cited_text,
                ..
            } => {
                assert_eq!(url, &Some("https://example.com/doc".to_string()));
                assert_eq!(title, &Some("Example Document".to_string()));
                assert_eq!(cited_text, &Some("The weather is sunny.".to_string()));
            }
            _ => panic!("Expected Citation annotation"),
        }
    }

    #[test]
    fn test_content_block_enum_roundtrip() {
        let text_block = ContentBlock::Text(TextContentBlock::new("Hello"));
        let proto: ProtoContentBlock = text_block.clone().into();
        let roundtrip: ContentBlock = proto.into();

        match roundtrip {
            ContentBlock::Text(block) => {
                assert_eq!(block.text, "Hello");
            }
            _ => panic!("Expected Text content block"),
        }
    }

    #[test]
    fn test_reasoning_content_block_roundtrip() {
        let original = ReasoningContentBlock::new("Let me think about this...");

        let proto: ProtoReasoningContentBlock = original.clone().into();
        let roundtrip: ReasoningContentBlock = proto.into();

        assert_eq!(roundtrip.reasoning(), Some("Let me think about this..."));
    }

    #[test]
    fn test_server_tool_call_roundtrip() {
        let mut args = HashMap::new();
        args.insert(
            "code".to_string(),
            serde_json::Value::String("print('hello')".to_string()),
        );

        let original = ServerToolCall::new("exec_123", "python_exec", args);

        let proto: ProtoServerToolCall = original.clone().into();
        let roundtrip: ServerToolCall = proto.into();

        assert_eq!(roundtrip.id, "exec_123");
        assert_eq!(roundtrip.name, "python_exec");
        assert!(roundtrip.args.contains_key("code"));
    }

    #[test]
    fn test_server_tool_result_roundtrip() {
        let original = ServerToolResult::success("call_123");

        let proto: ProtoServerToolResult = original.clone().into();
        let roundtrip: ServerToolResult = proto.into();

        assert_eq!(roundtrip.tool_call_id, "call_123");
        assert_eq!(roundtrip.status, ServerToolStatus::Success);
    }
}
