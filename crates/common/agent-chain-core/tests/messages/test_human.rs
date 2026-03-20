use agent_chain_core::load::Serializable;
use agent_chain_core::messages::BaseMessage;
use agent_chain_core::messages::{
    ContentBlock, ContentBlocks, HumanMessage, HumanMessageChunk, ImageContentBlock,
    SystemMessageChunk, TextContentBlock,
};

#[test]
fn test_init_basic() {
    let msg = HumanMessage::builder()
        .content("Hello, how are you?")
        .build();
    assert_eq!(msg.content.as_text(), "Hello, how are you?");
    assert_eq!(msg.message_type(), "human");
}

#[test]
fn test_init_with_name() {
    let msg = HumanMessage::builder()
        .content("Hello")
        .name("user1".to_string())
        .build();
    assert_eq!(msg.name, Some("user1".to_string()));
}

#[test]
fn test_init_with_id() {
    let msg = HumanMessage::builder()
        .content("Hello")
        .id("msg-123".to_string())
        .build();
    assert_eq!(msg.id, Some("msg-123".to_string()));
}

#[test]
fn test_init_with_additional_kwargs() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("custom".to_string(), serde_json::json!("value"));

    let msg = HumanMessage::builder()
        .content("Hello")
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
    response_metadata.insert("source".to_string(), serde_json::json!("web"));

    let msg = HumanMessage::builder()
        .content("Hello")
        .response_metadata(response_metadata)
        .build();
    assert_eq!(
        msg.response_metadata.get("source").unwrap(),
        &serde_json::json!("web")
    );
}

#[test]
fn test_type_is_human() {
    let msg = HumanMessage::builder().content("Test").build();
    assert_eq!(msg.message_type(), "human");
}

#[test]
fn test_serialization_roundtrip() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("custom".to_string(), serde_json::json!("value"));

    let msg = HumanMessage::builder()
        .content("Hello")
        .id("msg-123".to_string())
        .name("user1".to_string())
        .additional_kwargs(additional_kwargs)
        .build();

    let serialized = serde_json::to_value(&msg).unwrap();
    assert_eq!(serialized.get("type").unwrap().as_str().unwrap(), "human");

    let deserialized: HumanMessage = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content.as_text(), "Hello");
    assert_eq!(deserialized.name, Some("user1".to_string()));
    assert_eq!(deserialized.id, Some("msg-123".to_string()));
    assert_eq!(
        deserialized.additional_kwargs.get("custom").unwrap(),
        &serde_json::json!("value")
    );
}

#[test]
fn test_text_property() {
    let msg = HumanMessage::builder().content("Hello world").build();
    assert_eq!(msg.content.as_text(), "Hello world");
}

#[test]
fn test_empty_content() {
    let msg = HumanMessage::builder().content("").build();
    assert_eq!(msg.content.as_text(), "");
}

#[test]
fn test_chunk_init_basic() {
    let chunk = HumanMessageChunk::builder().content("Hello").build();
    assert_eq!(chunk.content.as_text(), "Hello");
    assert_eq!(chunk.message_type(), "HumanMessageChunk");
}

#[test]
fn test_chunk_type_is_human_message_chunk() {
    let chunk = HumanMessageChunk::builder().content("Test").build();
    assert_eq!(chunk.message_type(), "HumanMessageChunk");
}

#[test]
fn test_chunk_add_two_chunks() {
    let chunk1 = HumanMessageChunk::builder()
        .content("Hello")
        .id("1".to_string())
        .build();
    let chunk2 = HumanMessageChunk::builder().content(" world").build();
    let result = chunk1 + chunk2;
    assert_eq!(result.content.as_text(), "Hello world");
    assert_eq!(result.id, Some("1".to_string()));
}

