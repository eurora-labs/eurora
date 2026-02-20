use agent_chain_core::messages::{AIMessage, BaseMessage, HumanMessage, RemoveMessage};

#[test]
fn test_init_basic() {
    let msg = RemoveMessage::builder().id("msg-to-remove").build();
    assert_eq!(msg.id, "msg-to-remove");
    assert_eq!(msg.message_type(), "remove");
    assert_eq!(msg.content(), "");
}

#[test]
fn test_type_is_remove() {
    let msg = RemoveMessage::builder().id("msg-123").build();
    assert_eq!(msg.message_type(), "remove");
}

#[test]
fn test_content_is_empty_string() {
    let msg = RemoveMessage::builder().id("msg-123").build();
    assert_eq!(msg.content(), "");
}

#[test]
fn test_serialization_roundtrip() {
    let msg = RemoveMessage::builder().id("msg-to-remove").build();

    let serialized = serde_json::to_value(&msg).unwrap();
    assert_eq!(serialized.get("type").unwrap().as_str().unwrap(), "remove");

    let deserialized: RemoveMessage = serde_json::from_value(serialized).unwrap();
    assert_eq!(deserialized.id, "msg-to-remove");
    assert_eq!(deserialized.content(), "");
}

#[test]
fn test_with_name() {
    let msg = RemoveMessage::builder()
        .id("msg-123")
        .name("delete-marker".to_string())
        .build();
    assert_eq!(msg.name, Some("delete-marker".to_string()));
    assert_eq!(msg.id, "msg-123");
}

#[test]
fn test_with_additional_kwargs() {
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert("reason".to_string(), serde_json::json!("outdated"));

    let msg = RemoveMessage::builder()
        .id("msg-123")
        .additional_kwargs(additional_kwargs)
        .build();
    assert_eq!(
        msg.additional_kwargs.get("reason").unwrap(),
        &serde_json::json!("outdated")
    );
}

#[test]
fn test_with_response_metadata() {
    let mut response_metadata = std::collections::HashMap::new();
    response_metadata.insert("deleted_at".to_string(), serde_json::json!("2024-01-01"));

    let msg = RemoveMessage::builder()
        .id("msg-123")
        .response_metadata(response_metadata)
        .build();
    assert_eq!(
        msg.response_metadata.get("deleted_at").unwrap(),
        &serde_json::json!("2024-01-01")
    );
}

#[test]
fn test_content_property_is_empty() {
    let msg = RemoveMessage::builder().id("msg-123").build();
    assert_eq!(msg.content(), "");
}

#[test]
fn test_multiple_remove_messages() {
    let msg1 = RemoveMessage::builder().id("msg-1").build();
    let msg2 = RemoveMessage::builder().id("msg-2").build();
    let msg3 = RemoveMessage::builder().id("msg-3").build();

    assert_eq!(msg1.id, "msg-1");
    assert_eq!(msg2.id, "msg-2");
    assert_eq!(msg3.id, "msg-3");

    assert_eq!(msg1.content(), "");
    assert_eq!(msg2.content(), "");
    assert_eq!(msg3.content(), "");
}

#[test]
fn test_remove_message_in_list() {
    let messages = [
        BaseMessage::Human(
            HumanMessage::builder()
                .id("human-1".to_string())
                .content("Hello")
                .build(),
        ),
        BaseMessage::AI(
            AIMessage::builder()
                .content("Hi there!")
                .id("ai-1".to_string())
                .build(),
        ),
        BaseMessage::Remove(RemoveMessage::builder().id("human-1").build()),
    ];

    assert_eq!(messages.len(), 3);
    assert!(matches!(messages[2], BaseMessage::Remove(_)));
    assert_eq!(messages[2].id(), Some("human-1".to_string()));
}

#[test]
fn test_remove_message_serialization_in_list() {
    let messages = [
        BaseMessage::Human(
            HumanMessage::builder()
                .id("human-1".to_string())
                .content("Hello")
                .build(),
        ),
        BaseMessage::Remove(RemoveMessage::builder().id("human-1").build()),
    ];

    let serialized: Vec<serde_json::Value> = messages
        .iter()
        .map(|m| serde_json::to_value(m).unwrap())
        .collect();

    assert_eq!(serialized.len(), 2);
    assert_eq!(
        serialized[0].get("type").unwrap().as_str().unwrap(),
        "human"
    );
    assert_eq!(
        serialized[1].get("type").unwrap().as_str().unwrap(),
        "remove"
    );

    let deserialized: Vec<BaseMessage> = serialized
        .into_iter()
        .map(|s| serde_json::from_value(s).unwrap())
        .collect();

    assert!(matches!(deserialized[0], BaseMessage::Human(_)));
    assert!(matches!(deserialized[1], BaseMessage::Remove(_)));
    assert_eq!(deserialized[1].id(), Some("human-1".to_string()));
}

#[test]
fn test_remove_message_does_not_modify_content() {
    let msg = RemoveMessage::builder().id("msg-123").build();

    assert_eq!(msg.content(), "");

    assert!(msg.content_blocks().is_empty());
}

#[test]
fn test_text_property_is_empty() {
    let msg = RemoveMessage::builder().id("msg-123").build();
    assert_eq!(msg.text(), "");
}

#[test]
fn test_content_blocks_property_is_empty() {
    let msg = RemoveMessage::builder().id("msg-123").build();
    let blocks = msg.content_blocks();
    assert!(blocks.is_empty());
}

#[test]
fn test_pretty_repr() {
    let msg = RemoveMessage::builder().id("msg-123").build();
    let result = msg.pretty_repr(false);
    assert!(result.contains("Remove Message"));
}

#[test]
fn test_pretty_repr_html() {
    let msg = RemoveMessage::builder().id("html-test").build();
    let result = msg.pretty_repr(true);
    assert!(result.contains("Remove Message"));
    assert!(result.contains("\x1b[1m"));
}

#[test]
fn test_model_dump_snapshot() {
    let msg = RemoveMessage::builder().id("msg-dump-1").build();
    let dumped = serde_json::to_value(&msg).unwrap();
    let obj = dumped.as_object().unwrap();

    assert!(obj.contains_key("content"));
    assert!(obj.contains_key("id"));
    assert!(obj.contains_key("type"));
    assert!(obj.contains_key("additional_kwargs"));
    assert!(obj.contains_key("response_metadata"));

    assert_eq!(obj["content"].as_str().unwrap(), "");
    assert_eq!(obj["id"].as_str().unwrap(), "msg-dump-1");
    assert_eq!(obj["type"].as_str().unwrap(), "remove");
}

#[test]
fn test_model_dump_with_name() {
    let msg = RemoveMessage::builder()
        .id("msg-dump-2")
        .name("marker".to_string())
        .build();
    let dumped = serde_json::to_value(&msg).unwrap();
    let obj = dumped.as_object().unwrap();
    assert_eq!(obj["name"].as_str().unwrap(), "marker");
}

#[test]
fn test_same_id_equal() {
    let msg1 = RemoveMessage::builder().id("same-id").build();
    let msg2 = RemoveMessage::builder().id("same-id").build();
    assert_eq!(msg1, msg2);
}

#[test]
fn test_different_id_not_equal() {
    let msg1 = RemoveMessage::builder().id("id-a").build();
    let msg2 = RemoveMessage::builder().id("id-b").build();
    assert_ne!(msg1, msg2);
}
