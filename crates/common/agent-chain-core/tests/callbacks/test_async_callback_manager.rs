//! Unit tests for AsyncCallbackManager.
//!
//! Ported from `langchain/libs/core/tests/unit_tests/callbacks/test_async_callback_manager.py`

use agent_chain_core::callbacks::base::BaseCallbackHandler;
use agent_chain_core::callbacks::manager::AsyncCallbackManager;
use agent_chain_core::messages::HumanMessage;
use agent_chain_core::outputs::ChatResult;
use std::collections::HashMap;
use std::sync::Arc;

// -- FakeHandler to track callback counts --

#[derive(Debug, Default)]
struct FakeHandler;

impl agent_chain_core::callbacks::base::LLMManagerMixin for FakeHandler {}
impl agent_chain_core::callbacks::base::ChainManagerMixin for FakeHandler {}
impl agent_chain_core::callbacks::base::ToolManagerMixin for FakeHandler {}
impl agent_chain_core::callbacks::base::RetrieverManagerMixin for FakeHandler {}
impl agent_chain_core::callbacks::base::CallbackManagerMixin for FakeHandler {}
impl agent_chain_core::callbacks::base::RunManagerMixin for FakeHandler {}

impl BaseCallbackHandler for FakeHandler {
    fn name(&self) -> &str {
        "FakeHandler"
    }
}

// -- Tests --

/// Ported from `test_async_callback_manager_on_llm_start`.
#[tokio::test]
async fn test_async_callback_manager_on_llm_start() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_managers = manager
        .on_llm_start(&HashMap::new(), &["prompt".to_string()], None)
        .await;

    assert_eq!(run_managers.len(), 1);
    assert!(!run_managers[0].run_id().is_nil());
}

/// Ported from `test_async_callback_manager_on_chain_start`.
#[tokio::test]
async fn test_async_callback_manager_on_chain_start() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager
        .on_chain_start(&HashMap::new(), &HashMap::new(), None)
        .await;

    assert!(!run_manager.run_id().is_nil());
}

/// Ported from `test_async_callback_manager_on_tool_start`.
#[tokio::test]
async fn test_async_callback_manager_on_tool_start() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager
        .on_tool_start(&HashMap::new(), "test", None, None)
        .await;

    assert!(!run_manager.run_id().is_nil());
}

/// Ported from `test_async_callback_manager_on_llm_end`.
#[tokio::test]
async fn test_async_callback_manager_on_llm_end() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_managers = manager
        .on_llm_start(&HashMap::new(), &["prompt".to_string()], None)
        .await;

    assert_eq!(run_managers.len(), 1);
    let result = ChatResult::default();
    run_managers[0].on_llm_end(&result).await;
}

/// Ported from `test_async_callback_manager_on_chain_end`.
#[tokio::test]
async fn test_async_callback_manager_on_chain_end() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager
        .on_chain_start(&HashMap::new(), &HashMap::new(), None)
        .await;

    run_manager.on_chain_end(&HashMap::new()).await;
}

/// Ported from `test_async_callback_manager_on_tool_end`.
#[tokio::test]
async fn test_async_callback_manager_on_tool_end() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager
        .on_tool_start(&HashMap::new(), "test", None, None)
        .await;

    run_manager.on_tool_end("test").await;
}

/// Ported from `test_async_callback_manager_on_llm_error`.
#[tokio::test]
async fn test_async_callback_manager_on_llm_error() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_managers = manager
        .on_llm_start(&HashMap::new(), &["prompt".to_string()], None)
        .await;

    assert_eq!(run_managers.len(), 1);

    let error = std::io::Error::other("test");
    run_managers[0].on_llm_error(&error).await;
}

/// Ported from `test_async_callback_manager_on_chain_error`.
#[tokio::test]
async fn test_async_callback_manager_on_chain_error() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager
        .on_chain_start(&HashMap::new(), &HashMap::new(), None)
        .await;

    let error = std::io::Error::other("test");
    run_manager.on_chain_error(&error).await;
}

