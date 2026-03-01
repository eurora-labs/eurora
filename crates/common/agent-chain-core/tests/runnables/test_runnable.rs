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

#[test]
fn test_runnable_lambda_invoke() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x * 2)).build();
    assert_eq!(runnable.invoke(5, None).unwrap(), 10);
}

#[test]
fn test_runnable_lambda_named() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).name("add_one").build();
    assert_eq!(runnable.name(), Some("add_one".to_string()));
    assert_eq!(runnable.invoke(5, None).unwrap(), 6);
}

#[test]
fn test_runnable_lambda_error() {
    let runnable = RunnableLambda::builder().func(|_x: i32| Err::<i32, _>(Error::other("boom"))).build();
    let result = runnable.invoke(5, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("boom"));
}

#[tokio::test]
async fn test_runnable_lambda_stream() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();
    let output: Vec<i32> = runnable
        .stream(5, None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;
    assert_eq!(output, vec![6]);
}

#[tokio::test]
async fn test_runnable_lambda_astream() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();
    let output: Vec<i32> = runnable
        .astream(5, None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;
    assert_eq!(output, vec![6]);
}

#[test]
fn test_sequence_invoke() {
    let add_one = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();
    let double = RunnableLambda::builder().func(|x: i32| Ok(x * 2)).build();
    let seq = pipe(add_one, double);

    assert_eq!(seq.invoke(5, None).unwrap(), 12);
}

#[tokio::test]
async fn test_sequence_ainvoke() {
    let add_one = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();
    let double = RunnableLambda::builder().func(|x: i32| Ok(x * 2)).build();
    let seq = pipe(add_one, double);

    assert_eq!(seq.ainvoke(5, None).await.unwrap(), 12);
}

#[test]
fn test_sequence_three_steps() {
    let step1 = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();
    let step2 = RunnableLambda::builder().func(|x: i32| Ok(x * 2)).build();
    let step3 = RunnableLambda::builder().func(|x: i32| Ok(x - 3)).build();
    let seq = pipe(pipe(step1, step2), step3);

    assert_eq!(seq.invoke(5, None).unwrap(), 9);
}

#[test]
fn test_sequence_first_step_error() {
    let fail = RunnableLambda::builder().func(|_x: i32| Err::<i32, _>(Error::other("first failed"))).build();
    let double = RunnableLambda::builder().func(|x: i32| Ok(x * 2)).build();
    let seq = pipe(fail, double);

    let result = seq.invoke(5, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("first failed"));
}

#[test]
fn test_sequence_second_step_error() {
    let add_one = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();
    let fail = RunnableLambda::builder().func(|_x: i32| Err::<i32, _>(Error::other("second failed"))).build();
    let seq = pipe(add_one, fail);

    let result = seq.invoke(5, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("second failed"));
}

#[tokio::test]
async fn test_sequence_stream() {
    let add_one = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();
    let double = RunnableLambda::builder().func(|x: i32| Ok(x * 2)).build();
    let seq = pipe(add_one, double);

    let output: Vec<i32> = seq
        .stream(5, None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;
    assert_eq!(output, vec![12]);
}

#[tokio::test]
async fn test_runnable_sequence_transform() {
    let add_one = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();
    let double = RunnableLambda::builder().func(|x: i32| Ok(x * 2)).build();
    let seq = pipe(add_one, double);

    let input_stream = futures::stream::iter(vec![5]);
    let result: Vec<i32> = seq
        .transform(Box::pin(input_stream), None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(result, vec![12]);
}

#[tokio::test]
async fn test_runnable_sequence_atransform() {
    let add_one = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();
    let double = RunnableLambda::builder().func(|x: i32| Ok(x * 2)).build();
    let seq = pipe(add_one, double);

    let input_stream = futures::stream::iter(vec![5]);
    let result: Vec<i32> = seq
        .atransform(Box::pin(input_stream), None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(result, vec![12]);
}

#[test]
fn test_sequence_name() {
    let add = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).name("add").build();
    let double = RunnableLambda::builder().func(|x: i32| Ok(x * 2)).name("double").build();
    let seq = RunnableSequence::builder().first(add).last(double).name("my_seq").build();

    assert_eq!(seq.name(), Some("my_seq".to_string()));
}

#[test]
fn test_with_config_with_config() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();

    let mut config1 = RunnableConfig::default();
    config1.metadata.insert("a".into(), json!("b"));

    let mut config2 = RunnableConfig::default();
    config2.tags.push("a-tag".into());

    let bound = runnable.with_config(config1).with_config(config2);
    let result = bound.invoke(5, None).unwrap();
    assert_eq!(result, 6);
}

#[test]
fn test_bind_creates_binding() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();
    let kwargs: HashMap<String, Value> = HashMap::from([
        ("stop".into(), json!(["Thought:"])),
        ("one".into(), json!("two")),
    ]);
    let bound = runnable.bind(kwargs);
    assert_eq!(bound.invoke(5, None).unwrap(), 6);
}

