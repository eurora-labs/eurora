pub mod base;
pub mod file;
pub mod manager;
pub mod stdout;
pub mod streaming_stdout;
pub mod usage;

pub use base::{
    ArcCallbackHandler, AsyncCallbackHandler, BaseCallbackHandler, BaseCallbackManager,
    BoxedCallbackHandler, CallbackManagerMixin, Callbacks, ChainManagerMixin, LLMManagerMixin,
    RetrieverManagerMixin, RunManagerMixin, ToolManagerMixin,
};

pub use manager::{
    AsyncCallbackManager, AsyncCallbackManagerForChainGroup, AsyncCallbackManagerForChainRun,
    AsyncCallbackManagerForLLMRun, AsyncCallbackManagerForRetrieverRun,
    AsyncCallbackManagerForToolRun, AsyncParentRunManager, AsyncRunManager, BaseRunManager,
    CallbackManager, CallbackManagerForChainGroup, CallbackManagerForChainRun,
    CallbackManagerForLLMRun, CallbackManagerForRetrieverRun, CallbackManagerForToolRun,
    ParentRunManager, RunManager, adispatch_custom_event, ahandle_event, atrace_as_chain_group,
    dispatch_custom_event, handle_event, trace_as_chain_group,
};

pub use file::FileCallbackHandler;

pub use stdout::{StdOutCallbackHandler, colors};
pub use streaming_stdout::StreamingStdOutCallbackHandler;

pub use usage::{
    UsageMetadataCallbackGuard, UsageMetadataCallbackHandler, get_usage_metadata_callback,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_callback_system_integration() {
        let stdout_handler = StdOutCallbackHandler::new();
        let streaming_handler = StreamingStdOutCallbackHandler::new();

        let mut manager = CallbackManager::new();
        manager.add_handler(Arc::new(stdout_handler), true);
        manager.add_handler(Arc::new(streaming_handler), true);

        assert_eq!(manager.handlers.len(), 2);
        assert_eq!(manager.inheritable_handlers.len(), 2);
    }

    #[test]
    fn test_callback_manager_chain_lifecycle() {
        let mut manager = CallbackManager::new();
        let handler = StdOutCallbackHandler::new();
        manager.add_handler(Arc::new(handler), true);

        let run_manager = manager
            .on_chain_start()
            .serialized(&std::collections::HashMap::new())
            .inputs(&std::collections::HashMap::new())
            .call();

        assert!(!run_manager.run_id().is_nil());
        assert!(run_manager.parent_run_id().is_none());

        let child_manager = run_manager.get_child(Some("test"));
        assert!(child_manager.tags.contains(&"test".to_string()));
    }

    #[test]
    fn test_callbacks_from_handlers() {
        let handler: Arc<dyn BaseCallbackHandler> = Arc::new(StdOutCallbackHandler::new());
        let callbacks = Callbacks::from_handlers(vec![handler]);

        let manager = callbacks.to_manager();
        assert_eq!(manager.handlers.len(), 1);
    }
}
