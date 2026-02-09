//! Tests for RunnableBranch functionality.
//!
//! Converted from `langchain/libs/core/tests/unit_tests/runnables/test_branch.py`

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use futures::StreamExt;

use agent_chain_core::error::Error;
use agent_chain_core::runnables::base::{DynRunnable, Runnable, RunnableLambda, pipe};
use agent_chain_core::runnables::branch::{RunnableBranch, RunnableBranchBuilder};
use agent_chain_core::runnables::config::RunnableConfig;

// ============================================================================
// Initialization tests
// ============================================================================

#[test]
fn test_branch_initialization() {
    let branch = RunnableBranchBuilder::<i32, i32>::new()
        .branch(|x| Ok(x > 0), |x| Ok(x + 1))
        .branch(|x| Ok(x < 0), |x| Ok(x - 1))
        .default(Ok)
        .unwrap();

    assert_eq!(branch.name(), Some("RunnableBranch".to_string()));
}

#[test]
fn test_branch_requires_minimum_branches() {
    let result: agent_chain_core::error::Result<RunnableBranch<i32, i32>> =
        RunnableBranchBuilder::new().default(|x: i32| Ok(x));

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("at least one condition branch"),
        "Expected error about minimum branches, got: {err_msg}"
    );
}

#[test]
fn test_branch_requires_minimum_branches_new() {
    let result: agent_chain_core::error::Result<RunnableBranch<i32, i32>> =
        RunnableBranch::new(vec![], Arc::new(RunnableLambda::new(|x: i32| Ok(x))));

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("at least one condition branch"),
        "Expected error about minimum branches, got: {err_msg}"
    );
}

// ============================================================================
// Invoke tests
// ============================================================================

#[test]
fn test_branch_invoke_first_condition_true() {
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 0), |x: i32| Ok(x + 1))
        .branch(|x: i32| Ok(x < 0), |x: i32| Ok(x - 1))
        .default(|x: i32| Ok(x * 10))
        .unwrap();

    let result = branch.invoke(5, None).unwrap();
    assert_eq!(result, 6);
}

#[test]
fn test_branch_invoke_second_condition_true() {
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 10), |x: i32| Ok(x + 1))
        .branch(|x: i32| Ok(x > 0), |x: i32| Ok(x * 2))
        .default(|x: i32| Ok(x - 1))
        .unwrap();

    let result = branch.invoke(5, None).unwrap();
    assert_eq!(result, 10);
}

#[test]
fn test_branch_invoke_default() {
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 10), |x: i32| Ok(x + 1))
        .branch(|x: i32| Ok(x < 0), |x: i32| Ok(x - 1))
        .default(|x: i32| Ok(x * 100))
        .unwrap();

    let result = branch.invoke(5, None).unwrap();
    assert_eq!(result, 500);
}

// ============================================================================
// Async invoke tests
// ============================================================================

#[tokio::test]
async fn test_branch_ainvoke_first_condition() {
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 0), |x: i32| Ok(x + 1))
        .default(|x: i32| Ok(x * 10))
        .unwrap();

    let result = branch.ainvoke(5, None).await.unwrap();
    assert_eq!(result, 6);
}

#[tokio::test]
async fn test_branch_ainvoke_default() {
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 100), |x: i32| Ok(x + 1))
        .default(|x: i32| Ok(x * 10))
        .unwrap();

    let result = branch.ainvoke(5, None).await.unwrap();
    assert_eq!(result, 50);
}

// ============================================================================
// Batch tests
// ============================================================================

#[test]
fn test_branch_batch() {
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 5), |x: i32| Ok(x * 2))
        .branch(|x: i32| Ok(x > 0), |x: i32| Ok(x + 10))
        .default(|x: i32| Ok(x - 10))
        .unwrap();

    let results: Vec<i32> = branch
        .batch(vec![1, 3, 7, 10, -5], None, false)
        .into_iter()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(results, vec![11, 13, 14, 20, -15]);
}