#[test]
fn test_with_config_tags() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();
    let mut config = RunnableConfig::default();
    config.tags.push("my_key".into());

    let bound = runnable.with_config(config);
    let result = bound.invoke(5, None).unwrap();
    assert_eq!(result, 6);
}

#[test]
fn test_with_config_metadata() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();
    let mut config = RunnableConfig::default();
    config.metadata.insert("my_key".into(), json!("my_value"));

    let bound = runnable.with_config(config);
    let result = bound.invoke(5, None).unwrap();
    assert_eq!(result, 6);
}

#[test]
fn test_with_config_merge_at_invoke() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();
    let mut base_config = RunnableConfig::default();
    base_config.tags.push("base-tag".into());
    let bound = runnable.with_config(base_config);

    let mut invoke_config = RunnableConfig::default();
    invoke_config.tags.push("invoke-tag".into());

    let result = bound.invoke(5, Some(invoke_config)).unwrap();
    assert_eq!(result, 6);
}

#[test]
fn test_parallel_invoke() {
    let parallel = RunnableParallel::<i32>::builder().build()
        .add("doubled", RunnableLambda::builder().func(|x: i32| Ok(json!(x * 2))).build())
        .add("tripled", RunnableLambda::builder().func(|x: i32| Ok(json!(x * 3))).build());

    let result = parallel.invoke(5, None).unwrap();
    assert_eq!(result["doubled"], json!(10));
    assert_eq!(result["tripled"], json!(15));
}

#[tokio::test]
async fn test_parallel_ainvoke() {
    let parallel = RunnableParallel::<i32>::builder().build()
        .add("doubled", RunnableLambda::builder().func(|x: i32| Ok(json!(x * 2))).build())
        .add("tripled", RunnableLambda::builder().func(|x: i32| Ok(json!(x * 3))).build());

    let result = parallel.ainvoke(5, None).await.unwrap();
    assert_eq!(result["doubled"], json!(10));
    assert_eq!(result["tripled"], json!(15));
}

#[test]
fn test_parallel_name() {
    let parallel = RunnableParallel::<i32>::builder().build()
        .add("a", RunnableLambda::builder().func(|x: i32| Ok(json!(x))).build())
        .add("b", RunnableLambda::builder().func(|x: i32| Ok(json!(x))).build());

    let name = parallel.name().unwrap();
    assert!(name.starts_with("RunnableParallel<"));
    assert!(name.contains('a'));
    assert!(name.contains('b'));
}

#[test]
fn test_parallel_error_in_branch() {
    let parallel = RunnableParallel::<i32>::builder().build()
        .add("ok", RunnableLambda::builder().func(|x: i32| Ok(json!(x))).build())
        .add(
            "fail",
            RunnableLambda::builder().func(|_: i32| Err::<Value, _>(Error::other("branch error"))).build(),
        );

    let result = parallel.invoke(5, None);
    assert!(result.is_err());
}

#[test]
fn test_each_simple() {
    let double = RunnableLambda::builder().func(|x: i32| Ok(x * 2)).build();
    let each = RunnableEach::new(double);

    let result = each.invoke(vec![1, 2, 3], None).unwrap();
    assert_eq!(result, vec![2, 4, 6]);
}

