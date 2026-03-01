use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

use agent_chain_core::runnables::base::{Runnable, RunnableLambda, RunnableParallel};
use agent_chain_core::runnables::config::RunnableConfig;
use agent_chain_core::runnables::passthrough::{RunnableAssign, RunnablePassthrough, RunnablePick};
use futures::StreamExt;
use serde_json::{Value, json};

fn make_input(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

#[test]
fn test_passthrough_identity() {
    let passthrough: RunnablePassthrough<i32> = RunnablePassthrough::builder().build();
    assert_eq!(passthrough.invoke(5, None).unwrap(), 5);

    let passthrough_str: RunnablePassthrough<String> = RunnablePassthrough::builder().build();
    assert_eq!(
        passthrough_str.invoke("hello".to_string(), None).unwrap(),
        "hello"
    );

    let passthrough_vec: RunnablePassthrough<Vec<i32>> = RunnablePassthrough::builder().build();
    assert_eq!(
        passthrough_vec.invoke(vec![1, 2, 3], None).unwrap(),
        vec![1, 2, 3]
    );

    let passthrough_map: RunnablePassthrough<HashMap<String, Value>> =
        RunnablePassthrough::builder().build();
    let input = make_input(&[("key", json!("value"))]);
    assert_eq!(passthrough_map.invoke(input.clone(), None).unwrap(), input);
}

#[tokio::test]
async fn test_passthrough_identity_async() {
    let passthrough: RunnablePassthrough<i32> = RunnablePassthrough::builder().build();
    assert_eq!(passthrough.ainvoke(5, None).await.unwrap(), 5);

    let passthrough_str: RunnablePassthrough<String> = RunnablePassthrough::builder().build();
    assert_eq!(
        passthrough_str
            .ainvoke("hello".to_string(), None)
            .await
            .unwrap(),
        "hello"
    );

    let passthrough_map: RunnablePassthrough<HashMap<String, Value>> =
        RunnablePassthrough::builder().build();
    let input = make_input(&[("key", json!("value"))]);
    assert_eq!(
        passthrough_map.ainvoke(input.clone(), None).await.unwrap(),
        input
    );
}

#[test]
fn test_passthrough_with_func() {
    let calls: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
    let calls_clone = calls.clone();

    let passthrough: RunnablePassthrough<i32> =
        RunnablePassthrough::with_func(move |x: &i32, _config: &RunnableConfig| {
            calls_clone.lock().unwrap().push(*x);
        });

    let result = passthrough.invoke(5, None).unwrap();
    assert_eq!(result, 5);
    assert_eq!(*calls.lock().unwrap(), vec![5]);

    let result = passthrough.invoke(10, None).unwrap();
    assert_eq!(result, 10);
    assert_eq!(*calls.lock().unwrap(), vec![5, 10]);
}

#[tokio::test]
async fn test_passthrough_with_afunc() {
    let calls: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
    let calls_clone = calls.clone();

    let passthrough: RunnablePassthrough<i32> =
        RunnablePassthrough::with_afunc(move |x: &i32, _config: &RunnableConfig| {
            let val = *x;
            let calls = calls_clone.clone();
            async move {
                calls.lock().unwrap().push(val);
            }
        });

    let result = passthrough.ainvoke(5, None).await.unwrap();
    assert_eq!(result, 5);
    assert_eq!(*calls.lock().unwrap(), vec![5]);
}

#[tokio::test]
async fn test_passthrough_stream() {
    let passthrough: RunnablePassthrough<i32> = RunnablePassthrough::builder().build();
    let result: Vec<i32> = passthrough
        .stream(42, None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;
    assert_eq!(result, vec![42]);
}

#[tokio::test]
async fn test_passthrough_astream() {
    let passthrough: RunnablePassthrough<i32> = RunnablePassthrough::builder().build();
    let result: Vec<i32> = passthrough
        .stream(42, None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;
    assert_eq!(result, vec![42]);
}

#[test]
fn test_passthrough_batch() {
    let passthrough: RunnablePassthrough<i32> = RunnablePassthrough::builder().build();
    let inputs = vec![1, 2, 3, 4, 5];
    let results: Vec<i32> = inputs
        .into_iter()
        .map(|i| passthrough.invoke(i, None).unwrap())
        .collect();
    assert_eq!(results, vec![1, 2, 3, 4, 5]);
}

#[tokio::test]
async fn test_passthrough_transform() {
    let passthrough: RunnablePassthrough<i32> = RunnablePassthrough::builder().build();
    let input_stream = futures::stream::iter(vec![1, 2, 3]);
    let result: Vec<i32> = passthrough
        .transform(Box::pin(input_stream), None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;
    assert_eq!(result, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_passthrough_atransform() {
    let passthrough: RunnablePassthrough<i32> = RunnablePassthrough::builder().build();
    let input_stream = futures::stream::iter(vec![1, 2, 3]);
    let result: Vec<i32> = passthrough
        .atransform(Box::pin(input_stream), None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;
    assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn test_passthrough_with_func_and_config() {
    let tags_seen: Arc<Mutex<Vec<Vec<String>>>> = Arc::new(Mutex::new(Vec::new()));
    let tags_clone = tags_seen.clone();

    let passthrough: RunnablePassthrough<i32> =
        RunnablePassthrough::with_func(move |_x: &i32, config: &RunnableConfig| {
            tags_clone.lock().unwrap().push(config.tags.clone());
        });

    let mut config = RunnableConfig::default();
    config.tags.push("test-tag".to_string());

    let result = passthrough.invoke(5, Some(config)).unwrap();
    assert_eq!(result, 5);

    let seen = tags_seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert!(seen[0].contains(&"test-tag".to_string()));
}

#[test]
fn test_passthrough_with_side_effect_batch() {
    let calls: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
    let calls_clone = calls.clone();

    let passthrough: RunnablePassthrough<i32> =
        RunnablePassthrough::with_func(move |x: &i32, _config: &RunnableConfig| {
            calls_clone.lock().unwrap().push(*x);
        });

    let results: Vec<i32> = vec![1, 2, 3]
        .into_iter()
        .map(|i| passthrough.invoke(i, None).unwrap())
        .collect();
    assert_eq!(results, vec![1, 2, 3]);

    let mut recorded = calls.lock().unwrap().clone();
    recorded.sort();
    assert_eq!(recorded, vec![1, 2, 3]);
}

#[test]
fn test_passthrough_repr() {
    let passthrough: RunnablePassthrough<i32> = RunnablePassthrough::builder().build();
    let repr_str = format!("{:?}", passthrough);
    assert!(repr_str.contains("RunnablePassthrough"));
}

#[test]
fn test_passthrough_with_none_func() {
    let passthrough: RunnablePassthrough<i32> = RunnablePassthrough::builder().build();
    let result = passthrough.invoke(42, None).unwrap();
    assert_eq!(result, 42);
}

#[test]
fn test_passthrough_in_parallel() {
    let parallel = RunnableParallel::<i32>::builder()
        .build()
        .add(
            "original",
            RunnableLambda::builder()
                .func(|x: i32| Ok(json!(x)))
                .build(),
        )
        .add(
            "modified",
            RunnableLambda::builder()
                .func(|x: i32| Ok(json!(x + 1)))
                .build(),
        );

    let result = parallel.invoke(5, None).unwrap();
    assert_eq!(result["original"], json!(5));
    assert_eq!(result["modified"], json!(6));
}

#[tokio::test]
async fn test_passthrough_transform_with_func() {
    let calls: Arc<Mutex<Vec<i32>>> = Arc::new(Mutex::new(Vec::new()));
    let calls_clone = calls.clone();

    let passthrough: RunnablePassthrough<i32> =
        RunnablePassthrough::with_func(move |x: &i32, _config: &RunnableConfig| {
            calls_clone.lock().unwrap().push(*x);
        });

    let input_stream = futures::stream::iter(vec![1, 2, 3]);
    let result: Vec<i32> = passthrough
        .transform(Box::pin(input_stream), None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(result, vec![1, 2, 3]);
    let recorded = calls.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0], 3);
}

#[test]
fn test_assign_basic() {
    let mapper = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "new_key",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let val = x.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(val * 2))
                })
                .build(),
        );
    let assign = RunnableAssign::builder().mapper(mapper).build();

    let input = make_input(&[("value", json!(5))]);
    let result = assign.invoke(input, None).unwrap();

    assert_eq!(result["value"], json!(5));
    assert_eq!(result["new_key"], json!(10));
}

#[tokio::test]
async fn test_assign_basic_async() {
    let mapper = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "new_key",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let val = x.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(val * 2))
                })
                .build(),
        );
    let assign = RunnableAssign::builder().mapper(mapper).build();

    let input = make_input(&[("value", json!(5))]);
    let result = assign.ainvoke(input, None).await.unwrap();

    assert_eq!(result["value"], json!(5));
    assert_eq!(result["new_key"], json!(10));
}

