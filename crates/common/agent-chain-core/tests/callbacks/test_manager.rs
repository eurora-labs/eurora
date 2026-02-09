//! Unit tests for callback run managers.
//!
//! Ported from `langchain/libs/core/tests/unit_tests/callbacks/test_manager.py`

use agent_chain_core::callbacks::base::{
    BaseCallbackHandler, CallbackManagerMixin, ChainManagerMixin, LLMManagerMixin,
    RetrieverManagerMixin, RunManagerMixin, ToolManagerMixin,
};
use agent_chain_core::callbacks::manager::{
    AsyncCallbackManager, AsyncCallbackManagerForLLMRun, BaseRunManager, CallbackManager,
    CallbackManagerForChainRun, CallbackManagerForLLMRun, CallbackManagerForRetrieverRun,
    CallbackManagerForToolRun, ParentRunManager, RunManager,
};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

// -- Minimal handler --

#[derive(Debug, Default)]
struct TestHandler;

impl LLMManagerMixin for TestHandler {}
impl ChainManagerMixin for TestHandler {}
impl ToolManagerMixin for TestHandler {}
impl RetrieverManagerMixin for TestHandler {}
impl CallbackManagerMixin for TestHandler {}
impl RunManagerMixin for TestHandler {}

impl BaseCallbackHandler for TestHandler {
    fn name(&self) -> &str {
        "TestHandler"
    }
}

// ---- BaseRunManager tests ----

/// Ported from `test_base_run_manager_initialization`.
#[test]
fn test_base_run_manager_initialization() {
    let run_id = Uuid::new_v4();
    let parent_run_id = Uuid::new_v4();
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);

    let manager = BaseRunManager::new(
        run_id,
        vec![handler.clone()],
        vec![handler],
        Some(parent_run_id),
        Some(vec!["tag1".to_string()]),
        Some(vec!["tag2".to_string()]),
        Some(HashMap::from([(
            "key".to_string(),
            serde_json::json!("value"),
        )])),
        Some(HashMap::from([(
            "key2".to_string(),
            serde_json::json!("value2"),
        )])),
    );

    assert_eq!(manager.run_id, run_id);
    assert_eq!(manager.parent_run_id, Some(parent_run_id));
    assert_eq!(manager.handlers.len(), 1);
    assert_eq!(manager.inheritable_handlers.len(), 1);
    assert!(manager.tags.contains(&"tag1".to_string()));
    assert!(manager.inheritable_tags.contains(&"tag2".to_string()));
    assert_eq!(manager.metadata["key"], serde_json::json!("value"));
    assert_eq!(
        manager.inheritable_metadata["key2"],
        serde_json::json!("value2")
    );
}

/// Ported from `test_base_run_manager_get_noop_manager`.
#[test]
fn test_base_run_manager_get_noop_manager() {
    let manager = BaseRunManager::get_noop_manager();

    assert!(!manager.run_id.is_nil());
    assert!(manager.handlers.is_empty());
    assert!(manager.inheritable_handlers.is_empty());
}

// ---- RunManager tests ----

/// Ported from `test_run_manager_on_text`.
#[test]
fn test_run_manager_on_text() {
    let run_id = Uuid::new_v4();
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);

    let manager = RunManager::new(run_id, vec![handler], vec![], None, None, None, None, None);

    // Should not panic with handler present
    manager.on_text("Hello");
    manager.on_text("World");
}

/// Ported from `test_run_manager_empty_handlers`.
#[test]
fn test_run_manager_empty_handlers() {
    let manager = RunManager::new(Uuid::new_v4(), vec![], vec![], None, None, None, None, None);

    // Should not panic
    manager.on_text("test");
}

// ---- ParentRunManager tests ----

/// Ported from `test_parent_run_manager_get_child`.
#[test]
fn test_parent_run_manager_get_child() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let run_id = Uuid::new_v4();

    let parent = ParentRunManager::new(
        run_id,
        vec![handler],
        vec![Arc::new(TestHandler)],
        None,
        Some(vec!["parent_tag".to_string()]),
        Some(vec!["inheritable_tag".to_string()]),
        Some(HashMap::from([(
            "key".to_string(),
            serde_json::json!("value"),
        )])),
        Some(HashMap::from([(
            "key2".to_string(),
            serde_json::json!("value2"),
        )])),
    );

    let child = parent.get_child(None);

    assert_eq!(child.parent_run_id, Some(run_id));
    assert!(!child.inheritable_handlers.is_empty());
    assert!(child.tags.contains(&"inheritable_tag".to_string()));
    assert_eq!(
        child.inheritable_metadata["key2"],
        serde_json::json!("value2")
    );
}

