//! Unit tests for dispatch_custom_event and adispatch_custom_event.
//!
//! Ported from `langchain/libs/core/tests/unit_tests/callbacks/test_dispatch_custom_event.py`

use agent_chain_core::callbacks::base::{
    BaseCallbackHandler, CallbackManagerMixin, ChainManagerMixin, LLMManagerMixin,
    RetrieverManagerMixin, RunManagerMixin, ToolManagerMixin,
};
use agent_chain_core::callbacks::manager::{
    AsyncCallbackManager, CallbackManager, adispatch_custom_event, dispatch_custom_event,
};
use std::collections::HashMap;
use std::sync::Arc;

// -- FakeHandler --

#[derive(Debug, Default)]
struct FakeHandler;

impl LLMManagerMixin for FakeHandler {}
impl ChainManagerMixin for FakeHandler {}
impl ToolManagerMixin for FakeHandler {}
impl RetrieverManagerMixin for FakeHandler {}
impl CallbackManagerMixin for FakeHandler {}
impl RunManagerMixin for FakeHandler {}

impl BaseCallbackHandler for FakeHandler {
    fn name(&self) -> &str {
        "FakeHandler"
    }
}

// -- Tests --

/// Ported from `test_custom_event_root_dispatch`.
///
/// Dispatching a custom event without a parent_run_id should fail.
#[test]
fn test_custom_event_root_dispatch() {
    let manager = CallbackManager::new();
    let result = dispatch_custom_event("event1", &serde_json::json!({"x": 1}), &manager);

    // With no handlers, the function short-circuits to Ok (no work to do)
    assert!(result.is_ok());

    // With a handler but no parent_run_id, it should fail
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = CallbackManager::new();
    manager.add_handler(handler, true);

    let result = dispatch_custom_event("event1", &serde_json::json!({"x": 1}), &manager);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("parent run id"));
}

/// Ported from `test_async_custom_event_root_dispatch`.
///
/// Dispatching a custom event asynchronously without a parent_run_id should fail.
#[tokio::test]
async fn test_async_custom_event_root_dispatch() {
    let manager = AsyncCallbackManager::new();
    let result = adispatch_custom_event("event1", &serde_json::json!({"x": 1}), &manager).await;

    // With no handlers, the function short-circuits to Ok
    assert!(result.is_ok());

    // With a handler but no parent_run_id, it should fail
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let result = adispatch_custom_event("event1", &serde_json::json!({"x": 1}), &manager).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("parent run id"));
}

/// Ported from `test_sync_callback_manager`.
///
/// Dispatching a custom event with a properly configured manager (with
/// parent_run_id and handlers) should succeed.
#[test]
fn test_sync_callback_manager() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = CallbackManager::new();
    manager.add_handler(handler, true);

    // Start a chain to establish a parent_run_id
    let chain_run = manager.on_chain_start(&HashMap::new(), &HashMap::new(), None);
    let child_manager = chain_run.get_child(None);

    // Dispatch custom events through the child manager (which has parent_run_id set)
    let result1 = dispatch_custom_event("event1", &serde_json::json!({"x": 1}), &child_manager);
    assert!(result1.is_ok());

    let result2 = dispatch_custom_event("event2", &serde_json::json!({"x": 1}), &child_manager);
    assert!(result2.is_ok());
}

/// Ported from `test_async_callback_manager`.
///
/// Dispatching a custom event asynchronously with a properly configured
/// manager (with parent_run_id and handlers) should succeed.
#[tokio::test]
async fn test_async_callback_manager() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    // Start a chain to establish a parent_run_id
    let chain_run = manager
        .on_chain_start(&HashMap::new(), &HashMap::new(), None)
        .await;
    let child_manager = chain_run.get_child(None);

    // Dispatch custom events through the child manager (which has parent_run_id set)
    let result1 =
        adispatch_custom_event("event1", &serde_json::json!({"x": 1}), &child_manager).await;
    assert!(result1.is_ok());

    let result2 =
        adispatch_custom_event("event2", &serde_json::json!({"x": 1}), &child_manager).await;
    assert!(result2.is_ok());
}

/// Additional test: verify empty handlers short-circuit without error.
#[test]
fn test_dispatch_custom_event_no_handlers() {
    let mut manager = CallbackManager::new();
    manager.parent_run_id = Some(uuid::Uuid::nil());

    let result = dispatch_custom_event("event1", &serde_json::json!({"x": 1}), &manager);
    assert!(result.is_ok());
}

/// Additional test: async version with empty handlers short-circuits.
#[tokio::test]
async fn test_adispatch_custom_event_no_handlers() {
    let manager = AsyncCallbackManager::new();

    let result = adispatch_custom_event("event1", &serde_json::json!({"x": 1}), &manager).await;
    assert!(result.is_ok());
}
