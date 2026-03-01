use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use agent_chain_core::error::{Error, Result};
use agent_chain_core::runnables::base::{Runnable, RunnableLambda};
use agent_chain_core::runnables::config::RunnableConfig;
use agent_chain_core::runnables::retry::{
    ExponentialJitterParams, RetryErrorPredicate, RunnableRetry, RunnableRetryConfig,
    RunnableRetryExt,
};

#[test]
fn test_retry_initialization() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();

    let config = RunnableRetryConfig::builder().build();
    assert_eq!(config.max_attempt_number, 3);
    assert!(config.wait_exponential_jitter);

    let retry = RunnableRetry::new(runnable, config);
    let result = retry.invoke(5, None).unwrap();
    assert_eq!(result, 6);
}

#[test]
fn test_retry_initialization_custom() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();

    let config = RunnableRetryConfig::builder()
        .max_attempt_number(5)
        .wait_exponential_jitter(false)
        .retry_predicate(RetryErrorPredicate::Custom(|e| {
            matches!(e, Error::Other(_))
        }))
        .build();

    assert_eq!(config.max_attempt_number, 5);
    assert!(!config.wait_exponential_jitter);

    let retry = RunnableRetry::new(runnable, config);
    let result = retry.invoke(5, None).unwrap();
    assert_eq!(result, 6);
}

#[test]
fn test_retry_invoke_success_no_retry() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let counter = call_count.clone();

    let runnable = RunnableLambda::builder()
        .func(move |x: i32| {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(x * 2)
        })
        .build();

    let retry = RunnableRetry::new(
        runnable,
        RunnableRetryConfig::builder()
            .max_attempt_number(3)
            .wait_exponential_jitter(false)
            .build(),
    );

    let result = retry.invoke(5, None).unwrap();
    assert_eq!(result, 10);
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
}

#[test]
fn test_retry_invoke_with_retryable_exception() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let counter = call_count.clone();

    let runnable = RunnableLambda::builder()
        .func(move |x: i32| {
            let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
            if count < 3 {
                Err(Error::other(format!("Attempt {count} failed")))
            } else {
                Ok(x * 2)
            }
        })
        .build();

    let retry = RunnableRetry::new(
        runnable,
        RunnableRetryConfig::builder()
            .max_attempt_number(3)
            .wait_exponential_jitter(false)
            .build(),
    );

    let result = retry.invoke(5, None).unwrap();
    assert_eq!(result, 10);
    assert_eq!(call_count.load(Ordering::SeqCst), 3);
}

#[test]
fn test_retry_invoke_exhausts_retries() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let counter = call_count.clone();

    let runnable = RunnableLambda::builder()
        .func(move |_x: i32| {
            counter.fetch_add(1, Ordering::SeqCst);
            Err::<i32, _>(Error::other("Always fails"))
        })
        .build();

    let retry = RunnableRetry::new(
        runnable,
        RunnableRetryConfig::builder()
            .max_attempt_number(2)
            .wait_exponential_jitter(false)
            .build(),
    );

    let result = retry.invoke(5, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Always fails"));
    assert_eq!(call_count.load(Ordering::SeqCst), 2);
}