#[test]
fn test_chunk_add_with_additional_kwargs() {
    let mut kwargs1 = std::collections::HashMap::new();
    kwargs1.insert("key1".to_string(), serde_json::json!("value1"));

    let mut kwargs2 = std::collections::HashMap::new();
    kwargs2.insert("key2".to_string(), serde_json::json!("value2"));

    let chunk1 = HumanMessageChunk::builder()
        .content("Hello")
        .additional_kwargs(kwargs1)
        .build();
    let chunk2 = HumanMessageChunk::builder()
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

    let chunk1 = HumanMessageChunk::builder()
        .content("Hello")
        .response_metadata(meta1)
        .build();
    let chunk2 = HumanMessageChunk::builder()
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
    let chunk1 = HumanMessageChunk::builder()
        .content("Hello")
        .id("original-id".to_string())
        .build();
    let chunk2 = HumanMessageChunk::builder()
        .content(" world")
        .id("other-id".to_string())
        .build();
    let result = chunk1 + chunk2;
    assert_eq!(result.id, Some("original-id".to_string()));
}

#[test]
fn test_chunk_serialization_roundtrip() {
    let chunk = HumanMessageChunk::builder()
        .content("Hello")
        .id("chunk-123".to_string())
        .name("user1".to_string())
        .build();

    let serialized = serde_json::to_value(&chunk).unwrap();
    assert_eq!(
        serialized.get("type").unwrap().as_str().unwrap(),
        "HumanMessageChunk"
    );

    let deserialized: HumanMessageChunk = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.content.as_text(), "Hello");
    assert_eq!(deserialized.name, Some("user1".to_string()));
    assert_eq!(deserialized.id, Some("chunk-123".to_string()));
}

#[test]
fn test_chunk_multiple_additions() {
    let chunk1 = HumanMessageChunk::builder().content("a").build();
    let chunk2 = HumanMessageChunk::builder().content("b").build();
    let chunk3 = HumanMessageChunk::builder().content("c").build();
    let result = chunk1 + chunk2 + chunk3;
    assert_eq!(result.content.as_text(), "abc");
}

#[test]
fn test_chunk_empty_content() {
    let chunk1 = HumanMessageChunk::builder().content("Hello").build();
    let chunk2 = HumanMessageChunk::builder().content("").build();
    let result = chunk1 + chunk2;
    assert_eq!(result.content.as_text(), "Hello");
}

#[test]
fn test_chunk_text_property() {
    let chunk = HumanMessageChunk::builder().content("Hello world").build();
    assert_eq!(chunk.content.as_text(), "Hello world");
}

#[test]
fn test_chunk_to_message() {
    let chunk = HumanMessageChunk::builder()
        .content("Hello!")
        .id("chunk-1".to_string())
        .build();
    let message: HumanMessage = chunk.clone().into();
    assert_eq!(message.content.as_text(), "Hello!");
    assert_eq!(message.id, Some("chunk-1".to_string()));
}

#[test]
fn test_chunk_sum() {
    let chunks = vec![
        HumanMessageChunk::builder().content("Hello ").build(),
        HumanMessageChunk::builder().content("beautiful ").build(),
        HumanMessageChunk::builder().content("world!").build(),
    ];
    let result: HumanMessageChunk = chunks.into_iter().sum();
    assert_eq!(result.content.as_text(), "Hello beautiful world!");
}

#[test]
fn test_init_with_list_content() {
    let blocks = ContentBlocks::from(vec![ContentBlock::Text(TextContentBlock::new("Hello"))]);
    let msg = HumanMessage::builder().content(blocks).build();
    assert_eq!(msg.content.len(), 1);
    if let ContentBlock::Text(tb) = &msg.content[0] {
        assert_eq!(tb.text, "Hello");
    } else {
        panic!("expected Text content block, got {:?}", &msg.content[0]);
    }
}

#[test]
fn test_init_with_multimodal_content() {
    let blocks = ContentBlocks::from(vec![
        ContentBlock::Text(TextContentBlock::new("What's in this image?")),
        ContentBlock::Image(ImageContentBlock::from_url("https://example.com/img.png")),
    ]);
    let msg = HumanMessage::builder().content(blocks).build();
    assert_eq!(msg.content.len(), 2);
    assert!(matches!(&msg.content[0], ContentBlock::Text(_)));
    assert!(matches!(&msg.content[1], ContentBlock::Image(_)));
}

