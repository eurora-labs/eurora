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

    let ih_names: Vec<&str> = merged
        .inheritable_handlers
        .iter()
        .map(|h| h.name())
        .collect();
    assert!(ih_names.contains(&"ih1"));
    assert!(ih_names.contains(&"ih2"));
}
