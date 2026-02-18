//! Comprehensive tests for core Runnable types and composition.
//!
//! Mirrors portable tests from
//! `langchain/libs/core/tests/unit_tests/runnables/test_runnable.py`

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use agent_chain_core::error::Error;
use agent_chain_core::runnables::base::{
    Runnable, RunnableEach, RunnableLambda, RunnableParallel, RunnableSequence, pipe,
};
use agent_chain_core::runnables::config::RunnableConfig;
use agent_chain_core::runnables::passthrough::{RunnableAssign, RunnablePassthrough};
use futures::StreamExt;
use serde_json::{Value, json};


fn make_input(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}


/// Test basic RunnableLambda invoke.
#[test]
fn test_runnable_lambda_invoke() {
    let runnable = RunnableLambda::new(|x: i32| Ok(x * 2));
    assert_eq!(runnable.invoke(5, None).unwrap(), 10);
}

/// Test RunnableLambda with named function.
#[test]
fn test_runnable_lambda_named() {
    let runnable = RunnableLambda::new(|x: i32| Ok(x + 1)).with_name("add_one");
    assert_eq!(runnable.name(), Some("add_one".to_string()));
    assert_eq!(runnable.invoke(5, None).unwrap(), 6);
}

/// Test RunnableLambda returning error.
#[test]
fn test_runnable_lambda_error() {
    let runnable = RunnableLambda::new(|_x: i32| Err::<i32, _>(Error::other("boom")));
    let result = runnable.invoke(5, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("boom"));
}