#[test]
fn test_assign_multiple_keys() {
    let mapper = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "doubled",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let val = x.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(val * 2))
                })
                .build(),
        )
        .add(
            "tripled",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let val = x.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(val * 3))
                })
                .build(),
        )
        .add(
            "quadrupled",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let val = x.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(val * 4))
                })
                .build(),
        );
    let assign = RunnableAssign::builder().mapper(mapper).build();

    let input = make_input(&[("value", json!(5))]);
    let result = assign.invoke(input, None).unwrap();

    assert_eq!(result["value"], json!(5));
    assert_eq!(result["doubled"], json!(10));
    assert_eq!(result["tripled"], json!(15));
    assert_eq!(result["quadrupled"], json!(20));
}

#[test]
fn test_assign_overwrite_existing() {
    let mapper = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "value",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let val = x.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(val * 2))
                })
                .build(),
        );
    let assign = RunnableAssign::builder().mapper(mapper).build();

    let input = make_input(&[("value", json!(5)), ("other", json!("data"))]);
    let result = assign.invoke(input, None).unwrap();

    assert_eq!(result["value"], json!(10));
    assert_eq!(result["other"], json!("data"));
}

#[test]
fn test_assign_with_runnable() {
    let double = RunnableLambda::builder()
        .func(|x: HashMap<String, Value>| {
            let val = x.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
            Ok(json!(val * 2))
        })
        .build();
    let mapper = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add("new_key", double);
    let assign = RunnableAssign::builder().mapper(mapper).build();

    let input = make_input(&[("value", json!(5))]);
    let result = assign.invoke(input, None).unwrap();

    assert_eq!(result["value"], json!(5));
    assert_eq!(result["new_key"], json!(10));
}

