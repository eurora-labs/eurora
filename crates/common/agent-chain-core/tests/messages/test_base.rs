//! Tests for base message functionality.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/messages/test_base.py`

use agent_chain_core::messages::{
    AIMessage, BaseMessage, ContentBlock, ContentPart, HumanMessage, HumanMessageChunk,
    MergeableContent, MessageContent, SystemMessage, SystemMessageChunk, TextContentBlock,
    extract_reasoning_from_additional_kwargs, get_msg_title_repr, merge_content,
    merge_content_complex, message_to_dict, messages_to_dict,
};
use serde_json::json;


#[test]
fn test_text_property_string_content() {
    let msg = HumanMessage::builder().content("Hello, world!").build();
    assert_eq!(msg.content.as_text(), "Hello, world!");
}

#[test]
fn test_text_property_list_content_with_text_blocks() {
    let msg = HumanMessage::builder()
        .content(MessageContent::Parts(vec![
            ContentPart::Text {
                text: "First part".to_string(),
            },
            ContentPart::Text {
                text: "second part".to_string(),
            },
        ]))
        .build();
    assert_eq!(msg.content.as_text(), "First part second part");
}

#[test]
fn test_text_property_list_content_with_mixed_blocks() {
    let msg = HumanMessage::builder()
        .content(MessageContent::Parts(vec![
            ContentPart::Text {
                text: "Hello".to_string(),
            },
            ContentPart::Image {
                source: agent_chain_core::messages::ImageSource::Url {
                    url: "http://example.com/img.png".to_string(),
                },
                detail: None,
            },
            ContentPart::Text {
                text: "world".to_string(),
            },
        ]))
        .build();
    assert_eq!(msg.content.as_text(), "Hello world");
}

#[test]
fn test_text_property_empty_content() {
    let msg = HumanMessage::builder().content("").build();
    assert_eq!(msg.content.as_text(), "");
}

#[test]
fn test_text_property_empty_list_content() {
    let msg = HumanMessage::builder()
        .content(MessageContent::Parts(vec![]))
        .build();
    assert_eq!(msg.content.as_text(), "");
}

#[test]
fn test_text_property_no_text_blocks() {
    let msg = HumanMessage::builder()
        .content(MessageContent::Parts(vec![ContentPart::Image {
            source: agent_chain_core::messages::ImageSource::Url {
                url: "http://example.com".to_string(),
            },
            detail: None,
        }]))
        .build();
    assert_eq!(msg.content.as_text(), "");
}


#[test]
fn test_merge_two_strings() {
    let result = merge_content("Hello", " world");
    assert_eq!(result, "Hello world");
}

#[test]
fn test_merge_string_and_list() {
    let result = merge_content_complex(
        MergeableContent::Text("Hello".to_string()),
        vec![MergeableContent::List(vec![
            json!({"type": "text", "text": " world"}),
        ])],
    );
    match result {
        MergeableContent::List(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], json!("Hello"));
            assert_eq!(items[1], json!({"type": "text", "text": " world"}));
        }
        other => panic!("Expected List, got {:?}", other),
    }
}

#[test]
fn test_merge_list_and_string() {
    let result = merge_content_complex(
        MergeableContent::List(vec![json!({"type": "text", "text": "Hello"})]),
        vec![MergeableContent::Text(" world".to_string())],
    );
    match result {
        MergeableContent::List(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], json!({"type": "text", "text": "Hello"}));
            assert_eq!(items[1], json!(" world"));
        }
        other => panic!("Expected List, got {:?}", other),
    }
}

#[test]
fn test_merge_two_lists() {
    let result = merge_content_complex(
        MergeableContent::List(vec![json!({"type": "text", "text": "Hello"})]),
        vec![MergeableContent::List(vec![
            json!({"type": "text", "text": " world"}),
        ])],
    );
    match result {
        MergeableContent::List(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0]["text"], json!("Hello"));
            assert_eq!(items[1]["text"], json!(" world"));
        }
        other => panic!("Expected List, got {:?}", other),
    }
}

