//! Unit tests for sync callback manager.
//!
//! Ported from `langchain/libs/core/tests/unit_tests/callbacks/test_sync_callback_manager.py`

use std::sync::Arc;

use agent_chain_core::callbacks::base::{
    BaseCallbackHandler, BaseCallbackManager, CallbackManagerMixin, ChainManagerMixin,
    LLMManagerMixin, RetrieverManagerMixin, RunManagerMixin, ToolManagerMixin,
};

#[derive(Debug)]
struct TestHandler {
    label: &'static str,
}

impl TestHandler {
    fn new(label: &'static str) -> Self {
        Self { label }
    }
}

impl LLMManagerMixin for TestHandler {}
impl ChainManagerMixin for TestHandler {}
impl ToolManagerMixin for TestHandler {}
impl RetrieverManagerMixin for TestHandler {}
impl CallbackManagerMixin for TestHandler {}
impl RunManagerMixin for TestHandler {}

impl BaseCallbackHandler for TestHandler {
    fn name(&self) -> &str {
        self.label
    }
}

/// Ported from `test_remove_handler`.
///
/// Test removing handler does not raise an error on removal.
/// A handler can be inheritable or not. This test checks that
/// removing a handler does not raise an error if the handler
/// is not inheritable.
#[test]
fn test_remove_handler() {
    let handler1: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler::new("h1"));
    let handler2: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler::new("h2"));
    let mut manager = BaseCallbackManager::with_handlers(
        vec![handler1.clone()],
        Some(vec![handler2.clone()]),
        None,
        None,
        None,
        None,
        None,
    );
    manager.remove_handler(&handler1);
    manager.remove_handler(&handler2);

    assert!(manager.handlers.is_empty());
    assert!(manager.inheritable_handlers.is_empty());
}

/// Ported from `test_merge_preserves_handler_distinction`.
///
/// The Python test is marked xfail because merge() incorrectly mixes
/// handlers and inheritable_handlers. The Rust implementation has the
/// same behavior (inheritable handlers also get added to handlers via
/// add_handler), so this test documents the current behavior rather
/// than the ideal behavior.
///
/// When the Python bug (#32028) is fixed, both implementations should
/// be updated and this test should assert strict separation.
#[test]
fn test_merge_preserves_handler_distinction() {
    let h1: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler::new("h1"));
    let h2: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler::new("h2"));
    let ih1: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler::new("ih1"));
    let ih2: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler::new("ih2"));

    let m1 = BaseCallbackManager::with_handlers(
        vec![h1.clone()],
        Some(vec![ih1.clone()]),
        None,
        None,
        None,
        None,
        None,
    );
    let m2 = BaseCallbackManager::with_handlers(
        vec![h2.clone()],
        Some(vec![ih2.clone()]),
        None,
        None,
        None,
        None,
        None,
    );

    let merged = m1.merge(&m2);

    // Current behavior (matches Python bug): inheritable handlers also
    // appear in handlers, so handlers contains h1, h2, ih1, ih2.
    // Ideal behavior would be: handlers = {h1, h2}, inheritable_handlers = {ih1, ih2}
    assert_eq!(
        merged.handlers.len(),
        4,
        "handlers should contain all 4 (current buggy behavior)"
    );
    assert_eq!(
        merged.inheritable_handlers.len(),
        2,
        "inheritable_handlers should contain ih1 and ih2"
    );

    // Verify the inheritable_handlers are correct
    let ih_names: Vec<&str> = merged
        .inheritable_handlers
        .iter()
        .map(|h| h.name())
        .collect();
    assert!(ih_names.contains(&"ih1"));
    assert!(ih_names.contains(&"ih2"));
}
