//! **Tracers** are classes for tracing runs.
//!
//! This module provides the tracing infrastructure for tracking execution of
//! chains, LLMs, tools, and retrievers.
//!
//! # Overview
//!
//! The tracing system consists of:
//!
//! - **Run schema** ([`schemas::Run`]): The main data structure representing a run.
//! - **TracerCore trait** ([`core::TracerCore`]): Base trait with common run management logic.
//! - **BaseTracer trait** ([`base::BaseTracer`]): Synchronous tracer interface.
//! - **AsyncBaseTracer trait** ([`base::AsyncBaseTracer`]): Asynchronous tracer interface.
//! - **Concrete tracers**: Various implementations like [`run_collector::RunCollectorCallbackHandler`]
//!   and [`stdout::ConsoleCallbackHandler`].
//! - **Context management** ([`context`]): Thread-local context for tracers.
//! - **Memory stream** ([`memory_stream`]): Async communication between tasks.
//! - **Log stream** ([`log_stream`]): Run log streaming with JSON patches.
//!
//! # Example
//!
//! ```ignore
//! use agent_chain_core::tracers::{
//!     RunCollectorCallbackHandler, ConsoleCallbackHandler, BaseTracer,
//! };
//!
//! // Create a run collector to gather all runs
//! let mut collector = RunCollectorCallbackHandler::new(None);
//!
//! // Use the tracer during execution...
//!
//! // Access collected runs
//! for run in &collector.traced_runs {
//!     println!("Run: {} ({})", run.name, run.run_type);
//! }
//! ```
//!
//! # Streaming
//!
//! For streaming use cases, the [`streaming::StreamingCallbackHandler`] trait
//! provides methods to tap into output streams.
//!
//! # Context Management
//!
//! Use the [`context`] module to manage tracer context in thread-local storage:
//!
//! ```ignore
//! use agent_chain_core::tracers::context::{tracing_v2_enabled, collect_runs};
//! use agent_chain_core::tracers::RunCollectorCallbackHandler;
//!
//! // Collect runs in the current context
//! let collector = RunCollectorCallbackHandler::new(None);
//! let (_guard, collector_arc) = collect_runs(collector);
//!
//! // ... run some chains ...
//!
//! // Access collected runs
//! let collector = collector_arc.lock().unwrap();
//! for run in &collector.traced_runs {
//!     println!("Run: {}", run.name);
//! }
//! ```
//!
//! Mirrors `langchain_core.tracers`.

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
