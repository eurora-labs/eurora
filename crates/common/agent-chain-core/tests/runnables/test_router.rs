use std::collections::HashMap;

use agent_chain_core::error::{Error, Result};
use agent_chain_core::runnables::base::{Runnable, RunnableLambda};
use agent_chain_core::runnables::config::RunnableConfig;
use agent_chain_core::runnables::router::{RouterInput, RouterRunnable};
use futures::StreamExt;
use serde_json::Value;

#[test]
fn test_router_initialization() {
    let router = RouterRunnable::<i32, i32>::new()
        .add("add", RunnableLambda::new(|x: i32| Ok(x + 1)))
        .add("multiply", RunnableLambda::new(|x: i32| Ok(x * 2)));

    let debug_str = format!("{:?}", router);
    assert!(debug_str.contains("add"));
    assert!(debug_str.contains("multiply"));
}

#[test]
fn test_router_initialization_with_runnables() {
    let add_runnable = RunnableLambda::new(|x: i32| Ok(x + 1));
    let multiply_runnable = RunnableLambda::new(|x: i32| Ok(x * 2));

    let router = RouterRunnable::new()
        .add("add", add_runnable)
        .add("multiply", multiply_runnable);

    assert_eq!(router.invoke(RouterInput::new("add", 5), None).unwrap(), 6);
    assert_eq!(
        router
            .invoke(RouterInput::new("multiply", 5), None)
            .unwrap(),
        10
    );
}

#[test]
fn test_router_invoke() {
    let router = RouterRunnable::new()
        .add("add", RunnableLambda::new(|x: i32| Ok(x + 1)))
        .add("multiply", RunnableLambda::new(|x: i32| Ok(x * 2)));

    assert_eq!(router.invoke(RouterInput::new("add", 5), None).unwrap(), 6);
    assert_eq!(
        router
            .invoke(RouterInput::new("multiply", 5), None)
            .unwrap(),
        10
    );
}

#[test]
fn test_router_invoke_invalid_key() {
    let router = RouterRunnable::new().add("add", RunnableLambda::new(|x: i32| Ok(x + 1)));

    let result = router.invoke(RouterInput::new("invalid", 5), None);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No runnable associated with key 'invalid'")
    );
}

#[test]
fn test_router_invoke_with_spy() {
    let router = RouterRunnable::new()
        .add("a", RunnableLambda::new(|x: i32| Ok(x + 10)))
        .add("b", RunnableLambda::new(|x: i32| Ok(x + 20)));

    assert_eq!(router.invoke(RouterInput::new("a", 5), None).unwrap(), 15);
    assert_eq!(router.invoke(RouterInput::new("b", 5), None).unwrap(), 25);
}

#[tokio::test]
async fn test_router_ainvoke() {
    let router = RouterRunnable::new()
        .add("add", RunnableLambda::new(|x: i32| Ok(x + 1)))
        .add("multiply", RunnableLambda::new(|x: i32| Ok(x * 2)));

    assert_eq!(
        router
            .ainvoke(RouterInput::new("add", 5), None)
            .await
            .unwrap(),
        6
    );
    assert_eq!(
        router
            .ainvoke(RouterInput::new("multiply", 5), None)
            .await
            .unwrap(),
        10
    );
}

#[tokio::test]
async fn test_router_ainvoke_invalid_key() {
    let router = RouterRunnable::new().add("add", RunnableLambda::new(|x: i32| Ok(x + 1)));

    let result = router.ainvoke(RouterInput::new("invalid", 5), None).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No runnable associated with key 'invalid'")
    );
}

#[test]
fn test_router_batch() {
    let router = RouterRunnable::new()
        .add("add", RunnableLambda::new(|x: i32| Ok(x + 1)))
        .add("multiply", RunnableLambda::new(|x: i32| Ok(x * 2)));

    let inputs = vec![
        RouterInput::new("add", 1),
        RouterInput::new("multiply", 2),
        RouterInput::new("add", 3),
    ];

    let results = router.batch(inputs, None, false);
    assert_eq!(*results[0].as_ref().unwrap(), 2);
    assert_eq!(*results[1].as_ref().unwrap(), 4);
    assert_eq!(*results[2].as_ref().unwrap(), 4);
}