#[test]
fn test_assign_batch() {
    let mapper = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "new_key",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let val = x.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(val * 2))
                })
                .build(),
        );
    let assign = RunnableAssign::builder().mapper(mapper).build();

    let inputs = vec![
        make_input(&[("value", json!(1))]),
        make_input(&[("value", json!(2))]),
        make_input(&[("value", json!(3))]),
    ];

    let results: Vec<HashMap<String, Value>> = inputs
        .into_iter()
        .map(|i| assign.invoke(i, None).unwrap())
        .collect();

    assert_eq!(results[0]["value"], json!(1));
    assert_eq!(results[0]["new_key"], json!(2));
    assert_eq!(results[1]["value"], json!(2));
    assert_eq!(results[1]["new_key"], json!(4));
    assert_eq!(results[2]["value"], json!(3));
    assert_eq!(results[2]["new_key"], json!(6));
}

#[tokio::test]
async fn test_assign_stream() {
    let mapper = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "doubled",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let val = x.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(val * 2))
                })
                .build(),
        );
    let assign = RunnableAssign::builder().mapper(mapper).build();

    let input = make_input(&[("value", json!(5))]);
    let chunks: Vec<HashMap<String, Value>> = assign
        .stream(input, None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    let mut final_result: HashMap<String, Value> = HashMap::new();
    for chunk in chunks {
        final_result.extend(chunk);
    }

    assert_eq!(final_result["value"], json!(5));
    assert_eq!(final_result["doubled"], json!(10));
}

#[tokio::test]
async fn test_assign_transform() {
    let mapper = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "doubled",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let val = x.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(val * 2))
                })
                .build(),
        );
    let assign = RunnableAssign::builder().mapper(mapper).build();

    let input_stream = futures::stream::iter(vec![make_input(&[("value", json!(5))])]);
    let chunks: Vec<HashMap<String, Value>> = assign
        .transform(Box::pin(input_stream), None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    let mut final_result: HashMap<String, Value> = HashMap::new();
    for chunk in chunks {
        final_result.extend(chunk);
    }

    assert_eq!(final_result["value"], json!(5));
    assert_eq!(final_result["doubled"], json!(10));
}

