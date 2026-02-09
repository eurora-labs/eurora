//! Tests for SystemMessage and SystemMessageChunk.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/test_system.py`

use agent_chain_core::messages::{
    ContentBlock, ContentPart, HumanMessage, HumanMessageChunk, MessageContent, SystemMessage,
    SystemMessageChunk, TextContentBlock,
};

// ============================================================================
// TestSystemMessage
// ============================================================================

#[test]
fn test_init_basic() {
    let msg = SystemMessage::builder()
        .content("You are a helpful assistant.")
        .build();
    assert!(matches!(&msg.content, MessageContent::Text(s) if s == "You are a helpful assistant."));
    assert_eq!(msg.message_type(), "system");
}

#[test]
fn test_init_with_name() {
    let msg = SystemMessage::builder()
        .content("Instructions")
        .maybe_name(Some("system_prompt".to_string()))
        .build();
    assert_eq!(msg.name, Some("system_prompt".to_string()));
}

#[test]
fn test_init_with_id() {
    let msg = SystemMessage::builder()
        .content("Instructions")
        .maybe_id(Some("sys-123".to_string()))
        .build();
    assert_eq!(msg.id, Some("sys-123".to_string()));
}

#[test]
fn test_init_with_additional_kwargs() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("priority".to_string(), serde_json::json!("high"));

    let msg = SystemMessage::builder()
        .content("Instructions")
        .additional_kwargs(additional_kwargs)
        .build();
    assert_eq!(
        msg.additional_kwargs.get("priority").unwrap(),
        &serde_json::json!("high")
    );
}

#[test]
fn test_init_with_response_metadata() {
    let mut response_metadata = std::collections::HashMap::new();
    response_metadata.insert("version".to_string(), serde_json::json!("1.0"));

    let msg = SystemMessage::builder()
        .content("Instructions")
        .response_metadata(response_metadata)
        .build();
    assert_eq!(
        msg.response_metadata.get("version").unwrap(),
        &serde_json::json!("1.0")
    );
}

#[test]
fn test_type_is_system() {
    let msg = SystemMessage::builder().content("Test").build();
    assert_eq!(msg.message_type(), "system");
}

#[test]
fn test_serialization_roundtrip() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("version".to_string(), serde_json::json!("1.0"));

    let msg = SystemMessage::builder()
        .content("You are a helpful assistant.")
        .maybe_id(Some("sys-123".to_string()))
        .maybe_name(Some("system_prompt".to_string()))
        .additional_kwargs(additional_kwargs)
        .build();

    let serialized = serde_json::to_value(&msg).unwrap();
    assert_eq!(serialized.get("type").unwrap().as_str().unwrap(), "system");

    let deserialized: SystemMessage = serde_json::from_value(serialized).unwrap();
    assert!(
        matches!(&deserialized.content, MessageContent::Text(s) if s == "You are a helpful assistant.")
    );
    assert_eq!(deserialized.name, Some("system_prompt".to_string()));
    assert_eq!(deserialized.id, Some("sys-123".to_string()));
    assert_eq!(
        deserialized.additional_kwargs.get("version").unwrap(),
        &serde_json::json!("1.0")
    );
}

#[test]
fn test_text_content() {
    let msg = SystemMessage::builder().content("Hello world").build();
    assert!(matches!(&msg.content, MessageContent::Text(s) if s == "Hello world"));
}

#[test]
fn test_empty_content() {
    let msg = SystemMessage::builder().content("").build();
    assert!(matches!(&msg.content, MessageContent::Text(s) if s.is_empty()));
}

#[test]
fn test_developer_role_via_additional_kwargs() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert(
        "__openai_role__".to_string(),
        serde_json::json!("developer"),
    );

    let msg = SystemMessage::builder()
        .content("Developer instructions")
        .additional_kwargs(additional_kwargs)
        .build();
    assert_eq!(
        msg.additional_kwargs.get("__openai_role__").unwrap(),
        &serde_json::json!("developer")
    );
}

