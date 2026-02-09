//! Tests for ChatMessage and ChatMessageChunk.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/test_chat.py`

use agent_chain_core::messages::{
    ChatMessage, ChatMessageChunk, ContentBlock, ContentPart, HumanMessageChunk, MessageContent,
};

// ============================================================================
// TestChatMessage
// ============================================================================

#[test]
fn test_init_basic() {
    let msg = ChatMessage::builder().content("Hello").role("user").build();
    assert_eq!(msg.content.as_text(), "Hello");
    assert_eq!(msg.role, "user");
    assert_eq!(msg.message_type(), "chat");
}

#[test]
fn test_init_with_name() {
    let msg = ChatMessage::builder()
        .content("Hello")
        .role("assistant")
        .name("bot".to_string())
        .build();
    assert_eq!(msg.name, Some("bot".to_string()));
    assert_eq!(msg.role, "assistant");
}

#[test]
fn test_init_with_id() {
    let msg = ChatMessage::builder()
        .id("msg-123".to_string())
        .content("Hello")
        .role("user")
        .build();
    assert_eq!(msg.id, Some("msg-123".to_string()));
}

#[test]
fn test_init_with_additional_kwargs() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("custom".to_string(), serde_json::json!("value"));

    let msg = ChatMessage::builder()
        .content("Hello")
        .role("user")
        .additional_kwargs(additional_kwargs)
        .build();
    assert_eq!(
        msg.additional_kwargs.get("custom").unwrap(),
        &serde_json::json!("value")
    );
}

#[test]
fn test_init_with_response_metadata() {
    let mut response_metadata = std::collections::HashMap::new();
    response_metadata.insert("model".to_string(), serde_json::json!("custom"));

    let msg = ChatMessage::builder()
        .content("Hello")
        .role("system")
        .response_metadata(response_metadata)
        .build();
    assert_eq!(
        msg.response_metadata.get("model").unwrap(),
        &serde_json::json!("custom")
    );
}

#[test]
fn test_init_with_list_content() {
    let content = MessageContent::Parts(vec![ContentPart::Other(
        serde_json::json!({"type": "text", "text": "Hello"}),
    )]);
    let msg = ChatMessage::builder()
        .content(content.clone())
        .role("user")
        .build();
    assert_eq!(msg.content, content);
}

#[test]
fn test_different_roles() {
    let roles = vec!["user", "assistant", "system", "admin", "custom_role"];
    for role in roles {
        let msg = ChatMessage::builder().content("Test").role(role).build();
        assert_eq!(msg.role, role);
    }
}

#[test]
fn test_type_is_chat() {
    let msg = ChatMessage::builder().content("Test").role("user").build();
    assert_eq!(msg.message_type(), "chat");
}

#[test]
fn test_serialization_roundtrip() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("priority".to_string(), serde_json::json!("high"));

    let msg = ChatMessage::builder()
        .id("chat-123".to_string())
        .content("Hello")
        .role("moderator")
        .name("mod1".to_string())
        .additional_kwargs(additional_kwargs)
        .build();

    let serialized = serde_json::to_value(&msg).unwrap();
    assert_eq!(serialized.get("type").unwrap().as_str().unwrap(), "chat");

    let deserialized: ChatMessage = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content.as_text(), "Hello");
    assert_eq!(deserialized.role, "moderator");
    assert_eq!(deserialized.name, Some("mod1".to_string()));
    assert_eq!(deserialized.id, Some("chat-123".to_string()));
    assert_eq!(
        deserialized.additional_kwargs.get("priority").unwrap(),
        &serde_json::json!("high")
    );
}

#[test]
fn test_text_property() {
    let msg = ChatMessage::builder()
        .content("Hello world")
        .role("user")
        .build();
    assert_eq!(msg.text(), "Hello world");
}

#[test]
fn test_text_property_list_content() {
    let msg = ChatMessage::builder()
        .content(MessageContent::Parts(vec![
            ContentPart::Text {
                text: "Part 1".to_string(),
            },
            ContentPart::Text {
                text: "Part 2".to_string(),
            },
        ]))
        .role("user")
        .build();
    // Rust as_text() joins text parts with spaces (matching existing convention)
    assert_eq!(msg.text(), "Part 1 Part 2");
}