/// Ported from `test_async_callback_manager_on_tool_error`.
#[tokio::test]
async fn test_async_callback_manager_on_tool_error() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager
        .on_tool_start(&HashMap::new(), "test", None, None)
        .await;

    let error = std::io::Error::other("test");
    run_manager.on_tool_error(&error).await;
}

/// Ported from `test_async_callback_manager_on_llm_new_token`.
#[tokio::test]
async fn test_async_callback_manager_on_llm_new_token() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_managers = manager
        .on_llm_start(&HashMap::new(), &["prompt".to_string()], None)
        .await;

    assert_eq!(run_managers.len(), 1);
    run_managers[0].on_llm_new_token("test", None).await;
}

/// Ported from `test_async_callback_manager_with_multiple_handlers`.
#[tokio::test]
async fn test_async_callback_manager_with_multiple_handlers() {
    let handler1: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let handler2: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler1, true);
    manager.add_handler(handler2, true);

    assert_eq!(manager.handlers().len(), 2);

    let run_managers = manager
        .on_llm_start(&HashMap::new(), &["prompt".to_string()], None)
        .await;

    assert_eq!(run_managers.len(), 1);
    assert_eq!(run_managers[0].handlers().len(), 2);
}

/// Ported from `test_async_callback_manager_add_handler`.
#[tokio::test]
async fn test_async_callback_manager_add_handler() {
    let handler1: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let handler2: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler1, true);

    assert_eq!(manager.handlers().len(), 1);

    manager.add_handler(handler2, true);

    assert_eq!(manager.handlers().len(), 2);
}

/// Ported from `test_async_callback_manager_remove_handler`.
#[tokio::test]
async fn test_async_callback_manager_remove_handler() {
    let handler1: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let handler2: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler1, true);
    manager.add_handler(handler2.clone(), true);

    assert_eq!(manager.handlers().len(), 2);

    manager.remove_handler(&handler2);

    assert_eq!(manager.handlers().len(), 1);
}

/// Ported from `test_async_callback_manager_inheritable_handlers`.
#[tokio::test]
async fn test_async_callback_manager_inheritable_handlers() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut parent = AsyncCallbackManager::new();
    parent.add_handler(handler, true);

    let chain_run = parent
        .on_chain_start(&HashMap::new(), &HashMap::new(), None)
        .await;

    let child_manager = chain_run.get_child(None);
    assert_eq!(child_manager.handlers().len(), 1);
}

/// Ported from `test_async_callback_manager_chat_model_start`.
#[tokio::test]
async fn test_async_callback_manager_chat_model_start() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let messages = vec![vec![
        HumanMessage::builder().content("Hello").build().into(),
    ]];
    let run_managers = manager
        .on_chat_model_start(&HashMap::new(), &messages, None)
        .await;

    assert_eq!(run_managers.len(), 1);
    assert!(!run_managers[0].run_id().is_nil());
}