#[test]
fn test_map_convenience() {
    let double = RunnableLambda::builder().func(|x: i32| Ok(x * 2)).build();
    let each = double.map();

    let result = each.invoke(vec![1, 2, 3, 4, 5], None).unwrap();
    assert_eq!(result, vec![2, 4, 6, 8, 10]);
}

#[test]
fn test_map_nested() {
    let add_one = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();
    let inner_each = add_one.map();
    let outer_each = inner_each.map();

    let input = vec![vec![1, 2, 3], vec![10, 20]];
    let result = outer_each.invoke(input, None).unwrap();
    assert_eq!(result, vec![vec![2, 3, 4], vec![11, 21]]);
}

#[test]
fn test_each_error() {
    let fail_on_3 = RunnableLambda::builder().func(|x: i32| {
        if x == 3 {
            Err(Error::other("no threes"))
        } else {
            Ok(x * 2)
        }
    }).build();
    let each = RunnableEach::new(fail_on_3);

    let result = each.invoke(vec![1, 2, 3, 4], None);
    assert!(result.is_err());
}

#[test]
fn test_each_name() {
    let named = RunnableLambda::builder().func(|x: i32| Ok(x)).name("identity").build();
    let each = named.map();
    assert_eq!(each.name(), Some("RunnableEach<identity>".to_string()));
}