#[test]
fn test_retry_invoke_non_retryable_exception() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let counter = call_count.clone();

    let runnable = RunnableLambda::builder()
        .func(move |_x: i32| {
            counter.fetch_add(1, Ordering::SeqCst);
            Err::<i32, _>(Error::InvalidConfig("Runtime error".into()))
        })
        .build();

    let retry = RunnableRetry::new(
        runnable,
        RunnableRetryConfig::builder()
            .max_attempt_number(3)
            .retry_predicate(RetryErrorPredicate::HttpErrors)
            .wait_exponential_jitter(false)
            .build(),
    );

    let result = retry.invoke(5, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Runtime error"));
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_retry_ainvoke_success_no_retry() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let counter = call_count.clone();

    let runnable = RunnableLambda::builder()
        .func(move |x: i32| {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(x * 2)
        })
        .build();

    let retry = RunnableRetry::new(
        runnable,
        RunnableRetryConfig::builder()
            .max_attempt_number(3)
            .wait_exponential_jitter(false)
            .build(),
    );

    let result = retry.ainvoke(5, None).await.unwrap();
    assert_eq!(result, 10);
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_retry_ainvoke_with_retryable_exception() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let counter = call_count.clone();

    let runnable = RunnableLambda::builder()
        .func(move |x: i32| {
            let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
            if count < 3 {
                Err(Error::other(format!("Attempt {count} failed")))
            } else {
                Ok(x * 2)
            }
        })
        .build();

    let retry = RunnableRetry::new(
        runnable,
        RunnableRetryConfig::builder()
            .max_attempt_number(3)
            .wait_exponential_jitter(false)
            .build(),
    );

    let result = retry.ainvoke(5, None).await.unwrap();
    assert_eq!(result, 10);
    assert_eq!(call_count.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_retry_ainvoke_exhausts_retries() {
    let runnable = RunnableLambda::builder()
        .func(|_x: i32| Err::<i32, _>(Error::other("Always fails")))
        .build();

    let retry = RunnableRetry::new(
        runnable,
        RunnableRetryConfig::builder()
            .max_attempt_number(2)
            .wait_exponential_jitter(false)
            .build(),
    );

    let result = retry.ainvoke(5, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Always fails"));
}

#[test]
fn test_retry_batch_partial_failures() {
    let call_counts: Arc<std::sync::Mutex<std::collections::HashMap<i32, usize>>> =
        Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    let counts = call_counts.clone();

    let runnable = RunnableLambda::builder()
        .func(move |x: i32| {
            let mut map = counts.lock().unwrap();
            let count = map.entry(x).or_insert(0);
            *count += 1;
            if (x == 1 || x == 2) && *count < 2 {
                Err(Error::other(format!("Fail {x} on attempt {count}")))
            } else {
                Ok(x * 2)
            }
        })
        .build();

    let retry = RunnableRetry::new(
        runnable,
        RunnableRetryConfig::builder()
            .max_attempt_number(2)
            .wait_exponential_jitter(false)
            .build(),
    );

    let results = retry.batch(vec![0, 1, 2, 3], None, false);
    for (i, result) in results.iter().enumerate() {
        let expected = (i as i32) * 2;
        assert_eq!(result.as_ref().unwrap(), &expected, "input {i} failed");
    }

    let map = call_counts.lock().unwrap();
    assert_eq!(*map.get(&0).unwrap(), 1); // No retry needed
    assert_eq!(*map.get(&1).unwrap(), 2); // Retried once
    assert_eq!(*map.get(&2).unwrap(), 2); // Retried once
    assert_eq!(*map.get(&3).unwrap(), 1); // No retry needed
}

#[test]
fn test_retry_batch_with_return_exceptions() {
    let runnable = RunnableLambda::builder()
        .func(|x: i32| {
            if x == 1 {
                Err(Error::other("Always fails on 1"))
            } else {
                Ok(x * 2)
            }
        })
        .build();

    let retry = RunnableRetry::new(
        runnable,
        RunnableRetryConfig::builder()
            .max_attempt_number(2)
            .wait_exponential_jitter(false)
            .build(),
    );

    let results = retry.batch(vec![0, 1, 2], None, true);
    assert_eq!(*results[0].as_ref().unwrap(), 0);
    assert!(results[1].is_err());
    assert!(
        results[1]
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("Always fails on 1")
    );
    assert_eq!(*results[2].as_ref().unwrap(), 4);
}

#[tokio::test]
async fn test_retry_abatch_partial_failures() {
    let call_counts: Arc<std::sync::Mutex<std::collections::HashMap<i32, usize>>> =
        Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    let counts = call_counts.clone();

    let runnable = RunnableLambda::builder()
        .func(move |x: i32| {
            let mut map = counts.lock().unwrap();
            let count = map.entry(x).or_insert(0);
            *count += 1;
            if (x == 1 || x == 2) && *count < 2 {
                Err(Error::other(format!("Fail {x}")))
            } else {
                Ok(x * 2)
            }
        })
        .build();

    let retry = RunnableRetry::new(
        runnable,
        RunnableRetryConfig::builder()
            .max_attempt_number(2)
            .wait_exponential_jitter(false)
            .build(),
    );

    let results = retry.abatch(vec![0, 1, 2, 3], None, false).await;
    for (i, result) in results.iter().enumerate() {
        let expected = (i as i32) * 2;
        assert_eq!(result.as_ref().unwrap(), &expected, "input {i} failed");
    }
}

#[tokio::test]
async fn test_retry_abatch_with_return_exceptions() {
    let runnable = RunnableLambda::builder()
        .func(|x: i32| {
            if x == 1 {
                Err(Error::other("Always fails on 1"))
            } else {
                Ok(x * 2)
            }
        })
        .build();

    let retry = RunnableRetry::new(
        runnable,
        RunnableRetryConfig::builder()
            .max_attempt_number(2)
            .wait_exponential_jitter(false)
            .build(),
    );

    let results = retry.abatch(vec![0, 1, 2], None, true).await;
    assert_eq!(*results[0].as_ref().unwrap(), 0);
    assert!(results[1].is_err());
    assert_eq!(*results[2].as_ref().unwrap(), 4);
}

#[test]
fn test_retry_with_exponential_jitter() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let counter = call_count.clone();

    let runnable = RunnableLambda::builder()
        .func(move |_x: i32| {
            let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
            if count == 1 {
                Err(Error::other("First attempt fails"))
            } else {
                Ok(42)
            }
        })
        .build();

    let retry = RunnableRetry::new(
        runnable,
        RunnableRetryConfig::builder()
            .max_attempt_number(2)
            .wait_exponential_jitter(true)
            .exponential_jitter_params(
                ExponentialJitterParams::builder()
                    .initial(0.01)
                    .max(0.1)
                    .jitter(0.0)
                    .build(),
            )
            .build(),
    );

    let result = retry.invoke(1, None).unwrap();
    assert_eq!(result, 42);
    assert_eq!(call_count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_retry_async_with_exponential_jitter() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let counter = call_count.clone();

    let runnable = RunnableLambda::builder()
        .func(move |_x: i32| {
            let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
            if count == 1 {
                Err(Error::other("First attempt fails"))
            } else {
                Ok(42)
            }
        })
        .build();

    let retry = RunnableRetry::new(
        runnable,
        RunnableRetryConfig::builder()
            .max_attempt_number(2)
            .wait_exponential_jitter(true)
            .exponential_jitter_params(
                ExponentialJitterParams::builder()
                    .initial(0.01)
                    .max(0.1)
                    .jitter(0.0)
                    .build(),
            )
            .build(),
    );

    let result = retry.ainvoke(1, None).await.unwrap();
    assert_eq!(result, 42);
    assert_eq!(call_count.load(Ordering::SeqCst), 2);
}

#[test]
fn test_retry_with_config() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();

    let retry = RunnableRetry::new(
        runnable,
        RunnableRetryConfig::builder()
            .max_attempt_number(2)
            .wait_exponential_jitter(false)
            .build(),
    );

    let mut config = RunnableConfig::default();
    config.tags.push("test-tag".to_string());
    config
        .metadata
        .insert("key".to_string(), serde_json::json!("value"));

    let result = retry.invoke(5, Some(config)).unwrap();
    assert_eq!(result, 6);
}

#[test]
fn test_retry_config_propagation() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let counter = call_count.clone();

    let runnable = RunnableLambda::builder()
        .func(move |x: i32| {
            let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
            if count < 2 {
                Err(Error::other("First attempt fails"))
            } else {
                Ok(x * 2)
            }
        })
        .build();

    let retry = RunnableRetry::new(
        runnable,
        RunnableRetryConfig::builder()
            .max_attempt_number(2)
            .wait_exponential_jitter(false)
            .build(),
    );

    let mut config = RunnableConfig::default();
    config.tags.push("my-tag".to_string());

    let result = retry.invoke(5, Some(config)).unwrap();
    assert_eq!(result, 10);
    assert_eq!(call_count.load(Ordering::SeqCst), 2);
}

#[test]
fn test_retry_multiple_exception_types() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let counter = call_count.clone();

    let runnable = RunnableLambda::builder()
        .func(move |x: i32| {
            let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
            if count == 1 {
                Err(Error::other("ValueError"))
            } else if count == 2 {
                Err(Error::InvalidConfig("TypeError".into()))
            } else {
                Ok(x * 2)
            }
        })
        .build();

    let retry = RunnableRetry::new(
        runnable,
        RunnableRetryConfig::builder()
            .max_attempt_number(3)
            .wait_exponential_jitter(false)
            .retry_predicate(RetryErrorPredicate::Custom(|e| {
                matches!(e, Error::Other(_) | Error::InvalidConfig(_))
            }))
            .build(),
    );

    let result = retry.invoke(5, None).unwrap();
    assert_eq!(result, 10);
    assert_eq!(call_count.load(Ordering::SeqCst), 3);
}

