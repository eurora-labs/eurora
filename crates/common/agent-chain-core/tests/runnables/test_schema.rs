use std::collections::HashMap;

use agent_chain_core::runnables::schema::{
    BaseStreamEvent, CUSTOM_EVENT_TYPE, CustomStreamEvent, EventData, StandardStreamEvent,
    StreamEvent,
};
use serde_json::json;

#[test]
fn test_event_data_structure() {
    let data = EventData::builder()
        .input(json!({"question": "test"}))
        .output(json!({"answer": "response"}))
        .chunk(json!({"partial": "data"}))
        .build();

    assert_eq!(data.input, Some(json!({"question": "test"})));
    assert_eq!(data.output, Some(json!({"answer": "response"})));
    assert_eq!(data.chunk, Some(json!({"partial": "data"})));

    let minimal = EventData::builder().build();
    assert!(minimal.input.is_none());
    assert!(minimal.output.is_none());
    assert!(minimal.chunk.is_none());
    assert!(minimal.error.is_none());
}

#[test]
fn test_event_data_with_error() {
    let data = EventData::builder()
        .input(json!("test"))
        .error("Test error")
        .build();

    assert_eq!(data.error, Some("Test error".to_string()));
    assert_eq!(data.input, Some(json!("test")));
}

#[test]
fn test_event_data_empty() {
    let data = EventData::builder().build();
    let json_str = serde_json::to_string(&data).unwrap();
    assert_eq!(json_str, "{}");
}

#[test]
fn test_event_data_chunk_field() {
    let data = EventData::builder().chunk(json!("partial output")).build();
    assert_eq!(data.chunk, Some(json!("partial output")));

    let chunk_list = EventData::builder().chunk(json!([1, 2, 3])).build();
    assert_eq!(chunk_list.chunk, Some(json!([1, 2, 3])));
}

#[test]
fn test_event_data_supports_various_input_types() {
    let data1 = EventData::builder().input(json!("simple string")).build();
    assert_eq!(data1.input, Some(json!("simple string")));

    let data2 = EventData::builder().input(json!({"key": "value"})).build();
    assert_eq!(data2.input.as_ref().unwrap()["key"], json!("value"));

    let data3 = EventData::builder().input(json!([1, 2, 3])).build();
    assert_eq!(data3.input, Some(json!([1, 2, 3])));

    let data4 = EventData::builder()
        .input(json!({"field1": "test", "field2": 42}))
        .build();
    assert_eq!(data4.input.as_ref().unwrap()["field1"], json!("test"));
    assert_eq!(data4.input.as_ref().unwrap()["field2"], json!(42));
}

#[test]
fn test_event_data_supports_various_output_types() {
    let data1 = EventData::builder().output(json!("result")).build();
    assert_eq!(data1.output, Some(json!("result")));

    let data2 = EventData::builder()
        .output(json!({"result": "value"}))
        .build();
    assert_eq!(data2.output.as_ref().unwrap()["result"], json!("value"));

    let data3 = EventData::builder().output(json!([1, 2, 3])).build();
    assert_eq!(data3.output, Some(json!([1, 2, 3])));
}

#[test]
fn test_standard_event_with_all_data_fields() {
    let data = EventData::builder()
        .input(json!({"query": "test"}))
        .output(json!({"result": "answer"}))
        .chunk(json!({"partial": "data"}))
        .error("test error")
        .build();

    assert!(data.input.is_some());
    assert!(data.output.is_some());
    assert!(data.chunk.is_some());
    assert!(data.error.is_some());
}

