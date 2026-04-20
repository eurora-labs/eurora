//! Ports `langgraph/libs/langgraph/tests/test_channels.py` plus additional
//! coverage for behaviour the Python suite omits (`*AfterFinish` variants,
//! `consume`/`finish` transitions, checkpoint round-trips).

use std::sync::Arc;

use agent_graph::channels::{
    AnyValue, BaseChannel, BinaryOperatorAggregate, EphemeralValue, LastValue,
    LastValueAfterFinish, NamedBarrierValue, NamedBarrierValueAfterFinish, Topic, UntrackedValue,
    Value,
};
use agent_graph::errors::Error;
use agent_graph::types::Overwrite;
use serde_json::json;

fn rehydrate(channel: &dyn BaseChannel) -> Box<dyn BaseChannel> {
    channel
        .from_checkpoint(channel.checkpoint())
        .expect("checkpoint round-trip")
}

#[test]
fn last_value_accepts_single_update_and_round_trips() {
    let mut channel = LastValue::new();

    assert!(matches!(channel.get(), Err(Error::EmptyChannel(_))));
    assert!(matches!(
        channel.update(vec![json!(5), json!(6)]),
        Err(Error::InvalidUpdate(_))
    ));

    assert!(channel.update(vec![json!(3)]).unwrap());
    assert_eq!(channel.get().unwrap(), json!(3));

    assert!(channel.update(vec![json!(4)]).unwrap());
    assert_eq!(channel.get().unwrap(), json!(4));

    let restored = rehydrate(&channel);
    assert_eq!(restored.get().unwrap(), json!(4));
}

#[test]
fn last_value_empty_update_is_noop() {
    let mut channel = LastValue::new();
    assert!(!channel.update(vec![]).unwrap());
    assert!(!channel.is_available());
}

#[test]
fn last_value_after_finish_requires_finish_before_read() {
    let mut channel = LastValueAfterFinish::new();
    assert!(channel.update(vec![json!("hello")]).unwrap());

    assert!(matches!(channel.get(), Err(Error::EmptyChannel(_))));
    assert!(!channel.is_available());

    assert!(channel.finish());
    assert!(!channel.finish(), "second finish is a no-op");
    assert_eq!(channel.get().unwrap(), json!("hello"));
    assert!(channel.is_available());

    let restored = rehydrate(&channel);
    assert_eq!(restored.get().unwrap(), json!("hello"));

    assert!(channel.consume());
    assert!(!channel.is_available());
    assert!(!channel.consume(), "second consume is a no-op");
}

#[test]
fn last_value_after_finish_update_resets_finished() {
    let mut channel = LastValueAfterFinish::new();
    channel.update(vec![json!(1)]).unwrap();
    assert!(channel.finish());
    channel.update(vec![json!(2)]).unwrap();
    assert!(!channel.is_available());
    assert!(channel.finish());
    assert_eq!(channel.get().unwrap(), json!(2));
}

#[test]
fn any_value_clears_on_empty_update() {
    let mut channel = AnyValue::new();
    assert!(channel.update(vec![json!("a")]).unwrap());
    assert_eq!(channel.get().unwrap(), json!("a"));
    assert!(channel.update(vec![]).unwrap());
    assert!(matches!(channel.get(), Err(Error::EmptyChannel(_))));
    assert!(!channel.update(vec![]).unwrap());
}

#[test]
fn any_value_takes_the_last_update() {
    let mut channel = AnyValue::new();
    assert!(channel.update(vec![json!(1), json!(2), json!(3)]).unwrap());
    assert_eq!(channel.get().unwrap(), json!(3));
}

#[test]
fn untracked_value_is_never_checkpointed() {
    let mut channel = UntrackedValue::new();
    assert!(matches!(channel.get(), Err(Error::EmptyChannel(_))));

    let payload = json!({"session": "test", "temp": "dir"});
    assert!(channel.update(vec![payload.clone()]).unwrap());
    assert_eq!(channel.get().unwrap(), payload);

    let new_payload = json!({"session": "updated"});
    assert!(channel.update(vec![new_payload.clone()]).unwrap());
    assert_eq!(channel.get().unwrap(), new_payload);

    assert!(channel.checkpoint().is_none());
    let restored = rehydrate(&channel);
    assert!(matches!(restored.get(), Err(Error::EmptyChannel(_))));
}