#[test]
fn test_content_blocks_property() {
    let msg = ChatMessage::builder().content("Hello").role("user").build();
    let blocks = msg.content_blocks();
    assert_eq!(blocks.len(), 1);
    if let ContentBlock::Text(text_block) = &blocks[0] {
        assert_eq!(text_block.text, "Hello");
    } else {
        panic!("Expected ContentBlock::Text");
    }
}

#[test]
fn test_pretty_repr() {
    let msg = ChatMessage::builder().content("Hello").role("user").build();
    let result = msg.pretty_repr(false);
    assert!(result.contains("Chat Message"));
    assert!(result.contains("Hello"));
}

// ============================================================================
// TestChatMessageChunk
// ============================================================================

#[test]
fn test_chunk_init_basic() {
    let chunk = ChatMessageChunk::builder()
        .content("Hello")
        .role("user")
        .build();
    assert_eq!(chunk.content.as_text(), "Hello");
    assert_eq!(chunk.role, "user");
    assert_eq!(chunk.message_type(), "ChatMessageChunk");
}

#[test]
fn test_chunk_type_is_chat_message_chunk() {
    let chunk = ChatMessageChunk::builder()
        .content("Test")
        .role("user")
        .build();
    assert_eq!(chunk.message_type(), "ChatMessageChunk");
}

#[test]
fn test_chunk_add_same_role_chunks() {
    let chunk1 = ChatMessageChunk::builder()
        .id("1".to_string())
        .content("Hello")
        .role("user")
        .build();
    let chunk2 = ChatMessageChunk::builder()
        .content(" world")
        .role("user")
        .build();
    let result = chunk1 + chunk2;
    assert_eq!(result.content.as_text(), "Hello world");
    assert_eq!(result.role, "user");
    assert_eq!(result.id, Some("1".to_string()));
}

#[test]
#[should_panic(expected = "Cannot concatenate")]
fn test_chunk_add_different_role_chunks_raises_error() {
    let chunk1 = ChatMessageChunk::builder()
        .content("Hello")
        .role("user")
        .build();
    let chunk2 = ChatMessageChunk::builder()
        .content(" world")
        .role("assistant")
        .build();
    let _result = chunk1 + chunk2;
}

#[test]
fn test_chunk_add_with_additional_kwargs() {
    let mut kwargs1 = std::collections::HashMap::new();
    kwargs1.insert("key1".to_string(), serde_json::json!("value1"));

    let mut kwargs2 = std::collections::HashMap::new();
    kwargs2.insert("key2".to_string(), serde_json::json!("value2"));

    let chunk1 = ChatMessageChunk::builder()
        .content("Hello")
        .role("user")
        .additional_kwargs(kwargs1)
        .build();
    let chunk2 = ChatMessageChunk::builder()
        .content(" world")
        .role("user")
        .additional_kwargs(kwargs2)
        .build();

    let result = chunk1 + chunk2;
    assert_eq!(
        result.additional_kwargs.get("key1").unwrap(),
        &serde_json::json!("value1")
    );
    assert_eq!(
        result.additional_kwargs.get("key2").unwrap(),
        &serde_json::json!("value2")
    );
}

#[test]
fn test_chunk_add_with_response_metadata() {
    let mut meta1 = std::collections::HashMap::new();
    meta1.insert("meta1".to_string(), serde_json::json!("data1"));

    let mut meta2 = std::collections::HashMap::new();
    meta2.insert("meta2".to_string(), serde_json::json!("data2"));

    let chunk1 = ChatMessageChunk::builder()
        .content("Hello")
        .role("user")
        .response_metadata(meta1)
        .build();
    let chunk2 = ChatMessageChunk::builder()
        .content(" world")
        .role("user")
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
fn test_chunk_add_with_list_content() {
    let chunk1 = ChatMessageChunk::builder()
        .content(MessageContent::Parts(vec![ContentPart::Other(
            serde_json::json!({"type": "text", "text": "Hello"}),
        )]))
        .role("user")
        .build();
    let chunk2 = ChatMessageChunk::builder()
        .content(MessageContent::Parts(vec![ContentPart::Other(
            serde_json::json!({"type": "text", "text": " world"}),
        )]))
        .role("user")
        .build();
    let result = chunk1 + chunk2;
    // Items without 'index' key are appended, not merged
    if let MessageContent::Parts(parts) = &result.content {
        assert_eq!(parts.len(), 2);
    } else {
        panic!("Expected MessageContent::Parts");
    }
}

#[test]
fn test_chunk_add_with_list_content_with_index() {
    let chunk1 = ChatMessageChunk::builder()
        .content(MessageContent::Parts(vec![ContentPart::Other(
            serde_json::json!({"type": "text", "text": "Hello", "index": 0}),
        )]))
        .role("user")
        .build();
    let chunk2 = ChatMessageChunk::builder()
        .content(MessageContent::Parts(vec![ContentPart::Other(
            serde_json::json!({"type": "text", "text": " world", "index": 0}),
        )]))
        .role("user")
        .build();
    let result = chunk1 + chunk2;
    // Items with same 'index' key are merged
    if let MessageContent::Parts(parts) = &result.content {
        assert_eq!(parts.len(), 1);
        let part_value = serde_json::to_value(&parts[0]).unwrap();
        // The merged result may be wrapped in a JSON object or may be the
        // ContentPart::Other variant. Extract the text from whatever structure
        // the serialized part has.
        let text = part_value
            .get("text")
            .or_else(|| part_value.get("Other").and_then(|o| o.get("text")))
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("Could not find text in: {:?}", part_value));
        assert_eq!(text, "Hello world");
    } else {
        panic!("Expected MessageContent::Parts");
    }
}