#[test]
fn test_event_data_with_messages() {
    use agent_chain_core::messages::{AIMessage, BaseMessage, HumanMessage};

    let human = BaseMessage::Human(HumanMessage::builder().content("hello").build());
    let ai = BaseMessage::AI(AIMessage::builder().content("hi there").build());

    let input_val = serde_json::to_value([&human]).unwrap();
    let output_val = serde_json::to_value(&ai).unwrap();
    let chunk_val = serde_json::to_value(&ai).unwrap();

    let data = EventData::builder()
        .input(input_val.clone())
        .output(output_val.clone())
        .chunk(chunk_val)
        .build();

    assert!(data.input.is_some());
    assert!(data.output.is_some());
    assert!(data.chunk.is_some());
}

#[test]
fn test_event_with_multiple_chunks() {
    let chunks = [
        EventData::builder().chunk(json!("Hello")).build(),
        EventData::builder().chunk(json!(" ")).build(),
        EventData::builder().chunk(json!("World")).build(),
        EventData::builder().chunk(json!("!")).build(),
    ];

    let accumulated: String = chunks
        .iter()
        .filter_map(|c| c.chunk.as_ref().and_then(|v| v.as_str()))
        .collect();
    assert_eq!(accumulated, "Hello World!");
}

#[test]
fn test_base_stream_event_structure() {
    let event = BaseStreamEvent::builder()
        .event("on_chain_start")
        .run_id("test-run-id")
        .build();
    assert_eq!(event.event, "on_chain_start");
    assert_eq!(event.run_id, "test-run-id");
    assert!(event.parent_ids.is_empty());
}

#[test]
fn test_base_stream_event_with_optional_fields() {
    let event = BaseStreamEvent::builder()
        .event("on_chain_end")
        .run_id("test-run-id")
        .parent_ids(vec!["parent-1".into(), "parent-2".into()])
        .tags(vec!["tag1".into(), "tag2".into()])
        .metadata(HashMap::from([("key".into(), json!("value"))]))
        .build();

    assert_eq!(event.tags, vec!["tag1", "tag2"]);
    assert_eq!(event.metadata["key"], json!("value"));
    assert_eq!(event.parent_ids.len(), 2);
}

#[test]
fn test_parent_ids_hierarchy() {
    let root = BaseStreamEvent::builder()
        .event("on_chain_start")
        .run_id("root-id")
        .build();
    assert!(root.parent_ids.is_empty());

    let child = BaseStreamEvent::builder()
        .event("on_chain_start")
        .run_id("child-id")
        .parent_ids(vec!["root-id".into()])
        .build();
    assert_eq!(child.parent_ids, vec!["root-id"]);

    let grandchild = BaseStreamEvent::builder()
        .event("on_chain_start")
        .run_id("grandchild-id")
        .parent_ids(vec!["root-id".into(), "child-id".into()])
        .build();
    assert_eq!(grandchild.parent_ids, vec!["root-id", "child-id"]);
}

#[test]
fn test_event_parent_ids_can_be_nested() {
    let parent_chain: Vec<String> = (0..10).map(|i| format!("parent-{i}")).collect();

    let event = BaseStreamEvent::builder()
        .event("on_chain_start")
        .run_id("leaf-id")
        .parent_ids(parent_chain)
        .build();

    assert_eq!(event.parent_ids.len(), 10);
    assert_eq!(event.parent_ids[0], "parent-0");
    assert_eq!(event.parent_ids[9], "parent-9");
}

#[test]
fn test_event_run_id_format() {
    let run_id = uuid::Uuid::new_v4().to_string();
    let event = BaseStreamEvent::builder()
        .event("on_chain_start")
        .run_id(&run_id)
        .build();
    assert_eq!(event.run_id, run_id);
    assert!(uuid::Uuid::parse_str(&event.run_id).is_ok());
}