#[tokio::test]
async fn test_branch_abatch() {
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 5), |x: i32| Ok(x * 2))
        .branch(|x: i32| Ok(x > 0), |x: i32| Ok(x + 10))
        .default(|x: i32| Ok(x - 10))
        .unwrap();

    let results: Vec<i32> = branch
        .abatch(vec![1, 3, 7, 10, -5], None, false)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(results, vec![11, 13, 14, 20, -15]);
}

#[test]
fn test_branch_empty_batch() {
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 0), |x: i32| Ok(x + 1))
        .default(|x: i32| Ok(x))
        .unwrap();

    let results = branch.batch(vec![], None, false);
    assert!(results.is_empty());
}

#[test]
fn test_branch_batch_different_routes() {
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 10), |_: i32| Ok("large".to_string()))
        .branch(|x: i32| Ok(x > 0), |_: i32| Ok("small".to_string()))
        .default(|_: i32| Ok("negative".to_string()))
        .unwrap();

    let results: Vec<String> = branch
        .batch(vec![15, 5, -3, 0, 20], None, false)
        .into_iter()
        .map(|r| r.unwrap())
        .collect();
    assert_eq!(
        results,
        vec!["large", "small", "negative", "negative", "large"]
    );
}

#[tokio::test]
async fn test_branch_batch_preserves_order() {
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 5), |x: i32| Ok(x * 2))
        .default(|x: i32| Ok(x + 1))
        .unwrap();

    let inputs = vec![1, 10, 3, 8, 2];
    let results: Vec<i32> = branch
        .abatch(inputs, None, false)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(results, vec![2, 20, 4, 16, 3]);
}

// ============================================================================
// Stream tests
// ============================================================================

#[tokio::test]
async fn test_branch_stream() {
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 0), |x: i32| Ok(x + 1))
        .default(|x: i32| Ok(x - 1))
        .unwrap();

    let results: Vec<i32> = branch.stream(5, None).map(|r| r.unwrap()).collect().await;
    assert_eq!(results, vec![6]);
}

#[tokio::test]
async fn test_branch_astream() {
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 0), |x: i32| Ok(x + 1))
        .default(|x: i32| Ok(x - 1))
        .unwrap();

    let results: Vec<i32> = branch.astream(5, None).map(|r| r.unwrap()).collect().await;
    assert_eq!(results, vec![6]);
}

// ============================================================================
// Tests with Runnable objects (Arc'd)
// ============================================================================

#[test]
fn test_branch_with_runnable_objects() {
    let condition: DynRunnable<i32, bool> = Arc::new(RunnableLambda::new(|x: i32| Ok(x > 0)));
    let action_true: DynRunnable<i32, i32> = Arc::new(RunnableLambda::new(|x: i32| Ok(x + 1)));
    let action_false: DynRunnable<i32, i32> = Arc::new(RunnableLambda::new(|x: i32| Ok(x - 1)));

    let branch = RunnableBranch::new(vec![(condition, action_true)], action_false).unwrap();

    assert_eq!(branch.invoke(5, None).unwrap(), 6);
    assert_eq!(branch.invoke(-5, None).unwrap(), -6);
}

// ============================================================================
// Multiple conditions test
// ============================================================================

#[test]
fn test_branch_multiple_conditions() {
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 100), |_: i32| Ok("very large".to_string()))
        .branch(|x: i32| Ok(x > 50), |_: i32| Ok("large".to_string()))
        .branch(|x: i32| Ok(x > 10), |_: i32| Ok("medium".to_string()))
        .branch(|x: i32| Ok(x > 0), |_: i32| Ok("small".to_string()))
        .default(|_: i32| Ok("negative or zero".to_string()))
        .unwrap();

    assert_eq!(branch.invoke(150, None).unwrap(), "very large");
    assert_eq!(branch.invoke(75, None).unwrap(), "large");
    assert_eq!(branch.invoke(25, None).unwrap(), "medium");
    assert_eq!(branch.invoke(5, None).unwrap(), "small");
    assert_eq!(branch.invoke(-5, None).unwrap(), "negative or zero");
    assert_eq!(branch.invoke(0, None).unwrap(), "negative or zero");
}