#[test]
fn test_merge_two_lists_with_index() {
    let result = merge_content_complex(
        MergeableContent::List(vec![json!({"type": "text", "text": "Hello", "index": 0})]),
        vec![MergeableContent::List(vec![
            json!({"type": "text", "text": " world", "index": 0}),
        ])],
    );
    match result {
        MergeableContent::List(items) => {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0]["text"], json!("Hello world"));
            assert_eq!(items[0]["index"], json!(0));
        }
        other => panic!("Expected List, got {:?}", other),
    }
}

#[test]
fn test_merge_multiple_strings() {
    let mut result = merge_content("a", "b");
    result = merge_content(&result, "c");
    result = merge_content(&result, "d");
    assert_eq!(result, "abcd");
}

#[test]
fn test_merge_empty_string_first() {
    let result = merge_content("", "Hello");
    assert_eq!(result, "Hello");
}

#[test]
fn test_merge_empty_string_second() {
    let result = merge_content("Hello", "");
    assert_eq!(result, "Hello");
}

#[test]
fn test_merge_list_with_empty_string() {
    let result = merge_content_complex(
        MergeableContent::List(vec![json!({"type": "text", "text": "Hello"})]),
        vec![MergeableContent::Text(String::new())],
    );
    match result {
        MergeableContent::List(items) => {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0], json!({"type": "text", "text": "Hello"}));
        }
        other => panic!("Expected List, got {:?}", other),
    }
}


#[test]
fn test_merge_list_plus_list_last_element_string_concatenates() {
    let result = merge_content_complex(
        MergeableContent::List(vec![json!("Hello")]),
        vec![MergeableContent::Text(" world".to_string())],
    );
    match result {
        MergeableContent::List(items) => {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0], json!("Hello world"));
        }
        other => panic!("Expected List, got {:?}", other),
    }
}

#[test]
fn test_merge_list_plus_string_last_element_dict_appends() {
    let result = merge_content_complex(
        MergeableContent::List(vec![json!({"type": "text", "text": "Hello"})]),
        vec![MergeableContent::Text(" world".to_string())],
    );
    match result {
        MergeableContent::List(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], json!({"type": "text", "text": "Hello"}));
            assert_eq!(items[1], json!(" world"));
        }
        other => panic!("Expected List, got {:?}", other),
    }
}

#[test]
fn test_merge_list_string_last_plus_empty_string() {
    let result = merge_content_complex(
        MergeableContent::List(vec![json!("Hello")]),
        vec![MergeableContent::Text(String::new())],
    );
    match result {
        MergeableContent::List(items) => {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0], json!("Hello"));
        }
        other => panic!("Expected List, got {:?}", other),
    }
}

#[test]
fn test_merge_list_plus_empty_string_no_op() {
    let result = merge_content_complex(
        MergeableContent::List(vec![json!({"type": "text", "text": "Hello"})]),
        vec![MergeableContent::Text(String::new())],
    );
    match result {
        MergeableContent::List(items) => {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0], json!({"type": "text", "text": "Hello"}));
        }
        other => panic!("Expected List, got {:?}", other),
    }
}


#[test]
fn test_message_to_dict_human_message() {
    let msg = HumanMessage::builder()
        .content("Hello")
        .id("msg1".to_string())
        .name("user1".to_string())
        .build();
    let result = message_to_dict(&BaseMessage::Human(msg));
    assert_eq!(result.get("type").unwrap().as_str().unwrap(), "human");
    assert_eq!(
        result
            .get("data")
            .unwrap()
            .get("content")
            .unwrap()
            .as_str()
            .unwrap(),
        "Hello"
    );
    assert_eq!(
        result
            .get("data")
            .unwrap()
            .get("name")
            .unwrap()
            .as_str()
            .unwrap(),
        "user1"
    );
    assert_eq!(
        result
            .get("data")
            .unwrap()
            .get("id")
            .unwrap()
            .as_str()
            .unwrap(),
        "msg1"
    );
}

#[test]
fn test_message_to_dict_ai_message() {
    let msg = AIMessage::builder()
        .content("Hi there")
        .id("ai1".to_string())
        .build();
    let result = message_to_dict(&BaseMessage::AI(msg));
    assert_eq!(result.get("type").unwrap().as_str().unwrap(), "ai");
    assert_eq!(
        result
            .get("data")
            .unwrap()
            .get("content")
            .unwrap()
            .as_str()
            .unwrap(),
        "Hi there"
    );
    assert_eq!(
        result
            .get("data")
            .unwrap()
            .get("id")
            .unwrap()
            .as_str()
            .unwrap(),
        "ai1"
    );
}

