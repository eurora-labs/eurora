use agent_chain_core::callbacks::Callbacks;
use agent_chain_core::callbacks::base::{
    BaseCallbackHandler, CallbackManagerMixin, ChainManagerMixin, LLMManagerMixin,
    RetrieverManagerMixin, RunManagerMixin, ToolManagerMixin,
};
use agent_chain_core::callbacks::manager::{
    AsyncCallbackManager, CallbackManager, adispatch_custom_event, dispatch_custom_event,
};
use agent_chain_core::runnables::base::Runnable;
use agent_chain_core::runnables::config::get_callback_manager_for_config;
use agent_chain_core::runnables::{RunnableConfig, RunnableLambdaWithConfig};
use std::sync::Arc;

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

#[test]
fn test_custom_event_root_dispatch() {
    let manager = CallbackManager::new();
    let result = dispatch_custom_event("event1", &serde_json::json!({"x": 1}), &manager);

    assert!(result.is_ok());

    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = CallbackManager::new();
    manager.add_handler(handler, true);

    let result = dispatch_custom_event("event1", &serde_json::json!({"x": 1}), &manager);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("parent run id"));
}

#[tokio::test]
async fn test_async_custom_event_root_dispatch() {
    let manager = AsyncCallbackManager::new();
    let result = adispatch_custom_event("event1", &serde_json::json!({"x": 1}), &manager).await;

    assert!(result.is_ok());

    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);
    let mut manager = AsyncCallbackManager::new();
    manager.add_handler(handler, true);

    let result = adispatch_custom_event("event1", &serde_json::json!({"x": 1}), &manager).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("parent run id"));
}

#[test]
fn test_sync_callback_manager() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);

    let runnable = RunnableLambdaWithConfig::new_with_config(|x: i32, config: &RunnableConfig| {
        let manager = get_callback_manager_for_config(config);
        dispatch_custom_event("event1", &serde_json::json!({"x": x}), &manager)
            .map_err(agent_chain_core::error::Error::other)?;
        dispatch_custom_event("event2", &serde_json::json!({"x": x}), &manager)
            .map_err(agent_chain_core::error::Error::other)?;
        Ok(x)
    });

    let config = RunnableConfig {
        callbacks: Some(Callbacks::from_handlers(vec![handler])),
        ..Default::default()
    };

    let result = runnable.invoke(1, Some(config));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);
}

#[tokio::test]
async fn test_async_callback_manager() {
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);

    let runnable = RunnableLambdaWithConfig::new_with_config(|x: i32, config: &RunnableConfig| {
        let manager = get_callback_manager_for_config(config);
        dispatch_custom_event("event1", &serde_json::json!({"x": x}), &manager)
            .map_err(agent_chain_core::error::Error::other)?;
        dispatch_custom_event("event2", &serde_json::json!({"x": x}), &manager)
            .map_err(agent_chain_core::error::Error::other)?;
        Ok(x)
    });

    let config = RunnableConfig {
        callbacks: Some(Callbacks::from_handlers(vec![handler])),
        ..Default::default()
    };

    let result = runnable.ainvoke(1, Some(config)).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);
}

#[test]
fn test_runnable_lambda_callback_lifecycle() {
    use agent_chain_core::runnables::base::RunnableLambda;

    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);

    let runnable = RunnableLambda::builder().func(|x: i32| Ok(x + 1)).build();

    let config = RunnableConfig {
        callbacks: Some(Callbacks::from_handlers(vec![handler])),
        ..Default::default()
    };

    let result = runnable.invoke(1, Some(config));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 2);
}

#[test]
fn test_runnable_lambda_callback_error_lifecycle() {
    use agent_chain_core::runnables::base::RunnableLambda;

    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(FakeHandler);

    let runnable = RunnableLambda::builder().func(|_x: i32| -> agent_chain_core::error::Result<i32> {
        Err(agent_chain_core::error::Error::other("test error"))
    }).build();

    let config = RunnableConfig {
        callbacks: Some(Callbacks::from_handlers(vec![handler])),
        ..Default::default()
    };

    let result = runnable.invoke(1, Some(config));
    assert!(result.is_err());
}

#[test]
fn test_dispatch_custom_event_no_handlers() {
    let mut manager = CallbackManager::new();
    manager.parent_run_id = Some(uuid::Uuid::nil());

    let result = dispatch_custom_event("event1", &serde_json::json!({"x": 1}), &manager);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_adispatch_custom_event_no_handlers() {
    let manager = AsyncCallbackManager::new();

    let result = adispatch_custom_event("event1", &serde_json::json!({"x": 1}), &manager).await;
    assert!(result.is_ok());
}
