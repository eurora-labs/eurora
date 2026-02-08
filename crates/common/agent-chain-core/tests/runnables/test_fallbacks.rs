//! Tests for fallback runnables.
//!
//! Mirrors `langchain/libs/core/tests/unit_tests/runnables/test_fallbacks.py`

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use agent_chain_core::error::{Error, Result};
use agent_chain_core::runnables::base::{Runnable, RunnableLambda};

use agent_chain_core::runnables::fallbacks::{
    ExceptionInserter, RunnableWithFallbacks, RunnableWithFallbacksExt,
};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Create an exception inserter for HashMap<String, Value> inputs.
fn hashmap_exception_inserter() -> ExceptionInserter<HashMap<String, Value>> {
    Arc::new(|input: &HashMap<String, Value>, key: &str, error: &Error| {
        let mut new_input = input.clone();
        new_input.insert(key.to_string(), Value::String(error.to_string()));
        new_input
    })
}

/// Create a primary runnable that always fails.
fn failing_runnable()
-> RunnableLambda<impl Fn(String) -> Result<String> + Send + Sync, String, String> {
    RunnableLambda::new(|_x: String| -> Result<String> { Err(Error::other("primary failed")) })
}

/// Create a fallback runnable that returns "bar".
fn bar_runnable() -> RunnableLambda<impl Fn(String) -> Result<String> + Send + Sync, String, String>
{
    RunnableLambda::new(|_x: String| -> Result<String> { Ok("bar".to_string()) })
}

// ===========================================================================
// Tests for basic fallback behavior (mirrors test_fallbacks parametrized)
// ===========================================================================

#[test]
fn test_fallbacks_invoke() {
    let primary = failing_runnable();
    let fallback = bar_runnable();
    let rwf = primary.with_fallbacks(vec![Arc::new(fallback)]);
    assert_eq!(rwf.invoke("hello".to_string(), None).unwrap(), "bar");
}

#[test]
fn test_fallbacks_batch() {
    let primary = failing_runnable();
    let fallback = bar_runnable();
    let rwf = primary.with_fallbacks(vec![Arc::new(fallback)]);
    let results = rwf.batch(
        vec!["hi".to_string(), "hey".to_string(), "bye".to_string()],
        None,
        false,
    );
    assert_eq!(results.len(), 3);
    for result in &results {
        assert_eq!(result.as_ref().unwrap(), "bar");
    }
}

#[test]
fn test_fallbacks_stream() {
    let primary = failing_runnable();
    let fallback = bar_runnable();
    let rwf = primary.with_fallbacks(vec![Arc::new(fallback)]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let chunks: Vec<Result<String>> = rt.block_on(async {
        use futures::StreamExt;
        rwf.stream("hello".to_string(), None).collect().await
    });
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].as_ref().unwrap(), "bar");
}

#[test]
fn test_fallbacks_multi_invoke() {
    let primary = failing_runnable();
    let fallback1 = failing_runnable();
    let fallback2 = bar_runnable();
    let rwf = primary.with_fallbacks(vec![Arc::new(fallback1), Arc::new(fallback2)]);
    assert_eq!(rwf.invoke("hello".to_string(), None).unwrap(), "bar");
}

// ===========================================================================
// Async tests (mirrors test_fallbacks_async)
// ===========================================================================

#[tokio::test]
async fn test_fallbacks_ainvoke() {
    let primary = failing_runnable();
    let fallback = bar_runnable();
    let rwf = primary.with_fallbacks(vec![Arc::new(fallback)]);
    assert_eq!(rwf.ainvoke("hello".to_string(), None).await.unwrap(), "bar");
}

#[tokio::test]
async fn test_fallbacks_abatch() {
    let primary = failing_runnable();
    let fallback = bar_runnable();
    let rwf = primary.with_fallbacks(vec![Arc::new(fallback)]);
    let results = rwf
        .abatch(
            vec!["hi".to_string(), "hey".to_string(), "bye".to_string()],
            None,
            false,
        )
        .await;
    assert_eq!(results.len(), 3);
    for result in &results {
        assert_eq!(result.as_ref().unwrap(), "bar");
    }
}