// ============================================================================
// Dict-like input tests (using HashMap)
// ============================================================================

#[test]
fn test_branch_with_hashmap_input() {
    use std::collections::HashMap;

    type Input = HashMap<String, String>;

    let branch = RunnableBranchBuilder::new()
        .branch(
            |x: Input| Ok(x.get("type").is_some_and(|v| v == "add")),
            |x: Input| {
                let a: i32 = x.get("a").unwrap().parse().unwrap();
                let b: i32 = x.get("b").unwrap().parse().unwrap();
                Ok(format!("{}", a + b))
            },
        )
        .branch(
            |x: Input| Ok(x.get("type").is_some_and(|v| v == "multiply")),
            |x: Input| {
                let a: i32 = x.get("a").unwrap().parse().unwrap();
                let b: i32 = x.get("b").unwrap().parse().unwrap();
                Ok(format!("{}", a * b))
            },
        )
        .default(|_: Input| Ok("0".to_string()))
        .unwrap();

    let mut add_input = HashMap::new();
    add_input.insert("type".to_string(), "add".to_string());
    add_input.insert("a".to_string(), "5".to_string());
    add_input.insert("b".to_string(), "3".to_string());
    assert_eq!(branch.invoke(add_input, None).unwrap(), "8");

    let mut mul_input = HashMap::new();
    mul_input.insert("type".to_string(), "multiply".to_string());
    mul_input.insert("a".to_string(), "5".to_string());
    mul_input.insert("b".to_string(), "3".to_string());
    assert_eq!(branch.invoke(mul_input, None).unwrap(), "15");

    let mut unknown_input = HashMap::new();
    unknown_input.insert("type".to_string(), "unknown".to_string());
    unknown_input.insert("a".to_string(), "5".to_string());
    unknown_input.insert("b".to_string(), "3".to_string());
    assert_eq!(branch.invoke(unknown_input, None).unwrap(), "0");
}

// ============================================================================
// Error propagation tests
// ============================================================================

