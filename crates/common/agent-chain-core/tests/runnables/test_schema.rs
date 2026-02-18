//! Tests for runnable schema types and structures.
//!
//! Mirrors `langchain/libs/core/tests/unit_tests/runnables/test_schema.py`

use std::collections::HashMap;

use agent_chain_core::runnables::schema::{
    BaseStreamEvent, CUSTOM_EVENT_TYPE, CustomStreamEvent, EventData, StandardStreamEvent,
    StreamEvent,
};
use serde_json::json;

/// Mirrors `test_event_data_structure`.
#[test]
fn test_event_data_structure() {
    let data = EventData::new()
        .with_input(json!({"question": "test"}))
        .with_output(json!({"answer": "response"}))
        .with_chunk(json!({"partial": "data"}));

    assert_eq!(data.input, Some(json!({"question": "test"})));
    assert_eq!(data.output, Some(json!({"answer": "response"})));
    assert_eq!(data.chunk, Some(json!({"partial": "data"})));

    let minimal = EventData::new();
    assert!(minimal.input.is_none());
    assert!(minimal.output.is_none());
    assert!(minimal.chunk.is_none());
    assert!(minimal.error.is_none());
}

/// Mirrors `test_event_data_with_error`.
#[test]
fn test_event_data_with_error() {
    let data = EventData::new()
        .with_input(json!("test"))
        .with_error("Test error");

    assert_eq!(data.error, Some("Test error".to_string()));
    assert_eq!(data.input, Some(json!("test")));
}

/// Mirrors `test_event_data_empty`.
#[test]
fn test_event_data_empty() {
    let data = EventData::new();
    let json_str = serde_json::to_string(&data).unwrap();
    assert_eq!(json_str, "{}");
}

/// Mirrors `test_event_data_chunk_field`.
#[test]
fn test_event_data_chunk_field() {
    let data = EventData::new().with_chunk(json!("partial output"));
    assert_eq!(data.chunk, Some(json!("partial output")));

    let chunk_list = EventData::new().with_chunk(json!([1, 2, 3]));
    assert_eq!(chunk_list.chunk, Some(json!([1, 2, 3])));
}

/// Mirrors `test_event_data_supports_various_input_types`.
#[test]
fn test_event_data_supports_various_input_types() {
    let data1 = EventData::new().with_input(json!("simple string"));
    assert_eq!(data1.input, Some(json!("simple string")));

    let data2 = EventData::new().with_input(json!({"key": "value"}));
    assert_eq!(data2.input.as_ref().unwrap()["key"], json!("value"));

    let data3 = EventData::new().with_input(json!([1, 2, 3]));
    assert_eq!(data3.input, Some(json!([1, 2, 3])));

    let data4 = EventData::new().with_input(json!({"field1": "test", "field2": 42}));
    assert_eq!(data4.input.as_ref().unwrap()["field1"], json!("test"));
    assert_eq!(data4.input.as_ref().unwrap()["field2"], json!(42));
}

/// Mirrors `test_event_data_supports_various_output_types`.
#[test]
fn test_event_data_supports_various_output_types() {
    let data1 = EventData::new().with_output(json!("result"));
    assert_eq!(data1.output, Some(json!("result")));

    let data2 = EventData::new().with_output(json!({"result": "value"}));
    assert_eq!(data2.output.as_ref().unwrap()["result"], json!("value"));

    let data3 = EventData::new().with_output(json!([1, 2, 3]));
    assert_eq!(data3.output, Some(json!([1, 2, 3])));
}

/// Mirrors `test_standard_event_with_all_data_fields`.
#[test]
fn test_standard_event_with_all_data_fields() {
    let data = EventData::new()
        .with_input(json!({"query": "test"}))
        .with_output(json!({"result": "answer"}))
        .with_chunk(json!({"partial": "data"}))
        .with_error("test error");

    assert!(data.input.is_some());
    assert!(data.output.is_some());
    assert!(data.chunk.is_some());
    assert!(data.error.is_some());
}

/// Mirrors `test_event_data_with_base_message`.
///
/// In Rust, messages are serialized to JSON Value.
#[test]
fn test_event_data_with_messages() {
    use agent_chain_core::messages::{AIMessage, BaseMessage, HumanMessage};

    let human = BaseMessage::Human(HumanMessage::builder().content("hello").build());
    let ai = BaseMessage::AI(AIMessage::builder().content("hi there").build());

    let input_val = serde_json::to_value([&human]).unwrap();
    let output_val = serde_json::to_value(&ai).unwrap();
    let chunk_val = serde_json::to_value(&ai).unwrap();

    let data = EventData::new()
        .with_input(input_val.clone())
        .with_output(output_val.clone())
        .with_chunk(chunk_val);

    assert!(data.input.is_some());
    assert!(data.output.is_some());
    assert!(data.chunk.is_some());
}