/// Ported from `test_parent_run_manager_get_child_with_tag`.
#[test]
fn test_parent_run_manager_get_child_with_tag() {
    let parent =
        ParentRunManager::new(Uuid::new_v4(), vec![], vec![], None, None, None, None, None);

    let child = parent.get_child(Some("child_tag"));

    assert!(child.tags.contains(&"child_tag".to_string()));
    // Child tag should not be inheritable
    assert!(!child.inheritable_tags.contains(&"child_tag".to_string()));
}

// ---- CallbackManagerForLLMRun tests ----

/// Ported from `test_callback_manager_for_llm_run_on_llm_new_token`.
#[test]
fn test_callback_manager_for_llm_run_on_llm_new_token() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let manager = CallbackManagerForLLMRun::new(
        Uuid::new_v4(),
        vec![handler],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );

    // Should not panic
    manager.on_llm_new_token("Hello", None);
    manager.on_llm_new_token(" ", None);
    manager.on_llm_new_token("World", None);
}

/// Ported from `test_callback_manager_for_llm_run_on_llm_end`.
#[test]
fn test_callback_manager_for_llm_run_on_llm_end() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let manager = CallbackManagerForLLMRun::new(
        Uuid::new_v4(),
        vec![handler],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );

    let result = agent_chain_core::outputs::ChatResult::default();
    manager.on_llm_end(&result);
}

/// Ported from `test_callback_manager_for_llm_run_on_llm_error`.
#[test]
fn test_callback_manager_for_llm_run_on_llm_error() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let manager = CallbackManagerForLLMRun::new(
        Uuid::new_v4(),
        vec![handler],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );

    let error = std::io::Error::other("Test error");
    manager.on_llm_error(&error);
}

// ---- AsyncCallbackManagerForLLMRun tests ----

/// Ported from `test_async_callback_manager_for_llm_run_get_sync`.
#[test]
fn test_async_callback_manager_for_llm_run_get_sync() {
    let run_id = Uuid::new_v4();
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);

    let sync_manager = CallbackManagerForLLMRun::new(
        run_id,
        vec![handler],
        vec![Arc::new(TestHandler)],
        None,
        Some(vec!["test".to_string()]),
        None,
        None,
        None,
    );
    let async_manager = AsyncCallbackManagerForLLMRun::from_sync(sync_manager);

    let sync_back = async_manager.get_sync();

    assert_eq!(sync_back.run_id(), run_id);
    assert_eq!(sync_back.handlers().len(), 1);
    assert!(sync_back.tags().contains(&"test".to_string()));
}

// ---- CallbackManagerForChainRun tests ----

/// Ported from `test_callback_manager_for_chain_run_on_chain_end`.
#[test]
fn test_callback_manager_for_chain_run_on_chain_end() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let manager = CallbackManagerForChainRun::new(
        Uuid::new_v4(),
        vec![handler],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );

    manager.on_chain_end(&HashMap::from([(
        "result".to_string(),
        serde_json::json!("success"),
    )]));
}

/// Ported from `test_callback_manager_for_chain_run_on_chain_error`.
#[test]
fn test_callback_manager_for_chain_run_on_chain_error() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let manager = CallbackManagerForChainRun::new(
        Uuid::new_v4(),
        vec![handler],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );

    let error = std::io::Error::other("Chain failed");
    manager.on_chain_error(&error);
}

// ---- CallbackManagerForToolRun tests ----

/// Ported from `test_callback_manager_for_tool_run_on_tool_end`.
#[test]
fn test_callback_manager_for_tool_run_on_tool_end() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let manager = CallbackManagerForToolRun::new(
        Uuid::new_v4(),
        vec![handler],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );

    manager.on_tool_end("Tool result");
}

/// Ported from `test_callback_manager_for_tool_run_on_tool_error`.
#[test]
fn test_callback_manager_for_tool_run_on_tool_error() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let manager = CallbackManagerForToolRun::new(
        Uuid::new_v4(),
        vec![handler],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );

    let error = std::io::Error::other("Tool failed");
    manager.on_tool_error(&error);
}