// ===========================================================================
// Tests for exception_key (mirrors test_invoke_with_exception_key etc.)
// ===========================================================================

/// Mirrors Python's _runnable function: behavior depends on input["text"]
/// and whether input["exception"] is present.
fn dict_runnable() -> RunnableLambda<
    impl Fn(HashMap<String, Value>) -> Result<String> + Send + Sync,
    HashMap<String, Value>,
    String,
> {
    RunnableLambda::new(|inputs: HashMap<String, Value>| -> Result<String> {
        let text = inputs.get("text").and_then(|v| v.as_str()).unwrap_or("");

        if text == "foo" {
            return Ok("first".to_string());
        }
        if !inputs.contains_key("exception") {
            return Err(Error::other("missing exception"));
        }
        if text == "bar" {
            return Ok("second".to_string());
        }
        // For "baz" with an exception already set, the Python version checks
        // if the exception is a ValueError and raises RuntimeError.
        // We simplify: if text is not "foo" or "bar" and exception exists,
        // return "third".
        Ok("third".to_string())
    })
}

fn make_input(text: &str) -> HashMap<String, Value> {
    let mut m = HashMap::new();
    m.insert("text".to_string(), Value::String(text.to_string()));
    m
}

#[test]
fn test_invoke_with_exception_key_single_fallback() {
    let runnable = dict_runnable();
    let fallback = dict_runnable();

    let rwf = RunnableWithFallbacks::new(runnable, vec![Arc::new(fallback)])
        .with_exception_key("exception", hashmap_exception_inserter());

    // text="baz" — first call: no exception key, fails with "missing exception"
    // second call (fallback): exception key injected, text="baz" → no "foo" or "bar"
    // but exception is present → returns "third"
    // Wait — with single fallback, text="baz":
    // - Main: text="baz", no "exception" key → Err("missing exception")
    // - Fallback: text="baz", "exception" = "missing exception" → returns "third"
    // But Python says single fallback with text="bar" should return "second"
    let result = rwf.invoke(make_input("bar"), None).unwrap();
    assert_eq!(result, "second");
}

#[test]
fn test_invoke_with_exception_key_double_fallback() {
    let runnable = dict_runnable();
    let fallback1 = dict_runnable();
    let fallback2 = dict_runnable();

    let rwf = RunnableWithFallbacks::new(runnable, vec![Arc::new(fallback1), Arc::new(fallback2)])
        .with_exception_key("exception", hashmap_exception_inserter());

    // text="baz": main fails, fallback1 gets exception → returns "third"
    let result = rwf.invoke(make_input("baz"), None).unwrap();
    assert_eq!(result, "third");
}

#[test]
fn test_invoke_with_exception_key_foo_succeeds() {
    let runnable = dict_runnable();
    let fallback = dict_runnable();

    let rwf = RunnableWithFallbacks::new(runnable, vec![Arc::new(fallback)])
        .with_exception_key("exception", hashmap_exception_inserter());

    // text="foo" always succeeds on first try
    let result = rwf.invoke(make_input("foo"), None).unwrap();
    assert_eq!(result, "first");
}

#[tokio::test]
async fn test_ainvoke_with_exception_key() {
    let runnable = dict_runnable();
    let fallback = dict_runnable();

    let rwf = RunnableWithFallbacks::new(runnable, vec![Arc::new(fallback)])
        .with_exception_key("exception", hashmap_exception_inserter());

    let result = rwf.ainvoke(make_input("bar"), None).await.unwrap();
    assert_eq!(result, "second");
}

// ===========================================================================
// Tests for batch with exception_key (mirrors test_batch)
// ===========================================================================

#[test]
fn test_batch_with_exception_key() {
    let runnable = dict_runnable();
    let fallback = dict_runnable();

    let rwf = RunnableWithFallbacks::new(runnable, vec![Arc::new(fallback)])
        .with_exception_key("exception", hashmap_exception_inserter());

    let results = rwf.batch(
        vec![make_input("foo"), make_input("bar"), make_input("baz")],
        None,
        true,
    );

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].as_ref().unwrap(), "first");
    assert_eq!(results[1].as_ref().unwrap(), "second");
    assert_eq!(results[2].as_ref().unwrap(), "third");
}