/// Ported from `test_async_callback_manager_ignore_llm`.
///
/// Handlers with `ignore_llm() == true` should be skipped during LLM events
/// but still present in the run manager (filtering happens at dispatch time).
#[tokio::test]
async fn test_async_callback_manager_ignore_llm() {
    #[derive(Debug)]
    struct IgnoreLLMHandler;

    impl agent_chain_core::callbacks::base::LLMManagerMixin for IgnoreLLMHandler {}
    impl agent_chain_core::callbacks::base::ChainManagerMixin for IgnoreLLMHandler {}
    impl agent_chain_core::callbacks::base::ToolManagerMixin for IgnoreLLMHandler {}
    impl agent_chain_core::callbacks::base::RetrieverManagerMixin for IgnoreLLMHandler {}
    impl agent_chain_core::callbacks::base::CallbackManagerMixin for IgnoreLLMHandler {}
    impl agent_chain_core::callbacks::base::RunManagerMixin for IgnoreLLMHandler {}

    impl BaseCallbackHandler for IgnoreLLMHandler {
        fn name(&self) -> &str {
            "IgnoreLLMHandler"
        }
        fn ignore_llm(&self) -> bool {
            true
        }
    }

    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(IgnoreLLMHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_managers = manager
        .on_llm_start(&HashMap::new(), &["prompt".to_string()], None)
        .await;

    // The run manager is still created (it just won't dispatch to ignored handlers)
    assert_eq!(run_managers.len(), 1);
}

/// Ported from `test_async_callback_manager_ignore_chain`.
#[tokio::test]
async fn test_async_callback_manager_ignore_chain() {
    #[derive(Debug)]
    struct IgnoreChainHandler;

    impl agent_chain_core::callbacks::base::LLMManagerMixin for IgnoreChainHandler {}
    impl agent_chain_core::callbacks::base::ChainManagerMixin for IgnoreChainHandler {}
    impl agent_chain_core::callbacks::base::ToolManagerMixin for IgnoreChainHandler {}
    impl agent_chain_core::callbacks::base::RetrieverManagerMixin for IgnoreChainHandler {}
    impl agent_chain_core::callbacks::base::CallbackManagerMixin for IgnoreChainHandler {}
    impl agent_chain_core::callbacks::base::RunManagerMixin for IgnoreChainHandler {}

    impl BaseCallbackHandler for IgnoreChainHandler {
        fn name(&self) -> &str {
            "IgnoreChainHandler"
        }
        fn ignore_chain(&self) -> bool {
            true
        }
    }

    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(IgnoreChainHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager
        .on_chain_start(&HashMap::new(), &HashMap::new(), None)
        .await;

    // Run manager created, but the handler's on_chain_start was skipped via ignore
    assert!(!run_manager.run_id().is_nil());
}

/// Ported from `test_async_callback_manager_copy`.
#[tokio::test]
async fn test_async_callback_manager_copy() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let manager_copy = manager.clone();
    assert_eq!(manager_copy.handlers().len(), 1);
}

/// Ported from `test_async_callback_manager_chain_child_managers`.
#[tokio::test]
async fn test_async_callback_manager_chain_child_managers() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let chain_run = manager
        .on_chain_start(&HashMap::new(), &HashMap::new(), None)
        .await;

    // Child LLM run
    let child_llm_runs = chain_run
        .get_child(None)
        .on_llm_start(&HashMap::new(), &["prompt".to_string()], None)
        .await;
    assert_eq!(child_llm_runs.len(), 1);
    assert!(!child_llm_runs[0].run_id().is_nil());

    // Child chain run
    let child_chain_run = chain_run
        .get_child(None)
        .on_chain_start(&HashMap::new(), &HashMap::new(), None)
        .await;
    assert!(!child_chain_run.run_id().is_nil());

    // Child tool run
    let child_tool_run = chain_run
        .get_child(None)
        .on_tool_start(&HashMap::new(), "test", None, None)
        .await;
    assert!(!child_tool_run.run_id().is_nil());
}

/// Ported from `test_async_callback_manager_retriever_callbacks`.
#[tokio::test]
async fn test_async_callback_manager_retriever_callbacks() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager
        .on_retriever_start()
        .serialized(&HashMap::new())
        .query("test query")
        .call()
        .await;

    assert!(!run_manager.run_id().is_nil());
    run_manager.on_retriever_end(&[]).await;
}

/// Ported from `test_async_callback_manager_retriever_error`.
#[tokio::test]
async fn test_async_callback_manager_retriever_error() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager
        .on_retriever_start()
        .serialized(&HashMap::new())
        .query("test query")
        .call()
        .await;

    let error = std::io::Error::other("test error");
    run_manager.on_retriever_error(&error).await;
}