#[test]
fn test_assign_empty_dict() {
    let mapper = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "new_key",
            RunnableLambda::builder()
                .func(|_x: HashMap<String, Value>| Ok(json!(42)))
                .build(),
        );
    let assign = RunnableAssign::builder().mapper(mapper).build();

    let result = assign.invoke(HashMap::new(), None).unwrap();
    assert_eq!(result["new_key"], json!(42));
}

#[test]
fn test_assign_preserves_original_order() {
    let mapper = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "z",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let a = x.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
                    let b = x.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(a + b))
                })
                .build(),
        );
    let assign = RunnableAssign::builder().mapper(mapper).build();

    let input = make_input(&[("a", json!(1)), ("b", json!(2)), ("c", json!(3))]);
    let result = assign.invoke(input, None).unwrap();

    assert!(result.contains_key("a"));
    assert!(result.contains_key("b"));
    assert!(result.contains_key("c"));
    assert_eq!(result["z"], json!(3));
}

#[test]
fn test_assign_with_config_propagation() {
    let configs_seen = Arc::new(AtomicI32::new(0));
    let configs_clone = configs_seen.clone();

    let mapper = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "new_key",
            RunnableLambda::builder()
                .func(move |x: HashMap<String, Value>| {
                    configs_clone.fetch_add(1, Ordering::SeqCst);
                    let val = x.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(val * 2))
                })
                .build(),
        );
    let assign = RunnableAssign::builder().mapper(mapper).build();

    let mut config = RunnableConfig::default();
    config.tags.push("my-tag".to_string());

    let input = make_input(&[("value", json!(5))]);
    let result = assign.invoke(input, Some(config)).unwrap();

    assert_eq!(result["value"], json!(5));
    assert_eq!(result["new_key"], json!(10));
    assert_eq!(configs_seen.load(Ordering::SeqCst), 1);
}

#[test]
fn test_assign_with_multiple_parallel_ops() {
    let mapper = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "sum",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let a = x.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
                    let b = x.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(a + b))
                })
                .build(),
        )
        .add(
            "product",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let a = x.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
                    let b = x.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(a * b))
                })
                .build(),
        )
        .add(
            "difference",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let a = x.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
                    let b = x.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(a - b))
                })
                .build(),
        );
    let assign = RunnableAssign::builder().mapper(mapper).build();

    let input = make_input(&[("a", json!(10)), ("b", json!(3))]);
    let result = assign.invoke(input, None).unwrap();

    assert_eq!(result["a"], json!(10));
    assert_eq!(result["b"], json!(3));
    assert_eq!(result["sum"], json!(13));
    assert_eq!(result["product"], json!(30));
    assert_eq!(result["difference"], json!(7));
}

#[test]
fn test_assign_direct_instantiation() {
    let mapper = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "new_field",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let val = x.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(val * 2))
                })
                .build(),
        );
    let assign = RunnableAssign::builder().mapper(mapper).build();

    let input = make_input(&[("value", json!(5))]);
    let result = assign.invoke(input, None).unwrap();

    assert_eq!(result["value"], json!(5));
    assert_eq!(result["new_field"], json!(10));
}

#[test]
fn test_assign_nested() {
    let mapper1 = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "step1",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let val = x.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(val + 1))
                })
                .build(),
        );
    let assign1 = RunnableAssign::builder().mapper(mapper1).build();

    let mapper2 = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "step2",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let val = x.get("step1").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(val * 2))
                })
                .build(),
        );
    let assign2 = RunnableAssign::builder().mapper(mapper2).build();

    let input = make_input(&[("value", json!(5))]);
    let intermediate = assign1.invoke(input, None).unwrap();
    let result = assign2.invoke(intermediate, None).unwrap();

    assert_eq!(result["value"], json!(5));
    assert_eq!(result["step1"], json!(6));
    assert_eq!(result["step2"], json!(12));
}

#[test]
fn test_assign_with_parallel() {
    let mapper = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "doubled",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let val = x.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(val * 2))
                })
                .build(),
        )
        .add(
            "tripled",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let val = x.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(val * 3))
                })
                .build(),
        );
    let assign = RunnableAssign::builder().mapper(mapper).build();

    let input = make_input(&[("value", json!(5))]);
    let result = assign.invoke(input, None).unwrap();

    assert_eq!(result["value"], json!(5));
    assert_eq!(result["doubled"], json!(10));
    assert_eq!(result["tripled"], json!(15));
}