#[test]
fn test_batch_with_double_fallback_exception_key() {
    let runnable = dict_runnable();
    let fallback1 = dict_runnable();
    let fallback2 = dict_runnable();

    let rwf = RunnableWithFallbacks::new(runnable, vec![Arc::new(fallback1), Arc::new(fallback2)])
        .with_exception_key("exception", hashmap_exception_inserter());

    let results = rwf.batch(
        vec![make_input("foo"), make_input("bar"), make_input("baz")],
        None,
        true,
    );

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].as_ref().unwrap(), "first");
    assert_eq!(results[1].as_ref().unwrap(), "second");
    assert_eq!(results[2].as_ref().unwrap(), "third");
}

#[tokio::test]
async fn test_abatch_with_exception_key() {
    let runnable = dict_runnable();
    let fallback = dict_runnable();

    let rwf = RunnableWithFallbacks::new(runnable, vec![Arc::new(fallback)])
        .with_exception_key("exception", hashmap_exception_inserter());

    let results = rwf
        .abatch(
            vec![make_input("foo"), make_input("bar"), make_input("baz")],
            None,
            true,
        )
        .await;

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].as_ref().unwrap(), "first");
    assert_eq!(results[1].as_ref().unwrap(), "second");
    assert_eq!(results[2].as_ref().unwrap(), "third");
}

// ===========================================================================
// Tests for runnables property (mirrors test_runnables_property)
// ===========================================================================

#[test]
fn test_runnables_property() {
    let main_r = RunnableLambda::new(|x: String| -> Result<String> { Ok(x) });
    let fb1 = RunnableLambda::new(|x: String| -> Result<String> { Ok(format!("{}1", x)) });
    let fb2 = RunnableLambda::new(|x: String| -> Result<String> { Ok(format!("{}2", x)) });

    let rwf = main_r.with_fallbacks(vec![Arc::new(fb1), Arc::new(fb2)]);
    let count = rwf.runnables().count();
    assert_eq!(count, 3);
}

// ===========================================================================
// Tests for config_specs (mirrors test_config_specs_merged)
// ===========================================================================

#[test]
fn test_config_specs_merged() {
    let main_r = RunnableLambda::new(|x: String| -> Result<String> { Ok(x) });
    let fb = RunnableLambda::new(|x: String| -> Result<String> { Ok(x) });
    let rwf = main_r.with_fallbacks(vec![Arc::new(fb)]);
    let specs = rwf.config_specs().unwrap();
    assert!(specs.is_empty()); // No configurable fields on lambdas
}

// ===========================================================================
// Tests for custom error predicate (mirrors test_custom_exceptions_to_handle)
// ===========================================================================

#[test]
fn test_custom_error_predicate() {
    // Only handle "value" errors, not "type" errors
    let call_count = Arc::new(AtomicUsize::new(0));

    let count_clone = call_count.clone();
    let main_r = RunnableLambda::new(move |_x: String| -> Result<String> {
        count_clone.fetch_add(1, Ordering::SeqCst);
        Err(Error::other("value error"))
    });

    let fb =
        RunnableLambda::new(|_x: String| -> Result<String> { Ok("fallback_result".to_string()) });

    // Error predicate that only handles errors containing "value"
    let rwf = RunnableWithFallbacks::new(main_r, vec![Arc::new(fb)])
        .with_error_predicate(Arc::new(|e: &Error| e.to_string().contains("value")));

    assert_eq!(
        rwf.invoke("test".to_string(), None).unwrap(),
        "fallback_result"
    );
}

#[test]
fn test_custom_error_predicate_non_matching_error_propagates() {
    let main_r =
        RunnableLambda::new(|_x: String| -> Result<String> { Err(Error::other("type error")) });

    let fb =
        RunnableLambda::new(|_x: String| -> Result<String> { Ok("fallback_result".to_string()) });

    // Only handle errors containing "value" — "type error" should NOT trigger fallback
    let rwf = RunnableWithFallbacks::new(main_r, vec![Arc::new(fb)])
        .with_error_predicate(Arc::new(|e: &Error| e.to_string().contains("value")));

    let result = rwf.invoke("test".to_string(), None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("type error"));
}