#[test]
fn test_branch_exception_in_condition() {
    let branch = RunnableBranchBuilder::new()
        .branch(
            |_: i32| Err(Error::Other("Condition failed".to_string())),
            |x: i32| Ok(x + 1),
        )
        .default(|x: i32| Ok(x))
        .unwrap();

    let result = branch.invoke(5, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Condition failed"));
}

#[test]
fn test_branch_exception_in_action() {
    let branch = RunnableBranchBuilder::new()
        .branch(
            |x: i32| Ok(x > 0),
            |_: i32| Err(Error::Other("Action failed".to_string())),
        )
        .default(|x: i32| Ok(x))
        .unwrap();

    let result = branch.invoke(5, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Action failed"));
}

#[tokio::test]
async fn test_branch_exception_in_async_action() {
    let branch = RunnableBranchBuilder::new()
        .branch(
            |x: i32| Ok(x > 0),
            |_: i32| Err(Error::Other("Action failed".to_string())),
        )
        .default(|x: i32| Ok(x))
        .unwrap();

    let result = branch.ainvoke(5, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Action failed"));
}

// ============================================================================
// Condition evaluation order tests
// ============================================================================

#[test]
fn test_branch_conditions_evaluated_in_order() {
    let evaluations = Arc::new(Mutex::new(Vec::<usize>::new()));

    let eval1 = evaluations.clone();
    let eval2 = evaluations.clone();
    let eval3 = evaluations.clone();

    let condition1 = Arc::new(RunnableLambda::new(move |x: i32| {
        eval1.lock().unwrap().push(1);
        Ok(x > 10)
    })) as DynRunnable<i32, bool>;

    let condition2 = Arc::new(RunnableLambda::new(move |x: i32| {
        eval2.lock().unwrap().push(2);
        Ok(x > 5)
    })) as DynRunnable<i32, bool>;

    let condition3 = Arc::new(RunnableLambda::new(move |x: i32| {
        eval3.lock().unwrap().push(3);
        Ok(x > 0)
    })) as DynRunnable<i32, bool>;

    let action1: DynRunnable<i32, String> =
        Arc::new(RunnableLambda::new(|_: i32| Ok("first".to_string())));
    let action2: DynRunnable<i32, String> =
        Arc::new(RunnableLambda::new(|_: i32| Ok("second".to_string())));
    let action3: DynRunnable<i32, String> =
        Arc::new(RunnableLambda::new(|_: i32| Ok("third".to_string())));
    let default: DynRunnable<i32, String> =
        Arc::new(RunnableLambda::new(|_: i32| Ok("default".to_string())));

    let branch = RunnableBranch::new(
        vec![
            (condition1, action1),
            (condition2, action2),
            (condition3, action3),
        ],
        default,
    )
    .unwrap();

    // Input > 10: only first condition should be evaluated
    evaluations.lock().unwrap().clear();
    let result = branch.invoke(15, None).unwrap();
    assert_eq!(result, "first");
    assert_eq!(*evaluations.lock().unwrap(), vec![1]);

    // Input between 5 and 10: first two conditions evaluated
    evaluations.lock().unwrap().clear();
    let result = branch.invoke(7, None).unwrap();
    assert_eq!(result, "second");
    assert_eq!(*evaluations.lock().unwrap(), vec![1, 2]);

    // Input between 0 and 5: all three conditions evaluated
    evaluations.lock().unwrap().clear();
    let result = branch.invoke(3, None).unwrap();
    assert_eq!(result, "third");
    assert_eq!(*evaluations.lock().unwrap(), vec![1, 2, 3]);

    // Input <= 0: all conditions evaluated, use default
    evaluations.lock().unwrap().clear();
    let result = branch.invoke(-5, None).unwrap();
    assert_eq!(result, "default");
    assert_eq!(*evaluations.lock().unwrap(), vec![1, 2, 3]);
}

#[test]
fn test_branch_short_circuit_evaluation() {
    let condition_calls = Arc::new(Mutex::new(Vec::<String>::new()));

    let calls1 = condition_calls.clone();
    let calls2 = condition_calls.clone();
    let calls3 = condition_calls.clone();

    let condition1: DynRunnable<i32, bool> = Arc::new(RunnableLambda::new(move |x: i32| {
        calls1.lock().unwrap().push("first".to_string());
        Ok(x > 0)
    }));

    let condition2: DynRunnable<i32, bool> = Arc::new(RunnableLambda::new(move |x: i32| {
        calls2.lock().unwrap().push("second".to_string());
        Ok(x > 0)
    }));

    let condition3: DynRunnable<i32, bool> = Arc::new(RunnableLambda::new(move |x: i32| {
        calls3.lock().unwrap().push("third".to_string());
        Ok(x > 0)
    }));

    let action1: DynRunnable<i32, i32> = Arc::new(RunnableLambda::new(|x: i32| Ok(x + 1)));
    let action2: DynRunnable<i32, i32> = Arc::new(RunnableLambda::new(|x: i32| Ok(x + 2)));
    let action3: DynRunnable<i32, i32> = Arc::new(RunnableLambda::new(|x: i32| Ok(x + 3)));
    let default: DynRunnable<i32, i32> = Arc::new(RunnableLambda::new(|x: i32| Ok(x)));

    let branch = RunnableBranch::new(
        vec![
            (condition1, action1),
            (condition2, action2),
            (condition3, action3),
        ],
        default,
    )
    .unwrap();

    condition_calls.lock().unwrap().clear();
    let result = branch.invoke(5, None).unwrap();
    assert_eq!(result, 6);
    // Should only evaluate first condition
    assert_eq!(*condition_calls.lock().unwrap(), vec!["first"]);
}

// ============================================================================
// Complex condition tests
// ============================================================================

#[test]
fn test_branch_with_complex_conditions() {
    let branch = RunnableBranchBuilder::new()
        .branch(
            |x: i32| Ok(x > 0 && x % 2 == 0),
            |x: i32| Ok(format!("even: {x}")),
        )
        .branch(
            |x: i32| Ok(x > 0 && x % 2 == 1),
            |x: i32| Ok(format!("odd: {x}")),
        )
        .default(|x: i32| Ok(format!("non-positive: {x}")))
        .unwrap();

    assert_eq!(branch.invoke(4, None).unwrap(), "even: 4");
    assert_eq!(branch.invoke(5, None).unwrap(), "odd: 5");
    assert_eq!(branch.invoke(0, None).unwrap(), "non-positive: 0");
    assert_eq!(branch.invoke(-3, None).unwrap(), "non-positive: -3");
}

// ============================================================================
// Config propagation tests
// ============================================================================

#[test]
fn test_branch_config_propagation() {
    let condition: DynRunnable<i32, bool> = Arc::new(RunnableLambda::new(|x: i32| Ok(x > 0)));
    let action: DynRunnable<i32, i32> = Arc::new(RunnableLambda::new(|x: i32| Ok(x + 1)));
    let default: DynRunnable<i32, i32> = Arc::new(RunnableLambda::new(|x: i32| Ok(x)));

    let branch = RunnableBranch::new(vec![(condition, action)], default).unwrap();

    let config = RunnableConfig::new().with_tags(vec!["my-tag".to_string()]);
    let result = branch.invoke(5, Some(config)).unwrap();
    assert_eq!(result, 6);
}

// ============================================================================
// Serialization tests
// ============================================================================

#[test]
fn test_branch_serialization() {
    use agent_chain_core::load::Serializable;

    assert!(RunnableBranch::<i32, i32>::is_lc_serializable());
    assert_eq!(
        RunnableBranch::<i32, i32>::get_lc_namespace(),
        vec!["langchain", "schema", "runnable"]
    );
}

// ============================================================================
// All conditions false tests
// ============================================================================

#[test]
fn test_branch_all_conditions_false_uses_default() {
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 100), |_: i32| Ok("a".to_string()))
        .branch(|x: i32| Ok(x > 50), |_: i32| Ok("b".to_string()))
        .branch(|x: i32| Ok(x > 25), |_: i32| Ok("c".to_string()))
        .default(|_: i32| Ok("default".to_string()))
        .unwrap();

    let result = branch.invoke(10, None).unwrap();
    assert_eq!(result, "default");
}

// ============================================================================
// Type preservation tests
// ============================================================================

#[test]
fn test_branch_preserves_intermediate_types() {
    let branch = RunnableBranchBuilder::new()
        .branch(
            |x: String| Ok(x.len() > 5),
            |x: String| Ok(x.to_uppercase()),
        )
        .default(|x: String| Ok(x.to_lowercase()))
        .unwrap();

    assert_eq!(
        branch.invoke("hello world".to_string(), None).unwrap(),
        "HELLO WORLD"
    );
    assert_eq!(branch.invoke("hi".to_string(), None).unwrap(), "hi");
}

#[test]
fn test_branch_with_complex_return_types() {
    let branch = RunnableBranchBuilder::new()
        .branch(
            |x: (String, i32)| Ok(x.0 == "list"),
            |x: (String, i32)| Ok(vec![x.1, x.1 * 2]),
        )
        .default(|x: (String, i32)| Ok(vec![x.1]))
        .unwrap();

    assert_eq!(
        branch.invoke(("list".to_string(), 5), None).unwrap(),
        vec![5, 10]
    );
    assert_eq!(
        branch.invoke(("other".to_string(), 5), None).unwrap(),
        vec![5]
    );
}

// ============================================================================
// Type annotation tests (Rust equivalent: explicit generic types)
// ============================================================================

#[test]
fn test_branch_with_type_annotations() {
    fn condition_typed(x: i32) -> agent_chain_core::error::Result<bool> {
        Ok(x > 0)
    }

    fn action_typed(x: i32) -> agent_chain_core::error::Result<String> {
        Ok(format!("positive: {x}"))
    }

    fn default_typed(x: i32) -> agent_chain_core::error::Result<String> {
        Ok(format!("non-positive: {x}"))
    }

    let branch = RunnableBranchBuilder::<i32, String>::new()
        .branch(condition_typed, action_typed)
        .default(default_typed)
        .unwrap();

    assert_eq!(branch.invoke(5, None).unwrap(), "positive: 5");
    assert_eq!(branch.invoke(-5, None).unwrap(), "non-positive: -5");
}

// ============================================================================
// Chain composition tests
// ============================================================================

#[test]
fn test_branch_with_runnables_in_chain() {
    let preprocess = RunnableLambda::new(|x: (i32,)| Ok(x.0));

    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 10), |x: i32| Ok(x * 2))
        .branch(|x: i32| Ok(x > 0), |x: i32| Ok(x + 10))
        .default(|x: i32| Ok(x - 10))
        .unwrap();

    let postprocess = RunnableLambda::new(|x: i32| Ok(format!("Result: {x}")));

    let chain = pipe(pipe(preprocess, branch), postprocess);

    assert_eq!(chain.invoke((15,), None).unwrap(), "Result: 30");
    assert_eq!(chain.invoke((5,), None).unwrap(), "Result: 15");
    assert_eq!(chain.invoke((-5,), None).unwrap(), "Result: -15");
}