#[test]
fn test_message_to_dict_system_message() {
    let msg = SystemMessage::builder()
        .content("You are a helpful assistant")
        .build();
    let result = message_to_dict(&BaseMessage::System(msg));
    assert_eq!(result.get("type").unwrap().as_str().unwrap(), "system");
    assert_eq!(
        result
            .get("data")
            .unwrap()
            .get("content")
            .unwrap()
            .as_str()
            .unwrap(),
        "You are a helpful assistant"
    );
}

#[test]
fn test_message_to_dict_with_additional_kwargs() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert(
        "function_call".to_string(),
        json!({"name": "test", "arguments": "{}"}),
    );

    let msg = AIMessage::builder()
        .content("Hello")
        .additional_kwargs(additional_kwargs)
        .build();
    let result = message_to_dict(&BaseMessage::AI(msg));
    assert_eq!(
        result
            .get("data")
            .unwrap()
            .get("additional_kwargs")
            .unwrap()
            .get("function_call")
            .unwrap()
            .get("name")
            .unwrap()
            .as_str()
            .unwrap(),
        "test"
    );
}

#[test]
fn test_messages_to_dict_multiple_messages() {
    let messages = vec![
        BaseMessage::System(SystemMessage::builder().content("System").build()),
        BaseMessage::Human(HumanMessage::builder().content("Hello").build()),
        BaseMessage::AI(AIMessage::builder().content("Hi").build()),
    ];
    let result = messages_to_dict(&messages);
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].get("type").unwrap().as_str().unwrap(), "system");
    assert_eq!(result[1].get("type").unwrap().as_str().unwrap(), "human");
    assert_eq!(result[2].get("type").unwrap().as_str().unwrap(), "ai");
}

#[test]
fn test_messages_to_dict_empty_list() {
    let messages: Vec<BaseMessage> = vec![];
    let result = messages_to_dict(&messages);
    assert!(result.is_empty());
}


#[test]
fn test_content_blocks_string_content() {
    let msg = HumanMessage::builder().content("Hello").build();
    let blocks = msg.content_blocks();
    assert_eq!(blocks.len(), 1);
    match &blocks[0] {
        ContentBlock::Text(tb) => {
            assert_eq!(tb.block_type, "text");
            assert_eq!(tb.text, "Hello");
        }
        other => panic!("Expected Text block, got {:?}", other),
    }
}

#[test]
fn test_content_blocks_empty_string() {
    let msg = HumanMessage::builder().content("").build();
    let blocks = msg.content_blocks();
    assert!(blocks.is_empty());
}

#[test]
fn test_content_blocks_list_with_string() {
    let msg = HumanMessage::builder()
        .content(MessageContent::Parts(vec![
            ContentPart::Text {
                text: "Hello".to_string(),
            },
            ContentPart::Text {
                text: "world".to_string(),
            },
        ]))
        .build();
    let blocks = msg.content_blocks();
    assert_eq!(blocks.len(), 2);
    match &blocks[0] {
        ContentBlock::Text(tb) => {
            assert_eq!(tb.text, "Hello");
        }
        other => panic!("Expected Text block, got {:?}", other),
    }
    match &blocks[1] {
        ContentBlock::Text(tb) => {
            assert_eq!(tb.text, "world");
        }
        other => panic!("Expected Text block, got {:?}", other),
    }
}

#[test]
fn test_content_blocks_standard_text_block() {
    let msg = HumanMessage::builder()
        .content(MessageContent::Parts(vec![ContentPart::Other(
            json!({"type": "text", "text": "Hello"}),
        )]))
        .build();
    let blocks = msg.content_blocks();
    assert_eq!(blocks.len(), 1);
    match &blocks[0] {
        ContentBlock::Text(tb) => {
            assert_eq!(tb.text, "Hello");
        }
        other => panic!("Expected Text block, got {:?}", other),
    }
}

