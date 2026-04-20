//! Coverage for Phase 2's public type surface.
//!
//! Ports the structural assertions from
//! `langgraph/libs/langgraph/tests/test_types.py` and the `Send`/`Command`
//! helpers in `test_pregel.py`, with the Python-only allowlist tests
//! dropped. Types that transitively contain a `RunnableConfig` cannot
//! implement `PartialEq` (see the module doc on `types.rs`), so those
//! round-trip assertions compare via `serde_json::Value`.

use agent_graph::config::RunnableConfig;
use agent_graph::types::{
    CacheKeyStrategy, CachePolicy, Command, Durability, GoTo, Interrupt, Overwrite, PARENT,
    PLACEHOLDER_INTERRUPT_ID, PregelTask, RetryPolicy, RetryPredicate, Send, StateSnapshot,
    StreamMode, TaskPath, TaskState,
};
use agent_graph_checkpoint::{CheckpointMetadata, CheckpointSource};
use serde_json::{Value, json};

fn round_trip<T>(value: &T) -> Value
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let encoded = serde_json::to_value(value).expect("serialize");
    let decoded: T = serde_json::from_value(encoded.clone()).expect("deserialize");
    // Re-serialize the decoded copy to prove the round-trip preserved the
    // shape. We return the value to let callers assert on it directly.
    let again = serde_json::to_value(&decoded).expect("re-serialize");
    assert_eq!(encoded, again, "round-trip diverged");
    encoded
}

// ---------------------------------------------------------------------------
// StreamMode / Durability
// ---------------------------------------------------------------------------

#[test]
fn stream_mode_serializes_lowercase() {
    let modes = [
        (StreamMode::Values, "values"),
        (StreamMode::Updates, "updates"),
        (StreamMode::Checkpoints, "checkpoints"),
        (StreamMode::Tasks, "tasks"),
        (StreamMode::Debug, "debug"),
        (StreamMode::Messages, "messages"),
        (StreamMode::Custom, "custom"),
    ];
    for (mode, expected) in modes {
        assert_eq!(serde_json::to_value(mode).unwrap(), json!(expected));
        let decoded: StreamMode = serde_json::from_value(json!(expected)).unwrap();
        assert_eq!(decoded, mode);
    }
}

#[test]
fn durability_serializes_lowercase() {
    assert_eq!(
        serde_json::to_value(Durability::Sync).unwrap(),
        json!("sync")
    );
    assert_eq!(
        serde_json::to_value(Durability::Async).unwrap(),
        json!("async")
    );
    assert_eq!(
        serde_json::to_value(Durability::Exit).unwrap(),
        json!("exit")
    );
}

// ---------------------------------------------------------------------------
// RetryPolicy / CachePolicy
// ---------------------------------------------------------------------------

#[test]
fn retry_policy_defaults_match_python() {
    let policy = RetryPolicy::default();
    assert_eq!(policy.initial_interval, 0.5);
    assert_eq!(policy.backoff_factor, 2.0);
    assert_eq!(policy.max_interval, 128.0);
    assert_eq!(policy.max_attempts, 3);
    assert!(policy.jitter);
    assert_eq!(policy.retry_on, RetryPredicate::TransientErrors);
}

#[test]
fn retry_policy_builder_overrides_defaults() {
    let policy = RetryPolicy::builder()
        .max_attempts(10)
        .jitter(false)
        .retry_on(RetryPredicate::All)
        .build();

    assert_eq!(policy.initial_interval, 0.5);
    assert_eq!(policy.max_attempts, 10);
    assert!(!policy.jitter);
    assert_eq!(policy.retry_on, RetryPredicate::All);
    round_trip(&policy);
}

#[test]
fn cache_policy_helpers_and_defaults() {
    let default = CachePolicy::default();
    assert_eq!(default.ttl, None);
    assert_eq!(default.key_strategy, CacheKeyStrategy::Hash);

    let ttl_only = CachePolicy::with_ttl(60);
    assert_eq!(ttl_only.ttl, Some(60));

    let encoded = serde_json::to_value(default).unwrap();
    assert_eq!(encoded, json!({"key_strategy": "hash"}));
}

// ---------------------------------------------------------------------------
// Send
// ---------------------------------------------------------------------------