#[test]
fn test_router_batch_invalid_key() {
    let router = RouterRunnable::new().add("add", RunnableLambda::new(|x: i32| Ok(x + 1)));

    let inputs = vec![RouterInput::new("add", 1), RouterInput::new("invalid", 2)];

    let results = router.batch(inputs, None, false);
    assert!(results.iter().any(|r| r.is_err()));
    let err = results.iter().find(|r| r.is_err()).unwrap();
    assert!(
        err.as_ref()
            .unwrap_err()
            .to_string()
            .contains("do not have a corresponding runnable")
    );
}

#[test]
fn test_router_batch_all_same_key() {
    let router = RouterRunnable::new().add("add", RunnableLambda::new(|x: i32| Ok(x + 1)));

    let inputs = vec![
        RouterInput::new("add", 1),
        RouterInput::new("add", 2),
        RouterInput::new("add", 3),
    ];

    let results = router.batch(inputs, None, false);
    assert_eq!(*results[0].as_ref().unwrap(), 2);
    assert_eq!(*results[1].as_ref().unwrap(), 3);
    assert_eq!(*results[2].as_ref().unwrap(), 4);
}

#[test]
fn test_router_batch_different_keys() {
    let router = RouterRunnable::new()
        .add("add", RunnableLambda::new(|x: i32| Ok(x + 1)))
        .add("multiply", RunnableLambda::new(|x: i32| Ok(x * 2)))
        .add("square", RunnableLambda::new(|x: i32| Ok(x * x)));

    let inputs = vec![
        RouterInput::new("add", 1),
        RouterInput::new("multiply", 2),
        RouterInput::new("square", 3),
        RouterInput::new("add", 4),
    ];

    let results = router.batch(inputs, None, false);
    assert_eq!(*results[0].as_ref().unwrap(), 2);
    assert_eq!(*results[1].as_ref().unwrap(), 4);
    assert_eq!(*results[2].as_ref().unwrap(), 9);
    assert_eq!(*results[3].as_ref().unwrap(), 5);
}

#[test]
fn test_router_batch_return_exceptions() {
    let router = RouterRunnable::new()
        .add("add", RunnableLambda::new(|x: i32| Ok(x + 1)))
        .add(
            "fail",
            RunnableLambda::new(|_x: i32| Err::<i32, _>(Error::other("Always fails"))),
        );

    let inputs = vec![
        RouterInput::new("add", 1),
        RouterInput::new("fail", 2),
        RouterInput::new("add", 3),
    ];

    let results = router.batch(inputs, None, true);
    assert_eq!(*results[0].as_ref().unwrap(), 2);
    assert!(results[1].is_err());
    assert!(
        results[1]
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("Always fails")
    );
    assert_eq!(*results[2].as_ref().unwrap(), 4);
}

#[test]
fn test_router_empty_batch() {
    let router = RouterRunnable::new().add("add", RunnableLambda::new(|x: i32| Ok(x + 1)));

    let results = router.batch(vec![], None, false);
    assert!(results.is_empty());
}

#[test]
fn test_router_batch_with_configs() {
    let router = RouterRunnable::new().add("add", RunnableLambda::new(|x: i32| Ok(x + 1)));

    let inputs = vec![
        RouterInput::new("add", 1),
        RouterInput::new("add", 2),
        RouterInput::new("add", 3),
    ];

    let results = router.batch(inputs, None, false);
    assert_eq!(*results[0].as_ref().unwrap(), 2);
    assert_eq!(*results[1].as_ref().unwrap(), 3);
    assert_eq!(*results[2].as_ref().unwrap(), 4);
}

#[tokio::test]
async fn test_router_abatch() {
    let router = RouterRunnable::new()
        .add("add", RunnableLambda::new(|x: i32| Ok(x + 1)))
        .add("multiply", RunnableLambda::new(|x: i32| Ok(x * 2)));

    let inputs = vec![
        RouterInput::new("add", 1),
        RouterInput::new("multiply", 2),
        RouterInput::new("add", 3),
    ];

    let results = router.abatch(inputs, None, false).await;
    assert_eq!(*results[0].as_ref().unwrap(), 2);
    assert_eq!(*results[1].as_ref().unwrap(), 4);
    assert_eq!(*results[2].as_ref().unwrap(), 4);
}

#[tokio::test]
async fn test_router_abatch_invalid_key() {
    let router = RouterRunnable::new().add("add", RunnableLambda::new(|x: i32| Ok(x + 1)));

    let inputs = vec![RouterInput::new("add", 1), RouterInput::new("invalid", 2)];

    let results = router.abatch(inputs, None, false).await;
    assert!(results.iter().any(|r| r.is_err()));
}