/// Mirrors `test_event_with_multiple_chunks`.
#[test]
fn test_event_with_multiple_chunks() {
    let chunks = [
        EventData::new().with_chunk(json!("Hello")),
        EventData::new().with_chunk(json!(" ")),
        EventData::new().with_chunk(json!("World")),
        EventData::new().with_chunk(json!("!")),
    ];

    let accumulated: String = chunks
        .iter()
        .filter_map(|c| c.chunk.as_ref().and_then(|v| v.as_str()))
        .collect();
    assert_eq!(accumulated, "Hello World!");
}

/// Mirrors `test_base_stream_event_structure`.
#[test]
fn test_base_stream_event_structure() {
    let event = BaseStreamEvent::new("on_chain_start", "test-run-id");
    assert_eq!(event.event, "on_chain_start");
    assert_eq!(event.run_id, "test-run-id");
    assert!(event.parent_ids.is_empty());
}

/// Mirrors `test_base_stream_event_with_optional_fields`.
#[test]
fn test_base_stream_event_with_optional_fields() {
    let event = BaseStreamEvent::new("on_chain_end", "test-run-id")
        .with_parent_ids(vec!["parent-1".into(), "parent-2".into()])
        .with_tags(vec!["tag1".into(), "tag2".into()])
        .with_metadata(HashMap::from([("key".into(), json!("value"))]));

    assert_eq!(event.tags, vec!["tag1", "tag2"]);
    assert_eq!(event.metadata["key"], json!("value"));
    assert_eq!(event.parent_ids.len(), 2);
}

/// Mirrors `test_parent_ids_hierarchy`.
#[test]
fn test_parent_ids_hierarchy() {
    let root = BaseStreamEvent::new("on_chain_start", "root-id");
    assert!(root.parent_ids.is_empty());

    let child =
        BaseStreamEvent::new("on_chain_start", "child-id").with_parent_ids(vec!["root-id".into()]);
    assert_eq!(child.parent_ids, vec!["root-id"]);

    let grandchild = BaseStreamEvent::new("on_chain_start", "grandchild-id")
        .with_parent_ids(vec!["root-id".into(), "child-id".into()]);
    assert_eq!(grandchild.parent_ids, vec!["root-id", "child-id"]);
}

/// Mirrors `test_event_parent_ids_can_be_nested` (deep hierarchy).
#[test]
fn test_event_parent_ids_can_be_nested() {
    let parent_chain: Vec<String> = (0..10).map(|i| format!("parent-{i}")).collect();

    let event = BaseStreamEvent::new("on_chain_start", "leaf-id").with_parent_ids(parent_chain);

    assert_eq!(event.parent_ids.len(), 10);
    assert_eq!(event.parent_ids[0], "parent-0");
    assert_eq!(event.parent_ids[9], "parent-9");
}

/// Mirrors `test_event_run_id_format`.
#[test]
fn test_event_run_id_format() {
    let run_id = uuid::Uuid::new_v4().to_string();
    let event = BaseStreamEvent::new("on_chain_start", &run_id);
    assert_eq!(event.run_id, run_id);
    assert!(uuid::Uuid::parse_str(&event.run_id).is_ok());
}

/// Mirrors `test_standard_stream_event_structure`.
#[test]
fn test_standard_stream_event_structure() {
    let event = StandardStreamEvent::new("on_llm_start", "test-run-id", "TestLLM")
        .with_data(EventData::new().with_input(json!("test input")));

    assert_eq!(event.name, "TestLLM");
    assert_eq!(event.data.input, Some(json!("test input")));
}

/// Mirrors `test_standard_stream_event_types`.
#[test]
fn test_standard_stream_event_types() {
    let event_types = [
        "on_llm_start",
        "on_llm_stream",
        "on_llm_end",
        "on_chat_model_start",
        "on_chat_model_stream",
        "on_chat_model_end",
        "on_chain_start",
        "on_chain_stream",
        "on_chain_end",
        "on_tool_start",
        "on_tool_stream",
        "on_tool_end",
        "on_retriever_start",
        "on_retriever_stream",
        "on_retriever_end",
        "on_prompt_start",
        "on_prompt_end",
    ];

    for event_type in &event_types {
        let event = StandardStreamEvent::new(*event_type, "test-id", "test");
        assert_eq!(event.base.event, *event_type);
    }
}