// ============================================================================
// None/Option output tests
// ============================================================================

#[test]
fn test_branch_with_none_output() {
    let branch = RunnableBranchBuilder::new()
        .branch(
            |x: String| Ok(x == "return_none"),
            |_: String| Ok(None::<String>),
        )
        .default(|x: String| Ok(Some(x)))
        .unwrap();

    let result = branch.invoke("return_none".to_string(), None).unwrap();
    assert!(result.is_none());

    let result = branch.invoke("other".to_string(), None).unwrap();
    assert_eq!(result, Some("other".to_string()));
}

// ============================================================================
// Naming tests
// ============================================================================

#[test]
fn test_branch_name() {
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 0), |x: i32| Ok(x.to_string()))
        .default(|_: i32| Ok("default".to_string()))
        .unwrap()
        .with_name("my_branch");

    assert_eq!(branch.name(), Some("my_branch".to_string()));
}

#[test]
fn test_branch_default_name() {
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 0), |x: i32| Ok(x.to_string()))
        .default(|_: i32| Ok("default".to_string()))
        .unwrap();

    assert_eq!(branch.name(), Some("RunnableBranch".to_string()));
}

// ============================================================================
// Stream error propagation tests
// ============================================================================

#[tokio::test]
async fn test_branch_stream_condition_error() {
    let branch = RunnableBranchBuilder::new()
        .branch(
            |_: i32| Err(Error::Other("stream condition failed".to_string())),
            |x: i32| Ok(x + 1),
        )
        .default(|x: i32| Ok(x))
        .unwrap();

    let results: Vec<agent_chain_core::error::Result<i32>> = branch.stream(5, None).collect().await;

    assert_eq!(results.len(), 1);
    assert!(results[0].is_err());
    assert!(
        results[0]
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("stream condition failed")
    );
}