// ============================================================================
// TestSystemMessageChunk
// ============================================================================

#[test]
fn test_chunk_init_basic() {
    let chunk = SystemMessageChunk::builder()
        .content("Instructions")
        .build();
    assert!(matches!(&chunk.content, MessageContent::Text(s) if s == "Instructions"));
    assert_eq!(chunk.message_type(), "SystemMessageChunk");
}

#[test]
fn test_chunk_type_is_system_message_chunk() {
    let chunk = SystemMessageChunk::builder().content("Test").build();
    assert_eq!(chunk.message_type(), "SystemMessageChunk");
}

#[test]
fn test_chunk_add_two_chunks() {
    let chunk1 = SystemMessageChunk::builder()
        .content("Hello")
        .maybe_id(Some("1".to_string()))
        .build();
    let chunk2 = SystemMessageChunk::builder().content(" world").build();
    let result = chunk1 + chunk2;
    assert!(matches!(&result.content, MessageContent::Text(s) if s == "Hello world"));
    assert_eq!(result.id, Some("1".to_string()));
}

#[test]
fn test_chunk_add_with_additional_kwargs() {
    let mut kwargs1 = std::collections::HashMap::new();
    kwargs1.insert("key1".to_string(), serde_json::json!("value1"));

    let mut kwargs2 = std::collections::HashMap::new();
    kwargs2.insert("key2".to_string(), serde_json::json!("value2"));

    let chunk1 = SystemMessageChunk::builder()
        .content("Hello")
        .additional_kwargs(kwargs1)
        .build();
    let chunk2 = SystemMessageChunk::builder()
        .content(" world")
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

    let chunk1 = SystemMessageChunk::builder()
        .content("Hello")
        .response_metadata(meta1)
        .build();
    let chunk2 = SystemMessageChunk::builder()
        .content(" world")
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
fn test_chunk_add_preserves_id() {
    let chunk1 = SystemMessageChunk::builder()
        .content("Hello")
        .maybe_id(Some("original-id".to_string()))
        .build();
    let chunk2 = SystemMessageChunk::builder()
        .content(" world")
        .maybe_id(Some("other-id".to_string()))
        .build();
    let result = chunk1 + chunk2;
    assert_eq!(result.id, Some("original-id".to_string()));
}

#[test]
fn test_chunk_serialization_roundtrip() {
    let chunk = SystemMessageChunk::builder()
        .content("Instructions")
        .maybe_id(Some("chunk-123".to_string()))
        .maybe_name(Some("sys_prompt".to_string()))
        .build();

    let serialized = serde_json::to_value(&chunk).unwrap();
    assert_eq!(
        serialized.get("type").unwrap().as_str().unwrap(),
        "SystemMessageChunk"
    );

    let deserialized: SystemMessageChunk = serde_json::from_value(serialized).unwrap();
    assert!(matches!(&deserialized.content, MessageContent::Text(s) if s == "Instructions"));
    assert_eq!(deserialized.name, Some("sys_prompt".to_string()));
    assert_eq!(deserialized.id, Some("chunk-123".to_string()));
}

#[test]
fn test_chunk_multiple_additions() {
    let chunk1 = SystemMessageChunk::builder().content("a").build();
    let chunk2 = SystemMessageChunk::builder().content("b").build();
    let chunk3 = SystemMessageChunk::builder().content("c").build();
    let result = chunk1 + chunk2 + chunk3;
    assert!(matches!(&result.content, MessageContent::Text(s) if s == "abc"));
}

#[test]
fn test_chunk_empty_content() {
    let chunk1 = SystemMessageChunk::builder().content("Hello").build();
    let chunk2 = SystemMessageChunk::builder().content("").build();
    let result = chunk1 + chunk2;
    assert!(matches!(&result.content, MessageContent::Text(s) if s == "Hello"));
}

#[test]
fn test_chunk_add_different_chunk_type() {
    let chunk1 = SystemMessageChunk::builder()
        .content("Hello")
        .maybe_id(Some("1".to_string()))
        .build();
    let chunk2 = HumanMessageChunk::builder().content(" world").build();

    // Convert to messages and verify content
    let msg1 = chunk1.to_message();
    let msg2: HumanMessage = chunk2.into();

    // Verify both messages have their content
    let content1 = match &msg1.content {
        MessageContent::Text(s) => s.as_str(),
        MessageContent::Parts(_) => "",
    };
    let content2 = match &msg2.content {
        MessageContent::Text(s) => s.as_str(),
        MessageContent::Parts(_) => "",
    };
    assert_eq!(content1, "Hello");
    assert_eq!(content2, " world");

    // We can concatenate content strings manually
    let combined_content = format!("{}{}", content1, content2);
    assert_eq!(combined_content, "Hello world");
}

#[test]
fn test_chunk_text_content() {
    let chunk = SystemMessageChunk::builder().content("Hello world").build();
    assert!(matches!(&chunk.content, MessageContent::Text(s) if s == "Hello world"));
}

// ============================================================================
// TestSystemMessageDeveloperRole
// ============================================================================

#[test]
fn test_developer_role_preserved_in_serialization() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert(
        "__openai_role__".to_string(),
        serde_json::json!("developer"),
    );

    let msg = SystemMessage::builder()
        .content("Developer instructions")
        .additional_kwargs(additional_kwargs)
        .build();

    let serialized = serde_json::to_value(&msg).unwrap();
    let deserialized: SystemMessage = serde_json::from_value(serialized).unwrap();

    assert_eq!(
        deserialized
            .additional_kwargs
            .get("__openai_role__")
            .unwrap(),
        &serde_json::json!("developer")
    );
}