#[test]
fn test_init_with_content_blocks() {
    let blocks = ContentBlocks::from(vec![
        ContentBlock::Text(TextContentBlock::new("Hello")),
        ContentBlock::Image(ImageContentBlock::from_url("https://example.com/img.png")),
    ]);
    let msg = HumanMessage::builder().content(blocks).build();
    assert_eq!(msg.content.len(), 2);
}

#[test]
fn test_text_method() {
    let msg = HumanMessage::builder().content("Hello world").build();
    assert_eq!(msg.text(), "Hello world");
}

#[test]
fn test_text_method_list_content() {
    let blocks = ContentBlocks::from(vec![
        ContentBlock::Text(TextContentBlock::new("Part 1")),
        ContentBlock::Text(TextContentBlock::new("Part 2")),
    ]);
    let msg = HumanMessage::builder().content(blocks).build();
    assert_eq!(msg.text(), "Part 1 Part 2");
}

#[test]
fn test_text_method_multimodal_content() {
    let blocks = ContentBlocks::from(vec![
        ContentBlock::Text(TextContentBlock::new("Hello")),
        ContentBlock::Image(ImageContentBlock::from_url("https://example.com")),
        ContentBlock::Text(TextContentBlock::new("world")),
    ]);
    let msg = HumanMessage::builder().content(blocks).build();
    assert_eq!(msg.text(), "Hello world");
}

#[test]
fn test_content_blocks_property() {
    let msg = HumanMessage::builder().content("Hello").build();
    let blocks = msg.content_blocks();
    assert_eq!(blocks.len(), 1);
    match &blocks[0] {
        ContentBlock::Text(tb) => {
            assert_eq!(tb.block_type, "text");
            assert_eq!(tb.text, "Hello");
        }
        other => panic!("expected Text content block, got {:?}", other),
    }
}

#[test]
fn test_content_blocks_multimodal() {
    let blocks = ContentBlocks::from(vec![
        ContentBlock::Text(TextContentBlock::new("What's in this?")),
        ContentBlock::Image(ImageContentBlock::from_url("https://example.com/img.png")),
    ]);
    let msg = HumanMessage::builder().content(blocks).build();
    let blocks = msg.content_blocks();
    assert!(blocks.len() >= 2);
    assert!(matches!(&blocks[0], ContentBlock::Text(_)));
}

#[test]
fn test_pretty_repr() {
    let msg = HumanMessage::builder().content("Hello").build();
    let result = msg.pretty_repr(false);
    assert!(
        result.contains("Human Message"),
        "expected 'Human Message' in pretty_repr, got: {result}"
    );
    assert!(
        result.contains("Hello"),
        "expected 'Hello' in pretty_repr, got: {result}"
    );
}

#[test]
fn test_pretty_repr_with_name() {
    let msg = HumanMessage::builder()
        .content("Hello")
        .name("user1".to_string())
        .build();
    let result = msg.pretty_repr(false);
    assert!(
        result.contains("Name: user1"),
        "expected 'Name: user1' in pretty_repr, got: {result}"
    );
}

#[test]
fn test_empty_list_content() {
    let msg = HumanMessage::builder()
        .content(ContentBlocks::new())
        .build();
    assert!(msg.content.is_empty());
    assert_eq!(msg.text(), "");
}

#[test]
fn test_model_dump_exact_keys_and_values() {
    let msg = HumanMessage::builder()
        .content("Hello world")
        .id("msg-001".to_string())
        .name("alice".to_string())
        .build();
    let dumped = serde_json::to_value(&msg).unwrap();
    assert_eq!(dumped["content"], "Hello world");
    assert_eq!(dumped["type"], "human");
    assert_eq!(dumped["name"], "alice");
    assert_eq!(dumped["id"], "msg-001");
    assert_eq!(dumped["additional_kwargs"], serde_json::json!({}));
    assert_eq!(dumped["response_metadata"], serde_json::json!({}));
}