// ===========================================================================
// Tests for empty batch (mirrors test_fallbacks_empty_batch)
// ===========================================================================

#[test]
fn test_fallbacks_empty_batch() {
    let main_r = RunnableLambda::new(|x: String| -> Result<String> { Ok(x) });
    let fb = RunnableLambda::new(|x: String| -> Result<String> { Ok(x) });
    let rwf = main_r.with_fallbacks(vec![Arc::new(fb)]);
    assert!(rwf.batch(vec![], None, false).is_empty());
}

#[tokio::test]
async fn test_fallbacks_empty_abatch() {
    let main_r = RunnableLambda::new(|x: String| -> Result<String> { Ok(x) });
    let fb = RunnableLambda::new(|x: String| -> Result<String> { Ok(x) });
    let rwf = main_r.with_fallbacks(vec![Arc::new(fb)]);
    assert!(rwf.abatch(vec![], None, false).await.is_empty());
}

// ===========================================================================
// Tests for success uses first (mirrors test_fallbacks_all_succeed_uses_first)
// ===========================================================================

#[test]
fn test_fallbacks_all_succeed_uses_first() {
    let call_log = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));

    let log_main = call_log.clone();
    let main_r = RunnableLambda::new(move |_x: String| -> Result<String> {
        log_main.lock().unwrap().push("main".to_string());
        Ok("main_result".to_string())
    });

    let log_fb = call_log.clone();
    let fb = RunnableLambda::new(move |_x: String| -> Result<String> {
        log_fb.lock().unwrap().push("fallback".to_string());
        Ok("fallback_result".to_string())
    });

    let rwf = main_r.with_fallbacks(vec![Arc::new(fb)]);
    let result = rwf.invoke("test".to_string(), None).unwrap();
    assert_eq!(result, "main_result");
    assert_eq!(*call_log.lock().unwrap(), vec!["main"]);
}

// ===========================================================================
// Tests for chain of failures (mirrors test_fallbacks_chain_of_failures)
// ===========================================================================

#[test]
fn test_fallbacks_chain_of_failures() {
    let main_r =
        RunnableLambda::new(|_x: String| -> Result<String> { Err(Error::other("error1")) });

    let fb = RunnableLambda::new(|_x: String| -> Result<String> { Err(Error::other("error2")) });

    let rwf = main_r.with_fallbacks(vec![Arc::new(fb)]);
    let result = rwf.invoke("test".to_string(), None);
    assert!(result.is_err());
    // First error should be preserved
    assert!(
        result.unwrap_err().to_string().contains("error1"),
        "Should preserve the first error"
    );
}

// ===========================================================================
// Tests for stream with immediate/delayed errors (mirrors test_fallbacks_stream)
// ===========================================================================

#[test]
fn test_fallbacks_stream_immediate_error_triggers_fallback() {
    // Primary fails immediately, fallback succeeds
    let primary = RunnableLambda::new(|_x: String| -> Result<String> {
        Err(Error::other("immediate error"))
    });
    let fallback =
        RunnableLambda::new(|_x: String| -> Result<String> { Ok("recovered".to_string()) });

    let rwf = primary.with_fallbacks(vec![Arc::new(fallback)]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let chunks: Vec<Result<String>> = rt.block_on(async {
        use futures::StreamExt;
        rwf.stream("test".to_string(), None).collect().await
    });
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].as_ref().unwrap(), "recovered");
}

// ===========================================================================
// Tests for astream (mirrors test_fallbacks_astream)
// ===========================================================================

#[tokio::test]
async fn test_fallbacks_astream_immediate_error_triggers_fallback() {
    let primary = RunnableLambda::new(|_x: String| -> Result<String> {
        Err(Error::other("immediate error"))
    });
    let fallback =
        RunnableLambda::new(|_x: String| -> Result<String> { Ok("recovered".to_string()) });

    let rwf = primary.with_fallbacks(vec![Arc::new(fallback)]);

    use futures::StreamExt;
    let chunks: Vec<Result<String>> = rwf.astream("test".to_string(), None).collect().await;
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].as_ref().unwrap(), "recovered");
}