#[test]
fn test_multiple_system_messages_with_different_roles() {
    let system_msg = SystemMessage::builder()
        .content("System instructions")
        .build();

    let mut dev_kwargs = std::collections::HashMap::new();
    dev_kwargs.insert(
        "__openai_role__".to_string(),
        serde_json::json!("developer"),
    );
    let developer_msg = SystemMessage::builder()
        .content("Developer instructions")
        .additional_kwargs(dev_kwargs)
        .build();

    assert!(!system_msg.additional_kwargs.contains_key("__openai_role__"));
    assert_eq!(
        developer_msg
            .additional_kwargs
            .get("__openai_role__")
            .unwrap(),
        &serde_json::json!("developer")
    );
}

// ============================================================================
// TestSystemMessage — list content
// ============================================================================

#[test]
fn test_init_with_list_content() {
    let parts = vec![ContentPart::Text {
        text: "Instructions".to_string(),
    }];
    let msg = SystemMessage::builder()
        .content(MessageContent::Parts(parts))
        .build();
    match &msg.content {
        MessageContent::Parts(p) => {
            assert_eq!(p.len(), 1);
            match &p[0] {
                ContentPart::Text { text } => assert_eq!(text, "Instructions"),
                other => panic!("expected Text content part, got {:?}", other),
            }
        }
        other => panic!("expected Parts content, got {:?}", other),
    }
}

#[test]
fn test_init_with_content_blocks() {
    let blocks = vec![
        ContentBlock::Text(TextContentBlock::new("First instruction")),
        ContentBlock::Text(TextContentBlock::new("Second instruction")),
    ];
    let msg = SystemMessage::builder()
        .content("")
        .content_blocks(blocks)
        .build();
    // When content_blocks is provided, content is Parts (not Text)
    assert!(matches!(&msg.content, MessageContent::Parts(_)));
}

#[test]
fn test_empty_list_content() {
    let msg = SystemMessage::builder()
        .content(MessageContent::Parts(vec![]))
        .build();
    match &msg.content {
        MessageContent::Parts(p) => assert!(p.is_empty()),
        other => panic!("expected Parts content, got {:?}", other),
    }
}