#[test]
fn test_retry_batch_preserves_order() {
    let first_fail: Arc<std::sync::Mutex<std::collections::HashSet<i32>>> =
        Arc::new(std::sync::Mutex::new(std::collections::HashSet::from([1])));
    let fail_set = first_fail.clone();

    let runnable = RunnableLambda::builder()
        .func(move |x: i32| {
            let mut set = fail_set.lock().unwrap();
            if set.contains(&x) {
                set.remove(&x);
                Err(Error::other("fail once"))
            } else {
                Ok(x)
            }
        })
        .build();

    let retry = RunnableRetry::new(
        runnable,
        RunnableRetryConfig::builder()
            .max_attempt_number(2)
            .wait_exponential_jitter(false)
            .build(),
    );

    let results = retry.batch(vec![0, 1, 2], None, false);
    assert_eq!(*results[0].as_ref().unwrap(), 0);
    assert_eq!(*results[1].as_ref().unwrap(), 1);
    assert_eq!(*results[2].as_ref().unwrap(), 2);
}

#[tokio::test]
async fn test_retry_abatch_preserves_order() {
    let first_fail: Arc<std::sync::Mutex<std::collections::HashSet<i32>>> =
        Arc::new(std::sync::Mutex::new(std::collections::HashSet::from([1])));
    let fail_set = first_fail.clone();

    let runnable = RunnableLambda::builder()
        .func(move |x: i32| {
            let mut set = fail_set.lock().unwrap();
            if set.contains(&x) {
                set.remove(&x);
                Err(Error::other("fail once"))
            } else {
                Ok(x)
            }
        })
        .build();

    let retry = RunnableRetry::new(
        runnable,
        RunnableRetryConfig::builder()
            .max_attempt_number(2)
            .wait_exponential_jitter(false)
            .build(),
    );

    let results = retry.abatch(vec![0, 1, 2], None, false).await;
    assert_eq!(*results[0].as_ref().unwrap(), 0);
    assert_eq!(*results[1].as_ref().unwrap(), 1);
    assert_eq!(*results[2].as_ref().unwrap(), 2);
}

