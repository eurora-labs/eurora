//! Tests for concurrency behavior of batch and async batch operations.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/runnables/test_concurrency.py`

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use futures::StreamExt;

use agent_chain_core::error::Error;
use agent_chain_core::runnables::base::{Runnable, RunnableLambda};
use agent_chain_core::runnables::config::{ConfigOrList, RunnableConfig};

// ============================================================================
// Async batch concurrency tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_abatch_concurrency() {
    let running_tasks = Arc::new(AtomicUsize::new(0));
    let max_running_tasks = Arc::new(AtomicUsize::new(0));

    let running = running_tasks.clone();
    let max_running = max_running_tasks.clone();

    let runnable = RunnableLambda::new(move |x: i32| {
        let current = running.fetch_add(1, Ordering::SeqCst) + 1;
        max_running.fetch_max(current, Ordering::SeqCst);

        std::thread::sleep(std::time::Duration::from_millis(50));

        running.fetch_sub(1, Ordering::SeqCst);
        Ok(format!("Completed {}", x))
    });

    let num_tasks: usize = 10;
    let max_concurrency = 3;

    let config = RunnableConfig::new().with_max_concurrency(max_concurrency);
    let results = runnable
        .abatch(
            (0..num_tasks as i32).collect(),
            Some(ConfigOrList::from(config)),
            false,
        )
        .await;

    assert_eq!(results.len(), num_tasks);
    for result in &results {
        assert!(result.is_ok());
    }
    assert!(max_running_tasks.load(Ordering::SeqCst) <= max_concurrency);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_abatch_as_completed_concurrency() {
    let running_tasks = Arc::new(AtomicUsize::new(0));
    let max_running_tasks = Arc::new(AtomicUsize::new(0));

    let running = running_tasks.clone();
    let max_running = max_running_tasks.clone();

    let runnable = RunnableLambda::new(move |x: i32| {
        let current = running.fetch_add(1, Ordering::SeqCst) + 1;
        max_running.fetch_max(current, Ordering::SeqCst);

        std::thread::sleep(std::time::Duration::from_millis(50));

        running.fetch_sub(1, Ordering::SeqCst);
        Ok(format!("Completed {}", x))
    });

    let num_tasks: usize = 10;
    let max_concurrency = 3;

    let config = RunnableConfig::new().with_max_concurrency(max_concurrency);
    let mut stream = runnable.abatch_as_completed(
        (0..num_tasks as i32).collect(),
        Some(ConfigOrList::from(config)),
        false,
    );

    let mut results = Vec::new();
    while let Some((_idx, result)) = stream.next().await {
        results.push(result);
    }

    assert_eq!(results.len(), num_tasks);
    for result in &results {
        assert!(result.is_ok());
    }
    assert!(max_running_tasks.load(Ordering::SeqCst) <= max_concurrency);
}

// ============================================================================
// Sync batch concurrency tests
// ============================================================================

#[test]
fn test_batch_concurrency() {
    let running_tasks = Arc::new(AtomicUsize::new(0));
    let max_running_tasks = Arc::new(AtomicUsize::new(0));

    let running = running_tasks.clone();
    let max_running = max_running_tasks.clone();

    let runnable = RunnableLambda::new(move |x: i32| {
        let current = running.fetch_add(1, Ordering::SeqCst) + 1;
        max_running.fetch_max(current, Ordering::SeqCst);

        std::thread::sleep(std::time::Duration::from_millis(50));

        running.fetch_sub(1, Ordering::SeqCst);
        Ok(format!("Completed {}", x))
    });

    let num_tasks: usize = 10;
    let max_concurrency = 3;

    let config = RunnableConfig::new().with_max_concurrency(max_concurrency);
    let results = runnable.batch(
        (0..num_tasks as i32).collect(),
        Some(ConfigOrList::from(config)),
        false,
    );

    assert_eq!(results.len(), num_tasks);
    for result in &results {
        assert!(result.is_ok());
    }
    assert!(max_running_tasks.load(Ordering::SeqCst) <= max_concurrency);
}

#[test]
fn test_batch_as_completed_concurrency() {
    let running_tasks = Arc::new(AtomicUsize::new(0));
    let max_running_tasks = Arc::new(AtomicUsize::new(0));

    let running = running_tasks.clone();
    let max_running = max_running_tasks.clone();

    let runnable = RunnableLambda::new(move |x: i32| {
        let current = running.fetch_add(1, Ordering::SeqCst) + 1;
        max_running.fetch_max(current, Ordering::SeqCst);

        std::thread::sleep(std::time::Duration::from_millis(50));

        running.fetch_sub(1, Ordering::SeqCst);
        Ok(format!("Completed {}", x))
    });

    let num_tasks: usize = 10;
    let max_concurrency = 3;

    let config = RunnableConfig::new().with_max_concurrency(max_concurrency);
    let results = runnable.batch_as_completed(
        (0..num_tasks as i32).collect(),
        Some(ConfigOrList::from(config)),
        false,
    );

    assert_eq!(results.len(), num_tasks);
    for (_idx, result) in &results {
        assert!(result.is_ok());
    }
    assert!(max_running_tasks.load(Ordering::SeqCst) <= max_concurrency);
}

