use std::collections::HashMap;

use agent_chain_core::runnables::schema::{
    BaseStreamEvent, CUSTOM_EVENT_TYPE, CustomStreamEvent, EventData, StandardStreamEvent,
    StreamEvent,
};
use agent_chain_core::runnables::utils::RootEventFilter;
use serde_json::json;

#[test]
fn test_event_data_default() {
    let data = EventData::builder().build();
    assert!(data.input.is_none());
    assert!(data.output.is_none());
    assert!(data.error.is_none());
    assert!(data.chunk.is_none());
}

#[test]
fn test_event_data_with_input() {
    let data = EventData::builder().input(json!({"key": "value"})).build();
    assert_eq!(data.input, Some(json!({"key": "value"})));
    assert!(data.output.is_none());
}

#[test]
fn test_event_data_with_output() {
    let data = EventData::builder().output(json!("result")).build();
    assert_eq!(data.output, Some(json!("result")));
}

#[test]
fn test_event_data_with_error() {
    let data = EventData::builder().error("something went wrong").build();
    assert_eq!(data.error, Some("something went wrong".to_string()));
}

#[test]
fn test_event_data_with_chunk() {
    let data = EventData::builder().chunk(json!({"x": 5})).build();
    assert_eq!(data.chunk, Some(json!({"x": 5})));
}

#[test]
fn test_event_data_builder_chain() {
    let data = EventData::builder()
        .input(json!("hello"))
        .output(json!("world"))
        .chunk(json!("w"))
        .build();

    assert_eq!(data.input, Some(json!("hello")));
    assert_eq!(data.output, Some(json!("world")));
    assert_eq!(data.chunk, Some(json!("w")));
    assert!(data.error.is_none());
}

#[test]
fn test_event_data_serialization_skips_none() {
    let data = EventData::builder().input(json!("hello")).build();
    let json_str = serde_json::to_string(&data).unwrap();
    assert!(json_str.contains("input"));
    assert!(!json_str.contains("output"));
    assert!(!json_str.contains("error"));
    assert!(!json_str.contains("chunk"));
}

#[test]
fn test_event_data_serialization_roundtrip() {
    let data = EventData::builder()
        .input(json!({"x": 1}))
        .output(json!({"y": 2}))
        .chunk(json!("partial"))
        .build();

    let json_str = serde_json::to_string(&data).unwrap();
    let deserialized: EventData = serde_json::from_str(&json_str).unwrap();

    assert_eq!(deserialized.input, data.input);
    assert_eq!(deserialized.output, data.output);
    assert_eq!(deserialized.chunk, data.chunk);
    assert!(deserialized.error.is_none());
}

#[test]
fn test_base_stream_event_new() {
    let event = BaseStreamEvent::builder()
        .event("on_chain_start")
        .run_id("run-123")
        .build();
    assert_eq!(event.event, "on_chain_start");
    assert_eq!(event.run_id, "run-123");
    assert!(event.tags.is_empty());
    assert!(event.metadata.is_empty());
    assert!(event.parent_ids.is_empty());
}

#[test]
fn test_base_stream_event_with_tags() {
    let event = BaseStreamEvent::builder()
        .event("on_chain_start")
        .run_id("run-1")
        .tags(vec!["seq:step:1".into(), "my_tag".into()])
        .build();
    assert_eq!(event.tags, vec!["seq:step:1", "my_tag"]);
}

#[test]
fn test_base_stream_event_with_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("foo".into(), json!("bar"));
    let event = BaseStreamEvent::builder()
        .event("on_chain_end")
        .run_id("run-2")
        .metadata(metadata)
        .build();
    assert_eq!(event.metadata["foo"], json!("bar"));
}

#[test]
fn test_base_stream_event_with_parent_ids() {
    let event = BaseStreamEvent::builder()
        .event("on_chain_stream")
        .run_id("run-child")
        .parent_ids(vec!["run-root".into(), "run-parent".into()])
        .build();
    assert_eq!(event.parent_ids, vec!["run-root", "run-parent"]);
}