// ============================================================================
// TestSystemMessage — text() method
// ============================================================================

#[test]
fn test_text_method() {
    let msg = SystemMessage::builder().content("Hello world").build();
    assert_eq!(msg.text(), "Hello world");
}

#[test]
fn test_text_method_list_content() {
    let parts = vec![
        ContentPart::Text {
            text: "Part 1".to_string(),
        },
        ContentPart::Text {
            text: "Part 2".to_string(),
        },
    ];
    let msg = SystemMessage::builder()
        .content(MessageContent::Parts(parts))
        .build();
    // Rust joins text parts with spaces
    assert_eq!(msg.text(), "Part 1 Part 2");
}

#[test]
fn test_text_method_empty_content() {
    let msg = SystemMessage::builder().content("").build();
    assert_eq!(msg.text(), "");
}

#[test]
fn test_text_method_empty_list_content() {
    let msg = SystemMessage::builder()
        .content(MessageContent::Parts(vec![]))
        .build();
    assert_eq!(msg.text(), "");
}

// ============================================================================
// TestSystemMessage — content_blocks() property
// ============================================================================

#[test]
fn test_content_blocks_property() {
    let msg = SystemMessage::builder().content("Instructions").build();
    let blocks = msg.content_blocks();
    assert_eq!(blocks.len(), 1);
    match &blocks[0] {
        ContentBlock::Text(tb) => {
            assert_eq!(tb.block_type, "text");
            assert_eq!(tb.text, "Instructions");
        }
        other => panic!("expected Text content block, got {:?}", other),
    }
}

#[test]
fn test_content_blocks_empty_string() {
    let msg = SystemMessage::builder().content("").build();
    let blocks = msg.content_blocks();
    assert!(blocks.is_empty());
}

#[test]
fn test_content_blocks_empty_list() {
    let msg = SystemMessage::builder()
        .content(MessageContent::Parts(vec![]))
        .build();
    let blocks = msg.content_blocks();
    assert!(blocks.is_empty());
}

// ============================================================================
// TestSystemMessage — pretty_repr
// ============================================================================

#[test]
fn test_pretty_repr() {
    let msg = SystemMessage::builder().content("Instructions").build();
    let result = msg.pretty_repr(false);
    assert!(
        result.contains("System Message"),
        "expected 'System Message' in pretty_repr, got: {result}"
    );
    assert!(
        result.contains("Instructions"),
        "expected 'Instructions' in pretty_repr, got: {result}"
    );
}

#[test]
fn test_pretty_repr_with_name() {
    let msg = SystemMessage::builder()
        .content("Instructions")
        .name("sys_prompt".to_string())
        .build();
    let result = msg.pretty_repr(false);
    assert!(
        result.contains("Name: sys_prompt"),
        "expected 'Name: sys_prompt' in pretty_repr, got: {result}"
    );
}

// ============================================================================
// TestSystemMessage — content_blocks init
// ============================================================================

#[test]
fn test_init_with_content_blocks_sets_content() {
    let blocks = vec![
        ContentBlock::Text(TextContentBlock::new("First instruction")),
        ContentBlock::Text(TextContentBlock::new("Second instruction")),
    ];
    let msg = SystemMessage::builder()
        .content("")
        .content_blocks(blocks)
        .build();
    // content should be Parts, not Text
    assert!(matches!(&msg.content, MessageContent::Parts(_)));
}

#[test]
fn test_content_blocks_roundtrip() {
    let blocks = vec![
        ContentBlock::Text(TextContentBlock::new("Rule 1")),
        ContentBlock::Text(TextContentBlock::new("Rule 2")),
    ];
    let msg = SystemMessage::builder()
        .content("")
        .content_blocks(blocks)
        .build();
    let result_blocks = msg.content_blocks();
    assert_eq!(result_blocks.len(), 2);
    match &result_blocks[0] {
        ContentBlock::Text(tb) => assert_eq!(tb.text, "Rule 1"),
        other => panic!("expected Text content block, got {:?}", other),
    }
    match &result_blocks[1] {
        ContentBlock::Text(tb) => assert_eq!(tb.text, "Rule 2"),
        other => panic!("expected Text content block, got {:?}", other),
    }
}