#[test]
fn test_chunk_add_chat_chunk_to_human_chunk() {
    let chunk1 = ChatMessageChunk::builder()
        .id("1".to_string())
        .content("Hello")
        .role("user")
        .build();
    let chunk2 = HumanMessageChunk::builder().content(" world").build();
    let result = chunk1 + chunk2;
    assert_eq!(result.content.as_text(), "Hello world");
    assert_eq!(result.role, "user");
    assert_eq!(result.id, Some("1".to_string()));
}

#[test]
fn test_chunk_add_preserves_id() {
    let chunk1 = ChatMessageChunk::builder()
        .id("original-id".to_string())
        .content("Hello")
        .role("user")
        .build();
    let chunk2 = ChatMessageChunk::builder()
        .id("other-id".to_string())
        .content(" world")
        .role("user")
        .build();
    let result = chunk1 + chunk2;
    assert_eq!(result.id, Some("original-id".to_string()));
}

#[test]
fn test_chunk_serialization_roundtrip() {
    let chunk = ChatMessageChunk::builder()
        .id("chunk-123".to_string())
        .content("Hello")
        .role("moderator")
        .build();

    let serialized = serde_json::to_value(&chunk).unwrap();
    assert_eq!(
        serialized.get("type").unwrap().as_str().unwrap(),
        "ChatMessageChunk"
    );

    let deserialized: ChatMessageChunk = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content.as_text(), "Hello");
    assert_eq!(deserialized.role, "moderator");
    assert_eq!(deserialized.id, Some("chunk-123".to_string()));
}

#[test]
fn test_chunk_multiple_additions() {
    let chunk1 = ChatMessageChunk::builder()
        .content("a")
        .role("user")
        .build();
    let chunk2 = ChatMessageChunk::builder()
        .content("b")
        .role("user")
        .build();
    let chunk3 = ChatMessageChunk::builder()
        .content("c")
        .role("user")
        .build();
    let result = chunk1 + chunk2 + chunk3;
    assert_eq!(result.content.as_text(), "abc");
    assert_eq!(result.role, "user");
}

#[test]
fn test_chunk_empty_content() {
    let chunk1 = ChatMessageChunk::builder()
        .content("Hello")
        .role("user")
        .build();
    let chunk2 = ChatMessageChunk::builder().content("").role("user").build();
    let result = chunk1 + chunk2;
    assert_eq!(result.content.as_text(), "Hello");
}

#[test]
fn test_chunk_text_property() {
    let chunk = ChatMessageChunk::builder()
        .content("Hello world")
        .role("user")
        .build();
    assert_eq!(chunk.text(), "Hello world");
}

#[test]
fn test_chunk_content_blocks_property() {
    let chunk = ChatMessageChunk::builder()
        .content("Hello")
        .role("user")
        .build();
    let blocks = chunk.content_blocks();
    assert_eq!(blocks.len(), 1);
    if let ContentBlock::Text(text_block) = &blocks[0] {
        assert_eq!(text_block.text, "Hello");
    } else {
        panic!("Expected ContentBlock::Text");
    }
}