#[tokio::test]
async fn test_branch_astream_condition_error() {
    let branch = RunnableBranchBuilder::new()
        .branch(
            |_: i32| Err(Error::Other("astream condition failed".to_string())),
            |x: i32| Ok(x + 1),
        )
        .default(|x: i32| Ok(x))
        .unwrap();

    let results: Vec<agent_chain_core::error::Result<i32>> =
        branch.astream(5, None).collect().await;

    assert_eq!(results.len(), 1);
    assert!(results[0].is_err());
    assert!(
        results[0]
            .as_ref()
            .unwrap_err()
            .to_string()
            .contains("astream condition failed")
    );
}

// ============================================================================
// Stream with default path tests
// ============================================================================

#[tokio::test]
async fn test_branch_stream_default_path() {
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 100), |x: i32| Ok(x + 1))
        .default(|x: i32| Ok(x * 10))
        .unwrap();

    let results: Vec<i32> = branch.stream(5, None).map(|r| r.unwrap()).collect().await;
    assert_eq!(results, vec![50]);
}

#[tokio::test]
async fn test_branch_astream_default_path() {
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 100), |x: i32| Ok(x + 1))
        .default(|x: i32| Ok(x * 10))
        .unwrap();

    let results: Vec<i32> = branch.astream(5, None).map(|r| r.unwrap()).collect().await;
    assert_eq!(results, vec![50]);
}