/// Mirrors `test_standard_event_naming_convention`.
#[test]
fn test_standard_event_naming_convention() {
    let valid_patterns = [
        "on_llm_start",
        "on_llm_stream",
        "on_llm_end",
        "on_chat_model_start",
        "on_chat_model_stream",
        "on_chat_model_end",
        "on_chain_start",
        "on_chain_stream",
        "on_chain_end",
        "on_tool_start",
        "on_tool_end",
        "on_retriever_start",
        "on_retriever_end",
        "on_prompt_start",
        "on_prompt_end",
    ];

    for pattern in &valid_patterns {
        assert!(pattern.starts_with("on_"));
        let parts: Vec<&str> = pattern.split('_').collect();
        assert_eq!(parts[0], "on");
        assert!(["start", "stream", "end"].contains(parts.last().unwrap()));
    }
}

/// Mirrors `test_metadata_serializable`.
#[test]
fn test_metadata_serializable() {
    let metadata = HashMap::from([
        ("string".into(), json!("value")),
        ("number".into(), json!(42)),
        ("boolean".into(), json!(true)),
        ("null".into(), json!(null)),
        ("nested".into(), json!({"key": "value"})),
        ("list".into(), json!([1, 2, 3])),
    ]);

    let event =
        StandardStreamEvent::new("on_chain_start", "id", "test").with_metadata(metadata.clone());

    let serialized = serde_json::to_string(&event.base.metadata).unwrap();
    let deserialized: HashMap<String, serde_json::Value> =
        serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized["string"], json!("value"));
    assert_eq!(deserialized["number"], json!(42));
    assert_eq!(deserialized["boolean"], json!(true));
    assert_eq!(deserialized["null"], json!(null));
}

/// Mirrors `test_tags_list_of_strings`.
#[test]
fn test_tags_list_of_strings() {
    let event = StandardStreamEvent::new("on_chain_start", "id", "test").with_tags(vec![
        "tag1".into(),
        "tag2".into(),
        "tag3".into(),
    ]);

    assert_eq!(event.base.tags.len(), 3);
    for tag in &event.base.tags {
        assert!(!tag.is_empty());
    }
}

/// Mirrors `test_event_metadata_empty_dict`.
#[test]
fn test_event_metadata_empty_dict() {
    let event =
        StandardStreamEvent::new("on_chain_start", "id", "test").with_metadata(HashMap::new());
    assert!(event.base.metadata.is_empty());
}

/// Mirrors `test_event_tags_empty_list`.
#[test]
fn test_event_tags_empty_list() {
    let event = StandardStreamEvent::new("on_chain_start", "id", "test").with_tags(vec![]);
    assert!(event.base.tags.is_empty());
}

/// Mirrors `test_event_minimal_required_fields`.
#[test]
fn test_event_minimal_required_fields() {
    let base = BaseStreamEvent::new("on_chain_start", "id");
    assert_eq!(base.event, "on_chain_start");
    assert_eq!(base.run_id, "id");

    let standard = StandardStreamEvent::new("on_chain_start", "id", "test");
    assert_eq!(standard.name, "test");
    assert!(standard.data.input.is_none());

    let custom = CustomStreamEvent::new("id", "custom", json!("any"));
    assert_eq!(custom.base.event, "on_custom_event");
}

/// Mirrors `test_event_all_optional_fields`.
#[test]
fn test_event_all_optional_fields() {
    let event = StandardStreamEvent::new("on_chain_start", "test-run-id", "TestChain")
        .with_parent_ids(vec!["parent-1".into(), "parent-2".into()])
        .with_tags(vec!["tag1".into(), "tag2".into()])
        .with_metadata(HashMap::from([
            ("version".into(), json!("1.0")),
            ("environment".into(), json!("test")),
        ]))
        .with_data(
            EventData::new()
                .with_input(json!({"query": "test"}))
                .with_output(json!({"response": "result"}))
                .with_chunk(json!({"partial": "data"})),
        );

    assert_eq!(event.base.parent_ids.len(), 2);
    assert_eq!(event.base.tags.len(), 2);
    assert_eq!(event.base.metadata.len(), 2);
    assert!(event.data.input.is_some());
    assert!(event.data.output.is_some());
    assert!(event.data.chunk.is_some());
}

/// Mirrors `test_event_metadata_nested_structure`.
#[test]
fn test_event_metadata_nested_structure() {
    let metadata = HashMap::from([
        (
            "model_info".into(),
            json!({
                "provider": "openai",
                "model": "gpt-4",
                "parameters": {
                    "temperature": 0.7,
                    "max_tokens": 100
                }
            }),
        ),
        (
            "user_info".into(),
            json!({
                "user_id": "123",
                "session_id": "456"
            }),
        ),
    ]);

    let event = StandardStreamEvent::new("on_llm_start", "id", "llm").with_metadata(metadata);

    assert_eq!(
        event.base.metadata["model_info"]["provider"],
        json!("openai")
    );
    assert_eq!(event.base.metadata["user_info"]["user_id"], json!("123"));
}