#[test]
fn test_pick_single_key() {
    let pick = RunnablePick::new_single().key("name").call();

    let input = make_input(&[
        ("name", json!("Alice")),
        ("age", json!(30)),
        ("city", json!("NYC")),
    ]);

    let result = pick.invoke(input, None).unwrap();
    assert_eq!(result, json!("Alice"));
}

#[test]
fn test_pick_multiple_keys() {
    let pick = RunnablePick::new_multi(vec!["name", "age"], None);

    let input = make_input(&[
        ("name", json!("Alice")),
        ("age", json!(30)),
        ("city", json!("NYC")),
    ]);

    let result = pick.invoke(input, None).unwrap();
    let result_map: HashMap<String, Value> = serde_json::from_value(result).unwrap();
    assert_eq!(result_map.len(), 2);
    assert_eq!(result_map["name"], json!("Alice"));
    assert_eq!(result_map["age"], json!(30));
}

#[tokio::test]
async fn test_pick_single_key_async() {
    let pick = RunnablePick::new_single().key("name").call();

    let input = make_input(&[("name", json!("Alice")), ("age", json!(30))]);
    let result = pick.ainvoke(input, None).await.unwrap();
    assert_eq!(result, json!("Alice"));
}

#[tokio::test]
async fn test_pick_multiple_keys_async() {
    let pick = RunnablePick::new_multi(vec!["name", "age"], None);

    let input = make_input(&[
        ("name", json!("Alice")),
        ("age", json!(30)),
        ("city", json!("NYC")),
    ]);

    let result = pick.ainvoke(input, None).await.unwrap();
    let result_map: HashMap<String, Value> = serde_json::from_value(result).unwrap();
    assert_eq!(result_map["name"], json!("Alice"));
    assert_eq!(result_map["age"], json!(30));
}

#[test]
fn test_pick_missing_key() {
    let pick = RunnablePick::new_single().key("missing").call();

    let input = make_input(&[("name", json!("Alice"))]);
    let result = pick.invoke(input, None);
    assert!(result.is_err());
}

#[test]
fn test_pick_partial_keys() {
    let pick = RunnablePick::new_multi(vec!["name", "missing"], None);

    let input = make_input(&[("name", json!("Alice")), ("age", json!(30))]);
    let result = pick.invoke(input, None).unwrap();

    let result_map: HashMap<String, Value> = serde_json::from_value(result).unwrap();
    assert_eq!(result_map.len(), 1);
    assert_eq!(result_map["name"], json!("Alice"));
}

#[test]
fn test_pick_all_missing_keys() {
    let pick = RunnablePick::new_multi(vec!["missing1", "missing2"], None);

    let input = make_input(&[("name", json!("Alice"))]);
    let result = pick.invoke(input, None);
    assert!(result.is_err());
}

#[test]
fn test_pick_batch() {
    let pick = RunnablePick::new_single().key("name").call();

    let inputs = vec![
        make_input(&[("name", json!("Alice"))]),
        make_input(&[("name", json!("Bob"))]),
        make_input(&[("name", json!("Charlie"))]),
    ];

    let results: Vec<Value> = inputs
        .into_iter()
        .map(|i| pick.invoke(i, None).unwrap())
        .collect();

    assert_eq!(
        results,
        vec![json!("Alice"), json!("Bob"), json!("Charlie")]
    );
}