// ============================================================================
// TestChatMessageContentBlocksMixedTypes
// ============================================================================

#[test]
fn test_content_blocks_with_mixed_list_content() {
    let content = MessageContent::Parts(vec![
        ContentPart::Other(serde_json::json!({"type": "text", "text": "Hello"})),
        ContentPart::Other(
            serde_json::json!({"type": "image", "source_media_type": "image/png", "source_data": "abc"}),
        ),
    ]);
    let msg = ChatMessage::builder().content(content).role("user").build();
    let blocks = msg.content_blocks();
    assert_eq!(blocks.len(), 2);
    if let ContentBlock::Text(text_block) = &blocks[0] {
        assert_eq!(text_block.text, "Hello");
    } else {
        panic!("Expected ContentBlock::Text for first block");
    }
    if let ContentBlock::Image(_) = &blocks[1] {
        // image block parsed correctly
    } else {
        panic!("Expected ContentBlock::Image for second block");
    }
}

// ============================================================================
// TestChatMessageModelDump
// ============================================================================

#[test]
fn test_model_dump_includes_role() {
    let msg = ChatMessage::builder()
        .content("Hello")
        .role("moderator")
        .build();
    let dumped = serde_json::to_value(&msg).unwrap();
    assert_eq!(dumped.get("role").unwrap().as_str().unwrap(), "moderator");
    assert_eq!(dumped.get("content").unwrap().as_str().unwrap(), "Hello");
    assert_eq!(dumped.get("type").unwrap().as_str().unwrap(), "chat");
    assert!(dumped.get("additional_kwargs").is_some());
    assert!(dumped.get("response_metadata").is_some());
}

#[test]
fn test_model_dump_all_fields() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("key".to_string(), serde_json::json!("val"));

    let mut response_metadata = std::collections::HashMap::new();
    response_metadata.insert("meta".to_string(), serde_json::json!("data"));

    let msg = ChatMessage::builder()
        .content("Test")
        .role("custom")
        .name("speaker".to_string())
        .id("chat-456".to_string())
        .additional_kwargs(additional_kwargs)
        .response_metadata(response_metadata)
        .build();

    let dumped = serde_json::to_value(&msg).unwrap();
    assert_eq!(dumped.get("role").unwrap().as_str().unwrap(), "custom");
    assert_eq!(dumped.get("name").unwrap().as_str().unwrap(), "speaker");
    assert_eq!(dumped.get("id").unwrap().as_str().unwrap(), "chat-456");
    assert_eq!(
        dumped
            .get("additional_kwargs")
            .unwrap()
            .get("key")
            .unwrap()
            .as_str()
            .unwrap(),
        "val"
    );
    assert_eq!(
        dumped
            .get("response_metadata")
            .unwrap()
            .get("meta")
            .unwrap()
            .as_str()
            .unwrap(),
        "data"
    );
}

// ============================================================================
// TestChatMessageChunkAddMultiple
// ============================================================================

#[test]
fn test_add_multiple_chat_chunks_chained() {
    let chunk1 = ChatMessageChunk::builder()
        .content("a")
        .role("user")
        .id("1".to_string())
        .build();
    let chunk2 = ChatMessageChunk::builder()
        .content("b")
        .role("user")
        .build();
    let chunk3 = ChatMessageChunk::builder()
        .content("c")
        .role("user")
        .build();
    let chunk4 = ChatMessageChunk::builder()
        .content("d")
        .role("user")
        .build();
    let result = chunk1 + chunk2 + chunk3 + chunk4;
    assert_eq!(result.content.as_text(), "abcd");
    assert_eq!(result.role, "user");
    assert_eq!(result.id, Some("1".to_string()));
}

#[test]
fn test_add_multiple_mixed_chunks_chained() {
    let chunk1 = ChatMessageChunk::builder()
        .content("Hello")
        .role("user")
        .id("1".to_string())
        .build();
    let chunk2 = HumanMessageChunk::builder().content(" from").build();
    let chunk3 = ChatMessageChunk::builder()
        .content(" world")
        .role("user")
        .build();
    let result = (chunk1 + chunk2) + chunk3;
    assert_eq!(result.content.as_text(), "Hello from world");
    assert_eq!(result.id, Some("1".to_string()));
}

