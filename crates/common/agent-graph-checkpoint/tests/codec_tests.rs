//! Round-trip coverage for [`agent_graph_checkpoint::codec`].
//!
//! Mirrors the smoke tests in `langgraph/libs/checkpoint/tests/test_jsonplus.py`
//! — we verify that every tag survives a dump/load cycle, that the helpers
//! work with typed values, and that tag mismatches are reported cleanly.

use std::collections::HashMap;

use agent_graph_checkpoint::codec::{self, json, msgpack};
use agent_graph_checkpoint::{
    CheckpointMetadata, CheckpointSource, Error, JsonSerializer, MsgpackSerializer, Serializer,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Payload {
    name: String,
    value: i64,
    flags: Vec<String>,
    nested: HashMap<String, i64>,
}

fn payload() -> Payload {
    Payload {
        name: "phase-2".to_owned(),
        value: 42,
        flags: vec!["alpha".to_owned(), "beta".to_owned()],
        nested: HashMap::from([("a".to_owned(), 1), ("b".to_owned(), 2)]),
    }
}

#[test]
fn json_serializer_round_trips_values() {
    let serializer = JsonSerializer::new();
    let input = json!({"foo": "bar", "count": 7, "flags": [true, false]});

    let (tag, bytes) = serializer.dumps_typed(&input).unwrap();
    assert_eq!(tag, json::TYPE_TAG);

    let decoded = serializer.loads_typed(&tag, &bytes).unwrap();
    assert_eq!(decoded, input);
}

#[test]
fn msgpack_serializer_round_trips_values() {
    let serializer = MsgpackSerializer::new();
    let input = json!({"foo": "bar", "count": 7, "flags": [true, false]});

    let (tag, bytes) = serializer.dumps_typed(&input).unwrap();
    assert_eq!(tag, msgpack::TYPE_TAG);

    let decoded = serializer.loads_typed(&tag, &bytes).unwrap();
    assert_eq!(decoded, input);
}

#[test]
fn msgpack_output_is_distinct_from_json() {
    let value = json!({"k": "v"});
    let (_, json_bytes) = JsonSerializer::new().dumps_typed(&value).unwrap();
    let (_, msgpack_bytes) = MsgpackSerializer::new().dumps_typed(&value).unwrap();

    assert_ne!(json_bytes, msgpack_bytes);
    // JSON carries the surrounding braces, msgpack uses a map marker byte.
    assert_eq!(json_bytes.first(), Some(&b'{'));
    assert_ne!(msgpack_bytes.first(), Some(&b'{'));
}

#[test]
fn typed_dumps_and_loads_round_trip() {
    let serializer = MsgpackSerializer::new();
    let original = payload();

    let (tag, bytes) = codec::dumps(&serializer, &original).unwrap();
    let decoded: Payload = codec::loads(&serializer, &tag, &bytes).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn typed_helpers_work_via_trait_object() {
    let serializers: Vec<Box<dyn Serializer>> = vec![
        Box::new(JsonSerializer::new()),
        Box::new(MsgpackSerializer::new()),
    ];

    for serializer in &serializers {
        let (tag, bytes) = codec::dumps(serializer.as_ref(), &payload()).unwrap();
        let decoded: Payload = codec::loads(serializer.as_ref(), &tag, &bytes).unwrap();
        assert_eq!(decoded, payload());
    }
}

#[test]
fn wrong_type_tag_is_rejected() {
    let serializer = JsonSerializer::new();
    let (_tag, bytes) = serializer.dumps_typed(&json!({"k": "v"})).unwrap();

    let err = serializer.loads_typed("msgpack", &bytes).unwrap_err();
    match err {
        Error::UnknownTypeTag(tag) => assert_eq!(tag, "msgpack"),
        other => panic!("expected UnknownTypeTag, got {other:?}"),
    }
}

#[test]
fn checkpoint_metadata_round_trips_through_both_codecs() {
    let metadata = CheckpointMetadata {
        source: Some(CheckpointSource::Loop),
        step: Some(3),
        parents: HashMap::from([("".to_owned(), "ckpt-1".to_owned())]),
        run_id: Some("run-42".to_owned()),
    };

    for serializer in [
        &JsonSerializer::new() as &dyn Serializer,
        &MsgpackSerializer::new() as &dyn Serializer,
    ] {
        let (tag, bytes) = codec::dumps(serializer, &metadata).unwrap();
        let decoded: CheckpointMetadata = codec::loads(serializer, &tag, &bytes).unwrap();
        assert_eq!(decoded, metadata);
    }
}

#[test]
fn checkpoint_source_serializes_lowercase() {
    let value = serde_json::to_value(CheckpointSource::Update).unwrap();
    assert_eq!(value, json!("update"));
}

#[test]
fn checkpoint_metadata_elides_empty_fields() {
    let metadata = CheckpointMetadata::default();
    let encoded = serde_json::to_value(&metadata).unwrap();
    assert_eq!(encoded, json!({}));
}