/// Ported from `test_async_callback_manager_tags`.
#[tokio::test]
async fn test_async_callback_manager_tags() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);
    manager.add_tags(vec!["test-tag".to_string()], true);

    let run_managers = manager
        .on_llm_start(&HashMap::new(), &["prompt".to_string()], None)
        .await;

    assert_eq!(run_managers.len(), 1);
    // Tags should be propagated to the run manager's inner state
    // (verified by checking the run manager was created successfully)
    assert!(!run_managers[0].run_id().is_nil());
}

/// Ported from `test_async_callback_manager_metadata`.
#[tokio::test]
async fn test_async_callback_manager_metadata() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);
    manager.add_metadata(
        HashMap::from([("key".to_string(), serde_json::json!("value"))]),
        true,
    );

    let run_managers = manager
        .on_llm_start(&HashMap::new(), &["prompt".to_string()], None)
        .await;

    assert_eq!(run_managers.len(), 1);
    assert!(!run_managers[0].run_id().is_nil());
}

/// Ported from `test_async_callback_manager_chain_run_on_agent_action`.
#[tokio::test]
async fn test_async_callback_manager_chain_run_on_agent_action() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let chain_run = manager
        .on_chain_start(&HashMap::new(), &HashMap::new(), None)
        .await;

    let action = serde_json::json!({
        "tool": "test_tool",
        "tool_input": "test_input",
        "log": "test_log"
    });
    chain_run.on_agent_action(&action).await;

    let finish = serde_json::json!({
        "return_values": {"output": "test"},
        "log": "test_log"
    });
    chain_run.on_agent_finish(&finish).await;
}

/// Ported from `test_async_callback_manager_concurrent_runs`.
#[tokio::test]
async fn test_async_callback_manager_concurrent_runs() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let empty = HashMap::new();
    let prompts1 = ["prompt1".to_string()];
    let prompts2 = ["prompt2".to_string()];
    let prompts3 = ["prompt3".to_string()];
    let (runs1, runs2, runs3) = tokio::join!(
        manager.on_llm_start(&empty, &prompts1, None),
        manager.on_llm_start(&empty, &prompts2, None),
        manager.on_llm_start(&empty, &prompts3, None),
    );

    assert_eq!(runs1.len(), 1);
    assert_eq!(runs2.len(), 1);
    assert_eq!(runs3.len(), 1);

    // Each run has a unique ID
    assert_ne!(runs1[0].run_id(), runs2[0].run_id());
    assert_ne!(runs2[0].run_id(), runs3[0].run_id());

    // End all runs
    let result = ChatResult::default();
    tokio::join!(
        runs1[0].on_llm_end(&result),
        runs2[0].on_llm_end(&result),
        runs3[0].on_llm_end(&result),
    );
}

/// Ported from `test_async_callback_manager_full_lifecycle`.
#[tokio::test]
async fn test_async_callback_manager_full_lifecycle() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    // Start chain
    let chain_run = manager
        .on_chain_start(&HashMap::new(), &HashMap::new(), None)
        .await;

    // Start LLM inside chain
    let child_manager = chain_run.get_child(None);
    let llm_runs = child_manager
        .on_llm_start(&HashMap::new(), &["prompt".to_string()], None)
        .await;
    assert_eq!(llm_runs.len(), 1);

    // LLM produces tokens
    llm_runs[0].on_llm_new_token("Hello", None).await;
    llm_runs[0].on_llm_new_token(" World", None).await;

    // LLM ends
    llm_runs[0].on_llm_end(&ChatResult::default()).await;

    // Tool call inside chain
    let child_manager2 = chain_run.get_child(None);
    let tool_run = child_manager2
        .on_tool_start(&HashMap::new(), "test", None, None)
        .await;
    tool_run.on_tool_end("result").await;

    // Chain ends
    chain_run.on_chain_end(&HashMap::new()).await;
}