#[test]
fn test_combining_sequences() {
    let parallel = RunnableParallel::<i32>::builder().build()
        .add("doubled", RunnableLambda::builder().func(|x: i32| Ok(json!(x * 2))).build())
        .add("tripled", RunnableLambda::builder().func(|x: i32| Ok(json!(x * 3))).build());

    let pick = RunnableLambda::builder().func(|m: HashMap<String, Value>| {
        Ok(m.get("doubled").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
    }).build();

    let chain = pipe(parallel, pick);
    assert_eq!(chain.invoke(5, None).unwrap(), 10);
}

#[tokio::test]
async fn test_transform_of_runnable_lambda_with_dicts() {
    let runnable = RunnableLambda::builder().func(|x: HashMap<String, Value>| Ok(x)).build();

    let chunks = vec![make_input(&[("foo", json!("n"))])];
    let input_stream = futures::stream::iter(chunks);

    let result: Vec<HashMap<String, Value>> = runnable
        .transform(Box::pin(input_stream), None)
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(result, vec![make_input(&[("foo", json!("n"))])]);
}

#[tokio::test]
async fn test_transform_sequence_with_dicts() {
    let identity1 = RunnableLambda::builder().func(|x: HashMap<String, Value>| Ok(x)).build();
    let identity2 = RunnableLambda::builder().func(|x: HashMap<String, Value>| Ok(x)).build();
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

#[test]
fn test_lambda_batch() {
    let double = RunnableLambda::builder().func(|x: i32| Ok(x * 2)).build();
    let results = double.batch(vec![1, 2, 3, 4, 5], None, false);
    let values: Vec<i32> = results.into_iter().map(|r| r.unwrap()).collect();
    assert_eq!(values, vec![2, 4, 6, 8, 10]);
}

#[test]
fn test_sequence_batch() {
    let add_one = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();
    let double = RunnableLambda::builder().func(|x: i32| Ok(x * 2)).build();
    let seq = pipe(add_one, double);

    let results = seq.batch(vec![1, 2, 3], None, false);
    let values: Vec<i32> = results.into_iter().map(|r| r.unwrap()).collect();
    assert_eq!(values, vec![4, 6, 8]); // (1+1)*2=4, (2+1)*2=6, (3+1)*2=8
}

#[test]
fn test_seq_batch_return_exceptions() {
    let maybe_fail = RunnableLambda::builder().func(|x: i32| {
        if x == 2 {
            Err(Error::other("fail on 2"))
        } else {
            Ok(x + 1)
        }
    }).build();
    let double = RunnableLambda::builder().func(|x: i32| Ok(x * 2)).build();
    let seq = pipe(maybe_fail, double);

    let results = seq.batch(vec![1, 2, 3], None, true);
    assert!(results[0].is_ok());
    assert_eq!(*results[0].as_ref().unwrap(), 4); // (1+1)*2=4
    assert!(results[1].is_err()); // fails on 2
    assert!(results[2].is_ok());
    assert_eq!(*results[2].as_ref().unwrap(), 8); // (3+1)*2=8
}

#[test]
fn test_empty_batch() {
    let double = RunnableLambda::builder().func(|x: i32| Ok(x * 2)).build();
    let results = double.batch(vec![], None, false);
    assert!(results.is_empty());
}

#[tokio::test]
async fn test_lambda_abatch() {
    let double = RunnableLambda::builder().func(|x: i32| Ok(x * 2)).build();
    let results = double.abatch(vec![1, 2, 3], None, false).await;
    let values: Vec<i32> = results.into_iter().map(|r| r.unwrap()).collect();
    assert_eq!(values, vec![2, 4, 6]);
}

#[tokio::test]
async fn test_sequence_abatch() {
    let add_one = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();
    let double = RunnableLambda::builder().func(|x: i32| Ok(x * 2)).build();
    let seq = pipe(add_one, double);

    let results = seq.abatch(vec![1, 2, 3], None, false).await;
    let values: Vec<i32> = results.into_iter().map(|r| r.unwrap()).collect();
    assert_eq!(values, vec![4, 6, 8]);
}

#[test]
fn test_runnable_assign() {
    let mapper = RunnableParallel::<HashMap<String, Value>>::builder().build().add(
        "add_step",
        RunnableLambda::builder().func(|x: HashMap<String, Value>| {
            let input_val = x.get("input").and_then(|v| v.as_i64()).unwrap_or(0);
            Ok(json!({"added": input_val + 10}))
        }).build(),
    );
    let assign = RunnableAssign::builder().mapper(mapper).build();

    let input = make_input(&[("input", json!(5))]);
    let result = assign.invoke(input, None).unwrap();

    assert_eq!(result["input"], json!(5));
    assert_eq!(result["add_step"], json!({"added": 15}));
}

#[test]
fn test_representation_of_runnables() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x * 2)).build();
    let repr = format!("{:?}", runnable);
    assert!(repr.contains("RunnableLambda"));

    let seq = pipe(
        RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build(),
        RunnableLambda::builder().func(|x: i32| Ok(x * 2)).build(),
    );
    let repr = format!("{:?}", seq);
    assert!(repr.contains("RunnableSequence"));

    let parallel = RunnableParallel::<i32>::builder().build()
        .add("a", RunnableLambda::builder().func(|x: i32| Ok(json!(x))).build())
        .add("b", RunnableLambda::builder().func(|x: i32| Ok(json!(x))).build());
    let repr = format!("{:?}", parallel);
    assert!(repr.contains("RunnableParallel"));

    let binding = RunnableLambda::builder().func(|x: i32| Ok(x)).build().with_config(RunnableConfig::default());
    let repr = format!("{:?}", binding);
    assert!(repr.contains("RunnableBinding"));

    let each = RunnableLambda::builder().func(|x: i32| Ok(x)).build().map();
    let repr = format!("{:?}", each);
    assert!(repr.contains("RunnableEach"));
}

#[test]
fn test_default_method_implementations() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();

    assert_eq!(runnable.invoke(5, None).unwrap(), 6);

    let results = runnable.batch(vec![1, 2, 3], None, false);
    let values: Vec<i32> = results.into_iter().map(|r| r.unwrap()).collect();
    assert_eq!(values, vec![2, 3, 4]);
}

#[tokio::test]
async fn test_default_method_implementations_async() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();

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

#[test]
fn test_sequence_schema() {
    let step1 = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).name("step1").build();
    let step2 = RunnableLambda::builder().func(|x: i32| Ok(x * 2)).name("step2").build();
    let seq = pipe(step1, step2);

    let input_schema = seq.get_input_schema(None);
    assert_eq!(input_schema["type"], "object");

    let output_schema = seq.get_output_schema(None);
    assert_eq!(output_schema["type"], "object");
}