#[test]
fn test_standard_event_chain_lifecycle() {
    let run_id = "run-abc";
    let name = "reverse";

    let start = StandardStreamEvent::builder()
        .event("on_chain_start")
        .run_id(run_id)
        .name(name)
        .tags(vec!["seq:step:1".into()])
        .data(EventData::builder().input(json!("hello")).build())
        .build();
    assert_eq!(start.base.event, "on_chain_start");
    assert_eq!(start.name, "reverse");
    assert_eq!(start.data.input, Some(json!("hello")));

    let stream = StandardStreamEvent::builder()
        .event("on_chain_stream")
        .run_id(run_id)
        .name(name)
        .tags(vec!["seq:step:1".into()])
        .data(EventData::builder().chunk(json!("olleh")).build())
        .build();
    assert_eq!(stream.base.event, "on_chain_stream");
    assert_eq!(stream.data.chunk, Some(json!("olleh")));

    let end = StandardStreamEvent::builder()
        .event("on_chain_end")
        .run_id(run_id)
        .name(name)
        .tags(vec!["seq:step:1".into()])
        .data(
            EventData::builder()
                .input(json!("hello"))
                .output(json!("olleh"))
                .build(),
        )
        .build();
    assert_eq!(end.base.event, "on_chain_end");
    assert_eq!(end.data.input, Some(json!("hello")));
    assert_eq!(end.data.output, Some(json!("olleh")));
}

#[test]
fn test_standard_event_sequence_pattern() {
    let events = [
        StandardStreamEvent::builder()
            .event("on_chain_start")
            .run_id("run-seq")
            .name("RunnableSequence")
            .data(EventData::builder().input(json!({})).build())
            .build(),
        StandardStreamEvent::builder()
            .event("on_chain_start")
            .run_id("run-step1")
            .name("foo")
            .tags(vec!["seq:step:1".into()])
            .parent_ids(vec!["run-seq".into()])
            .build(),
        StandardStreamEvent::builder()
            .event("on_chain_end")
            .run_id("run-step1")
            .name("foo")
            .tags(vec!["seq:step:1".into()])
            .parent_ids(vec!["run-seq".into()])
            .data(
                EventData::builder()
                    .input(json!({}))
                    .output(json!({"x": 5}))
                    .build(),
            )
            .build(),
        StandardStreamEvent::builder()
            .event("on_chain_end")
            .run_id("run-seq")
            .name("RunnableSequence")
            .data(EventData::builder().output(json!({"x": 5})).build())
            .build(),
    ];

    assert_eq!(events.len(), 4);
    assert_eq!(events[0].base.event, "on_chain_start");
    assert_eq!(events[0].name, "RunnableSequence");
    assert_eq!(events[1].base.parent_ids, vec!["run-seq"]);
    assert_eq!(events[3].base.event, "on_chain_end");
}

#[test]
fn test_standard_event_serialization_roundtrip() {
    let event = StandardStreamEvent::builder()
        .event("on_chain_start")
        .run_id("run-123")
        .name("my_chain")
        .tags(vec!["tag1".into()])
        .metadata(HashMap::from([("key".into(), json!("value"))]))
        .parent_ids(vec!["run-parent".into()])
        .data(
            EventData::builder()
                .input(json!({"question": "What up"}))
                .build(),
        )
        .build();

    let json_str = serde_json::to_string(&event).unwrap();
    let deserialized: StandardStreamEvent = serde_json::from_str(&json_str).unwrap();

    assert_eq!(deserialized.base.event, "on_chain_start");
    assert_eq!(deserialized.base.run_id, "run-123");
    assert_eq!(deserialized.name, "my_chain");
    assert_eq!(deserialized.base.tags, vec!["tag1"]);
    assert_eq!(deserialized.base.metadata["key"], json!("value"));
    assert_eq!(deserialized.base.parent_ids, vec!["run-parent"]);
    assert_eq!(
        deserialized.data.input,
        Some(json!({"question": "What up"}))
    );
}

#[test]
fn test_custom_event_type_constant() {
    assert_eq!(CUSTOM_EVENT_TYPE, "on_custom_event");
}

#[test]
fn test_custom_event_construction() {
    let run_id = "run-007";

    let event1 = CustomStreamEvent::builder()
        .run_id(run_id)
        .name("event1")
        .data(json!({"x": 1}))
        .build();
    assert_eq!(event1.base.event, "on_custom_event");
    assert_eq!(event1.base.run_id, run_id);
    assert_eq!(event1.name, "event1");
    assert_eq!(event1.data, json!({"x": 1}));

    let event2 = CustomStreamEvent::builder()
        .run_id(run_id)
        .name("event2")
        .data(json!("foo"))
        .build();
    assert_eq!(event2.name, "event2");
    assert_eq!(event2.data, json!("foo"));
}

#[test]
fn test_custom_event_with_tags_and_metadata() {
    let event = CustomStreamEvent::builder()
        .run_id("run-1")
        .name("my_event")
        .data(json!(null))
        .tags(vec!["tag1".into()])
        .metadata(HashMap::from([("key".into(), json!("val"))]))
        .build();

    assert_eq!(event.base.tags, vec!["tag1"]);
    assert_eq!(event.base.metadata["key"], json!("val"));
}

