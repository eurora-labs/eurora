use agent_chain_core::callbacks::BaseCallbackHandler;
use agent_chain_core::callbacks::manager::{
    CallbackManager, CallbackManagerForChainRun, CallbackManagerForLLMRun,
    CallbackManagerForRetrieverRun, CallbackManagerForToolRun, ParentRunManager, RunManager,
    RunManagerCore,
};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Default)]
struct TestHandler;

impl BaseCallbackHandler for TestHandler {
    fn name(&self) -> &str {
        "TestHandler"
    }
}

#[test]
fn test_base_run_manager_initialization() {
    let run_id = Uuid::new_v4();
    let parent_run_id = Uuid::new_v4();
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);

    let manager = RunManagerCore::builder()
        .run_id(run_id)
        .handlers(vec![handler.clone()])
        .inheritable_handlers(vec![handler])
        .parent_run_id(parent_run_id)
        .tags(vec!["tag1".to_string()])
        .inheritable_tags(vec!["tag2".to_string()])
        .metadata(HashMap::from([(
            "key".to_string(),
            serde_json::json!("value"),
        )]))
        .inheritable_metadata(HashMap::from([(
            "key2".to_string(),
            serde_json::json!("value2"),
        )]))
        .build();

    assert_eq!(manager.run_id(), run_id);
    assert_eq!(manager.parent_run_id(), Some(parent_run_id));
    assert_eq!(manager.handlers().len(), 1);
    assert!(manager.tags().contains(&"tag1".to_string()));
}

#[test]
fn test_base_run_manager_get_noop_manager() {
    let manager = RunManagerCore::noop();

    assert!(!manager.run_id().is_nil());
    assert!(manager.handlers().is_empty());
}

#[test]
fn test_run_manager_on_text() {
    let run_id = Uuid::new_v4();
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);

    let manager = RunManager::new(
        RunManagerCore::builder()
            .run_id(run_id)
            .handlers(vec![handler])
            .build(),
    );

    manager.on_text("Hello");
    manager.on_text("World");
}

#[test]
fn test_run_manager_empty_handlers() {
    let manager = RunManager::new(RunManagerCore::builder().run_id(Uuid::new_v4()).build());

    manager.on_text("test");
}

#[test]
fn test_parent_run_manager_get_child() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let run_id = Uuid::new_v4();

    let parent = ParentRunManager::new(
        RunManagerCore::builder()
            .run_id(run_id)
            .handlers(vec![handler])
            .inheritable_handlers(vec![Arc::new(TestHandler)])
            .tags(vec!["parent_tag".to_string()])
            .inheritable_tags(vec!["inheritable_tag".to_string()])
            .metadata(HashMap::from([(
                "key".to_string(),
                serde_json::json!("value"),
            )]))
            .inheritable_metadata(HashMap::from([(
                "key2".to_string(),
                serde_json::json!("value2"),
            )]))
            .build(),
    );

    let child = parent.get_child(None);

    assert_eq!(child.parent_run_id(), Some(run_id));
    assert!(!child.inheritable_handlers().is_empty());
    assert!(child.tags().contains(&"inheritable_tag".to_string()));
    assert_eq!(
        child.inheritable_metadata()["key2"],
        serde_json::json!("value2")
    );
}

#[test]
fn test_parent_run_manager_get_child_with_tag() {
    let parent = ParentRunManager::new(RunManagerCore::builder().run_id(Uuid::new_v4()).build());

    let child = parent.get_child(Some("child_tag"));

    assert!(child.tags().contains(&"child_tag".to_string()));
    assert!(!child.inheritable_tags().contains(&"child_tag".to_string()));
}

#[test]
fn test_callback_manager_for_llm_run_on_llm_new_token() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let manager = CallbackManagerForLLMRun::new(
        RunManagerCore::builder()
            .run_id(Uuid::new_v4())
            .handlers(vec![handler])
            .build(),
    );

    manager.on_llm_new_token("Hello", None);
    manager.on_llm_new_token(" ", None);
    manager.on_llm_new_token("World", None);
}

#[test]
fn test_callback_manager_for_llm_run_on_llm_end() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let manager = CallbackManagerForLLMRun::new(
        RunManagerCore::builder()
            .run_id(Uuid::new_v4())
            .handlers(vec![handler])
            .build(),
    );

    let result = agent_chain_core::outputs::ChatResult::default();
    manager.on_llm_end(&result);
}

#[test]
fn test_callback_manager_for_llm_run_on_llm_error() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let manager = CallbackManagerForLLMRun::new(
        RunManagerCore::builder()
            .run_id(Uuid::new_v4())
            .handlers(vec![handler])
            .build(),
    );

    let error = std::io::Error::other("Test error");
    manager.on_llm_error(&error);
}