#[tokio::test]
async fn test_pick_stream() {
    let pick = RunnablePick::new_single().key("value").call();

    let input = make_input(&[("value", json!(42)), ("other", json!("data"))]);
    let result: Vec<Value> = pick
        .stream(input, None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(result, vec![json!(42)]);
}

#[tokio::test]
async fn test_pick_transform() {
    let pick = RunnablePick::new_single().key("value").call();

    let chunks = vec![
        make_input(&[("value", json!(1))]),
        make_input(&[("value", json!(2))]),
    ];
    let input_stream = futures::stream::iter(chunks);

    let result: Vec<Value> = pick
        .transform(Box::pin(input_stream), None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(result, vec![json!(1), json!(2)]);
}

#[test]
fn test_pick_get_name() {
    let pick_single = RunnablePick::new_single().key("key").call();
    assert_eq!(pick_single.name(), Some("RunnablePick<key>".to_string()));

    let pick_multiple = RunnablePick::new_multi(vec!["key1", "key2", "key3"], None);
    assert_eq!(
        pick_multiple.name(),
        Some("RunnablePick<key1,key2,key3>".to_string())
    );
}

#[test]
fn test_pick_maintains_types() {
    let pick = RunnablePick::new_multi(vec!["int_val", "str_val", "list_val"], None);

    let input = make_input(&[
        ("int_val", json!(42)),
        ("str_val", json!("hello")),
        ("list_val", json!([1, 2, 3])),
        ("extra", json!("ignored")),
    ]);

    let result = pick.invoke(input, None).unwrap();
    let result_map: HashMap<String, Value> = serde_json::from_value(result).unwrap();

    assert_eq!(result_map["int_val"], json!(42));
    assert_eq!(result_map["str_val"], json!("hello"));
    assert_eq!(result_map["list_val"], json!([1, 2, 3]));
    assert!(!result_map.contains_key("extra"));
}

#[test]
fn test_pick_direct_instantiation() {
    let pick = RunnablePick::new_single().key("selected").call();

    let input = make_input(&[("selected", json!("yes")), ("others", json!("no"))]);
    let result = pick.invoke(input, None).unwrap();
    assert_eq!(result, json!("yes"));
}

#[test]
fn test_pick_empty_dict() {
    let pick = RunnablePick::new_multi(vec!["key1", "key2"], None);
    let result = pick.invoke(HashMap::new(), None);
    assert!(result.is_err());
}

#[test]
fn test_passthrough_assign_pick_combination() {
    let passthrough: RunnablePassthrough<HashMap<String, Value>> =
        RunnablePassthrough::builder().build();

    let mapper = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "doubled",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let val = x.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(val * 2))
                })
                .build(),
        )
        .add(
            "tripled",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let val = x.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(val * 3))
                })
                .build(),
        );
    let assign = RunnableAssign::builder().mapper(mapper).build();

    let pick = RunnablePick::new_multi(vec!["value", "doubled"], None);

    let input = make_input(&[("value", json!(5))]);
    let step1 = passthrough.invoke(input, None).unwrap();
    let step2 = assign.invoke(step1, None).unwrap();
    let result = pick.invoke(step2, None).unwrap();

    let result_map: HashMap<String, Value> = serde_json::from_value(result).unwrap();
    assert_eq!(result_map["value"], json!(5));
    assert_eq!(result_map["doubled"], json!(10));
    assert!(!result_map.contains_key("tripled"));
}

#[test]
fn test_assign_with_dependencies() {
    let mapper1 = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "step1",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let val = x.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(val + 1))
                })
                .build(),
        );
    let assign1 = RunnableAssign::builder().mapper(mapper1).build();

    let mapper2 = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "step2",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let val = x.get("step1").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(val * 2))
                })
                .build(),
        );
    let assign2 = RunnableAssign::builder().mapper(mapper2).build();

    let input = make_input(&[("value", json!(5))]);
    let intermediate = assign1.invoke(input, None).unwrap();
    let result = assign2.invoke(intermediate, None).unwrap();

    assert_eq!(result["value"], json!(5));
    assert_eq!(result["step1"], json!(6));
    assert_eq!(result["step2"], json!(12));
}

#[tokio::test]
async fn test_pick_transform_filters_each_chunk() {
    let pick = RunnablePick::new_single().key("wanted").call();

    let chunks = vec![
        make_input(&[("wanted", json!(1)), ("unwanted", json!(10))]),
        make_input(&[("wanted", json!(2)), ("unwanted", json!(20))]),
    ];
    let input_stream = futures::stream::iter(chunks);

    let result: Vec<Value> = pick
        .transform(Box::pin(input_stream), None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(result, vec![json!(1), json!(2)]);
}

#[test]
fn test_assign_builder() {
    let assign = RunnablePassthrough::<HashMap<String, Value>>::assign()
        .add(
            "new_key",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let val = x.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(val * 2))
                })
                .build(),
        )
        .build();

    let input = make_input(&[("value", json!(5))]);
    let result = assign.invoke(input, None).unwrap();
    assert_eq!(result["value"], json!(5));
    assert_eq!(result["new_key"], json!(10));
}

#[test]
fn test_assign_with_name() {
    let mapper = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "x",
            RunnableLambda::builder()
                .func(|_: HashMap<String, Value>| Ok(json!(1)))
                .build(),
        );
    let assign = RunnableAssign::builder()
        .mapper(mapper)
        .name("my_assign")
        .build();
    assert_eq!(assign.name(), Some("my_assign".to_string()));
}