#[test]
fn test_custom_event_with_parent_ids() {
    let event = CustomStreamEvent::builder()
        .run_id("run-child")
        .name("nested_event")
        .data(json!(42))
        .parent_ids(vec!["run-root".into(), "run-parent".into()])
        .build();

    assert_eq!(event.base.parent_ids, vec!["run-root", "run-parent"]);
}

#[test]
fn test_custom_event_serialization_roundtrip() {
    let event = CustomStreamEvent::builder()
        .run_id("run-456")
        .name("my_custom_event")
        .data(json!({"custom": "data"}))
        .tags(vec!["t1".into()])
        .build();

    let json_str = serde_json::to_string(&event).unwrap();
    let deserialized: CustomStreamEvent = serde_json::from_str(&json_str).unwrap();

    assert_eq!(deserialized.base.event, "on_custom_event");
    assert_eq!(deserialized.base.run_id, "run-456");
    assert_eq!(deserialized.name, "my_custom_event");
    assert_eq!(deserialized.data, json!({"custom": "data"}));
    assert_eq!(deserialized.base.tags, vec!["t1"]);
}

#[test]
fn test_stream_event_standard_variant() {
    let inner = StandardStreamEvent::builder()
        .event("on_chain_end")
        .run_id("run-1")
        .name("chain")
        .build();
    let event: StreamEvent = inner.into();

    assert!(event.is_standard());
    assert!(!event.is_custom());
    assert_eq!(event.event(), "on_chain_end");
    assert_eq!(event.run_id(), "run-1");
    assert_eq!(event.name(), "chain");
    assert!(event.tags().is_empty());
    assert!(event.metadata().is_empty());
    assert!(event.parent_ids().is_empty());
}

#[test]
fn test_stream_event_custom_variant() {
    let inner = CustomStreamEvent::builder()
        .run_id("run-2")
        .name("my_event")
        .data(json!("data"))
        .build();
    let event: StreamEvent = inner.into();

    assert!(event.is_custom());
    assert!(!event.is_standard());
    assert_eq!(event.event(), "on_custom_event");
    assert_eq!(event.run_id(), "run-2");
    assert_eq!(event.name(), "my_event");
}

#[test]
fn test_stream_event_with_tags() {
    let inner = StandardStreamEvent::builder()
        .event("on_chain_start")
        .run_id("run-1")
        .name("chain")
        .tags(vec!["tag_a".into(), "tag_b".into()])
        .build();
    let event: StreamEvent = inner.into();

    assert_eq!(event.tags(), &["tag_a", "tag_b"]);
}

#[test]
fn test_stream_event_with_metadata() {
    let inner = StandardStreamEvent::builder()
        .event("on_chain_start")
        .run_id("run-1")
        .name("chain")
        .metadata(HashMap::from([("key".into(), json!("val"))]))
        .build();
    let event: StreamEvent = inner.into();

    assert_eq!(event.metadata()["key"], json!("val"));
}

#[test]
fn test_stream_event_with_parent_ids() {
    let inner = StandardStreamEvent::builder()
        .event("on_chain_start")
        .run_id("run-child")
        .name("child")
        .parent_ids(vec!["run-root".into()])
        .build();
    let event: StreamEvent = inner.into();

    assert_eq!(event.parent_ids(), &["run-root"]);
}

#[test]
fn test_filter_default_includes_all() {
    let filter = RootEventFilter::new();
    assert!(filter.include_event("any_name", &[], "chain"));
    assert!(filter.include_event("foo", &["tag1".into()], "llm"));
}

#[test]
fn test_filter_include_names() {
    let filter = RootEventFilter {
        include_names: Some(vec!["1".into()]),
        ..Default::default()
    };

    assert!(filter.include_event("1", &[], "chain"));
    assert!(!filter.include_event("2", &[], "chain"));
    assert!(!filter.include_event("3", &[], "chain"));
}

#[test]
fn test_filter_include_tags() {
    let filter = RootEventFilter {
        include_tags: Some(vec!["my_tag".into()]),
        ..Default::default()
    };

    let tagged = vec!["my_tag".into(), "seq:step:2".into()];
    let untagged: Vec<String> = vec!["seq:step:1".into()];

    assert!(filter.include_event("2", &tagged, "chain"));
    assert!(!filter.include_event("1", &untagged, "chain"));
}