#[test]
fn test_retry_batch_all_fail() {
    let runnable = RunnableLambda::builder()
        .func(|_x: i32| Err::<i32, _>(Error::other("Always fails")))
        .build();

    let retry = RunnableRetry::new(
        runnable,
        RunnableRetryConfig::builder()
            .max_attempt_number(2)
            .wait_exponential_jitter(false)
            .build(),
    );

    let results = retry.batch(vec![1, 2, 3], None, true);
    assert!(results.iter().all(|r| r.is_err()));
}

#[test]
fn test_retry_batch_empty_input() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x)).build();

    let retry = RunnableRetry::new(runnable, RunnableRetryConfig::builder().build());

    let results = retry.batch(vec![], None, false);
    assert!(results.is_empty());
}

#[tokio::test]
async fn test_retry_abatch_empty_input() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x)).build();

    let retry = RunnableRetry::new(runnable, RunnableRetryConfig::builder().build());

    let results = retry.abatch(vec![], None, false).await;
    assert!(results.is_empty());
}

#[tokio::test]
async fn test_retry_stream_uses_invoke_with_retries() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let counter = call_count.clone();

    let runnable = RunnableLambda::builder()
        .func(move |x: i32| {
            let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
            if count == 1 {
                Err(Error::other("First attempt fails"))
            } else {
                Ok(x * 2)
            }
        })
        .build();

    let retry = RunnableRetry::new(
        runnable,
        RunnableRetryConfig::builder()
            .max_attempt_number(3)
            .wait_exponential_jitter(false)
            .build(),
    );

    use futures::StreamExt;
    let results: Vec<Result<i32>> = retry.stream(5, None).collect().await;
    assert_eq!(results.len(), 1);
    assert_eq!(*results[0].as_ref().unwrap(), 10);
    assert_eq!(call_count.load(Ordering::SeqCst), 2);
}

#[test]
fn test_retry_chain_composition() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let counter = call_count.clone();

    let reliable_step_1 = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();

    let unreliable_step = RunnableLambda::builder()
        .func(move |x: i32| {
            let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
            if count == 1 {
                Err(Error::other("First attempt fails"))
            } else {
                Ok(x * 2)
            }
        })
        .build();
    let unreliable_with_retry = RunnableRetry::new(
        unreliable_step,
        RunnableRetryConfig::builder()
            .max_attempt_number(2)
            .wait_exponential_jitter(false)
            .build(),
    );

    let reliable_step_2 = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();

    let step1_result = reliable_step_1.invoke(5, None).unwrap();
    let step2_result = unreliable_with_retry.invoke(step1_result, None).unwrap();
    let final_result = reliable_step_2.invoke(step2_result, None).unwrap();

    assert_eq!(final_result, 13);
    assert_eq!(call_count.load(Ordering::SeqCst), 2);
}