#[test]
fn test_parallel_schema() {
    let parallel =
        RunnableParallel::<i32>::builder().build().add("a", RunnableLambda::builder().func(|x: i32| Ok(json!(x))).build());

    let input_schema = parallel.get_input_schema(None);
    assert_eq!(input_schema["type"], "object");
}

#[test]
fn test_binding_schema_delegation() {
    let inner = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).name("inner").build();
    let bound = inner.with_config(RunnableConfig::default());

    let inner_schema = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).name("inner").build()
        .get_input_schema(None);

    assert_eq!(bound.get_input_schema(None), inner_schema);
}

#[test]
fn test_get_name_with_suffix() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x)).name("MyRunnable").build();
    let name = runnable.get_name(Some("Input"), None);
    assert_eq!(name, "MyRunnableInput");
}

#[test]
fn test_get_name_lowercase_with_suffix() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x)).name("my_runnable").build();
    let name = runnable.get_name(Some("input"), None);
    assert_eq!(name, "my_runnable_input");
}

#[test]
fn test_get_name_override() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x)).name("original").build();
    let name = runnable.get_name(None, Some("override"));
    assert_eq!(name, "override");
}

#[test]
fn test_each_call_count() {
    let counter = Arc::new(AtomicUsize::new(0));
    let c = counter.clone();

    let runnable = RunnableLambda::builder().func(move |x: i32| {
        c.fetch_add(1, Ordering::SeqCst);
        Ok(x)
    }).build();

    let each = RunnableEach::new(runnable);
    let _ = each.invoke(vec![1, 2, 3, 4, 5], None).unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 5);
}

#[test]
fn test_sequence_step_count() {
    let counter1 = Arc::new(AtomicUsize::new(0));
    let counter2 = Arc::new(AtomicUsize::new(0));
    let c1 = counter1.clone();
    let c2 = counter2.clone();

    let step1 = RunnableLambda::builder().func(move |x: i32| {
        c1.fetch_add(1, Ordering::SeqCst);
        Ok(x + 1)
    }).build();
    let step2 = RunnableLambda::builder().func(move |x: i32| {
        c2.fetch_add(1, Ordering::SeqCst);
        Ok(x * 2)
    }).build();

    let seq = pipe(step1, step2);
    let _ = seq.invoke(5, None).unwrap();

    assert_eq!(counter1.load(Ordering::SeqCst), 1);
    assert_eq!(counter2.load(Ordering::SeqCst), 1);
}

#[test]
fn test_parallel_call_count() {
    let counter_a = Arc::new(AtomicUsize::new(0));
    let counter_b = Arc::new(AtomicUsize::new(0));
    let ca = counter_a.clone();
    let cb = counter_b.clone();

    let parallel = RunnableParallel::<i32>::builder().build()
        .add(
            "a",
            RunnableLambda::builder().func(move |x: i32| {
                ca.fetch_add(1, Ordering::SeqCst);
                Ok(json!(x))
            }).build(),
        )
        .add(
            "b",
            RunnableLambda::builder().func(move |x: i32| {
                cb.fetch_add(1, Ordering::SeqCst);
                Ok(json!(x))
            }).build(),
        );

    let _ = parallel.invoke(5, None).unwrap();
    assert_eq!(counter_a.load(Ordering::SeqCst), 1);
    assert_eq!(counter_b.load(Ordering::SeqCst), 1);
}

#[test]
fn test_type_name() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x)).build();
    let name = runnable.type_name();
    assert!(name.contains("RunnableLambda"));
}

#[test]
fn test_parallel_default() {
    let parallel = RunnableParallel::<i32>::default();
    let result = parallel.invoke(5, None).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_binding_debug() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x)).build();
    let bound = runnable.bind(HashMap::from([("key".into(), json!("val"))]));
    let debug = format!("{:?}", bound);
    assert!(debug.contains("RunnableBinding"));
    assert!(debug.contains("key"));
}

