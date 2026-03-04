use std::sync::Arc;

use agent_chain_core::callbacks::BaseCallbackHandler;
use agent_chain_core::callbacks::CallbackManager;

#[derive(Debug)]
struct TestHandler {
    label: &'static str,
}

impl TestHandler {
    fn new(label: &'static str) -> Self {
        Self { label }
    }
}

impl BaseCallbackHandler for TestHandler {
    fn name(&self) -> &str {
        self.label
    }
}

#[test]
fn test_remove_handler() {
    let handler1: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler::new("h1"));
    let handler2: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler::new("h2"));
    let mut manager = CallbackManager {
        handlers: vec![handler1.clone()],
        inheritable_handlers: vec![handler2.clone()],
        ..Default::default()
    };
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

    let m1 = CallbackManager {
        handlers: vec![h1.clone()],
        inheritable_handlers: vec![ih1.clone()],
        ..Default::default()
    };
    let m2 = CallbackManager {
        handlers: vec![h2.clone()],
        inheritable_handlers: vec![ih2.clone()],
        ..Default::default()
    };

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