#[tokio::test]
async fn test_router_abatch_return_exceptions() {
    let router = RouterRunnable::new()
        .add("add", RunnableLambda::new(|x: i32| Ok(x + 1)))
        .add(
            "fail",
            RunnableLambda::new(|_x: i32| Err::<i32, _>(Error::other("Always fails"))),
        );

    let inputs = vec![
        RouterInput::new("add", 1),
        RouterInput::new("fail", 2),
        RouterInput::new("add", 3),
    ];

    let results = router.abatch(inputs, None, true).await;
    assert_eq!(*results[0].as_ref().unwrap(), 2);
    assert!(results[1].is_err());
    assert_eq!(*results[2].as_ref().unwrap(), 4);
}

#[tokio::test]
async fn test_router_empty_abatch() {
    let router = RouterRunnable::new().add("add", RunnableLambda::new(|x: i32| Ok(x + 1)));

    let results = router.abatch(vec![], None, false).await;
    assert!(results.is_empty());
}

#[tokio::test]
async fn test_router_abatch_different_keys() {
    let router = RouterRunnable::new()
        .add("add", RunnableLambda::new(|x: i32| Ok(x + 1)))
        .add("multiply", RunnableLambda::new(|x: i32| Ok(x * 2)));

    let inputs = vec![
        RouterInput::new("add", 1),
        RouterInput::new("multiply", 2),
        RouterInput::new("add", 3),
    ];

    let results = router.abatch(inputs, None, false).await;
    assert_eq!(*results[0].as_ref().unwrap(), 2);
    assert_eq!(*results[1].as_ref().unwrap(), 4);
    assert_eq!(*results[2].as_ref().unwrap(), 4);
}