#[test]
fn test_content_blocks_non_standard_block() {
    let msg = HumanMessage::builder()
        .content(MessageContent::Parts(vec![ContentPart::Other(
            json!({"type": "custom_type", "data": "value"}),
        )]))
        .build();
    let blocks = msg.content_blocks();
    assert_eq!(blocks.len(), 1);
    match &blocks[0] {
        ContentBlock::NonStandard(ns) => {
            assert_eq!(ns.block_type, "non_standard");
            assert_eq!(
                ns.value.get("type").unwrap().as_str().unwrap(),
                "custom_type"
            );
        }
        other => panic!("Expected NonStandard block, got {:?}", other),
    }
}

#[test]
fn test_content_blocks_mixed_content() {
    let msg = HumanMessage::builder()
        .content(MessageContent::Parts(vec![
            ContentPart::Text {
                text: "Plain string".to_string(),
            },
            ContentPart::Other(json!({"type": "text", "text": "Text block"})),
            ContentPart::Other(json!({"type": "image", "url": "http://example.com/img.png"})),
        ]))
        .build();
    let blocks = msg.content_blocks();
    assert_eq!(blocks.len(), 3);
    match &blocks[0] {
        ContentBlock::Text(_) => {}
        other => panic!("Expected Text block at index 0, got {:?}", other),
    }
    match &blocks[1] {
        ContentBlock::Text(_) => {}
        other => panic!("Expected Text block at index 1, got {:?}", other),
    }
    match &blocks[2] {
        ContentBlock::Image(_) => {}
        other => panic!("Expected Image block at index 2, got {:?}", other),
    }
}


#[test]
fn test_dict_with_type_not_in_known_block_types() {
    let msg = HumanMessage::builder()
        .content(MessageContent::Parts(vec![ContentPart::Other(
            json!({"type": "completely_unknown_type_xyz", "payload": {"key": "value"}}),
        )]))
        .build();
    let blocks = msg.content_blocks();
    assert_eq!(blocks.len(), 1);
    match &blocks[0] {
        ContentBlock::NonStandard(ns) => {
            assert_eq!(ns.block_type, "non_standard");
            assert_eq!(
                ns.value.get("type").unwrap().as_str().unwrap(),
                "completely_unknown_type_xyz"
            );
        }
        other => panic!("Expected NonStandard block, got {:?}", other),
    }
}

#[test]
fn test_dict_with_no_type_key() {
    let msg = HumanMessage::builder()
        .content(MessageContent::Parts(vec![ContentPart::Other(
            json!({"data": "some data", "format": "raw"}),
        )]))
        .build();
    let blocks = msg.content_blocks();
    assert_eq!(blocks.len(), 1);
    match &blocks[0] {
        ContentBlock::NonStandard(ns) => {
            assert_eq!(ns.block_type, "non_standard");
        }
        other => panic!("Expected NonStandard block, got {:?}", other),
    }
}


#[test]
fn test_add_human_message_chunks() {
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
fn test_add_system_message_chunks() {
    let chunk1 = SystemMessageChunk::builder().content("You are").build();
    let chunk2 = SystemMessageChunk::builder().content(" helpful").build();
    let result = chunk1 + chunk2;
    assert_eq!(result.content.as_text(), "You are helpful");
}

#[test]
fn test_add_chunks_with_additional_kwargs() {
    let mut kwargs1 = std::collections::HashMap::new();
    kwargs1.insert("key1".to_string(), json!("value1"));

    let mut kwargs2 = std::collections::HashMap::new();
    kwargs2.insert("key2".to_string(), json!("value2"));

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
        &json!("value1")
    );
    assert_eq!(
        result.additional_kwargs.get("key2").unwrap(),
        &json!("value2")
    );
}

#[test]
fn test_add_chunks_with_response_metadata() {
    let mut meta1 = std::collections::HashMap::new();
    meta1.insert("meta1".to_string(), json!("data1"));

    let mut meta2 = std::collections::HashMap::new();
    meta2.insert("meta2".to_string(), json!("data2"));

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
        &json!("data1")
    );
    assert_eq!(
        result.response_metadata.get("meta2").unwrap(),
        &json!("data2")
    );
}