#[test]
fn test_standard_stream_event_structure() {
    let event = StandardStreamEvent::builder()
        .event("on_llm_start")
        .run_id("test-run-id")
        .name("TestLLM")
        .data(EventData::builder().input(json!("test input")).build())
        .build();

    assert_eq!(event.name, "TestLLM");
    assert_eq!(event.data.input, Some(json!("test input")));
}

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
        let event = StandardStreamEvent::builder()
            .event(*event_type)
            .run_id("test-id")
            .name("test")
            .build();
        assert_eq!(event.base.event, *event_type);
    }
}

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

    let event = StandardStreamEvent::builder()
        .event("on_chain_start")
        .run_id("id")
        .name("test")
        .metadata(metadata.clone())
        .build();

    let serialized = serde_json::to_string(&event.base.metadata).unwrap();
    let deserialized: HashMap<String, serde_json::Value> =
        serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized["string"], json!("value"));
    assert_eq!(deserialized["number"], json!(42));
    assert_eq!(deserialized["boolean"], json!(true));
    assert_eq!(deserialized["null"], json!(null));
}

#[test]
fn test_tags_list_of_strings() {
    let event = StandardStreamEvent::builder()
        .event("on_chain_start")
        .run_id("id")
        .name("test")
        .tags(vec!["tag1".into(), "tag2".into(), "tag3".into()])
        .build();

    assert_eq!(event.base.tags.len(), 3);
    for tag in &event.base.tags {
        assert!(!tag.is_empty());
    }
}

#[test]
fn test_event_metadata_empty_dict() {
    let event = StandardStreamEvent::builder()
        .event("on_chain_start")
        .run_id("id")
        .name("test")
        .metadata(HashMap::new())
        .build();
    assert!(event.base.metadata.is_empty());
}

#[test]
fn test_event_tags_empty_list() {
    let event = StandardStreamEvent::builder()
        .event("on_chain_start")
        .run_id("id")
        .name("test")
        .tags(vec![])
        .build();
    assert!(event.base.tags.is_empty());
}

#[test]
fn test_event_minimal_required_fields() {
    let base = BaseStreamEvent::builder()
        .event("on_chain_start")
        .run_id("id")
        .build();
    assert_eq!(base.event, "on_chain_start");
    assert_eq!(base.run_id, "id");

    let standard = StandardStreamEvent::builder()
        .event("on_chain_start")
        .run_id("id")
        .name("test")
        .build();
    assert_eq!(standard.name, "test");
    assert!(standard.data.input.is_none());

    let custom = CustomStreamEvent::builder()
        .run_id("id")
        .name("custom")
        .data(json!("any"))
        .build();
    assert_eq!(custom.base.event, "on_custom_event");
}

#[test]
fn test_event_all_optional_fields() {
    let event = StandardStreamEvent::builder()
        .event("on_chain_start")
        .run_id("test-run-id")
        .name("TestChain")
        .parent_ids(vec!["parent-1".into(), "parent-2".into()])
        .tags(vec!["tag1".into(), "tag2".into()])
        .metadata(HashMap::from([
            ("version".into(), json!("1.0")),
            ("environment".into(), json!("test")),
        ]))
        .data(
            EventData::builder()
                .input(json!({"query": "test"}))
                .output(json!({"response": "result"}))
                .chunk(json!({"partial": "data"}))
                .build(),
        )
        .build();

    assert_eq!(event.base.parent_ids.len(), 2);
    assert_eq!(event.base.tags.len(), 2);
    assert_eq!(event.base.metadata.len(), 2);
    assert!(event.data.input.is_some());
    assert!(event.data.output.is_some());
    assert!(event.data.chunk.is_some());
}

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

    let event = StandardStreamEvent::builder()
        .event("on_llm_start")
        .run_id("id")
        .name("llm")
        .metadata(metadata)
        .build();

    assert_eq!(
        event.base.metadata["model_info"]["provider"],
        json!("openai")
    );
    assert_eq!(event.base.metadata["user_info"]["user_id"], json!("123"));
}

#[test]
fn test_standard_event_data_field_required() {
    let event = StandardStreamEvent::builder()
        .event("on_chain_start")
        .run_id("id")
        .name("test")
        .build();
    let _ = &event.data;
}