#[test]
fn test_model_dump_default_values() {
    let msg = HumanMessage::builder().content("Test").build();
    let dumped = serde_json::to_value(&msg).unwrap();
    assert_eq!(dumped["content"], "Test");
    assert_eq!(dumped["type"], "human");
    assert!(dumped["id"].is_null());
    assert_eq!(dumped["additional_kwargs"], serde_json::json!({}));
    assert_eq!(dumped["response_metadata"], serde_json::json!({}));
    assert!(dumped.get("name").is_none() || dumped["name"].is_null());
}

#[test]
fn test_same_content_messages_are_equal() {
    let msg1 = HumanMessage::builder().content("Hello").build();
    let msg2 = HumanMessage::builder().content("Hello").build();
    assert_eq!(msg1, msg2);
}

#[test]
fn test_different_content_messages_are_not_equal() {
    let msg1 = HumanMessage::builder().content("Hello").build();
    let msg2 = HumanMessage::builder().content("World").build();
    assert_ne!(msg1, msg2);
}

#[test]
fn test_same_content_different_id_are_not_equal() {
    let msg1 = HumanMessage::builder()
        .content("Hello")
        .id("1".to_string())
        .build();
    let msg2 = HumanMessage::builder()
        .content("Hello")
        .id("2".to_string())
        .build();
    assert_ne!(msg1, msg2);
}

#[test]
fn test_same_content_and_metadata_are_equal() {
    let msg1 = HumanMessage::builder()
        .content("Hello")
        .name("user1".to_string())
        .id("msg-1".to_string())
        .build();
    let msg2 = HumanMessage::builder()
        .content("Hello")
        .name("user1".to_string())
        .id("msg-1".to_string())
        .build();
    assert_eq!(msg1, msg2);
}

#[test]
fn test_init_with_content_blocks_sets_content() {
    let blocks = ContentBlocks::from(vec![
        ContentBlock::Text(TextContentBlock::new("Hello")),
        ContentBlock::Text(TextContentBlock::new(" world")),
    ]);
    let msg = HumanMessage::builder().content(blocks).build();
    assert_eq!(msg.content.len(), 2);
}

#[test]
fn test_content_blocks_roundtrip() {
    let blocks = ContentBlocks::from(vec![
        ContentBlock::Text(TextContentBlock::new("First")),
        ContentBlock::Text(TextContentBlock::new("Second")),
    ]);
    let msg = HumanMessage::builder().content(blocks).build();
    let result_blocks = msg.content_blocks();
    assert!(result_blocks.len() >= 2);
}

#[test]
fn test_chunk_add_with_list_content() {
    let chunk1 = HumanMessageChunk::builder()
        .content(ContentBlocks::from(vec![ContentBlock::Text(
            TextContentBlock::new("Hello"),
        )]))
        .build();
    let chunk2 = HumanMessageChunk::builder()
        .content(ContentBlocks::from(vec![ContentBlock::Text(
            TextContentBlock::new(" world"),
        )]))
        .build();
    let result = chunk1 + chunk2;
    assert_eq!(result.content.len(), 2);
    if let ContentBlock::Text(tb) = &result.content[0] {
        assert_eq!(tb.text, "Hello");
    } else {
        panic!("expected Text, got {:?}", &result.content[0]);
    }
    if let ContentBlock::Text(tb) = &result.content[1] {
        assert_eq!(tb.text, " world");
    } else {
        panic!("expected Text, got {:?}", &result.content[1]);
    }
}

#[test]
fn test_chunk_add_list_of_chunks() {
    let chunk1 = HumanMessageChunk::builder()
        .content("a")
        .id("1".to_string())
        .build();
    let chunk2 = HumanMessageChunk::builder().content("b").build();
    let chunk3 = HumanMessageChunk::builder().content("c").build();
    let result = vec![chunk2, chunk3]
        .into_iter()
        .fold(chunk1, |acc, c| acc + c);
    assert_eq!(result.content.as_text(), "abc");
    assert_eq!(result.id, Some("1".to_string()));
}

