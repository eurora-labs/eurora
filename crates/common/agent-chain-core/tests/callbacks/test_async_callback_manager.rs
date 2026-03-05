use agent_chain_core::callbacks::BaseCallbackHandler;
use agent_chain_core::callbacks::manager::AsyncCallbackManager;
use agent_chain_core::messages::HumanMessage;
use agent_chain_core::outputs::ChatResult;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Default)]
struct FakeHandler;

impl BaseCallbackHandler for FakeHandler {
    fn name(&self) -> &str {
        "FakeHandler"
    }
}

#[test]
fn test_async_callback_manager_on_llm_start() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_managers = manager.on_llm_start(&HashMap::new(), &["prompt".to_string()], None);

    assert_eq!(run_managers.len(), 1);
    assert!(!run_managers[0].run_id().is_nil());
}

#[test]
fn test_async_callback_manager_on_chain_start() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager
        .on_chain_start()
        .serialized(&HashMap::new())
        .inputs(&HashMap::new())
        .call();

    assert!(!run_manager.run_id().is_nil());
}

#[test]
fn test_async_callback_manager_on_tool_start() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager.on_tool_start(&HashMap::new(), "test", None, None);

    assert!(!run_manager.run_id().is_nil());
}

#[test]
fn test_async_callback_manager_on_llm_end() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_managers = manager.on_llm_start(&HashMap::new(), &["prompt".to_string()], None);

    assert_eq!(run_managers.len(), 1);
    let result = ChatResult::default();
    run_managers[0].on_llm_end(&result);
}

#[test]
fn test_async_callback_manager_on_chain_end() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager
        .on_chain_start()
        .serialized(&HashMap::new())
        .inputs(&HashMap::new())
        .call();

    run_manager.on_chain_end(&HashMap::new());
}

#[test]
fn test_async_callback_manager_on_tool_end() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager.on_tool_start(&HashMap::new(), "test", None, None);

    run_manager.on_tool_end("test");
}

#[test]
fn test_async_callback_manager_on_llm_error() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_managers = manager.on_llm_start(&HashMap::new(), &["prompt".to_string()], None);

    assert_eq!(run_managers.len(), 1);

    let error = std::io::Error::other("test");
    run_managers[0].on_llm_error(&error);
}

#[test]
fn test_async_callback_manager_on_chain_error() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager
        .on_chain_start()
        .serialized(&HashMap::new())
        .inputs(&HashMap::new())
        .call();

    let error = std::io::Error::other("test");
    run_manager.on_chain_error(&error);
}

#[test]
fn test_async_callback_manager_on_tool_error() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager.on_tool_start(&HashMap::new(), "test", None, None);

    let error = std::io::Error::other("test");
    run_manager.on_tool_error(&error);
}

#[test]
fn test_async_callback_manager_on_llm_new_token() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_managers = manager.on_llm_start(&HashMap::new(), &["prompt".to_string()], None);

    assert_eq!(run_managers.len(), 1);
    run_managers[0].on_llm_new_token("test", None);
}

#[test]
fn test_async_callback_manager_with_multiple_handlers() {
    let handler1: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let handler2: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler1, true);
    manager.add_handler(handler2, true);

    assert_eq!(manager.handlers().len(), 2);

    let run_managers = manager.on_llm_start(&HashMap::new(), &["prompt".to_string()], None);

    assert_eq!(run_managers.len(), 1);
    assert_eq!(run_managers[0].handlers().len(), 2);
}

#[test]
fn test_async_callback_manager_add_handler() {
    let handler1: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let handler2: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler1, true);

    assert_eq!(manager.handlers().len(), 1);

    manager.add_handler(handler2, true);

    assert_eq!(manager.handlers().len(), 2);
}

#[test]
fn test_async_callback_manager_remove_handler() {
    let handler1: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let handler2: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler1, true);
    manager.add_handler(handler2.clone(), true);

    assert_eq!(manager.handlers().len(), 2);

    manager.remove_handler(&handler2);

    assert_eq!(manager.handlers().len(), 1);
}

#[test]
fn test_async_callback_manager_inheritable_handlers() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut parent = AsyncCallbackManager::new();
    parent.add_handler(handler, true);

    let chain_run = parent
        .on_chain_start()
        .serialized(&HashMap::new())
        .inputs(&HashMap::new())
        .call();

    let child_manager = chain_run.get_child(None);
    assert_eq!(child_manager.handlers().len(), 1);
}

#[test]
fn test_async_callback_manager_chat_model_start() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let messages = vec![vec![
        HumanMessage::builder().content("Hello").build().into(),
    ]];
    let run_managers = manager.on_chat_model_start(&HashMap::new(), &messages, None, None);

    assert_eq!(run_managers.len(), 1);
    assert!(!run_managers[0].run_id().is_nil());
}

#[test]
fn test_async_callback_manager_ignore_llm() {
    #[derive(Debug)]
    struct IgnoreLLMHandler;

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

    let run_managers = manager.on_llm_start(&HashMap::new(), &["prompt".to_string()], None);

    assert_eq!(run_managers.len(), 1);
}