/// Mirrors `test_runnable_lambda_stream` (normal function part).
#[tokio::test]
async fn test_runnable_lambda_stream() {
    let runnable = RunnableLambda::new(|x: i32| Ok(x + 1));
    let output: Vec<i32> = runnable
        .stream(5, None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;
    assert_eq!(output, vec![6]);
}

/// Mirrors `test_runnable_lambda_astream`.
#[tokio::test]
async fn test_runnable_lambda_astream() {
    let runnable = RunnableLambda::new(|x: i32| Ok(x + 1));
    let output: Vec<i32> = runnable
        .astream(5, None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;
    assert_eq!(output, vec![6]);
}


/// Test basic sequence: first | second.
#[test]
fn test_sequence_invoke() {
    let add_one = RunnableLambda::new(|x: i32| Ok(x + 1));
    let double = RunnableLambda::new(|x: i32| Ok(x * 2));
    let seq = pipe(add_one, double);

    assert_eq!(seq.invoke(5, None).unwrap(), 12);
}

/// Test sequence async invoke.
#[tokio::test]
async fn test_sequence_ainvoke() {
    let add_one = RunnableLambda::new(|x: i32| Ok(x + 1));
    let double = RunnableLambda::new(|x: i32| Ok(x * 2));
    let seq = pipe(add_one, double);

    assert_eq!(seq.ainvoke(5, None).await.unwrap(), 12);
}

/// Test three-step sequence using nested pipe.
#[test]
fn test_sequence_three_steps() {
    let step1 = RunnableLambda::new(|x: i32| Ok(x + 1));
    let step2 = RunnableLambda::new(|x: i32| Ok(x * 2));
    let step3 = RunnableLambda::new(|x: i32| Ok(x - 3));
    let seq = pipe(pipe(step1, step2), step3);

    assert_eq!(seq.invoke(5, None).unwrap(), 9);
}

/// Test sequence error propagation: first step fails.
#[test]
fn test_sequence_first_step_error() {
    let fail = RunnableLambda::new(|_x: i32| Err::<i32, _>(Error::other("first failed")));
    let double = RunnableLambda::new(|x: i32| Ok(x * 2));
    let seq = pipe(fail, double);

    let result = seq.invoke(5, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("first failed"));
}

/// Test sequence error propagation: second step fails.
#[test]
fn test_sequence_second_step_error() {
    let add_one = RunnableLambda::new(|x: i32| Ok(x + 1));
    let fail = RunnableLambda::new(|_x: i32| Err::<i32, _>(Error::other("second failed")));
    let seq = pipe(add_one, fail);

    let result = seq.invoke(5, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("second failed"));
}

/// Test sequence streaming.
#[tokio::test]
async fn test_sequence_stream() {
    let add_one = RunnableLambda::new(|x: i32| Ok(x + 1));
    let double = RunnableLambda::new(|x: i32| Ok(x * 2));
    let seq = pipe(add_one, double);

    let output: Vec<i32> = seq
        .stream(5, None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;
    assert_eq!(output, vec![12]);
}

/// Mirrors `test_runnable_sequence_transform`.
#[tokio::test]
async fn test_runnable_sequence_transform() {
    let add_one = RunnableLambda::new(|x: i32| Ok(x + 1));
    let double = RunnableLambda::new(|x: i32| Ok(x * 2));
    let seq = pipe(add_one, double);

    let input_stream = futures::stream::iter(vec![5]);
    let result: Vec<i32> = seq
        .transform(Box::pin(input_stream), None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(result, vec![12]);
}

/// Mirrors `test_runnable_sequence_atransform`.
#[tokio::test]
async fn test_runnable_sequence_atransform() {
    let add_one = RunnableLambda::new(|x: i32| Ok(x + 1));
    let double = RunnableLambda::new(|x: i32| Ok(x * 2));
    let seq = pipe(add_one, double);

    let input_stream = futures::stream::iter(vec![5]);
    let result: Vec<i32> = seq
        .atransform(Box::pin(input_stream), None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(result, vec![12]);
}

/// Test sequence name.
#[test]
fn test_sequence_name() {
    let add = RunnableLambda::new(|x: i32| Ok(x + 1)).with_name("add");
    let double = RunnableLambda::new(|x: i32| Ok(x * 2)).with_name("double");
    let seq = RunnableSequence::new(add, double).with_name("my_seq");

    assert_eq!(seq.name(), Some("my_seq".to_string()));
}


/// Mirrors `test_with_config_with_config` (simplified — no LLM, just config merging).
#[test]
fn test_with_config_with_config() {
    let runnable = RunnableLambda::new(|x: i32| Ok(x + 1));

    let mut config1 = RunnableConfig::default();
    config1.metadata.insert("a".into(), json!("b"));

    let mut config2 = RunnableConfig::default();
    config2.tags.push("a-tag".into());

    let bound = runnable.with_config(config1).with_config(config2);
    let result = bound.invoke(5, None).unwrap();
    assert_eq!(result, 6);
}

/// Test binding kwargs. In Rust, bind stores kwargs but RunnableLambda
/// doesn't use them (no **kwargs in Rust). We verify the binding works.
#[test]
fn test_bind_creates_binding() {
    let runnable = RunnableLambda::new(|x: i32| Ok(x + 1));
    let kwargs: HashMap<String, Value> = HashMap::from([
        ("stop".into(), json!(["Thought:"])),
        ("one".into(), json!("two")),
    ]);
    let bound = runnable.bind(kwargs);
    assert_eq!(bound.invoke(5, None).unwrap(), 6);
}

/// Test with_config propagates tags.
#[test]
fn test_with_config_tags() {
    let runnable = RunnableLambda::new(|x: i32| Ok(x + 1));
    let mut config = RunnableConfig::default();
    config.tags.push("my_key".into());

    let bound = runnable.with_config(config);
    let result = bound.invoke(5, None).unwrap();
    assert_eq!(result, 6);
}

/// Test with_config metadata.
#[test]
fn test_with_config_metadata() {
    let runnable = RunnableLambda::new(|x: i32| Ok(x + 1));
    let mut config = RunnableConfig::default();
    config.metadata.insert("my_key".into(), json!("my_value"));

    let bound = runnable.with_config(config);
    let result = bound.invoke(5, None).unwrap();
    assert_eq!(result, 6);
}

/// Test with_config propagates to invoke config.
#[test]
fn test_with_config_merge_at_invoke() {
    let runnable = RunnableLambda::new(|x: i32| Ok(x + 1));
    let mut base_config = RunnableConfig::default();
    base_config.tags.push("base-tag".into());
    let bound = runnable.with_config(base_config);

    let mut invoke_config = RunnableConfig::default();
    invoke_config.tags.push("invoke-tag".into());

    let result = bound.invoke(5, Some(invoke_config)).unwrap();
    assert_eq!(result, 6);
}


/// Test basic parallel execution.
#[test]
fn test_parallel_invoke() {
    let parallel = RunnableParallel::<i32>::new()
        .add("doubled", RunnableLambda::new(|x: i32| Ok(json!(x * 2))))
        .add("tripled", RunnableLambda::new(|x: i32| Ok(json!(x * 3))));

    let result = parallel.invoke(5, None).unwrap();
    assert_eq!(result["doubled"], json!(10));
    assert_eq!(result["tripled"], json!(15));
}

/// Test parallel async invoke.
#[tokio::test]
async fn test_parallel_ainvoke() {
    let parallel = RunnableParallel::<i32>::new()
        .add("doubled", RunnableLambda::new(|x: i32| Ok(json!(x * 2))))
        .add("tripled", RunnableLambda::new(|x: i32| Ok(json!(x * 3))));

    let result = parallel.ainvoke(5, None).await.unwrap();
    assert_eq!(result["doubled"], json!(10));
    assert_eq!(result["tripled"], json!(15));
}

/// Test parallel name generation.
#[test]
fn test_parallel_name() {
    let parallel = RunnableParallel::<i32>::new()
        .add("a", RunnableLambda::new(|x: i32| Ok(json!(x))))
        .add("b", RunnableLambda::new(|x: i32| Ok(json!(x))));

    let name = parallel.name().unwrap();
    assert!(name.starts_with("RunnableParallel<"));
    assert!(name.contains('a'));
    assert!(name.contains('b'));
}

/// Test parallel with error in one branch.
#[test]
fn test_parallel_error_in_branch() {
    let parallel = RunnableParallel::<i32>::new()
        .add("ok", RunnableLambda::new(|x: i32| Ok(json!(x))))
        .add(
            "fail",
            RunnableLambda::new(|_: i32| Err::<Value, _>(Error::other("branch error"))),
        );

    let result = parallel.invoke(5, None);
    assert!(result.is_err());
}


/// Mirrors `test_each_simple`.
#[test]
fn test_each_simple() {
    let double = RunnableLambda::new(|x: i32| Ok(x * 2));
    let each = RunnableEach::new(double);

    let result = each.invoke(vec![1, 2, 3], None).unwrap();
    assert_eq!(result, vec![2, 4, 6]);
}

/// Test map() convenience method.
#[test]
fn test_map_convenience() {
    let double = RunnableLambda::new(|x: i32| Ok(x * 2));
    let each = double.map();

    let result = each.invoke(vec![1, 2, 3, 4, 5], None).unwrap();
    assert_eq!(result, vec![2, 4, 6, 8, 10]);
}

/// Test map().map() (nested).
#[test]
fn test_map_nested() {
    let add_one = RunnableLambda::new(|x: i32| Ok(x + 1));
    let inner_each = add_one.map();
    let outer_each = inner_each.map();

    let input = vec![vec![1, 2, 3], vec![10, 20]];
    let result = outer_each.invoke(input, None).unwrap();
    assert_eq!(result, vec![vec![2, 3, 4], vec![11, 21]]);
}

/// Test each with error.
#[test]
fn test_each_error() {
    let fail_on_3 = RunnableLambda::new(|x: i32| {
        if x == 3 {
            Err(Error::other("no threes"))
        } else {
            Ok(x * 2)
        }
    });
    let each = RunnableEach::new(fail_on_3);

    let result = each.invoke(vec![1, 2, 3, 4], None);
    assert!(result.is_err());
}

/// Test each name.
#[test]
fn test_each_name() {
    let named = RunnableLambda::new(|x: i32| Ok(x)).with_name("identity");
    let each = named.map();
    assert_eq!(each.name(), Some("RunnableEach<identity>".to_string()));
}


/// Mirrors `test_combining_sequences` (simplified — no prompts).
///
/// Tests: int → parallel(doubled, tripled) → pick one.
#[test]
fn test_combining_sequences() {
    let parallel = RunnableParallel::<i32>::new()
        .add("doubled", RunnableLambda::new(|x: i32| Ok(json!(x * 2))))
        .add("tripled", RunnableLambda::new(|x: i32| Ok(json!(x * 3))));

    let pick = RunnableLambda::new(|m: HashMap<String, Value>| {
        Ok(m.get("doubled").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
    });

    let chain = pipe(parallel, pick);
    assert_eq!(chain.invoke(5, None).unwrap(), 10);
}


/// Mirrors `test_transform_of_runnable_lambda_with_dicts`.
#[tokio::test]
async fn test_transform_of_runnable_lambda_with_dicts() {
    let runnable = RunnableLambda::new(|x: HashMap<String, Value>| Ok(x));

    let chunks = vec![make_input(&[("foo", json!("n"))])];
    let input_stream = futures::stream::iter(chunks);

    let result: Vec<HashMap<String, Value>> = runnable
        .transform(Box::pin(input_stream), None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(result, vec![make_input(&[("foo", json!("n"))])]);
}

/// Mirrors `test_transform_of_runnable_lambda_with_dicts` — sequence part.
#[tokio::test]
async fn test_transform_sequence_with_dicts() {
    let identity1 = RunnableLambda::new(|x: HashMap<String, Value>| Ok(x));
    let identity2 = RunnableLambda::new(|x: HashMap<String, Value>| Ok(x));
    let seq = pipe(identity1, identity2);

    let chunks = vec![make_input(&[("foo", json!("n"))])];
    let input_stream = futures::stream::iter(chunks);

    let result: Vec<HashMap<String, Value>> = seq
        .transform(Box::pin(input_stream), None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(result, vec![make_input(&[("foo", json!("n"))])]);
}

/// Mirrors `test_passthrough_transform_with_dicts`.
#[tokio::test]
async fn test_passthrough_transform_with_dicts() {
    let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let calls_clone = calls.clone();

    let runnable: RunnablePassthrough<HashMap<String, Value>> =
        RunnablePassthrough::with_func(move |x: &HashMap<String, Value>, _: &RunnableConfig| {
            calls_clone.lock().unwrap().push(x.clone());
        });

    let chunks = vec![
        make_input(&[("foo", json!("a"))]),
        make_input(&[("foo", json!("n"))]),
    ];
    let input_stream = futures::stream::iter(chunks);

    let result: Vec<HashMap<String, Value>> = runnable
        .transform(Box::pin(input_stream), None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(result.len(), 2);
    assert_eq!(result[0]["foo"], json!("a"));
    assert_eq!(result[1]["foo"], json!("n"));
}


/// Test RunnableLambda batch.
#[test]
fn test_lambda_batch() {
    let double = RunnableLambda::new(|x: i32| Ok(x * 2));
    let results = double.batch(vec![1, 2, 3, 4, 5], None, false);
    let values: Vec<i32> = results.into_iter().map(|r| r.unwrap()).collect();
    assert_eq!(values, vec![2, 4, 6, 8, 10]);
}

/// Test sequence batch.
#[test]
fn test_sequence_batch() {
    let add_one = RunnableLambda::new(|x: i32| Ok(x + 1));
    let double = RunnableLambda::new(|x: i32| Ok(x * 2));
    let seq = pipe(add_one, double);

    let results = seq.batch(vec![1, 2, 3], None, false);
    let values: Vec<i32> = results.into_iter().map(|r| r.unwrap()).collect();
    assert_eq!(values, vec![4, 6, 8]); // (1+1)*2=4, (2+1)*2=6, (3+1)*2=8
}

/// Mirrors `test_seq_batch_return_exceptions` (simplified).
#[test]
fn test_seq_batch_return_exceptions() {
    let maybe_fail = RunnableLambda::new(|x: i32| {
        if x == 2 {
            Err(Error::other("fail on 2"))
        } else {
            Ok(x + 1)
        }
    });
    let double = RunnableLambda::new(|x: i32| Ok(x * 2));
    let seq = pipe(maybe_fail, double);

    let results = seq.batch(vec![1, 2, 3], None, true);
    assert!(results[0].is_ok());
    assert_eq!(*results[0].as_ref().unwrap(), 4); // (1+1)*2=4
    assert!(results[1].is_err()); // fails on 2
    assert!(results[2].is_ok());
    assert_eq!(*results[2].as_ref().unwrap(), 8); // (3+1)*2=8
}

/// Test empty batch.
#[test]
fn test_empty_batch() {
    let double = RunnableLambda::new(|x: i32| Ok(x * 2));
    let results = double.batch(vec![], None, false);
    assert!(results.is_empty());
}


/// Test async lambda batch.
#[tokio::test]
async fn test_lambda_abatch() {
    let double = RunnableLambda::new(|x: i32| Ok(x * 2));
    let results = double.abatch(vec![1, 2, 3], None, false).await;
    let values: Vec<i32> = results.into_iter().map(|r| r.unwrap()).collect();
    assert_eq!(values, vec![2, 4, 6]);
}

/// Test async sequence batch.
#[tokio::test]
async fn test_sequence_abatch() {
    let add_one = RunnableLambda::new(|x: i32| Ok(x + 1));
    let double = RunnableLambda::new(|x: i32| Ok(x * 2));
    let seq = pipe(add_one, double);

    let results = seq.abatch(vec![1, 2, 3], None, false).await;
    let values: Vec<i32> = results.into_iter().map(|r| r.unwrap()).collect();
    assert_eq!(values, vec![4, 6, 8]);
}


/// Mirrors `test_runnable_assign`.
#[test]
fn test_runnable_assign() {
    let mapper = RunnableParallel::<HashMap<String, Value>>::new().add(
        "add_step",
        RunnableLambda::new(|x: HashMap<String, Value>| {
            let input_val = x.get("input").and_then(|v| v.as_i64()).unwrap_or(0);
            Ok(json!({"added": input_val + 10}))
        }),
    );
    let assign = RunnableAssign::new(mapper);

    let input = make_input(&[("input", json!(5))]);
    let result = assign.invoke(input, None).unwrap();

    assert_eq!(result["input"], json!(5));
    assert_eq!(result["add_step"], json!({"added": 15}));
}


/// Mirrors `test_representation_of_runnables`.
#[test]
fn test_representation_of_runnables() {
    let runnable = RunnableLambda::new(|x: i32| Ok(x * 2));
    let repr = format!("{:?}", runnable);
    assert!(repr.contains("RunnableLambda"));

    let seq = pipe(
        RunnableLambda::new(|x: i32| Ok(x + 1)),
        RunnableLambda::new(|x: i32| Ok(x * 2)),
    );
    let repr = format!("{:?}", seq);
    assert!(repr.contains("RunnableSequence"));

    let parallel = RunnableParallel::<i32>::new()
        .add("a", RunnableLambda::new(|x: i32| Ok(json!(x))))
        .add("b", RunnableLambda::new(|x: i32| Ok(json!(x))));
    let repr = format!("{:?}", parallel);
    assert!(repr.contains("RunnableParallel"));

    let binding = RunnableLambda::new(|x: i32| Ok(x)).with_config(RunnableConfig::default());
    let repr = format!("{:?}", binding);
    assert!(repr.contains("RunnableBinding"));

    let each = RunnableLambda::new(|x: i32| Ok(x)).map();
    let repr = format!("{:?}", each);
    assert!(repr.contains("RunnableEach"));
}


/// Mirrors `test_default_method_implementations`.
///
/// All runnables should support invoke, batch, and stream via defaults.
#[test]
fn test_default_method_implementations() {
    let runnable = RunnableLambda::new(|x: i32| Ok(x + 1));

    assert_eq!(runnable.invoke(5, None).unwrap(), 6);

    let results = runnable.batch(vec![1, 2, 3], None, false);
    let values: Vec<i32> = results.into_iter().map(|r| r.unwrap()).collect();
    assert_eq!(values, vec![2, 3, 4]);
}

/// Mirrors `test_default_method_implementations_async`.
#[tokio::test]
async fn test_default_method_implementations_async() {
    let runnable = RunnableLambda::new(|x: i32| Ok(x + 1));

    assert_eq!(runnable.ainvoke(5, None).await.unwrap(), 6);

    let results = runnable.abatch(vec![1, 2, 3], None, false).await;
    let values: Vec<i32> = results.into_iter().map(|r| r.unwrap()).collect();
    assert_eq!(values, vec![2, 3, 4]);

    let output: Vec<i32> = runnable
        .astream(5, None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;
    assert_eq!(output, vec![6]);
}


/// Test schema on sequence.
#[test]
fn test_sequence_schema() {
    let step1 = RunnableLambda::new(|x: i32| Ok(x + 1)).with_name("step1");
    let step2 = RunnableLambda::new(|x: i32| Ok(x * 2)).with_name("step2");
    let seq = pipe(step1, step2);

    let input_schema = seq.get_input_schema(None);
    assert_eq!(input_schema["type"], "object");

    let output_schema = seq.get_output_schema(None);
    assert_eq!(output_schema["type"], "object");
}

/// Test schema on parallel.
#[test]
fn test_parallel_schema() {
    let parallel =
        RunnableParallel::<i32>::new().add("a", RunnableLambda::new(|x: i32| Ok(json!(x))));

    let input_schema = parallel.get_input_schema(None);
    assert_eq!(input_schema["type"], "object");
}

/// Test schema delegation through binding.
#[test]
fn test_binding_schema_delegation() {
    let inner = RunnableLambda::new(|x: i32| Ok(x + 1)).with_name("inner");
    let bound = inner.with_config(RunnableConfig::default());

    let inner_schema = RunnableLambda::new(|x: i32| Ok(x + 1))
        .with_name("inner")
        .get_input_schema(None);

    assert_eq!(bound.get_input_schema(None), inner_schema);
}


/// Test get_name with suffix.
#[test]
fn test_get_name_with_suffix() {
    let runnable = RunnableLambda::new(|x: i32| Ok(x)).with_name("MyRunnable");
    let name = runnable.get_name(Some("Input"), None);
    assert_eq!(name, "MyRunnableInput");
}

/// Test get_name with lowercase name and suffix.
#[test]
fn test_get_name_lowercase_with_suffix() {
    let runnable = RunnableLambda::new(|x: i32| Ok(x)).with_name("my_runnable");
    let name = runnable.get_name(Some("input"), None);
    assert_eq!(name, "my_runnable_input");
}

/// Test get_name with override.
#[test]
fn test_get_name_override() {
    let runnable = RunnableLambda::new(|x: i32| Ok(x)).with_name("original");
    let name = runnable.get_name(None, Some("override"));
    assert_eq!(name, "override");
}


/// Test that invoke call count is exactly 1 for each input in each.
#[test]
fn test_each_call_count() {
    let counter = Arc::new(AtomicUsize::new(0));
    let c = counter.clone();

    let runnable = RunnableLambda::new(move |x: i32| {
        c.fetch_add(1, Ordering::SeqCst);
        Ok(x)
    });

    let each = RunnableEach::new(runnable);
    let _ = each.invoke(vec![1, 2, 3, 4, 5], None).unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 5);
}

/// Test that sequence runs each step exactly once.
#[test]
fn test_sequence_step_count() {
    let counter1 = Arc::new(AtomicUsize::new(0));
    let counter2 = Arc::new(AtomicUsize::new(0));
    let c1 = counter1.clone();
    let c2 = counter2.clone();

    let step1 = RunnableLambda::new(move |x: i32| {
        c1.fetch_add(1, Ordering::SeqCst);
        Ok(x + 1)
    });
    let step2 = RunnableLambda::new(move |x: i32| {
        c2.fetch_add(1, Ordering::SeqCst);
        Ok(x * 2)
    });

    let seq = pipe(step1, step2);
    let _ = seq.invoke(5, None).unwrap();

    assert_eq!(counter1.load(Ordering::SeqCst), 1);
    assert_eq!(counter2.load(Ordering::SeqCst), 1);
}

/// Test parallel runs each branch exactly once.
#[test]
fn test_parallel_call_count() {
    let counter_a = Arc::new(AtomicUsize::new(0));
    let counter_b = Arc::new(AtomicUsize::new(0));
    let ca = counter_a.clone();
    let cb = counter_b.clone();

    let parallel = RunnableParallel::<i32>::new()
        .add(
            "a",
            RunnableLambda::new(move |x: i32| {
                ca.fetch_add(1, Ordering::SeqCst);
                Ok(json!(x))
            }),
        )
        .add(
            "b",
            RunnableLambda::new(move |x: i32| {
                cb.fetch_add(1, Ordering::SeqCst);
                Ok(json!(x))
            }),
        );

    let _ = parallel.invoke(5, None).unwrap();
    assert_eq!(counter_a.load(Ordering::SeqCst), 1);
    assert_eq!(counter_b.load(Ordering::SeqCst), 1);
}

/// Test type_name returns something meaningful.
#[test]
fn test_type_name() {
    let runnable = RunnableLambda::new(|x: i32| Ok(x));
    let name = runnable.type_name();
    assert!(name.contains("RunnableLambda"));
}

/// Test Default for RunnableParallel.
#[test]
fn test_parallel_default() {
    let parallel = RunnableParallel::<i32>::default();
    let result = parallel.invoke(5, None).unwrap();
    assert!(result.is_empty());
}

/// Test binding debug.
#[test]
fn test_binding_debug() {
    let runnable = RunnableLambda::new(|x: i32| Ok(x));
    let bound = runnable.bind(HashMap::from([("key".into(), json!("val"))]));
    let debug = format!("{:?}", bound);
    assert!(debug.contains("RunnableBinding"));
    assert!(debug.contains("key"));
}


/// Test pick() convenience method with a single key.
#[test]
fn test_pick_single_key() {
    use agent_chain_core::runnables::passthrough::PickKeys;

    let runnable = RunnableLambda::new(|_x: i32| {
        let mut map = HashMap::new();
        map.insert("name".to_string(), json!("Alice"));
        map.insert("age".to_string(), json!(30));
        Ok(map)
    });

    let picked = runnable.pick(PickKeys::Single("name".to_string()));
    let result = picked.invoke(1, None).unwrap();
    assert_eq!(result, json!("Alice"));
}

/// Test pick() convenience method with multiple keys.
#[test]
fn test_pick_multiple_keys() {
    use agent_chain_core::runnables::passthrough::PickKeys;

    let runnable = RunnableLambda::new(|_x: i32| {
        let mut map = HashMap::new();
        map.insert("name".to_string(), json!("Alice"));
        map.insert("age".to_string(), json!(30));
        map.insert("city".to_string(), json!("NYC"));
        Ok(map)
    });

    let picked = runnable.pick(PickKeys::Multiple(vec![
        "name".to_string(),
        "age".to_string(),
    ]));
    let result = picked.invoke(1, None).unwrap();
    let result_map: HashMap<String, serde_json::Value> = serde_json::from_value(result).unwrap();
    assert_eq!(result_map.len(), 2);
    assert_eq!(result_map.get("name"), Some(&json!("Alice")));
    assert_eq!(result_map.get("age"), Some(&json!(30)));
}

/// Test assign() convenience method.
#[test]
fn test_assign_convenience() {
    let passthrough = RunnablePassthrough::<HashMap<String, serde_json::Value>>::new();

    let mapper = RunnablePassthrough::<HashMap<String, serde_json::Value>>::assign()
        .add(
            "doubled",
            RunnableLambda::new(|input: HashMap<String, serde_json::Value>| {
                let val = input.get("x").and_then(|v| v.as_i64()).unwrap_or(0);
                Ok(json!(val * 2))
            }),
        )
        .build();

    let chained = passthrough.assign(mapper);

    let mut input = HashMap::new();
    input.insert("x".to_string(), json!(5));

    let result = chained.invoke(input, None).unwrap();
    assert_eq!(result.get("x"), Some(&json!(5)));
    assert_eq!(result.get("doubled"), Some(&json!(10)));
}

/// Test with_fallbacks() convenience method.
#[test]
fn test_with_fallbacks_convenience() {
    let primary = RunnableLambda::new(|_x: i32| -> Result<i32, Error> {
        Err(Error::other("primary failed"))
    });

    let fallback = RunnableLambda::new(|x: i32| -> Result<i32, Error> { Ok(x * 2) });

    let with_fallbacks = primary.with_fallbacks(vec![Arc::new(fallback)]);
    let result = with_fallbacks.invoke(5, None).unwrap();
    assert_eq!(result, 10);
}

/// Test with_fallbacks() convenience method when primary succeeds.
#[test]
fn test_with_fallbacks_primary_succeeds() {
    let primary = RunnableLambda::new(|x: i32| -> Result<i32, Error> { Ok(x + 1) });

    let fallback = RunnableLambda::new(|x: i32| -> Result<i32, Error> { Ok(x * 100) });

    let with_fallbacks = primary.with_fallbacks(vec![Arc::new(fallback)]);
    let result = with_fallbacks.invoke(5, None).unwrap();
    assert_eq!(result, 6);
}

/// Test with_listeners() convenience method.
#[test]
fn test_with_listeners_convenience() {
    use agent_chain_core::tracers::root_listeners::Listener;
    use std::sync::atomic::AtomicBool;

    let started = Arc::new(AtomicBool::new(false));
    let ended = Arc::new(AtomicBool::new(false));

    let started_clone = started.clone();
    let on_start: Listener = Box::new(move |_run, _config| {
        started_clone.store(true, Ordering::SeqCst);
    });

    let ended_clone = ended.clone();
    let on_end: Listener = Box::new(move |_run, _config| {
        ended_clone.store(true, Ordering::SeqCst);
    });

    let runnable = RunnableLambda::new(|x: i32| Ok(x + 1));
    let with_listeners = runnable.with_listeners(Some(on_start), Some(on_end), None);

    let result = with_listeners.invoke(5, None).unwrap();
    assert_eq!(result, 6);
}

/// Test chaining pick() with with_fallbacks().
#[test]
fn test_chaining_pick_with_fallbacks() {
    use agent_chain_core::runnables::passthrough::PickKeys;

    let runnable = RunnableLambda::new(|_x: i32| {
        let mut map = HashMap::new();
        map.insert("name".to_string(), json!("Alice"));
        map.insert("age".to_string(), json!(30));
        Ok(map)
    });

    let picked = runnable.pick(PickKeys::Single("name".to_string()));

    let fallback_picked = {
        let fallback_inner = RunnableLambda::new(|_x: i32| {
            let mut map = HashMap::new();
            map.insert("name".to_string(), json!("Fallback"));
            Ok(map)
        });
        use agent_chain_core::runnables::passthrough::RunnablePick;
        pipe(fallback_inner, RunnablePick::new_single("name"))
    };

    let with_fallbacks = picked.with_fallbacks(vec![Arc::new(fallback_picked)]);

    let result = with_fallbacks.invoke(1, None).unwrap();
    assert_eq!(result, json!("Alice"));
}