#[test]
fn test_add_accumulates_additional_kwargs() {
    let mut kwargs1 = std::collections::HashMap::new();
    kwargs1.insert("k1".to_string(), serde_json::json!("v1"));

    let mut kwargs2 = std::collections::HashMap::new();
    kwargs2.insert("k2".to_string(), serde_json::json!("v2"));

    let mut kwargs3 = std::collections::HashMap::new();
    kwargs3.insert("k3".to_string(), serde_json::json!("v3"));

    let chunk1 = ChatMessageChunk::builder()
        .content("a")
        .role("user")
        .additional_kwargs(kwargs1)
        .build();
    let chunk2 = ChatMessageChunk::builder()
        .content("b")
        .role("user")
        .additional_kwargs(kwargs2)
        .build();
    let chunk3 = ChatMessageChunk::builder()
        .content("c")
        .role("user")
        .additional_kwargs(kwargs3)
        .build();
    let result = chunk1 + chunk2 + chunk3;
    assert_eq!(result.content.as_text(), "abc");
    assert_eq!(
        result.additional_kwargs.get("k1").unwrap(),
        &serde_json::json!("v1")
    );
    assert_eq!(
        result.additional_kwargs.get("k2").unwrap(),
        &serde_json::json!("v2")
    );
    assert_eq!(
        result.additional_kwargs.get("k3").unwrap(),
        &serde_json::json!("v3")
    );
}

// ============================================================================
// TestChatMessagePrettyReprHtml
// ============================================================================

#[test]
fn test_pretty_repr_html_true() {
    let msg = ChatMessage::builder().content("Hello").role("user").build();
    let result = msg.pretty_repr(true);
    assert!(result.contains("Chat Message"));
    assert!(result.contains("Hello"));
    // When html=true, bold=true is passed, which wraps title in ANSI bold codes
    assert!(result.contains("\x1b[1m"));
}

#[test]
fn test_pretty_repr_html_false() {
    let msg = ChatMessage::builder().content("Hello").role("user").build();
    let result_html = msg.pretty_repr(true);
    let result_plain = msg.pretty_repr(false);
    assert!(result_html.contains("Hello"));
    assert!(result_plain.contains("Hello"));
    // Plain text version should NOT contain ANSI bold codes
    assert!(!result_plain.contains("\x1b[1m"));
}

// ============================================================================
// TestChatMessageContentBlocksInit
// ============================================================================

#[test]
fn test_init_with_content_blocks() {
    let blocks = vec![
        ContentBlock::Text(agent_chain_core::messages::TextContentBlock {
            block_type: "text".to_string(),
            text: "Hello".to_string(),
            id: None,
            index: None,
            annotations: None,
            extras: None,
        }),
        ContentBlock::Text(agent_chain_core::messages::TextContentBlock {
            block_type: "text".to_string(),
            text: " world".to_string(),
            id: None,
            index: None,
            annotations: None,
            extras: None,
        }),
    ];
    let msg = ChatMessage::builder()
        .content("")
        .content_blocks(blocks)
        .role("user")
        .build();
    // When content_blocks is provided, it overrides content
    if let MessageContent::Parts(_) = &msg.content {
        // content_blocks are serialized into Parts
    } else {
        panic!("Expected MessageContent::Parts when content_blocks provided");
    }
}

#[test]
fn test_content_blocks_roundtrip() {
    let blocks = vec![
        ContentBlock::Text(agent_chain_core::messages::TextContentBlock {
            block_type: "text".to_string(),
            text: "First".to_string(),
            id: None,
            index: None,
            annotations: None,
            extras: None,
        }),
        ContentBlock::Text(agent_chain_core::messages::TextContentBlock {
            block_type: "text".to_string(),
            text: "Second".to_string(),
            id: None,
            index: None,
            annotations: None,
            extras: None,
        }),
    ];
    let msg = ChatMessage::builder()
        .content("")
        .content_blocks(blocks)
        .role("assistant")
        .build();
    let result_blocks = msg.content_blocks();
    assert_eq!(result_blocks.len(), 2);
    if let ContentBlock::Text(tb) = &result_blocks[0] {
        assert_eq!(tb.text, "First");
    } else {
        panic!("Expected ContentBlock::Text for first block");
    }
    if let ContentBlock::Text(tb) = &result_blocks[1] {
        assert_eq!(tb.text, "Second");
    } else {
        panic!("Expected ContentBlock::Text for second block");
    }
}