#[test]
fn test_async_callback_manager_ignore_chain() {
    #[derive(Debug)]
    struct IgnoreChainHandler;

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
        .on_chain_start()
        .serialized(&HashMap::new())
        .inputs(&HashMap::new())
        .call();

    assert!(!run_manager.run_id().is_nil());
}

#[test]
fn test_async_callback_manager_copy() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let manager_copy = manager.clone();
    assert_eq!(manager_copy.handlers().len(), 1);
}

#[test]
fn test_async_callback_manager_chain_child_managers() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let chain_run = manager
        .on_chain_start()
        .serialized(&HashMap::new())
        .inputs(&HashMap::new())
        .call();

    let child_llm_runs =
        chain_run
            .get_child(None)
            .on_llm_start(&HashMap::new(), &["prompt".to_string()], None);
    assert_eq!(child_llm_runs.len(), 1);
    assert!(!child_llm_runs[0].run_id().is_nil());

    let child_chain_run = chain_run
        .get_child(None)
        .on_chain_start()
        .serialized(&HashMap::new())
        .inputs(&HashMap::new())
        .call();
    assert!(!child_chain_run.run_id().is_nil());

    let child_tool_run =
        chain_run
            .get_child(None)
            .on_tool_start(&HashMap::new(), "test", None, None);
    assert!(!child_tool_run.run_id().is_nil());
}

#[test]
fn test_async_callback_manager_retriever_callbacks() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager
        .on_retriever_start()
        .serialized(&HashMap::new())
        .query("test query")
        .call();

    assert!(!run_manager.run_id().is_nil());
    run_manager.on_retriever_end(&[]);
}

#[test]
fn test_async_callback_manager_retriever_error() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager
        .on_retriever_start()
        .serialized(&HashMap::new())
        .query("test query")
        .call();

    let error = std::io::Error::other("test error");
    run_manager.on_retriever_error(&error);
}

#[test]
fn test_async_callback_manager_tags() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);
    manager.add_tags(vec!["test-tag".to_string()], true);

    let run_managers = manager.on_llm_start(&HashMap::new(), &["prompt".to_string()], None);

    assert_eq!(run_managers.len(), 1);
    assert!(!run_managers[0].run_id().is_nil());
}

#[test]
fn test_async_callback_manager_metadata() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);
    manager.add_metadata(
        HashMap::from([("key".to_string(), serde_json::json!("value"))]),
        true,
    );

    let run_managers = manager.on_llm_start(&HashMap::new(), &["prompt".to_string()], None);

    assert_eq!(run_managers.len(), 1);
    assert!(!run_managers[0].run_id().is_nil());
}

#[test]
fn test_async_callback_manager_chain_run_on_agent_action() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let chain_run = manager
        .on_chain_start()
        .serialized(&HashMap::new())
        .inputs(&HashMap::new())
        .call();

    let action = serde_json::json!({
        "tool": "test_tool",
        "tool_input": "test_input",
        "log": "test_log"
    });
    chain_run.on_agent_action(&action);

    let finish = serde_json::json!({
        "return_values": {"output": "test"},
        "log": "test_log"
    });
    chain_run.on_agent_finish(&finish);
}

#[test]
fn test_async_callback_manager_concurrent_runs() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let empty = HashMap::new();
    let runs1 = manager.on_llm_start(&empty, &["prompt1".to_string()], None);
    let runs2 = manager.on_llm_start(&empty, &["prompt2".to_string()], None);
    let runs3 = manager.on_llm_start(&empty, &["prompt3".to_string()], None);

    assert_eq!(runs1.len(), 1);
    assert_eq!(runs2.len(), 1);
    assert_eq!(runs3.len(), 1);

    assert_ne!(runs1[0].run_id(), runs2[0].run_id());
    assert_ne!(runs2[0].run_id(), runs3[0].run_id());

    let result = ChatResult::default();
    runs1[0].on_llm_end(&result);
    runs2[0].on_llm_end(&result);
    runs3[0].on_llm_end(&result);
}

#[test]
fn test_async_callback_manager_full_lifecycle() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let chain_run = manager
        .on_chain_start()
        .serialized(&HashMap::new())
        .inputs(&HashMap::new())
        .call();

    let child_manager = chain_run.get_child(None);
    let llm_runs = child_manager.on_llm_start(&HashMap::new(), &["prompt".to_string()], None);
    assert_eq!(llm_runs.len(), 1);

    llm_runs[0].on_llm_new_token("Hello", None);
    llm_runs[0].on_llm_new_token(" World", None);

    llm_runs[0].on_llm_end(&ChatResult::default());

    let child_manager2 = chain_run.get_child(None);
    let tool_run = child_manager2.on_tool_start(&HashMap::new(), "test", None, None);
    tool_run.on_tool_end("result");

    chain_run.on_chain_end(&HashMap::new());
}