#[tokio::test]
async fn test_router_stream() {
    let router = RouterRunnable::new().add("gen", RunnableLambda::new(|x: i32| Ok(x + 1)));

    let result: Vec<i32> = router
        .stream(RouterInput::new("gen", 5), None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(result, vec![6]);
}

#[tokio::test]
async fn test_router_stream_invalid_key() {
    let router = RouterRunnable::new().add("gen", RunnableLambda::new(|x: i32| Ok(x + 1)));

    let results: Vec<Result<i32>> = router
        .stream(RouterInput::new("invalid", 5), None)
        .collect()
        .await;

    assert_eq!(results.len(), 1);
    assert!(results[0].is_err());
    assert!(
        results[0]
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("No runnable associated with key 'invalid'")
    );
}

#[tokio::test]
async fn test_router_astream() {
    let router = RouterRunnable::new().add("gen", RunnableLambda::new(|x: i32| Ok(x + 1)));

    let result: Vec<i32> = router
        .astream(RouterInput::new("gen", 5), None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(result, vec![6]);
}

#[tokio::test]
async fn test_router_astream_invalid_key() {
    let router = RouterRunnable::new().add("gen", RunnableLambda::new(|x: i32| Ok(x + 1)));

    let results: Vec<Result<i32>> = router
        .astream(RouterInput::new("invalid", 5), None)
        .collect()
        .await;

    assert_eq!(results.len(), 1);
    assert!(results[0].is_err());
}

#[tokio::test]
async fn test_router_stream_sync() {
    let router = RouterRunnable::new().add("id", RunnableLambda::new(|x: String| Ok(x)));

    let result: Vec<String> = router
        .stream(RouterInput::new("id", "test".to_string()), None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(result, vec!["test".to_string()]);
}

#[test]
fn test_router_with_config() {
    let router = RouterRunnable::new().add("add", RunnableLambda::new(|x: i32| Ok(x + 1)));

    let mut config = RunnableConfig::default();
    config.tags.push("test-tag".to_string());

    let result = router
        .invoke(RouterInput::new("add", 5), Some(config))
        .unwrap();
    assert_eq!(result, 6);
}

#[test]
fn test_router_with_different_input_types() {
    let router = RouterRunnable::<Value, Value>::new()
        .add(
            "string",
            RunnableLambda::new(|x: Value| {
                let s = x.as_str().unwrap_or("");
                Ok(Value::String(s.to_uppercase()))
            }),
        )
        .add(
            "int",
            RunnableLambda::new(|x: Value| {
                let n = x.as_i64().unwrap_or(0);
                Ok(serde_json::json!(n * 2))
            }),
        );

    let result = router
        .invoke(
            RouterInput::new("string", Value::String("hello".into())),
            None,
        )
        .unwrap();
    assert_eq!(result, Value::String("HELLO".into()));

    let result = router
        .invoke(RouterInput::new("int", serde_json::json!(5)), None)
        .unwrap();
    assert_eq!(result, serde_json::json!(10));
}

#[test]
fn test_router_with_dict_input() {
    let router = RouterRunnable::<Value, String>::new().add(
        "process",
        RunnableLambda::new(|x: Value| {
            let name = x.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let age = x.get("age").and_then(|v| v.as_i64()).unwrap_or(0);
            Ok(format!("{name} is {age} years old"))
        }),
    );

    let result = router
        .invoke(
            RouterInput::new("process", serde_json::json!({"name": "Alice", "age": 30})),
            None,
        )
        .unwrap();
    assert_eq!(result, "Alice is 30 years old");
}

#[test]
fn test_router_complex_routing_logic() {
    let router = RouterRunnable::<Value, String>::new()
        .add(
            "a",
            RunnableLambda::new(|x: Value| {
                let v = x.get("value").and_then(|v| v.as_str()).unwrap_or("");
                Ok(format!("Option A: {v}"))
            }),
        )
        .add(
            "b",
            RunnableLambda::new(|x: Value| {
                let v = x.get("value").and_then(|v| v.as_str()).unwrap_or("");
                Ok(format!("Option B: {v}"))
            }),
        )
        .add(
            "c",
            RunnableLambda::new(|x: Value| {
                let v = x.get("value").and_then(|v| v.as_str()).unwrap_or("");
                Ok(format!("Option C: {v}"))
            }),
        );

    let cases = vec![
        ("a", "test1", "Option A: test1"),
        ("b", "test2", "Option B: test2"),
        ("c", "test3", "Option C: test3"),
    ];

    for (key, value, expected) in cases {
        let result = router
            .invoke(
                RouterInput::new(key, serde_json::json!({"value": value})),
                None,
            )
            .unwrap();
        assert_eq!(result, expected);
    }
}

#[test]
fn test_router_single_route() {
    let router = RouterRunnable::new().add("only", RunnableLambda::new(|x: i32| Ok(x * 3)));

    let result = router.invoke(RouterInput::new("only", 4), None).unwrap();
    assert_eq!(result, 12);
}

#[test]
fn test_router_input_type() {
    let input = RouterInput::new("test", 42);
    assert_eq!(input.key, "test");
    assert_eq!(input.input, 42);
}

#[test]
fn test_router_serialization() {
    use agent_chain_core::load::Serializable;
    assert!(RouterRunnable::<i32, i32>::is_lc_serializable());
    assert_eq!(
        RouterRunnable::<i32, i32>::get_lc_namespace(),
        vec!["langchain", "schema", "runnable"]
    );
}

#[test]
fn test_router_debug() {
    let router = RouterRunnable::new().add("add", RunnableLambda::new(|x: i32| Ok(x + 1)));
    let debug = format!("{:?}", router);
    assert!(debug.contains("RouterRunnable"));
    assert!(debug.contains("add"));
}

#[test]
fn test_router_name() {
    let router = RouterRunnable::new()
        .add("add", RunnableLambda::new(|x: i32| Ok(x + 1)))
        .add("square", RunnableLambda::new(|x: i32| Ok(x * x)));

    let name = router.name().unwrap();
    assert!(name.starts_with("RouterRunnable<"));
    assert!(name.contains("add"));
    assert!(name.contains("square"));

    let named = RouterRunnable::new()
        .add("x", RunnableLambda::new(|x: i32| Ok(x)))
        .with_name("my_router");
    assert_eq!(named.name(), Some("my_router".to_string()));
}

#[test]
fn test_router_config_specs() {
    let router = RouterRunnable::new().add("add", RunnableLambda::new(|x: i32| Ok(x + 1)));

    let specs = router.config_specs().unwrap();
    assert!(specs.is_empty());
}

#[test]
fn test_router_default() {
    let router = RouterRunnable::<i32, i32>::default();
    let result = router.invoke(RouterInput::new("any", 5), None);
    assert!(result.is_err()); // No routes registered
}

#[test]
fn test_router_from_runnables() {
    use std::sync::Arc;
    let mut map: HashMap<String, Arc<dyn Runnable<Input = i32, Output = i32> + Send + Sync>> =
        HashMap::new();
    map.insert(
        "add".to_string(),
        Arc::new(RunnableLambda::new(|x: i32| Ok(x + 1))),
    );
    let router = RouterRunnable::from_runnables(map);

    assert_eq!(router.invoke(RouterInput::new("add", 5), None).unwrap(), 6);
}