#[test]
fn send_round_trips_and_compares_structurally() {
    let a = Send::new("node_a", json!({"x": 1}));
    let b = Send::new("node_a", json!({"x": 1}));
    let c = Send::new("node_b", json!({"x": 1}));

    assert_eq!(a, b);
    assert_ne!(a, c);

    let encoded = round_trip(&a);
    assert_eq!(encoded, json!({"node": "node_a", "arg": {"x": 1}}));
}

// ---------------------------------------------------------------------------
// Interrupt
// ---------------------------------------------------------------------------

#[test]
fn interrupt_new_uses_placeholder_id() {
    let interrupt = Interrupt::new(json!("pause"));
    assert_eq!(interrupt.id, PLACEHOLDER_INTERRUPT_ID);
    assert_eq!(interrupt.value, json!("pause"));
}

#[test]
fn interrupt_from_ns_is_deterministic_and_ns_sensitive() {
    let a = Interrupt::from_ns(json!("pause"), "ns/a");
    let b = Interrupt::from_ns(json!("pause"), "ns/a");
    let c = Interrupt::from_ns(json!("pause"), "ns/b");

    assert_eq!(a.id, b.id);
    assert_ne!(a.id, c.id);
    assert_eq!(a.id.len(), 32);
    assert!(a.id.chars().all(|ch| ch.is_ascii_hexdigit()));
}

#[test]
fn interrupt_round_trips() {
    let interrupt = Interrupt::with_id(json!({"ask": "name"}), "custom-id");
    let encoded = round_trip(&interrupt);
    assert_eq!(
        encoded,
        json!({"value": {"ask": "name"}, "id": "custom-id"})
    );
}

// ---------------------------------------------------------------------------
// Command / GoTo
// ---------------------------------------------------------------------------

#[test]
fn command_default_is_empty_and_elides_every_field() {
    let command = Command::default();
    let encoded = serde_json::to_value(&command).unwrap();
    assert_eq!(encoded, json!({}));
}

#[test]
fn command_builder_applies_parent_scope() {
    let command = Command::builder()
        .graph(PARENT)
        .resume(json!("hello"))
        .build();
    assert_eq!(command.graph.as_deref(), Some(PARENT));
    assert_eq!(command.resume, Some(json!("hello")));
    assert!(command.goto.is_empty());

    round_trip(&command);
}

#[test]
fn command_goto_accepts_both_node_names_and_sends() {
    let command = Command::builder()
        .goto(vec![
            GoTo::from("step_1"),
            GoTo::from(Send::new("step_2", json!({"idx": 0}))),
        ])
        .build();

    let encoded = serde_json::to_value(&command).unwrap();
    assert_eq!(
        encoded,
        json!({
            "goto": [
                "step_1",
                {"node": "step_2", "arg": {"idx": 0}},
            ],
        })
    );

    let decoded: Command = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, command);
}

#[test]
fn command_shorthand_constructors() {
    assert_eq!(
        Command::goto_node("end"),
        Command::builder().goto(vec![GoTo::from("end")]).build(),
    );
    assert_eq!(
        Command::resume(json!(42)),
        Command::builder().resume(json!(42)).build(),
    );
    assert_eq!(
        Command::update(json!({"k": 1})),
        Command::builder().update(json!({"k": 1})).build(),
    );
}

#[test]
fn goto_untagged_deserializes_strings_and_objects() {
    let from_string: GoTo = serde_json::from_value(json!("node_x")).unwrap();
    assert_eq!(from_string, GoTo::Node("node_x".to_owned()));

    let from_send: GoTo = serde_json::from_value(json!({"node": "node_x", "arg": 1})).unwrap();
    assert_eq!(from_send, GoTo::Send(Send::new("node_x", json!(1))));
}

// ---------------------------------------------------------------------------
// TaskPath
// ---------------------------------------------------------------------------

#[test]
fn task_path_round_trips_mixed_forms() {
    let path = vec![
        TaskPath::from("pull"),
        TaskPath::from(3_i64),
        TaskPath::Nested(vec![TaskPath::from("subgraph"), TaskPath::from(0_i64)]),
    ];
    let encoded = serde_json::to_value(&path).unwrap();
    assert_eq!(encoded, json!(["pull", 3, ["subgraph", 0]]));
    let decoded: Vec<TaskPath> = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, path);
}

// ---------------------------------------------------------------------------
// StateSnapshot / PregelTask
// ---------------------------------------------------------------------------

fn minimal_config() -> RunnableConfig {
    RunnableConfig::builder()
        .configurable(std::collections::HashMap::from([(
            "thread_id".to_owned(),
            json!("thread-1"),
        )]))
        .build()
}