#[test]
fn test_pick_single_key() {
    use agent_chain_core::runnables::passthrough::PickKeys;

    let runnable = RunnableLambda::builder().func(|_x: i32| {
        let mut map = HashMap::new();
        map.insert("name".to_string(), json!("Alice"));
        map.insert("age".to_string(), json!(30));
        Ok(map)
    }).build();

    let picked = runnable.pick(PickKeys::Single("name".to_string()));
    let result = picked.invoke(1, None).unwrap();
    assert_eq!(result, json!("Alice"));
}

#[test]
fn test_pick_multiple_keys() {
    use agent_chain_core::runnables::passthrough::PickKeys;

    let runnable = RunnableLambda::builder().func(|_x: i32| {
        let mut map = HashMap::new();
        map.insert("name".to_string(), json!("Alice"));
        map.insert("age".to_string(), json!(30));
        map.insert("city".to_string(), json!("NYC"));
        Ok(map)
    }).build();

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

#[test]
fn test_assign_convenience() {
    let passthrough = RunnablePassthrough::<HashMap<String, serde_json::Value>>::new();

    let mapper = RunnablePassthrough::<HashMap<String, serde_json::Value>>::assign()
        .add(
            "doubled",
            RunnableLambda::builder().func(|input: HashMap<String, serde_json::Value>| {
                let val = input.get("x").and_then(|v| v.as_i64()).unwrap_or(0);
                Ok(json!(val * 2))
            }).build(),
        )
        .build();

    let chained = passthrough.assign(mapper);

    let mut input = HashMap::new();
    input.insert("x".to_string(), json!(5));

    let result = chained.invoke(input, None).unwrap();
    assert_eq!(result.get("x"), Some(&json!(5)));
    assert_eq!(result.get("doubled"), Some(&json!(10)));
}

#[test]
fn test_with_fallbacks_convenience() {
    let primary = RunnableLambda::builder().func(|_x: i32| -> Result<i32, Error> {
        Err(Error::other("primary failed"))
    }).build();

    let fallback = RunnableLambda::builder().func(|x: i32| -> Result<i32, Error> { Ok(x * 2) }).build();

    let with_fallbacks = primary.with_fallbacks(vec![Arc::new(fallback)]);
    let result = with_fallbacks.invoke(5, None).unwrap();
    assert_eq!(result, 10);
}

#[test]
fn test_with_fallbacks_primary_succeeds() {
    let primary = RunnableLambda::builder().func(|x: i32| -> Result<i32, Error> { Ok(x + 1) }).build();

    let fallback = RunnableLambda::builder().func(|x: i32| -> Result<i32, Error> { Ok(x * 100) }).build();

    let with_fallbacks = primary.with_fallbacks(vec![Arc::new(fallback)]);
    let result = with_fallbacks.invoke(5, None).unwrap();
    assert_eq!(result, 6);
}

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

    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();
    let with_listeners = runnable.with_listeners(Some(on_start), Some(on_end), None);

    let result = with_listeners.invoke(5, None).unwrap();
    assert_eq!(result, 6);
}

#[test]
fn test_chaining_pick_with_fallbacks() {
    use agent_chain_core::runnables::passthrough::PickKeys;

    let runnable = RunnableLambda::builder().func(|_x: i32| {
        let mut map = HashMap::new();
        map.insert("name".to_string(), json!("Alice"));
        map.insert("age".to_string(), json!(30));
        Ok(map)
    }).build();

    let picked = runnable.pick(PickKeys::Single("name".to_string()));

    let fallback_picked = {
        let fallback_inner = RunnableLambda::builder().func(|_x: i32| {
            let mut map = HashMap::new();
            map.insert("name".to_string(), json!("Fallback"));
            Ok(map)
        }).build();
        use agent_chain_core::runnables::passthrough::RunnablePick;
        pipe(fallback_inner, RunnablePick::new_single_builder().key("name").build())
    };

    let with_fallbacks = picked.with_fallbacks(vec![Arc::new(fallback_picked)]);

    let result = with_fallbacks.invoke(1, None).unwrap();
    assert_eq!(result, json!("Alice"));
}