#[test]
fn test_add_chunk_list_content() {
    let chunk1 = HumanMessageChunk::builder()
        .content(MessageContent::Parts(vec![ContentPart::Other(
            json!({"type": "text", "text": "Hello"}),
        )]))
        .build();
    let chunk2 = HumanMessageChunk::builder()
        .content(MessageContent::Parts(vec![ContentPart::Other(
            json!({"type": "text", "text": " world"}),
        )]))
        .build();
    let result = chunk1 + chunk2;
    match &result.content {
        MessageContent::Parts(parts) => {
            assert_eq!(parts.len(), 2);
        }
        other => panic!("Expected Parts content, got {:?}", other),
    }
}


#[test]
fn test_add_list_of_mixed_message_chunks() {
    let chunk1 = HumanMessageChunk::builder()
        .content("Start")
        .id("main".to_string())
        .build();
    let others = vec![
        HumanMessageChunk::builder().content(" middle").build(),
        HumanMessageChunk::builder().content(" end").build(),
    ];
    let result = others.into_iter().fold(chunk1, |acc, c| acc + c);
    assert_eq!(result.content.as_text(), "Start middle end");
    assert_eq!(result.id, Some("main".to_string()));
}

#[test]
fn test_add_list_of_chunks_with_metadata() {
    let mut kwargs1 = std::collections::HashMap::new();
    kwargs1.insert("key1".to_string(), json!("val1"));
    let mut meta1 = std::collections::HashMap::new();
    meta1.insert("meta1".to_string(), json!("data1"));

    let chunk1 = HumanMessageChunk::builder()
        .content("a")
        .id("1".to_string())
        .additional_kwargs(kwargs1)
        .response_metadata(meta1)
        .build();

    let mut kwargs2 = std::collections::HashMap::new();
    kwargs2.insert("key2".to_string(), json!("val2"));
    let mut meta2 = std::collections::HashMap::new();
    meta2.insert("meta2".to_string(), json!("data2"));

    let chunk2 = HumanMessageChunk::builder()
        .content("b")
        .additional_kwargs(kwargs2)
        .response_metadata(meta2)
        .build();

    let mut kwargs3 = std::collections::HashMap::new();
    kwargs3.insert("key3".to_string(), json!("val3"));
    let mut meta3 = std::collections::HashMap::new();
    meta3.insert("meta3".to_string(), json!("data3"));

    let chunk3 = HumanMessageChunk::builder()
        .content("c")
        .additional_kwargs(kwargs3)
        .response_metadata(meta3)
        .build();

    let result = vec![chunk2, chunk3]
        .into_iter()
        .fold(chunk1, |acc, c| acc + c);

    assert_eq!(result.content.as_text(), "abc");
    assert_eq!(result.id, Some("1".to_string()));
    assert_eq!(
        result.additional_kwargs.get("key1").unwrap(),
        &json!("val1")
    );
    assert_eq!(
        result.additional_kwargs.get("key2").unwrap(),
        &json!("val2")
    );
    assert_eq!(
        result.additional_kwargs.get("key3").unwrap(),
        &json!("val3")
    );
    assert_eq!(
        result.response_metadata.get("meta1").unwrap(),
        &json!("data1")
    );
    assert_eq!(
        result.response_metadata.get("meta2").unwrap(),
        &json!("data2")
    );
    assert_eq!(
        result.response_metadata.get("meta3").unwrap(),
        &json!("data3")
    );
}

#[test]
fn test_add_single_element_list() {
    let chunk1 = HumanMessageChunk::builder()
        .content("Hello")
        .id("x".to_string())
        .build();
    let chunk2 = HumanMessageChunk::builder().content(" World").build();
    let result = chunk1 + chunk2;
    assert_eq!(result.content.as_text(), "Hello World");
    assert_eq!(result.id, Some("x".to_string()));
}


#[test]
fn test_pretty_repr_basic() {
    let msg = HumanMessage::builder().content("Hello").build();
    let result = msg.pretty_repr(false);
    assert!(result.contains("Human Message"));
    assert!(result.contains("Hello"));
}

#[test]
fn test_pretty_repr_with_name() {
    let msg = HumanMessage::builder()
        .content("Hello")
        .name("user1".to_string())
        .build();
    let result = msg.pretty_repr(false);
    assert!(result.contains("Name: user1"));
}

