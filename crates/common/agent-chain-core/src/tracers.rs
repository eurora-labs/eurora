pub mod base;
pub mod context;
pub mod core;
pub mod evaluation;
pub mod event_stream;
pub mod log_stream;
pub mod memory_stream;
pub mod root_listeners;
pub mod run_collector;
pub mod schemas;
pub mod stdout;
pub mod streaming;

pub use core::{SchemaFormat, TracerCore, TracerCoreConfig, TracerError};

pub use schemas::{Run, RunEvent, RunType};

pub use base::{AsyncBaseTracer, BaseTracer};

pub use streaming::{PassthroughStreamingHandler, StreamingCallbackHandler};

pub use context::{
    ConfigureHook, ConfigureHookRegistry, RunCollectorGuard, TracingCallback, TracingV2Guard,
    collect_runs, get_run_collector, get_tracing_callback, register_configure_hook,
    tracing_v2_enabled, tracing_v2_is_enabled,
};

pub use memory_stream::{MemoryStream, ReceiveStream, SendStream};

pub use log_stream::{
    JsonPatchOp, LogEntry, LogStreamCallbackHandler, LogStreamCallbackHandlerBridge,
    LogStreamConfig, RunLog, RunLogPatch, RunState, astream_log_implementation,
};

pub use root_listeners::{AsyncListener, AsyncRootListenersTracer, Listener, RootListenersTracer};
pub use run_collector::RunCollectorCallbackHandler;
pub use stdout::{ConsoleCallbackHandler, FunctionCallbackHandler, elapsed, try_json_stringify};

pub use event_stream::{AstreamEventsCallbackHandler, RunInfo, astream_events_implementation};

pub use evaluation::{
    EvaluationResult, EvaluatorCallbackHandler, LatencyEvaluator, NonEmptyOutputEvaluator,
    RunEvaluator, wait_for_all_evaluators,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use uuid::Uuid;

    #[test]
    fn test_module_exports() {
        let _run = Run::default();
        let _event = RunEvent::new("test");
        let _config = TracerCoreConfig::default();

        let _collector = RunCollectorCallbackHandler::new(None);
        let _console = ConsoleCallbackHandler::new();
    }

    #[test]
    fn test_tracer_integration() {
        use crate::tracers::base::BaseTracer;

        let mut collector = RunCollectorCallbackHandler::new(None);

        let run = collector.handle_chain_start(
            HashMap::new(),
            HashMap::new(),
            Uuid::new_v4(),
            None,
            Some(vec!["test".to_string()]),
            None,
            None,
            Some("test_chain".to_string()),
            HashMap::new(),
        );

        assert_eq!(run.name, "test_chain");
        assert_eq!(run.run_type, "chain");

        let run_id = run.id;
        let result = collector.handle_chain_end(
            [("result".to_string(), serde_json::json!("success"))]
                .into_iter()
                .collect(),
            run_id,
            None,
        );

        assert!(result.is_ok());
        assert_eq!(collector.traced_runs.len(), 1);
    }

    #[test]
    fn test_run_type_enum() {
        assert_eq!(RunType::Chain.to_string(), "chain");
        assert_eq!(RunType::Llm.to_string(), "llm");
        assert_eq!(RunType::Tool.to_string(), "tool");
        assert_eq!(RunType::Retriever.to_string(), "retriever");
        assert_eq!(RunType::ChatModel.to_string(), "chat_model");

        assert_eq!(RunType::from("chain"), RunType::Chain);
        assert_eq!(RunType::from("llm"), RunType::Llm);
    }

    #[test]
    fn test_schema_format() {
        assert_eq!(SchemaFormat::default(), SchemaFormat::Original);
    }
}