// ============================================================================
// Extended concurrency tests
// ============================================================================

#[test]
fn test_batch_empty_input() {
    let runnable = RunnableLambda::new(|x: i32| Ok(x));
    let results = runnable.batch(vec![], None, false);
    assert!(results.is_empty());
}

#[tokio::test]
async fn test_abatch_empty_input() {
    let runnable = RunnableLambda::new(|x: i32| Ok(x));
    let results = runnable.abatch(vec![], None, false).await;
    assert!(results.is_empty());
}

#[test]
fn test_batch_single_item_no_threading() {
    let runnable = RunnableLambda::new(|x: i32| Ok(x * 2));
    let results = runnable.batch(vec![5], None, false);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].as_ref().unwrap(), &10);
}

#[test]
fn test_batch_preserves_order() {
    let runnable = RunnableLambda::new(|x: i32| {
        // Reverse sleep so later items finish first
        std::thread::sleep(std::time::Duration::from_millis((10 - x) as u64));
        Ok(x)
    });

    let inputs: Vec<i32> = (0..10).collect();
    let results = runnable.batch(inputs.clone(), None, false);

    let output: Vec<i32> = results.into_iter().map(|r| r.unwrap()).collect();
    assert_eq!(output, inputs);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_abatch_preserves_order() {
    let runnable = RunnableLambda::new(|x: i32| {
        std::thread::sleep(std::time::Duration::from_millis((10 - x) as u64));
        Ok(x)
    });

    let inputs: Vec<i32> = (0..10).collect();
    let results = runnable.abatch(inputs.clone(), None, false).await;

    let output: Vec<i32> = results.into_iter().map(|r| r.unwrap()).collect();
    assert_eq!(output, inputs);
}

#[test]
fn test_batch_with_return_exceptions() {
    let runnable = RunnableLambda::new(|x: i32| {
        if x % 2 == 0 {
            Err(Error::Other(format!("failed on {}", x)))
        } else {
            Ok(x)
        }
    });

    let results = runnable.batch(vec![1, 2, 3, 4], None, true);
    assert_eq!(results.len(), 4);
    assert_eq!(results[0].as_ref().unwrap(), &1);
    assert!(results[1].is_err());
    assert_eq!(results[2].as_ref().unwrap(), &3);
    assert!(results[3].is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_abatch_with_return_exceptions() {
    let runnable = RunnableLambda::new(|x: i32| {
        if x % 2 == 0 {
            Err(Error::Other(format!("failed on {}", x)))
        } else {
            Ok(x)
        }
    });

    let results = runnable.abatch(vec![1, 2, 3, 4], None, true).await;
    assert_eq!(results.len(), 4);
    assert_eq!(results[0].as_ref().unwrap(), &1);
    assert!(results[1].is_err());
    assert_eq!(results[2].as_ref().unwrap(), &3);
    assert!(results[3].is_err());
}

#[test]
fn test_batch_no_concurrency_limit() {
    let runnable = RunnableLambda::new(|x: i32| Ok(x + 1));
    let results = runnable.batch((0..20).collect(), None, false);

    let output: Vec<i32> = results.into_iter().map(|r| r.unwrap()).collect();
    assert_eq!(output, (1..21).collect::<Vec<i32>>());
}

#[test]
fn test_batch_concurrency_of_one() {
    let order = Arc::new(Mutex::new(Vec::new()));

    let order_clone = order.clone();
    let runnable = RunnableLambda::new(move |x: i32| {
        order_clone.lock().unwrap().push(x);
        std::thread::sleep(std::time::Duration::from_millis(10));
        Ok(x)
    });

    let config = RunnableConfig::new().with_max_concurrency(1);
    let results = runnable.batch((0..5).collect(), Some(ConfigOrList::from(config)), false);

    let output: Vec<i32> = results.into_iter().map(|r| r.unwrap()).collect();
    assert_eq!(output, (0..5).collect::<Vec<i32>>());
    // With concurrency of 1, order should be sequential
    assert_eq!(*order.lock().unwrap(), (0..5).collect::<Vec<i32>>());
}

#[test]
fn test_batch_as_completed_returns_all() {
    let runnable = RunnableLambda::new(|x: i32| {
        std::thread::sleep(std::time::Duration::from_millis((10 - x) as u64));
        Ok(x * 2)
    });

    let results = runnable.batch_as_completed((0..5).collect(), None, false);
    let collected: HashMap<usize, i32> =
        results.into_iter().map(|(i, r)| (i, r.unwrap())).collect();

    assert_eq!(collected.len(), 5);
    for i in 0..5 {
        assert_eq!(collected[&i], i as i32 * 2);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_abatch_as_completed_returns_all() {
    let runnable = RunnableLambda::new(|x: i32| {
        std::thread::sleep(std::time::Duration::from_millis((5 - x) as u64));
        Ok(x * 2)
    });

    let mut stream = runnable.abatch_as_completed((0..5).collect(), None, false);

    let mut collected: HashMap<usize, i32> = HashMap::new();
    while let Some((idx, result)) = stream.next().await {
        collected.insert(idx, result.unwrap());
    }

    assert_eq!(collected.len(), 5);
    for i in 0..5 {
        assert_eq!(collected[&i], i as i32 * 2);
    }
}