#[test]
fn test_callback_manager_for_chain_run_on_chain_end() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let manager = CallbackManagerForChainRun::new(
        RunManagerCore::builder()
            .run_id(Uuid::new_v4())
            .handlers(vec![handler])
            .build(),
    );

    manager.on_chain_end(&HashMap::from([(
        "result".to_string(),
        serde_json::json!("success"),
    )]));
}

#[test]
fn test_callback_manager_for_chain_run_on_chain_error() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let manager = CallbackManagerForChainRun::new(
        RunManagerCore::builder()
            .run_id(Uuid::new_v4())
            .handlers(vec![handler])
            .build(),
    );

    let error = std::io::Error::other("Chain failed");
    manager.on_chain_error(&error);
}

#[test]
fn test_callback_manager_for_tool_run_on_tool_end() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let manager = CallbackManagerForToolRun::new(
        RunManagerCore::builder()
            .run_id(Uuid::new_v4())
            .handlers(vec![handler])
            .build(),
    );

    manager.on_tool_end("Tool result");
}

#[test]
fn test_callback_manager_for_tool_run_on_tool_error() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let manager = CallbackManagerForToolRun::new(
        RunManagerCore::builder()
            .run_id(Uuid::new_v4())
            .handlers(vec![handler])
            .build(),
    );

    let error = std::io::Error::other("Tool failed");
    manager.on_tool_error(&error);
}

#[test]
fn test_callback_manager_for_retriever_run_on_retriever_end() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let manager = CallbackManagerForRetrieverRun::new(
        RunManagerCore::builder()
            .run_id(Uuid::new_v4())
            .handlers(vec![handler])
            .build(),
    );

    manager.on_retriever_end(&[]);
}

#[test]
fn test_callback_manager_for_retriever_run_on_retriever_error() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let manager = CallbackManagerForRetrieverRun::new(
        RunManagerCore::builder()
            .run_id(Uuid::new_v4())
            .handlers(vec![handler])
            .build(),
    );

    let error = std::io::Error::other("Retriever failed");
    manager.on_retriever_error(&error);
}

#[test]
fn test_callback_manager_on_llm_start_single_prompt() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut manager = CallbackManager::new();
    manager.add_handler(handler, true);

    let run_managers = manager.on_llm_start(&HashMap::new(), &["Test prompt".to_string()], None);

    assert_eq!(run_managers.len(), 1);
    assert!(!run_managers[0].run_id().is_nil());
}

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

#[test]
fn test_callback_manager_on_chain_start() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut manager = CallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager
        .on_chain_start()
        .serialized(&HashMap::new())
        .inputs(&HashMap::new())
        .call();

    assert!(!run_manager.run_id().is_nil());
}

#[test]
fn test_callback_manager_on_tool_start() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut manager = CallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager.on_tool_start(&HashMap::new(), "test input", None, None);

    assert!(!run_manager.run_id().is_nil());
}

#[test]
fn test_callback_manager_on_retriever_start() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut manager = CallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager
        .on_retriever_start()
        .serialized(&HashMap::new())
        .query("search query")
        .call();

    assert!(!run_manager.run_id().is_nil());
}

#[test]
fn test_callback_manager_on_llm_start_sync() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut manager = CallbackManager::new();
    manager.add_handler(handler, true);

    let run_managers = manager.on_llm_start(&HashMap::new(), &["Test prompt".to_string()], None);

    assert_eq!(run_managers.len(), 1);
    assert!(!run_managers[0].run_id().is_nil());
}

#[test]
fn test_callback_manager_on_chain_start_returns_run_manager() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut manager = CallbackManager::new();
    manager.add_handler(handler, true);

    let run_manager = manager
        .on_chain_start()
        .serialized(&HashMap::new())
        .inputs(&HashMap::new())
        .call();

    assert!(!run_manager.run_id().is_nil());
}

#[test]
fn test_run_manager_on_text_sync() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);

    let manager = RunManager::new(
        RunManagerCore::builder()
            .run_id(Uuid::new_v4())
            .handlers(vec![handler])
            .build(),
    );

    manager.on_text("Hello");
    manager.on_text("World");
}

#[test]
fn test_parent_run_manager_get_child_creates_child_manager() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
    let mut manager = CallbackManager::new();
    manager.add_handler(handler, true);

    let chain_run = manager
        .on_chain_start()
        .serialized(&HashMap::new())
        .inputs(&HashMap::new())
        .call();

    let child = chain_run.get_child(None);

    assert_eq!(child.parent_run_id(), Some(chain_run.run_id()));
    assert_eq!(child.handlers().len(), 1);
}