// ============================================================================
// Coercion tests (builder automatically wraps closures into RunnableLambda)
// ============================================================================

#[test]
fn test_branch_coerces_conditions_and_actions() {
    // The builder wraps closures in RunnableLambda automatically
    // This test verifies the builder API works with plain closures
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 0), |x: i32| Ok(x + 1))
        .default(|x: i32| Ok(x - 1))
        .unwrap();

    // If it compiled and works, closures were successfully coerced
    assert_eq!(branch.invoke(5, None).unwrap(), 6);
    assert_eq!(branch.invoke(-5, None).unwrap(), -6);
}

// ============================================================================
// Concurrent execution test
// ============================================================================

#[tokio::test]
async fn test_branch_ainvoke_multiple_sequential() {
    let call_count = Arc::new(AtomicUsize::new(0));

    let count = call_count.clone();
    let condition: DynRunnable<i32, bool> = Arc::new(RunnableLambda::new(move |x: i32| {
        count.fetch_add(1, Ordering::SeqCst);
        Ok(x > 0)
    }));

    let action: DynRunnable<i32, i32> = Arc::new(RunnableLambda::new(|x: i32| Ok(x + 1)));
    let default: DynRunnable<i32, i32> = Arc::new(RunnableLambda::new(|x: i32| Ok(x - 1)));

    let branch = RunnableBranch::new(vec![(condition, action)], default).unwrap();

    let result1 = branch.ainvoke(5, None).await.unwrap();
    let result2 = branch.ainvoke(-5, None).await.unwrap();

    assert_eq!(result1, 6);
    assert_eq!(result2, -6);
    assert_eq!(call_count.load(Ordering::SeqCst), 2);
}

// ============================================================================
// Builder pattern tests
// ============================================================================

#[test]
fn test_branch_builder_branch_arc() {
    let condition: DynRunnable<i32, bool> = Arc::new(RunnableLambda::new(|x: i32| Ok(x > 10)));
    let action: DynRunnable<i32, String> =
        Arc::new(RunnableLambda::new(|x: i32| Ok(format!("big: {x}"))));
    let default: DynRunnable<i32, String> =
        Arc::new(RunnableLambda::new(|x: i32| Ok(format!("small: {x}"))));

    let branch = RunnableBranchBuilder::new()
        .branch_arc(condition, action)
        .default_arc(default)
        .unwrap();

    assert_eq!(branch.invoke(15, None).unwrap(), "big: 15");
    assert_eq!(branch.invoke(5, None).unwrap(), "small: 5");
}

// ============================================================================
// Truthy/falsy condition tests (Rust equivalent: conditions always return bool)
// ============================================================================

#[test]
fn test_branch_condition_edge_values() {
    // In Rust, conditions always return bool, but we can test boolean edge cases
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x != 0), |_: i32| Ok("truthy".to_string()))
        .default(|_: i32| Ok("falsy".to_string()))
        .unwrap();

    assert_eq!(branch.invoke(1, None).unwrap(), "truthy");
    assert_eq!(branch.invoke(5, None).unwrap(), "truthy");
    assert_eq!(branch.invoke(0, None).unwrap(), "falsy");
}

// ============================================================================
// Callbacks test
// ============================================================================