// ---- CallbackManagerForRetrieverRun tests ----

/// Ported from `test_callback_manager_for_retriever_run_on_retriever_end`.
#[test]
fn test_callback_manager_for_retriever_run_on_retriever_end() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let manager = CallbackManagerForRetrieverRun::new(
        Uuid::new_v4(),
        vec![handler],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );

    manager.on_retriever_end(&[]);
}

/// Ported from `test_callback_manager_for_retriever_run_on_retriever_error`.
#[test]
fn test_callback_manager_for_retriever_run_on_retriever_error() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let manager = CallbackManagerForRetrieverRun::new(
        Uuid::new_v4(),
        vec![handler],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );

    let error = std::io::Error::other("Retriever failed");
    manager.on_retriever_error(&error);
}

// ---- CallbackManager start tests ----

/// Ported from `test_callback_manager_on_llm_start_single_prompt`.
#[test]
fn test_callback_manager_on_llm_start_single_prompt() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut manager = CallbackManager::new();
    manager.add_handler(handler, true);

    let run_managers = manager.on_llm_start(&HashMap::new(), &["Test prompt".to_string()], None);

    assert_eq!(run_managers.len(), 1);
    assert!(!run_managers[0].run_id().is_nil());
}

/// Ported from `test_callback_manager_on_llm_start_multiple_prompts`.
#[test]
fn test_callback_manager_on_llm_start_multiple_prompts() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut manager = CallbackManager::new();
    manager.add_handler(handler, true);

    let run_managers = manager.on_llm_start(
        &HashMap::new(),
        &[
            "Prompt 1".to_string(),
            "Prompt 2".to_string(),
            "Prompt 3".to_string(),
        ],
        None,
    );

    assert_eq!(run_managers.len(), 3);
}

/// Ported from `test_callback_manager_on_chain_start`.
#[test]
fn test_callback_manager_on_chain_start() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut manager = CallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager.on_chain_start(&HashMap::new(), &HashMap::new(), None);

    assert!(!run_manager.run_id().is_nil());
}

/// Ported from `test_callback_manager_on_tool_start`.
#[test]
fn test_callback_manager_on_tool_start() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut manager = CallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager.on_tool_start(&HashMap::new(), "test input", None, None);

    assert!(!run_manager.run_id().is_nil());
}

/// Ported from `test_callback_manager_on_retriever_start`.
#[test]
fn test_callback_manager_on_retriever_start() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut manager = CallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager.on_retriever_start(&HashMap::new(), "search query", None);

    assert!(!run_manager.run_id().is_nil());
}

// ---- AsyncCallbackManager tests ----

/// Ported from `test_async_callback_manager_is_async`.
#[test]
fn test_async_callback_manager_is_async() {
    let manager = AsyncCallbackManager::new();
    assert!(manager.is_async());
}

/// Ported from `test_async_callback_manager_on_llm_start`.
#[tokio::test]
async fn test_async_callback_manager_on_llm_start() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_managers = manager
        .on_llm_start(&HashMap::new(), &["Test prompt".to_string()], None)
        .await;

    assert_eq!(run_managers.len(), 1);
    assert!(!run_managers[0].run_id().is_nil());
}

/// Ported from `test_async_callback_manager_on_chain_start`.
#[tokio::test]
async fn test_async_callback_manager_on_chain_start() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager
        .on_chain_start(&HashMap::new(), &HashMap::new(), None)
        .await;

    assert!(!run_manager.run_id().is_nil());
}

/// Ported from `test_async_run_manager_on_text`.
#[tokio::test]
async fn test_async_run_manager_on_text() {
    use agent_chain_core::callbacks::manager::AsyncRunManager;

    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);

    let manager = AsyncRunManager::new(
        Uuid::new_v4(),
        vec![handler],
        vec![],
        None,
        None,
        None,
        None,
        None,
    );

    // Should not panic
    manager.on_text("Hello").await;
    manager.on_text("World").await;
}

/// Ported from `test_async_parent_run_manager_get_child`.
#[tokio::test]
async fn test_async_parent_run_manager_get_child() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let chain_run = manager
        .on_chain_start(&HashMap::new(), &HashMap::new(), None)
        .await;

    let child = chain_run.get_child(None);

    assert_eq!(child.parent_run_id(), Some(chain_run.run_id()));
    assert_eq!(child.handlers().len(), 1);
}