#[test]
fn test_retry_batch_individual_tracking() {
    let call_tracker: Arc<std::sync::Mutex<std::collections::HashMap<i32, Vec<i32>>>> =
        Arc::new(std::sync::Mutex::new(std::collections::HashMap::from([
            (0, Vec::new()),
            (1, Vec::new()),
            (2, Vec::new()),
        ])));
    let tracker = call_tracker.clone();

    let runnable = RunnableLambda::builder()
        .func(move |x: i32| {
            let mut map = tracker.lock().unwrap();
            let calls = map.entry(x).or_default();
            calls.push(x);
            let count = calls.len();
            if x == 0 && count < 3 {
                Err(Error::other("Fail twice"))
            } else if x == 1 && count < 2 {
                Err(Error::other("Fail once"))
            } else {
                Ok(x * 2)
            }
        })
        .build();

    let retry = RunnableRetry::new(
        runnable,
        RunnableRetryConfig::builder()
            .max_attempt_number(3)
            .wait_exponential_jitter(false)
            .build(),
    );

    let results = retry.batch(vec![0, 1, 2], None, false);
    assert_eq!(*results[0].as_ref().unwrap(), 0);
    assert_eq!(*results[1].as_ref().unwrap(), 2);
    assert_eq!(*results[2].as_ref().unwrap(), 4);

    let map = call_tracker.lock().unwrap();
    assert_eq!(map[&0].len(), 3); // Failed twice, succeeded on third
    assert_eq!(map[&1].len(), 2); // Failed once, succeeded on second
    assert_eq!(map[&2].len(), 1); // Succeeded immediately
}

#[test]
fn test_exponential_jitter_params_calculation() {
    let params = ExponentialJitterParams::builder()
        .initial(0.1)
        .max(1.0)
        .exp_base(2.0)
        .jitter(0.0)
        .build();

    let wait1 = params.calculate_wait(1);
    assert!((wait1.as_secs_f64() - 0.1).abs() < 0.01);

    let wait2 = params.calculate_wait(2);
    assert!((wait2.as_secs_f64() - 0.2).abs() < 0.01);

    let wait3 = params.calculate_wait(3);
    assert!((wait3.as_secs_f64() - 0.4).abs() < 0.01);
}

#[test]
fn test_exponential_jitter_max_cap() {
    let params = ExponentialJitterParams::builder()
        .initial(1.0)
        .max(2.0)
        .exp_base(10.0)
        .jitter(0.0)
        .build();

    let wait = params.calculate_wait(10);
    assert!((wait.as_secs_f64() - 2.0).abs() < 0.01);
}

#[test]
fn test_exponential_jitter_defaults() {
    let params = ExponentialJitterParams::default();
    assert_eq!(params.initial, 1.0);
    assert_eq!(params.max, 60.0);
    assert_eq!(params.exp_base, 2.0);
    assert_eq!(params.jitter, 1.0);
}

#[test]
fn test_retry_ext_trait() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();
    let config = RunnableRetryConfig::builder().max_attempt_number(3).build();
    let retry = runnable.with_retry_config(config);

    let result = retry.invoke(1, None).unwrap();
    assert_eq!(result, 2);
}

#[test]
fn test_with_retry_convenience() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();
    let retry = runnable.with_retry(3, false);

    let result = retry.invoke(1, None).unwrap();
    assert_eq!(result, 2);
}

#[test]
fn test_retry_debug() {
    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();
    let retry = RunnableRetry::new(runnable, RunnableRetryConfig::builder().build());

    let debug_str = format!("{:?}", retry);
    assert!(debug_str.contains("RunnableRetry"));
    assert!(debug_str.contains("max_attempt_number"));
}

#[test]
fn test_retry_name_propagation() {
    let runnable = RunnableLambda::builder()
        .func(|x: i32| Ok(x + 1))
        .name("my_step")
        .build();
    let retry = RunnableRetry::new(runnable, RunnableRetryConfig::builder().build());

    assert_eq!(retry.name(), Some("my_step".to_string()));
}

#[test]
fn test_retry_preserves_schemas() {
    let runnable_for_schema = RunnableLambda::builder()
        .func(|x: i32| Ok(x.to_string()))
        .build();
    let runnable_for_retry = RunnableLambda::builder()
        .func(|x: i32| Ok(x.to_string()))
        .build();

    let retry_runnable =
        RunnableRetry::new(runnable_for_retry, RunnableRetryConfig::builder().build());

    assert_eq!(
        retry_runnable.get_input_schema(None),
        runnable_for_schema.get_input_schema(None),
    );
    assert_eq!(
        retry_runnable.get_output_schema(None),
        runnable_for_schema.get_output_schema(None),
    );
}