#[test]
fn test_branch_with_callbacks() {
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 0), |x: i32| Ok(x + 1))
        .default(|x: i32| Ok(x - 1))
        .unwrap();

    // Should not panic, even with a config containing tags for tracing
    let config = RunnableConfig::new().with_tags(vec!["test-tag".to_string()]);
    let result = branch.invoke(5, Some(config)).unwrap();
    assert_eq!(result, 6);
}

// ============================================================================
// Mixed sync/async test
// ============================================================================

#[tokio::test]
async fn test_branch_mixed_sync_async() {
    // In Rust, all closures are sync but ainvoke provides async execution
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x > 0), |x: i32| Ok(x + 1))
        .default(|x: i32| Ok(x - 1))
        .unwrap();

    // Should work via ainvoke
    let result = branch.ainvoke(5, None).await.unwrap();
    assert_eq!(result, 6);

    let result2 = branch.ainvoke(-5, None).await.unwrap();
    assert_eq!(result2, -6);
}

// ============================================================================
// Large number of branches test
// ============================================================================

#[test]
fn test_branch_many_branches() {
    let branch = RunnableBranchBuilder::new()
        .branch(|x: i32| Ok(x == 1), |_: i32| Ok("one".to_string()))
        .branch(|x: i32| Ok(x == 2), |_: i32| Ok("two".to_string()))
        .branch(|x: i32| Ok(x == 3), |_: i32| Ok("three".to_string()))
        .branch(|x: i32| Ok(x == 4), |_: i32| Ok("four".to_string()))
        .branch(|x: i32| Ok(x == 5), |_: i32| Ok("five".to_string()))
        .branch(|x: i32| Ok(x == 6), |_: i32| Ok("six".to_string()))
        .branch(|x: i32| Ok(x == 7), |_: i32| Ok("seven".to_string()))
        .branch(|x: i32| Ok(x == 8), |_: i32| Ok("eight".to_string()))
        .branch(|x: i32| Ok(x == 9), |_: i32| Ok("nine".to_string()))
        .branch(|x: i32| Ok(x == 10), |_: i32| Ok("ten".to_string()))
        .default(|_: i32| Ok("other".to_string()))
        .unwrap();

    assert_eq!(branch.invoke(1, None).unwrap(), "one");
    assert_eq!(branch.invoke(5, None).unwrap(), "five");
    assert_eq!(branch.invoke(10, None).unwrap(), "ten");
    assert_eq!(branch.invoke(11, None).unwrap(), "other");
    assert_eq!(branch.invoke(0, None).unwrap(), "other");
}

// ============================================================================
// Stream from multiple branches test
// ============================================================================

#[tokio::test]
async fn test_branch_stream_routes_to_correct_branch() {
    let branch = RunnableBranchBuilder::new()
        .branch(
            |x: String| Ok(x == "a"),
            |_: String| Ok("response one".to_string()),
        )
        .default(|_: String| Ok("response two".to_string()))
        .unwrap();

    // Route to first branch
    let result1: Vec<String> = branch
        .stream("a".to_string(), None)
        .map(|r| r.unwrap())
        .collect()
        .await;
    assert_eq!(result1.join(""), "response one");

    // Route to default
    let result2: Vec<String> = branch
        .stream("b".to_string(), None)
        .map(|r| r.unwrap())
        .collect()
        .await;
    assert_eq!(result2.join(""), "response two");
}

#[tokio::test]
async fn test_branch_astream_routes_to_correct_branch() {
    let branch = RunnableBranchBuilder::new()
        .branch(
            |x: String| Ok(x == "a"),
            |_: String| Ok("response one".to_string()),
        )
        .default(|_: String| Ok("response two".to_string()))
        .unwrap();

    let result1: Vec<String> = branch
        .astream("a".to_string(), None)
        .map(|r| r.unwrap())
        .collect()
        .await;
    assert_eq!(result1.join(""), "response one");

    let result2: Vec<String> = branch
        .astream("b".to_string(), None)
        .map(|r| r.unwrap())
        .collect()
        .await;
    assert_eq!(result2.join(""), "response two");
}