#[test]
fn test_pretty_repr_html_mode() {
    let msg = HumanMessage::builder().content("Hello").build();
    let result = msg.pretty_repr(true);
    assert!(result.contains("Human Message"));
}


#[test]
fn test_pretty_print_does_not_raise_human() {
    let msg = HumanMessage::builder()
        .content("Hello, how are you?")
        .build();
    msg.pretty_print(); // Should not panic
}

#[test]
fn test_pretty_print_does_not_raise_ai() {
    let msg = AIMessage::builder()
        .content("I'm doing well, thanks!")
        .build();
    let base = BaseMessage::AI(msg);
    base.pretty_print(); // Should not panic
}

#[test]
fn test_pretty_print_does_not_raise_system() {
    let msg = SystemMessage::builder()
        .content("You are a helpful assistant.")
        .build();
    msg.pretty_print(); // Should not panic
}

#[test]
fn test_pretty_print_does_not_raise_with_name() {
    let msg = HumanMessage::builder()
        .content("Hello")
        .name("user1".to_string())
        .build();
    msg.pretty_print(); // Should not panic
}

#[test]
fn test_pretty_print_does_not_raise_empty_content() {
    let msg = HumanMessage::builder().content("").build();
    msg.pretty_print(); // Should not panic
}

#[test]
fn test_pretty_print_does_not_raise_list_content() {
    let msg = HumanMessage::builder()
        .content(MessageContent::Parts(vec![ContentPart::Other(
            json!({"type": "text", "text": "Hello"}),
        )]))
        .build();
    msg.pretty_print(); // Should not panic
}


#[test]
fn test_get_msg_title_repr_basic() {
    let result = get_msg_title_repr("Test Title", false);
    assert!(result.contains("Test Title"));
    assert!(result.contains("="));
}

#[test]
fn test_get_msg_title_repr_bold() {
    let result = get_msg_title_repr("Test Title", true);
    assert!(result.contains("Test Title"));
}

#[test]
fn test_get_msg_title_repr_long_title() {
    let long_title = "A".repeat(100);
    let result = get_msg_title_repr(&long_title, false);
    assert!(result.contains(&long_title));
}

#[test]
fn test_get_msg_title_repr_moderate_title() {
    let title = "A".repeat(40);
    let result = get_msg_title_repr(&title, false);
    assert!(result.contains(&title));
}


#[test]
fn test_even_length_title_symmetric_padding() {
    let result = get_msg_title_repr("AB", false);
    assert!(result.contains("AB"));
    assert_eq!(result.len(), 80);
    let left_sep = result.split(" AB ").next().unwrap();
    let right_sep = result.split(" AB ").nth(1).unwrap();
    assert_eq!(left_sep.len(), 38);
    assert_eq!(right_sep.len(), 38);
}

#[test]
fn test_odd_length_title_asymmetric_padding() {
    let result = get_msg_title_repr("ABC", false);
    assert!(result.contains("ABC"));
    let left_sep = result.split(" ABC ").next().unwrap();
    let right_sep = result.split(" ABC ").nth(1).unwrap();
    assert_eq!(left_sep.len(), 37);
    assert_eq!(right_sep.len(), 38);
    assert_eq!(result.len(), 80);
}

#[test]
fn test_single_char_title() {
    let result = get_msg_title_repr("X", false);
    assert!(result.contains(" X "));
    let left_sep = result.split(" X ").next().unwrap();
    let right_sep = result.split(" X ").nth(1).unwrap();
    assert_eq!(left_sep.len(), 38);
    assert_eq!(right_sep.len(), 39);
    assert_eq!(result.len(), 80);
}

#[test]
fn test_empty_title() {
    let result = get_msg_title_repr("", false);
    assert_eq!(result.len(), 80);
}

#[test]
fn test_bold_does_not_change_content() {
    let result = get_msg_title_repr("Test", true);
    assert!(result.contains("Test"));
}

#[test]
fn test_known_title_exact_output() {
    let result = get_msg_title_repr("Human Message", false);
    let expected_left = "=".repeat(32);
    let expected_right = "=".repeat(33);
    assert_eq!(
        result,
        format!("{} Human Message {}", expected_left, expected_right)
    );
    assert_eq!(result.len(), 80);
}