#[test]
fn untracked_value_rejects_concurrent_updates_when_guarded() {
    let mut channel = UntrackedValue::new();
    assert!(matches!(
        channel.update(vec![json!(1), json!(2)]),
        Err(Error::InvalidUpdate(_))
    ));

    let mut unguarded = UntrackedValue::unguarded();
    assert!(unguarded.update(vec![json!(1), json!(2)]).unwrap());
    assert_eq!(unguarded.get().unwrap(), json!(2));
}

#[test]
fn ephemeral_value_clears_on_empty_update_and_round_trips() {
    let mut channel = EphemeralValue::new();
    assert!(channel.update(vec![json!(42)]).unwrap());
    assert_eq!(channel.get().unwrap(), json!(42));

    let restored = rehydrate(&channel);
    assert_eq!(restored.get().unwrap(), json!(42));

    assert!(channel.update(vec![]).unwrap());
    assert!(!channel.is_available());
    assert!(!channel.update(vec![]).unwrap());
}

#[test]
fn ephemeral_value_guard_rejects_multiple_values() {
    let mut channel = EphemeralValue::new();
    assert!(matches!(
        channel.update(vec![json!(1), json!(2)]),
        Err(Error::InvalidUpdate(_))
    ));

    let mut unguarded = EphemeralValue::unguarded();
    assert!(unguarded.update(vec![json!(1), json!(2)]).unwrap());
    assert_eq!(unguarded.get().unwrap(), json!(2));
}

#[test]
fn topic_flattens_arrays_and_replaces_across_steps() {
    let mut channel = Topic::new();

    assert!(channel.update(vec![json!("a"), json!("b")]).unwrap());
    assert_eq!(channel.get().unwrap(), json!(["a", "b"]));

    assert!(channel.update(vec![json!(["c", "d"]), json!("d")]).unwrap());
    assert_eq!(channel.get().unwrap(), json!(["c", "d", "d"]));

    assert!(channel.update(vec![]).unwrap());
    assert!(matches!(channel.get(), Err(Error::EmptyChannel(_))));
    assert!(!channel.update(vec![]).unwrap(), "empty topic stays empty");

    assert!(channel.update(vec![json!("e")]).unwrap());
    assert_eq!(channel.get().unwrap(), json!(["e"]));

    let restored = rehydrate(&channel);
    assert_eq!(restored.get().unwrap(), json!(["e"]));
}

#[test]
fn topic_accumulating_appends_across_steps() {
    let mut channel = Topic::accumulating();

    assert!(channel.update(vec![json!("a"), json!("b")]).unwrap());
    assert_eq!(channel.get().unwrap(), json!(["a", "b"]));

    assert!(
        channel
            .update(vec![json!("b"), json!(["c", "d"]), json!("d")])
            .unwrap()
    );
    assert_eq!(
        channel.get().unwrap(),
        json!(["a", "b", "b", "c", "d", "d"])
    );

    assert!(!channel.update(vec![]).unwrap());
    assert_eq!(
        channel.get().unwrap(),
        json!(["a", "b", "b", "c", "d", "d"])
    );

    let restored = rehydrate(&channel);
    assert_eq!(
        restored.get().unwrap(),
        json!(["a", "b", "b", "c", "d", "d"])
    );
}

fn integer_adder() -> agent_graph::channels::binop::Reducer {
    Arc::new(|acc: Value, next: Value| {
        let a = acc.as_i64().ok_or_else(|| {
            Error::InvalidUpdate(format!("expected integer accumulator, got {acc}"))
        })?;
        let b = next
            .as_i64()
            .ok_or_else(|| Error::InvalidUpdate(format!("expected integer update, got {next}")))?;
        Ok(json!(a + b))
    })
}