#[test]
fn test_event_tags_inherited_from_parent() {
    let _parent = StandardStreamEvent::builder()
        .event("on_chain_start")
        .run_id("parent")
        .name("parent")
        .tags(vec!["parent-tag".into()])
        .build();

    let child = StandardStreamEvent::builder()
        .event("on_chain_start")
        .run_id("child")
        .name("child")
        .parent_ids(vec!["parent".into()])
        .tags(vec!["parent-tag".into(), "child-tag".into()])
        .build();

    assert!(child.base.tags.contains(&"parent-tag".to_string()));
    assert!(child.base.tags.contains(&"child-tag".to_string()));
}

#[test]
fn test_custom_stream_event_structure() {
    let event = CustomStreamEvent::builder()
        .run_id("test-run-id")
        .name("my_custom_event")
        .data(json!({"custom_field": "custom_value"}))
        .build();

    assert_eq!(event.base.event, "on_custom_event");
    assert_eq!(event.name, "my_custom_event");
    assert_eq!(event.data["custom_field"], json!("custom_value"));
}

#[test]
fn test_custom_stream_event_with_any_data() {
    let event1 = CustomStreamEvent::builder()
        .run_id("id1")
        .name("event1")
        .data(json!("string data"))
        .build();
    assert_eq!(event1.data, json!("string data"));

    let event2 = CustomStreamEvent::builder()
        .run_id("id2")
        .name("event2")
        .data(json!([1, 2, 3]))
        .build();
    assert_eq!(event2.data, json!([1, 2, 3]));

    let event3 = CustomStreamEvent::builder()
        .run_id("id3")
        .name("event3")
        .data(json!({"nested": {"deeply": {"value": 42}}}))
        .build();
    assert_eq!(event3.data["nested"]["deeply"]["value"], json!(42));
}

#[test]
fn test_custom_event_must_be_on_custom_event() {
    let event = CustomStreamEvent::builder()
        .run_id("id")
        .name("my_event")
        .data(json!({}))
        .build();
    assert_eq!(event.base.event, CUSTOM_EVENT_TYPE);
    assert_eq!(event.base.event, "on_custom_event");
}

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
        let event = CustomStreamEvent::builder()
            .run_id("id")
            .name(*name)
            .data(json!({}))
            .build();
        assert_eq!(event.name, *name);
    }
}

#[test]
fn test_custom_event_data_can_be_none() {
    let event = CustomStreamEvent::builder()
        .run_id("id")
        .name("event")
        .data(json!(null))
        .build();
    assert_eq!(event.data, json!(null));
}

#[test]
fn test_custom_event_data_field_required() {
    let event = CustomStreamEvent::builder()
        .run_id("id")
        .name("test")
        .data(json!({"info": "required"}))
        .build();
    assert_eq!(event.data["info"], json!("required"));
}

#[test]
fn test_stream_event_union_type() {
    let standard: StreamEvent = StandardStreamEvent::builder()
        .event("on_chain_start")
        .run_id("id")
        .name("chain")
        .data(EventData::builder().input(json!("test")).build())
        .build()
        .into();
    assert_eq!(standard.event(), "on_chain_start");

    let custom: StreamEvent = CustomStreamEvent::builder()
        .run_id("id")
        .name("custom")
        .data(json!("anything"))
        .build()
        .into();
    assert_eq!(custom.event(), "on_custom_event");
}

#[test]
fn test_stream_event_can_be_either_type() {
    fn process_event(event: &StreamEvent) -> &str {
        event.event()
    }

    let standard: StreamEvent = StandardStreamEvent::builder()
        .event("on_llm_start")
        .run_id("id")
        .name("llm")
        .build()
        .into();
    let custom: StreamEvent = CustomStreamEvent::builder()
        .run_id("id")
        .name("custom")
        .data(json!({}))
        .build()
        .into();

    assert_eq!(process_event(&standard), "on_llm_start");
    assert_eq!(process_event(&custom), "on_custom_event");
}