/// Mirrors `test_standard_event_data_field_required`.
#[test]
fn test_standard_event_data_field_required() {
    let event = StandardStreamEvent::new("on_chain_start", "id", "test");
    let _ = &event.data;
}

/// Mirrors `test_event_tags_inherited_from_parent`.
#[test]
fn test_event_tags_inherited_from_parent() {
    let _parent = StandardStreamEvent::new("on_chain_start", "parent", "parent")
        .with_tags(vec!["parent-tag".into()]);

    let child = StandardStreamEvent::new("on_chain_start", "child", "child")
        .with_parent_ids(vec!["parent".into()])
        .with_tags(vec!["parent-tag".into(), "child-tag".into()]);

    assert!(child.base.tags.contains(&"parent-tag".to_string()));
    assert!(child.base.tags.contains(&"child-tag".to_string()));
}

/// Mirrors `test_custom_stream_event_structure`.
#[test]
fn test_custom_stream_event_structure() {
    let event = CustomStreamEvent::new(
        "test-run-id",
        "my_custom_event",
        json!({"custom_field": "custom_value"}),
    );

    assert_eq!(event.base.event, "on_custom_event");
    assert_eq!(event.name, "my_custom_event");
    assert_eq!(event.data["custom_field"], json!("custom_value"));
}

/// Mirrors `test_custom_stream_event_with_any_data`.
#[test]
fn test_custom_stream_event_with_any_data() {
    let event1 = CustomStreamEvent::new("id1", "event1", json!("string data"));
    assert_eq!(event1.data, json!("string data"));

    let event2 = CustomStreamEvent::new("id2", "event2", json!([1, 2, 3]));
    assert_eq!(event2.data, json!([1, 2, 3]));

    let event3 = CustomStreamEvent::new(
        "id3",
        "event3",
        json!({"nested": {"deeply": {"value": 42}}}),
    );
    assert_eq!(event3.data["nested"]["deeply"]["value"], json!(42));
}

/// Mirrors `test_custom_event_must_be_on_custom_event`.
#[test]
fn test_custom_event_must_be_on_custom_event() {
    let event = CustomStreamEvent::new("id", "my_event", json!({}));
    assert_eq!(event.base.event, CUSTOM_EVENT_TYPE);
    assert_eq!(event.base.event, "on_custom_event");
}

/// Mirrors `test_custom_event_name_can_be_any_string`.
#[test]
fn test_custom_event_name_can_be_any_string() {
    let names = [
        "my_custom_event",
        "progress_update",
        "step_completed",
        "intermediate_result",
        "debug_info",
    ];

    for name in &names {
        let event = CustomStreamEvent::new("id", *name, json!({}));
        assert_eq!(event.name, *name);
    }
}

/// Mirrors `test_custom_event_data_can_be_none`.
#[test]
fn test_custom_event_data_can_be_none() {
    let event = CustomStreamEvent::new("id", "event", json!(null));
    assert_eq!(event.data, json!(null));
}

/// Mirrors `test_custom_event_data_field_required`.
#[test]
fn test_custom_event_data_field_required() {
    let event = CustomStreamEvent::new("id", "test", json!({"info": "required"}));
    assert_eq!(event.data["info"], json!("required"));
}

/// Mirrors `test_stream_event_union_type`.
#[test]
fn test_stream_event_union_type() {
    let standard: StreamEvent = StandardStreamEvent::new("on_chain_start", "id", "chain")
        .with_data(EventData::new().with_input(json!("test")))
        .into();
    assert_eq!(standard.event(), "on_chain_start");

    let custom: StreamEvent = CustomStreamEvent::new("id", "custom", json!("anything")).into();
    assert_eq!(custom.event(), "on_custom_event");
}

/// Mirrors `test_stream_event_can_be_either_type`.
#[test]
fn test_stream_event_can_be_either_type() {
    fn process_event(event: &StreamEvent) -> &str {
        event.event()
    }

    let standard: StreamEvent = StandardStreamEvent::new("on_llm_start", "id", "llm").into();
    let custom: StreamEvent = CustomStreamEvent::new("id", "custom", json!({})).into();

    assert_eq!(process_event(&standard), "on_llm_start");
    assert_eq!(process_event(&custom), "on_custom_event");
}