#[test]
fn test_filter_include_tags_exclude_names() {
    let filter = RootEventFilter {
        include_tags: Some(vec!["my_tag".into()]),
        exclude_names: Some(vec!["2".into()]),
        ..Default::default()
    };

    let tagged_2 = vec!["my_tag".into(), "seq:step:2".into()];
    let tagged_3 = vec!["my_tag".into(), "seq:step:3".into()];
    let untagged = vec!["seq:step:1".into()];

    assert!(!filter.include_event("2", &tagged_2, "chain"));
    assert!(filter.include_event("3", &tagged_3, "chain"));
    assert!(!filter.include_event("1", &untagged, "chain"));
}

#[test]
fn test_filter_include_types() {
    let filter = RootEventFilter {
        include_types: Some(vec!["llm".into(), "chat_model".into()]),
        ..Default::default()
    };

    assert!(filter.include_event("gpt4", &[], "llm"));
    assert!(filter.include_event("claude", &[], "chat_model"));
    assert!(!filter.include_event("my_chain", &[], "chain"));
}

#[test]
fn test_filter_exclude_types() {
    let filter = RootEventFilter {
        exclude_types: Some(vec!["chain".into()]),
        ..Default::default()
    };

    assert!(!filter.include_event("my_chain", &[], "chain"));
    assert!(filter.include_event("my_llm", &[], "llm"));
}

#[test]
fn test_filter_exclude_names() {
    let filter = RootEventFilter {
        exclude_names: Some(vec!["secret".into()]),
        ..Default::default()
    };

    assert!(!filter.include_event("secret", &[], "chain"));
    assert!(filter.include_event("public", &[], "chain"));
}

#[test]
fn test_filter_exclude_tags() {
    let filter = RootEventFilter {
        exclude_tags: Some(vec!["internal".into()]),
        ..Default::default()
    };

    let internal = vec!["internal".into()];
    let public: Vec<String> = vec!["public".into()];

    assert!(!filter.include_event("foo", &internal, "chain"));
    assert!(filter.include_event("foo", &public, "chain"));
}

#[test]
fn test_filter_combined_include_exclude() {
    let filter = RootEventFilter {
        include_names: Some(vec!["a".into(), "b".into(), "c".into()]),
        exclude_tags: Some(vec!["skip".into()]),
        ..Default::default()
    };

    let skip_tags = vec!["skip".into()];
    let no_tags: Vec<String> = vec![];

    assert!(filter.include_event("a", &no_tags, "chain"));
    assert!(filter.include_event("b", &no_tags, "chain"));
    assert!(!filter.include_event("d", &no_tags, "chain")); // not in include list
    assert!(!filter.include_event("a", &skip_tags, "chain")); // excluded by tag
}

#[test]
fn test_filter_include_names_and_types() {
    let filter = RootEventFilter {
        include_names: Some(vec!["specific_name".into()]),
        include_types: Some(vec!["llm".into()]),
        ..Default::default()
    };

    assert!(filter.include_event("specific_name", &[], "chain"));
    assert!(filter.include_event("any_name", &[], "llm"));
    assert!(!filter.include_event("other_name", &[], "chain"));
}

#[test]
fn test_filter_empty_tags_list() {
    let filter = RootEventFilter {
        include_tags: Some(vec!["required_tag".into()]),
        ..Default::default()
    };

    let empty_tags: Vec<String> = vec![];
    assert!(!filter.include_event("foo", &empty_tags, "chain"));
}

#[test]
fn test_event_type_naming_conventions() {
    let event_types = [
        "on_chain_start",
        "on_chain_stream",
        "on_chain_end",
        "on_llm_start",
        "on_llm_stream",
        "on_llm_end",
        "on_chat_model_start",
        "on_chat_model_stream",
        "on_chat_model_end",
        "on_tool_start",
        "on_tool_end",
        "on_prompt_start",
        "on_prompt_end",
        "on_custom_event",
    ];

    for event_type in &event_types {
        let event = BaseStreamEvent::builder()
            .event(*event_type)
            .run_id("run-1")
            .build();
        assert_eq!(event.event, *event_type);
        assert!(event.event.starts_with("on_"));
    }
}

#[test]
fn test_event_data_conventions() {
    let start_data = EventData::builder().input(json!("hello")).build();
    assert!(start_data.input.is_some());
    assert!(start_data.output.is_none());
    assert!(start_data.chunk.is_none());

    let stream_data = EventData::builder().chunk(json!("partial")).build();
    assert!(stream_data.input.is_none());
    assert!(stream_data.output.is_none());
    assert!(stream_data.chunk.is_some());

    let end_data = EventData::builder()
        .input(json!("hello"))
        .output(json!("olleh"))
        .build();
    assert!(end_data.input.is_some());
    assert!(end_data.output.is_some());
    assert!(end_data.chunk.is_none());

    let error_data = EventData::builder()
        .error("ValueError: x is too large")
        .build();
    assert!(error_data.error.is_some());
}