#[test]
fn test_init_with_content_blocks() {
    let blocks = vec![
        ContentBlock::Text(TextContentBlock::new("Hello")),
        ContentBlock::Image(agent_chain_core::messages::ImageContentBlock::from_url(
            "http://example.com/img.png",
        )),
    ];
    let msg = HumanMessage::builder()
        .content("")
        .content_blocks(blocks)
        .build();
    match &msg.content {
        MessageContent::Parts(parts) => {
            assert_eq!(parts.len(), 2);
        }
        other => panic!("Expected Parts content, got {:?}", other),
    }
}

#[test]
fn test_init_with_string_content() {
    let msg = HumanMessage::builder().content("Hello world").build();
    assert_eq!(msg.content.as_text(), "Hello world");
}

#[test]
fn test_init_with_list_content() {
    let content = MessageContent::Parts(vec![ContentPart::Other(
        json!({"type": "text", "text": "Hello"}),
    )]);
    let msg = HumanMessage::builder().content(content).build();
    match &msg.content {
        MessageContent::Parts(parts) => {
            assert_eq!(parts.len(), 1);
        }
        other => panic!("Expected Parts content, got {:?}", other),
    }
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
fn test_init_with_name() {
    let msg = HumanMessage::builder()
        .content("Hello")
        .name("user1".to_string())
        .build();
    assert_eq!(msg.name, Some("user1".to_string()));
}

#[test]
fn test_init_with_additional_kwargs() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("custom_key".to_string(), json!("custom_value"));

    let msg = HumanMessage::builder()
        .content("Hello")
        .additional_kwargs(additional_kwargs)
        .build();
    assert_eq!(
        msg.additional_kwargs.get("custom_key").unwrap(),
        &json!("custom_value")
    );
}

#[test]
fn test_init_with_response_metadata() {
    let mut response_metadata = std::collections::HashMap::new();
    response_metadata.insert("model".to_string(), json!("gpt-4"));
    response_metadata.insert("tokens".to_string(), json!(10));

    let msg = AIMessage::builder()
        .content("Hello")
        .response_metadata(response_metadata)
        .build();
    assert_eq!(msg.response_metadata.get("model").unwrap(), &json!("gpt-4"));
    assert_eq!(msg.response_metadata.get("tokens").unwrap(), &json!(10));
}


#[test]
fn test_message_types_have_consistent_types() {
    let human_msg = HumanMessage::builder().content("Hello").build();
    let ai_msg = AIMessage::builder().content("Hi").build();
    let system_msg = SystemMessage::builder().content("You are helpful").build();

    assert_eq!(human_msg.message_type(), "human");
    assert_eq!(ai_msg.message_type(), "ai");
    assert_eq!(system_msg.message_type(), "system");
}


#[test]
fn test_string_reasoning_content_returns_reasoning_block() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert(
        "reasoning_content".to_string(),
        json!("I think therefore I am"),
    );
    let result = extract_reasoning_from_additional_kwargs(&additional_kwargs);
    assert!(result.is_some());
    let block = result.unwrap();
    assert_eq!(block.block_type, "reasoning");
    assert_eq!(block.reasoning(), Some("I think therefore I am"));
}

#[test]
fn test_none_reasoning_content_returns_none() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("reasoning_content".to_string(), json!(null));
    let result = extract_reasoning_from_additional_kwargs(&additional_kwargs);
    assert!(result.is_none());
}

#[test]
fn test_non_string_reasoning_content_returns_none() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert(
        "reasoning_content".to_string(),
        json!({"nested": "data", "value": 42}),
    );
    let result = extract_reasoning_from_additional_kwargs(&additional_kwargs);
    assert!(result.is_none());
}

#[test]
fn test_no_reasoning_content_key_returns_none() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("other_key".to_string(), json!("value"));
    let result = extract_reasoning_from_additional_kwargs(&additional_kwargs);
    assert!(result.is_none());
}

#[test]
fn test_empty_additional_kwargs_returns_none() {
    let additional_kwargs = std::collections::HashMap::new();
    let result = extract_reasoning_from_additional_kwargs(&additional_kwargs);
    assert!(result.is_none());
}