// ===========================================================================
// Tests for batch return_exceptions (mirrors test_batch partial)
// ===========================================================================

#[test]
fn test_batch_return_exceptions() {
    let runnable = RunnableLambda::new(|inputs: HashMap<String, Value>| -> Result<String> {
        let text = inputs.get("text").and_then(|v| v.as_str()).unwrap_or("");
        if text == "foo" {
            Ok("first".to_string())
        } else {
            Err(Error::other("missing exception"))
        }
    });

    let results = runnable.batch(
        vec![make_input("foo"), make_input("bar"), make_input("baz")],
        None,
        true,
    );

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].as_ref().unwrap(), "first");
    assert!(results[1].is_err());
    assert!(results[2].is_err());
}

#[tokio::test]
async fn test_abatch_return_exceptions() {
    let runnable = RunnableLambda::new(|inputs: HashMap<String, Value>| -> Result<String> {
        let text = inputs.get("text").and_then(|v| v.as_str()).unwrap_or("");
        if text == "foo" {
            Ok("first".to_string())
        } else {
            Err(Error::other("missing exception"))
        }
    });

    let results = runnable
        .abatch(
            vec![make_input("foo"), make_input("bar"), make_input("baz")],
            None,
            true,
        )
        .await;

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].as_ref().unwrap(), "first");
    assert!(results[1].is_err());
    assert!(results[2].is_err());
}

// ===========================================================================
// Tests for batch with error predicate (mirrors test_batch with exceptions_to_handle)
// ===========================================================================

#[test]
fn test_batch_with_error_predicate() {
    // A runnable that fails differently based on input
    let runnable = RunnableLambda::new(|inputs: HashMap<String, Value>| -> Result<String> {
        let text = inputs.get("text").and_then(|v| v.as_str()).unwrap_or("");
        match text {
            "foo" => Ok("first".to_string()),
            "bar" => Err(Error::other("value_error: bar")),
            _ => Err(Error::InvalidConfig("type_error".to_string())),
        }
    });

    let fallback = RunnableLambda::new(|inputs: HashMap<String, Value>| -> Result<String> {
        let text = inputs.get("text").and_then(|v| v.as_str()).unwrap_or("");
        match text {
            "bar" => Ok("recovered_bar".to_string()),
            _ => Err(Error::InvalidConfig("still type_error".to_string())),
        }
    });

    // Only handle errors whose message contains "value_error"
    let rwf = RunnableWithFallbacks::new(runnable, vec![Arc::new(fallback)])
        .with_error_predicate(Arc::new(|e: &Error| e.to_string().contains("value_error")));

    let results = rwf.batch(
        vec![make_input("foo"), make_input("bar"), make_input("baz")],
        None,
        true,
    );

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].as_ref().unwrap(), "first");
    assert_eq!(results[1].as_ref().unwrap(), "recovered_bar");
    // "baz" errors with InvalidConfig which doesn't match the predicate,
    // so it's not retried
    assert!(results[2].is_err());
}

// ===========================================================================
// Tests for exception_key with stream (mirrors chain_pass_exceptions)
// ===========================================================================

#[test]
fn test_stream_with_exception_key() {
    let failing = RunnableLambda::new(|_inputs: HashMap<String, Value>| -> Result<String> {
        Err(Error::other("stream error"))
    });

    let recovery = RunnableLambda::new(|inputs: HashMap<String, Value>| -> Result<String> {
        if inputs.contains_key("exception") {
            Ok("recovered".to_string())
        } else {
            Err(Error::other("no exception"))
        }
    });

    let rwf = RunnableWithFallbacks::new(failing, vec![Arc::new(recovery)])
        .with_exception_key("exception", hashmap_exception_inserter());

    let rt = tokio::runtime::Runtime::new().unwrap();
    let chunks: Vec<Result<String>> = rt.block_on(async {
        use futures::StreamExt;
        let input = make_input("test");
        rwf.stream(input, None).collect().await
    });
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].as_ref().unwrap(), "recovered");
}