#[test]
fn test_pick_with_name() {
    let pick = RunnablePick::new_single().key("key").name("my_pick").call();
    assert_eq!(pick.name(), Some("my_pick".to_string()));
}

#[test]
fn test_assign_mapper_accessor() {
    let mapper = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "a",
            RunnableLambda::builder()
                .func(|_: HashMap<String, Value>| Ok(json!(1)))
                .build(),
        )
        .add(
            "b",
            RunnableLambda::builder()
                .func(|_: HashMap<String, Value>| Ok(json!(2)))
                .build(),
        );
    let assign = RunnableAssign::builder().mapper(mapper).build();

    let _mapper_ref = assign.mapper();
}

#[test]
fn test_passthrough_default() {
    let passthrough: RunnablePassthrough<i32> = RunnablePassthrough::default();
    assert_eq!(passthrough.invoke(99, None).unwrap(), 99);
}

#[test]
fn test_passthrough_clone() {
    let passthrough: RunnablePassthrough<i32> = RunnablePassthrough::builder().build();
    let cloned = passthrough.clone();
    assert_eq!(cloned.invoke(42, None).unwrap(), 42);
}

#[test]
fn test_graph_passthrough() {
    use agent_chain_core::runnables::passthrough::graph_passthrough;
    let pt: RunnablePassthrough<String> = graph_passthrough();
    assert_eq!(pt.invoke("hello".into(), None).unwrap(), "hello");
}

#[test]
fn test_assign_input_output_schema() {
    let mapper = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "new_key",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| {
                    let val = x.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(json!(val * 2))
                })
                .build(),
        );
    let assign = RunnableAssign::builder().mapper(mapper).build();

    let input_schema = assign.get_input_schema(None);
    assert_eq!(input_schema["type"], "object");

    let output_schema = assign.get_output_schema(None);
    assert_eq!(output_schema["type"], "object");
    assert_eq!(output_schema["title"], "RunnableAssignOutput");
}

#[test]
fn test_assign_get_name() {
    let mapper = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "key1",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| Ok(json!(x)))
                .build(),
        )
        .add(
            "key2",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| Ok(json!(x)))
                .build(),
        );
    let assign = RunnableAssign::builder().mapper(mapper).build();

    let name = assign.name().unwrap();
    assert_eq!(name, "RunnableAssign");
}

#[test]
fn test_assign_graph_structure() {
    let mapper = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "new_key",
            RunnableLambda::builder()
                .func(|x: HashMap<String, Value>| Ok(json!(x.get("value"))))
                .build(),
        );
    let assign = RunnableAssign::builder().mapper(mapper).build();

    let _mapper = assign.mapper();
}

#[test]
fn test_passthrough_serialization() {
    let passthrough: RunnablePassthrough<i32> = RunnablePassthrough::builder().build();
    let debug = format!("{:?}", passthrough);
    assert!(debug.contains("RunnablePassthrough"));
}

#[test]
fn test_assign_serialization() {
    let mapper = RunnableParallel::<HashMap<String, Value>>::builder()
        .build()
        .add(
            "x",
            RunnableLambda::builder()
                .func(|_: HashMap<String, Value>| Ok(json!(1)))
                .build(),
        );
    let assign = RunnableAssign::builder().mapper(mapper).build();
    let debug = format!("{:?}", assign);
    assert!(debug.contains("RunnableAssign"));
}

#[test]
fn test_pick_serialization() {
    let pick = RunnablePick::new_single().key("key").call();
    let debug = format!("{:?}", pick);
    assert!(debug.contains("RunnablePick"));
}

#[test]
fn test_passthrough_schema_identity() {
    let passthrough: RunnablePassthrough<i32> = RunnablePassthrough::builder().build();
    let input_schema = passthrough.get_input_schema(None);
    let output_schema = passthrough.get_output_schema(None);
    assert_eq!(input_schema["type"], "object");
    assert_eq!(output_schema["type"], "object");
}

#[test]
fn test_pick_output_schema() {
    let pick = RunnablePick::new_multi(vec!["name", "age"], None);
    let output_schema = pick.get_output_schema(None);
    assert_eq!(output_schema["type"], "object");
    let properties = output_schema["properties"].as_object().unwrap();
    assert!(properties.contains_key("name"));
    assert!(properties.contains_key("age"));
}