#[test]
fn test_chunk_content_blocks_property() {
    let chunk = HumanMessageChunk::builder().content("Hello").build();
    let msg: HumanMessage = chunk.into();
    let blocks = msg.content_blocks();
    assert_eq!(blocks.len(), 1);
    match &blocks[0] {
        ContentBlock::Text(tb) => {
            assert_eq!(tb.block_type, "text");
            assert_eq!(tb.text, "Hello");
        }
        other => panic!("expected Text content block, got {:?}", other),
    }
}

#[test]
fn test_chunk_content_blocks_multimodal() {
    let blocks = ContentBlocks::from(vec![
        ContentBlock::Text(TextContentBlock::new("Check this:")),
        ContentBlock::Image(ImageContentBlock::from_url("https://example.com/img.png")),
    ]);
    let chunk = HumanMessageChunk::builder().content(blocks).build();
    let msg: HumanMessage = chunk.into();
    let blocks = msg.content_blocks();
    assert!(blocks.len() >= 2);
    assert!(matches!(&blocks[0], ContentBlock::Text(_)));
}

#[test]
fn test_chunk_content_blocks_empty_string() {
    let chunk = HumanMessageChunk::builder().content("").build();
    let msg: HumanMessage = chunk.into();
    let blocks = msg.content_blocks();
    assert!(blocks.is_empty());
}

#[test]
fn test_chunk_content_blocks_empty_list() {
    let chunk = HumanMessageChunk::builder()
        .content(ContentBlocks::new())
        .build();
    let msg: HumanMessage = chunk.into();
    let blocks = msg.content_blocks();
    assert!(blocks.is_empty());
}

#[test]
fn test_chunk_add_with_list_content_with_index() {
    let chunk1 = HumanMessageChunk::builder()
        .content(ContentBlocks::from(vec![
            serde_json::from_value::<ContentBlock>(
                serde_json::json!({"type": "text", "text": "Hello", "index": 0}),
            )
            .unwrap(),
        ]))
        .build();
    let chunk2 = HumanMessageChunk::builder()
        .content(ContentBlocks::from(vec![
            serde_json::from_value::<ContentBlock>(
                serde_json::json!({"type": "text", "text": " world", "index": 0}),
            )
            .unwrap(),
        ]))
        .build();
    let result = chunk1 + chunk2;
    assert_eq!(
        result.content.len(),
        1,
        "expected 1 merged part, got {}",
        result.content.len()
    );
    assert_eq!(result.content.as_text(), "Hello world");
}

#[test]
fn test_chunk_add_different_chunk_type() {
    let chunk1 = HumanMessageChunk::builder()
        .content("Hello")
        .id("1".to_string())
        .build();
    let chunk2 = SystemMessageChunk::builder().content(" world").build();
    let result = chunk1 + chunk2;
    assert!(
        matches!(&result, HumanMessageChunk { .. }),
        "result should be HumanMessageChunk"
    );
    assert_eq!(result.content.as_text(), "Hello world");
}

#[test]
fn test_is_lc_serializable() {
    assert!(HumanMessage::is_lc_serializable());
}

#[test]
fn test_get_lc_namespace() {
    let namespace = HumanMessage::get_lc_namespace();
    assert_eq!(
        namespace,
        vec![
            "langchain".to_string(),
            "schema".to_string(),
            "messages".to_string()
        ]
    );
}

#[test]
fn test_instance_is_lc_serializable() {
    assert!(HumanMessage::is_lc_serializable());
}

#[test]
fn test_instance_get_lc_namespace() {
    let _msg = HumanMessage::builder().content("Hello").build();
    assert_eq!(
        HumanMessage::get_lc_namespace(),
        vec![
            "langchain".to_string(),
            "schema".to_string(),
            "messages".to_string()
        ]
    );
}