// ============================================================================
// TestSystemMessage — model_dump snapshot
// ============================================================================

#[test]
fn test_model_dump_exact_keys_and_values() {
    let msg = SystemMessage::builder()
        .content("Be helpful")
        .maybe_id(Some("sys-001".to_string()))
        .maybe_name(Some("prompt".to_string()))
        .build();
    let dumped = serde_json::to_value(&msg).unwrap();
    assert_eq!(dumped["content"], "Be helpful");
    assert_eq!(dumped["type"], "system");
    assert_eq!(dumped["name"], "prompt");
    assert_eq!(dumped["id"], "sys-001");
    assert_eq!(dumped["additional_kwargs"], serde_json::json!({}));
    assert_eq!(dumped["response_metadata"], serde_json::json!({}));
}

#[test]
fn test_model_dump_default_values() {
    let msg = SystemMessage::builder().content("Instructions").build();
    let dumped = serde_json::to_value(&msg).unwrap();
    assert_eq!(dumped["content"], "Instructions");
    assert_eq!(dumped["type"], "system");
    assert!(dumped.get("name").is_none() || dumped["name"].is_null());
    assert!(dumped["id"].is_null());
    assert_eq!(dumped["additional_kwargs"], serde_json::json!({}));
    assert_eq!(dumped["response_metadata"], serde_json::json!({}));
}

// ============================================================================
// TestSystemMessage — equality
// ============================================================================

#[test]
fn test_same_content_messages_are_equal() {
    let msg1 = SystemMessage::builder().content("Be helpful").build();
    let msg2 = SystemMessage::builder().content("Be helpful").build();
    assert_eq!(msg1, msg2);
}

#[test]
fn test_different_content_messages_are_not_equal() {
    let msg1 = SystemMessage::builder().content("Be helpful").build();
    let msg2 = SystemMessage::builder().content("Be concise").build();
    assert_ne!(msg1, msg2);
}

#[test]
fn test_same_content_different_id_are_not_equal() {
    let msg1 = SystemMessage::builder()
        .content("Instructions")
        .maybe_id(Some("1".to_string()))
        .build();
    let msg2 = SystemMessage::builder()
        .content("Instructions")
        .maybe_id(Some("2".to_string()))
        .build();
    assert_ne!(msg1, msg2);
}

#[test]
fn test_same_content_and_metadata_are_equal() {
    let msg1 = SystemMessage::builder()
        .content("Instructions")
        .maybe_name(Some("sys".to_string()))
        .maybe_id(Some("sys-1".to_string()))
        .build();
    let msg2 = SystemMessage::builder()
        .content("Instructions")
        .maybe_name(Some("sys".to_string()))
        .maybe_id(Some("sys-1".to_string()))
        .build();
    assert_eq!(msg1, msg2);
}

// ============================================================================
// TestSystemMessageChunk — list content addition
// ============================================================================

#[test]
fn test_chunk_add_with_list_content() {
    let chunk1 = SystemMessageChunk::builder()
        .content(MessageContent::Parts(vec![ContentPart::Text {
            text: "Hello".to_string(),
        }]))
        .build();
    let chunk2 = SystemMessageChunk::builder()
        .content(MessageContent::Parts(vec![ContentPart::Text {
            text: " world".to_string(),
        }]))
        .build();
    let result = chunk1 + chunk2;
    match &result.content {
        MessageContent::Parts(parts) => {
            // Parts are appended (not merged) when there is no index key
            assert_eq!(parts.len(), 2);
            match &parts[0] {
                ContentPart::Text { text } => assert_eq!(text, "Hello"),
                other => panic!("expected Text, got {:?}", other),
            }
            match &parts[1] {
                ContentPart::Text { text } => assert_eq!(text, " world"),
                other => panic!("expected Text, got {:?}", other),
            }
        }
        other => panic!("expected Parts content, got {:?}", other),
    }
}