#[test]
fn pregel_task_round_trips_through_json() {
    let task = PregelTask {
        id: "task-1".to_owned(),
        name: "worker".to_owned(),
        path: vec![TaskPath::from("pull"), TaskPath::from(0)],
        error: Some("boom".to_owned()),
        interrupts: vec![Interrupt::with_id(json!("why?"), "int-1")],
        state: None,
        result: Some(json!({"done": true})),
    };

    let encoded = serde_json::to_value(&task).unwrap();
    let decoded: PregelTask = serde_json::from_value(encoded.clone()).unwrap();
    let again = serde_json::to_value(&decoded).unwrap();
    assert_eq!(encoded, again);
}

#[test]
fn state_snapshot_round_trips_with_nested_task_state() {
    let inner_snapshot = StateSnapshot {
        values: json!({"count": 1}),
        next: vec!["inner_b".to_owned()],
        config: minimal_config(),
        metadata: Some(CheckpointMetadata {
            source: Some(CheckpointSource::Loop),
            step: Some(2),
            parents: Default::default(),
            run_id: None,
        }),
        created_at: Some("2026-04-20T00:00:00Z".to_owned()),
        parent_config: None,
        tasks: vec![],
        interrupts: vec![],
    };

    let outer = StateSnapshot {
        values: json!({"count": 0}),
        next: vec!["router".to_owned()],
        config: minimal_config(),
        metadata: None,
        created_at: None,
        parent_config: Some(Box::new(minimal_config())),
        tasks: vec![PregelTask {
            id: "t1".to_owned(),
            name: "router".to_owned(),
            path: vec![TaskPath::from("pull")],
            error: None,
            interrupts: vec![],
            state: Some(TaskState::Snapshot(Box::new(inner_snapshot))),
            result: None,
        }],
        interrupts: vec![Interrupt::from_ns(json!("paused"), "root|router")],
    };

    let encoded = serde_json::to_value(&outer).unwrap();
    let decoded: StateSnapshot = serde_json::from_value(encoded.clone()).unwrap();
    let again = serde_json::to_value(&decoded).unwrap();
    assert_eq!(encoded, again);
}

#[test]
fn state_snapshot_elides_optional_fields() {
    let snapshot = StateSnapshot {
        values: json!({}),
        next: vec![],
        config: RunnableConfig::default(),
        metadata: None,
        created_at: None,
        parent_config: None,
        tasks: vec![],
        interrupts: vec![],
    };

    let encoded = serde_json::to_value(&snapshot).unwrap();
    let obj = encoded.as_object().unwrap();
    assert!(!obj.contains_key("metadata"));
    assert!(!obj.contains_key("created_at"));
    assert!(!obj.contains_key("parent_config"));
    assert!(!obj.contains_key("tasks"));
    assert!(!obj.contains_key("interrupts"));
}

// ---------------------------------------------------------------------------
// Overwrite (pre-existing) — sanity coverage via the types module re-export.
// ---------------------------------------------------------------------------

#[test]
fn overwrite_still_serializes_with_marker_key() {
    let overwrite = Overwrite::new(json!(99));
    let encoded = serde_json::to_value(&overwrite).unwrap();
    assert_eq!(encoded, json!({"__overwrite__": 99}));
}

// ---------------------------------------------------------------------------
// Config helpers
// ---------------------------------------------------------------------------

#[test]
fn config_helpers_read_reserved_keys() {
    let config = RunnableConfig::builder()
        .configurable(std::collections::HashMap::from([
            ("thread_id".to_owned(), json!("th-1")),
            ("checkpoint_ns".to_owned(), json!("root|sub")),
            ("checkpoint_id".to_owned(), json!("ck-1")),
        ]))
        .build();

    assert_eq!(agent_graph::config::thread_id(&config), Some("th-1"));
    assert_eq!(
        agent_graph::config::checkpoint_ns(&config),
        Some("root|sub")
    );
    assert_eq!(agent_graph::config::checkpoint_id(&config), Some("ck-1"));
}

#[test]
fn config_helpers_return_none_for_non_string_values() {
    let config = RunnableConfig::builder()
        .configurable(std::collections::HashMap::from([(
            "thread_id".to_owned(),
            json!(42),
        )]))
        .build();
    assert_eq!(agent_graph::config::thread_id(&config), None);
}