#[test]
fn binop_aggregates_across_steps() {
    let mut channel = BinaryOperatorAggregate::with_initial(integer_adder(), json!(0));
    assert_eq!(channel.get().unwrap(), json!(0));

    assert!(channel.update(vec![json!(1), json!(2), json!(3)]).unwrap());
    assert_eq!(channel.get().unwrap(), json!(6));

    assert!(channel.update(vec![json!(4)]).unwrap());
    assert_eq!(channel.get().unwrap(), json!(10));

    let restored = rehydrate(&channel);
    assert_eq!(restored.get().unwrap(), json!(10));
}

#[test]
fn binop_without_initial_seeds_from_first_update() {
    let mut channel = BinaryOperatorAggregate::new(integer_adder());
    assert!(matches!(channel.get(), Err(Error::EmptyChannel(_))));

    assert!(channel.update(vec![json!(5), json!(6)]).unwrap());
    assert_eq!(channel.get().unwrap(), json!(11));
}

#[test]
fn binop_overwrite_replaces_accumulator() {
    let mut channel = BinaryOperatorAggregate::with_initial(integer_adder(), json!(0));
    channel.update(vec![json!(10), json!(20)]).unwrap();
    assert_eq!(channel.get().unwrap(), json!(30));

    let overwrite = serde_json::to_value(Overwrite::new(json!(100))).unwrap();
    channel.update(vec![overwrite]).unwrap();
    assert_eq!(channel.get().unwrap(), json!(100));

    channel.update(vec![json!({"__overwrite__": 5})]).unwrap();
    assert_eq!(channel.get().unwrap(), json!(5));
}

#[test]
fn binop_rejects_multiple_overwrites_in_one_step() {
    let mut channel = BinaryOperatorAggregate::with_initial(integer_adder(), json!(0));
    let one = json!({"__overwrite__": 1});
    let two = json!({"__overwrite__": 2});
    let err = channel.update(vec![one, two]).unwrap_err();
    assert!(matches!(err, Error::InvalidUpdate(_)));
}

#[test]
fn named_barrier_value_blocks_until_all_seen() {
    let mut channel = NamedBarrierValue::new(["a", "b", "c"]);
    assert!(!channel.is_available());
    assert!(matches!(channel.get(), Err(Error::EmptyChannel(_))));

    assert!(channel.update(vec![json!("a"), json!("b")]).unwrap());
    assert!(!channel.is_available());

    assert!(channel.update(vec![json!("c")]).unwrap());
    assert!(channel.is_available());
    assert_eq!(channel.get().unwrap(), Value::Null);

    let restored = rehydrate(&channel);
    assert!(restored.is_available());

    assert!(channel.consume());
    assert!(!channel.is_available());
}

#[test]
fn named_barrier_value_rejects_unknown_names_and_non_strings() {
    let mut channel = NamedBarrierValue::new(["a", "b"]);
    assert!(matches!(
        channel.update(vec![json!("c")]),
        Err(Error::InvalidUpdate(_))
    ));
    assert!(matches!(
        channel.update(vec![json!(1)]),
        Err(Error::InvalidUpdate(_))
    ));
}

#[test]
fn named_barrier_value_after_finish_requires_finish() {
    let mut channel = NamedBarrierValueAfterFinish::new(["a", "b"]);
    channel.update(vec![json!("a"), json!("b")]).unwrap();
    assert!(!channel.is_available());

    assert!(channel.finish());
    assert!(channel.is_available());
    assert_eq!(channel.get().unwrap(), Value::Null);

    let restored = rehydrate(&channel);
    assert!(restored.is_available());

    assert!(channel.consume());
    assert!(!channel.is_available());
}

#[test]
fn key_is_plumbed_into_error_messages() {
    let mut channel = LastValue::new();
    channel.set_key("counter".to_owned());
    assert_eq!(channel.key(), "counter");

    let err = channel.get().unwrap_err();
    match err {
        Error::EmptyChannel(key) => assert_eq!(key, "counter"),
        other => panic!("expected EmptyChannel, got {other:?}"),
    }
}

#[test]
fn clone_channel_produces_independent_state() {
    let mut original = Topic::accumulating();
    original.update(vec![json!("a")]).unwrap();

    let mut fork = original.clone_channel();
    fork.update(vec![json!("b")]).unwrap();

    assert_eq!(original.get().unwrap(), json!(["a"]));
    assert_eq!(fork.get().unwrap(), json!(["a", "b"]));
}