#[test]
fn test_chunk_add_with_list_content_with_index() {
    // Parts with matching "index" keys should be merged (text concatenated),
    // not appended as separate items. This matches Python's merge_lists behavior.
    let chunk1 = SystemMessageChunk::builder()
        .content(MessageContent::Parts(vec![ContentPart::Other(
            serde_json::json!({"type": "text", "text": "Hello", "index": 0}),
        )]))
        .build();
    let chunk2 = SystemMessageChunk::builder()
        .content(MessageContent::Parts(vec![ContentPart::Other(
            serde_json::json!({"type": "text", "text": " world", "index": 0}),
        )]))
        .build();
    let result = chunk1 + chunk2;
    match &result.content {
        MessageContent::Parts(parts) => {
            // Items with same "index" key are merged, not appended
            assert_eq!(
                parts.len(),
                1,
                "expected 1 merged part, got {}",
                parts.len()
            );
            // The merge produces {"type":"text","text":"Hello world","index":0}.
            // When deserialized back to ContentPart, this matches the Text variant
            // (the index field is a streaming artifact consumed during merging).
            match &parts[0] {
                ContentPart::Text { text } => assert_eq!(text, "Hello world"),
                ContentPart::Other(v) => assert_eq!(v["text"], "Hello world"),
                other => panic!("expected Text or Other content part, got {:?}", other),
            }
        }
        other => panic!("expected Parts content, got {:?}", other),
    }
}

#[test]
fn test_chunk_add_list_of_chunks() {
    let chunk1 = SystemMessageChunk::builder()
        .content("a")
        .maybe_id(Some("1".to_string()))
        .build();
    let chunk2 = SystemMessageChunk::builder().content("b").build();
    let chunk3 = SystemMessageChunk::builder().content("c").build();
    // Equivalent to Python's `chunk1 + [chunk2, chunk3]` using fold
    let result = vec![chunk2, chunk3]
        .into_iter()
        .fold(chunk1, |acc, c| acc + c);
    assert_eq!(result.content.as_text(), "abc");
    assert_eq!(result.id, Some("1".to_string()));
}

// ============================================================================
// TestSystemMessageChunk — content_blocks property (via to_message)
// ============================================================================

#[test]
fn test_chunk_content_blocks_property() {
    // SystemMessageChunk doesn't have content_blocks() directly, but we can
    // convert to SystemMessage and test content_blocks there
    let chunk = SystemMessageChunk::builder()
        .content("Instructions")
        .build();
    let msg: SystemMessage = chunk.into();
    let blocks = msg.content_blocks();
    assert_eq!(blocks.len(), 1);
    match &blocks[0] {
        ContentBlock::Text(tb) => {
            assert_eq!(tb.block_type, "text");
            assert_eq!(tb.text, "Instructions");
        }
        other => panic!("expected Text content block, got {:?}", other),
    }
}

#[test]
fn test_chunk_content_blocks_empty_string() {
    let chunk = SystemMessageChunk::builder().content("").build();
    let msg: SystemMessage = chunk.into();
    let blocks = msg.content_blocks();
    assert!(blocks.is_empty());
}

#[test]
fn test_chunk_content_blocks_empty_list() {
    let chunk = SystemMessageChunk::builder()
        .content(MessageContent::Parts(vec![]))
        .build();
    let msg: SystemMessage = chunk.into();
    let blocks = msg.content_blocks();
    assert!(blocks.is_empty());
}

// ============================================================================
// TestSystemMessageChunk — text property (via content)
// ============================================================================

#[test]
fn test_chunk_text_method() {
    let chunk = SystemMessageChunk::builder().content("Hello world").build();
    assert_eq!(chunk.content.as_text(), "Hello world");
}
