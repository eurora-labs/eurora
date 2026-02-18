//! Base interfaces for tracing runs.
//!
//! This module provides the BaseTracer and AsyncBaseTracer traits.
//! Mirrors `langchain_core.tracers.base`.

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::messages::BaseMessage;
use crate::outputs::LLMResult;
use crate::tracers::core::{TracerCore, TracerError};
use crate::tracers::schemas::Run;

/// Base interface for tracers.
///
/// This trait extends TracerCore and adds callback handler-like methods
/// for tracking runs of chains, LLMs, tools, and retrievers.
pub trait BaseTracer: TracerCore {
    /// Persist a run (required implementation).
    fn persist_run_impl(&mut self, run: &Run);

    /// Start a trace for a run.
    fn start_trace_impl(&mut self, run: &mut Run) {
        self.start_trace(run);
        self.on_run_create(run);
    }

    /// End a trace for a run.
    fn end_trace_impl(&mut self, run: &Run) {
        if run.parent_run_id.is_none() {
            self.persist_run_impl(run);
        }
        self.end_trace(run);
        self.on_run_update(run);
    }

    /// Handle chat model start.
    #[allow(clippy::too_many_arguments)]
    fn handle_chat_model_start(
        &mut self,
        serialized: HashMap<String, Value>,
        messages: &[Vec<BaseMessage>],
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, Value>>,
        name: Option<String>,
        extra: HashMap<String, Value>,
    ) -> Result<Run, TracerError> {
        let mut chat_model_run = self.create_chat_model_run(
            serialized,
            messages,
            run_id,
            parent_run_id,
            tags,
            metadata,
            name,
            extra,
        )?;
        self.start_trace_impl(&mut chat_model_run);
        self.on_chat_model_start(&chat_model_run);
        Ok(chat_model_run)
    }

    /// Handle LLM start.
    #[allow(clippy::too_many_arguments)]
    fn handle_llm_start(
        &mut self,
        serialized: HashMap<String, Value>,
        prompts: &[String],
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, Value>>,
        name: Option<String>,
        extra: HashMap<String, Value>,
    ) -> Run {
        let mut llm_run = self.create_llm_run(
            serialized,
            prompts,
            run_id,
            parent_run_id,
            tags,
            metadata,
            name,
            extra,
        );
        self.start_trace_impl(&mut llm_run);
        self.on_llm_start(&llm_run);
        llm_run
    }

    /// Handle new LLM token.
    fn handle_llm_new_token(
        &mut self,
        token: &str,
        run_id: Uuid,
        chunk: Option<&dyn std::any::Any>,
        parent_run_id: Option<Uuid>,
    ) -> Result<Run, TracerError> {
        let llm_run = self.llm_run_with_token_event(token, run_id, chunk, parent_run_id)?;
        self.on_llm_new_token(&llm_run, token, chunk);
        Ok(llm_run)
    }

    /// Handle retry event.
    fn handle_retry(
        &mut self,
        retry_state: &HashMap<String, Value>,
        run_id: Uuid,
    ) -> Result<Run, TracerError> {
        self.llm_run_with_retry_event(retry_state, run_id)
    }

    /// Handle LLM end.
    fn handle_llm_end(&mut self, response: &LLMResult, run_id: Uuid) -> Result<Run, TracerError> {
        let llm_run = self.complete_llm_run(response, run_id)?;
        self.end_trace_impl(&llm_run);
        self.on_llm_end(&llm_run);
        Ok(llm_run)
    }

    /// Handle LLM error.
    fn handle_llm_error(
        &mut self,
        error: &dyn std::error::Error,
        run_id: Uuid,
        response: Option<&LLMResult>,
    ) -> Result<Run, TracerError> {
        let llm_run = self.errored_llm_run(error, run_id, response)?;
        self.end_trace_impl(&llm_run);
        self.on_llm_error(&llm_run);
        Ok(llm_run)
    }

    /// Handle chain start.
    #[allow(clippy::too_many_arguments)]
    fn handle_chain_start(
        &mut self,
        serialized: HashMap<String, Value>,
        inputs: HashMap<String, Value>,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, Value>>,
        run_type: Option<String>,
        name: Option<String>,
        extra: HashMap<String, Value>,
    ) -> Run {
        let mut chain_run = self.create_chain_run(
            serialized,
            inputs,
            run_id,
            parent_run_id,
            tags,
            metadata,
            run_type,
            name,
            extra,
        );
        self.start_trace_impl(&mut chain_run);
        self.on_chain_start(&chain_run);
        chain_run
    }

    /// Handle chain end.
    fn handle_chain_end(
        &mut self,
        outputs: HashMap<String, Value>,
        run_id: Uuid,
        inputs: Option<HashMap<String, Value>>,
    ) -> Result<Run, TracerError> {
        let chain_run = self.complete_chain_run(outputs, run_id, inputs)?;
        self.end_trace_impl(&chain_run);
        self.on_chain_end(&chain_run);
        Ok(chain_run)
    }

    /// Handle chain error.
    fn handle_chain_error(
        &mut self,
        error: &dyn std::error::Error,
        run_id: Uuid,
        inputs: Option<HashMap<String, Value>>,
    ) -> Result<Run, TracerError> {
        let chain_run = self.errored_chain_run(error, run_id, inputs)?;
        self.end_trace_impl(&chain_run);
        self.on_chain_error(&chain_run);
        Ok(chain_run)
    }

    /// Handle tool start.
    #[allow(clippy::too_many_arguments)]
    fn handle_tool_start(
        &mut self,
        serialized: HashMap<String, Value>,
        input_str: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, Value>>,
        name: Option<String>,
        inputs: Option<HashMap<String, Value>>,
        extra: HashMap<String, Value>,
    ) -> Run {
        let mut tool_run = self.create_tool_run(
            serialized,
            input_str,
            run_id,
            parent_run_id,
            tags,
            metadata,
            name,
            inputs,
            extra,
        );
        self.start_trace_impl(&mut tool_run);
        self.on_tool_start(&tool_run);
        tool_run
    }

    /// Handle tool end.
    fn handle_tool_end(&mut self, output: Value, run_id: Uuid) -> Result<Run, TracerError> {
        let tool_run = self.complete_tool_run(output, run_id)?;
        self.end_trace_impl(&tool_run);
        self.on_tool_end(&tool_run);
        Ok(tool_run)
    }

    /// Handle tool error.
    fn handle_tool_error(
        &mut self,
        error: &dyn std::error::Error,
        run_id: Uuid,
    ) -> Result<Run, TracerError> {
        let tool_run = self.errored_tool_run(error, run_id)?;
        self.end_trace_impl(&tool_run);
        self.on_tool_error(&tool_run);
        Ok(tool_run)
    }

    /// Handle retriever start.
    #[allow(clippy::too_many_arguments)]
    fn handle_retriever_start(
        &mut self,
        serialized: HashMap<String, Value>,
        query: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, Value>>,
        name: Option<String>,
        extra: HashMap<String, Value>,
    ) -> Run {
        let mut retrieval_run = self.create_retrieval_run(
            serialized,
            query,
            run_id,
            parent_run_id,
            tags,
            metadata,
            name,
            extra,
        );
        self.start_trace_impl(&mut retrieval_run);
        self.on_retriever_start(&retrieval_run);
        retrieval_run
    }

    /// Handle retriever end.
    fn handle_retriever_end(
        &mut self,
        documents: Vec<Value>,
        run_id: Uuid,
    ) -> Result<Run, TracerError> {
        let retrieval_run = self.complete_retrieval_run(documents, run_id)?;
        self.end_trace_impl(&retrieval_run);
        self.on_retriever_end(&retrieval_run);
        Ok(retrieval_run)
    }

    /// Handle retriever error.
    fn handle_retriever_error(
        &mut self,
        error: &dyn std::error::Error,
        run_id: Uuid,
    ) -> Result<Run, TracerError> {
        let retrieval_run = self.errored_retrieval_run(error, run_id)?;
        self.end_trace_impl(&retrieval_run);
        self.on_retriever_error(&retrieval_run);
        Ok(retrieval_run)
    }
}

/// Async base interface for tracers.
///
/// This trait provides async versions of the tracer methods.
#[async_trait]
pub trait AsyncBaseTracer: TracerCore + Send + Sync {
    /// Persist a run asynchronously (required implementation).
    async fn persist_run_async(&mut self, run: &Run);

    /// Start a trace for a run asynchronously.
    async fn start_trace_async(&mut self, run: &mut Run) {
        self.start_trace(run);
        self.on_run_create_async(run).await;
    }

    /// End a trace for a run asynchronously.
    async fn end_trace_async(&mut self, run: &Run) {
        if run.parent_run_id.is_none() {
            self.persist_run_async(run).await;
        }
        self.end_trace(run);
        self.on_run_update_async(run).await;
    }

    /// Handle chat model start asynchronously.
    #[allow(clippy::too_many_arguments)]
    async fn handle_chat_model_start_async(
        &mut self,
        serialized: HashMap<String, Value>,
        messages: &[Vec<BaseMessage>],
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, Value>>,
        name: Option<String>,
        extra: HashMap<String, Value>,
    ) -> Result<Run, TracerError> {
        let mut chat_model_run = self.create_chat_model_run(
            serialized,
            messages,
            run_id,
            parent_run_id,
            tags,
            metadata,
            name,
            extra,
        )?;

        self.start_trace_async(&mut chat_model_run).await;
        self.on_chat_model_start_async(&chat_model_run).await;

        Ok(chat_model_run)
    }

    /// Handle LLM start asynchronously.
    #[allow(clippy::too_many_arguments)]
    async fn handle_llm_start_async(
        &mut self,
        serialized: HashMap<String, Value>,
        prompts: &[String],
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, Value>>,
        name: Option<String>,
        extra: HashMap<String, Value>,
    ) -> Run {
        let mut llm_run = self.create_llm_run(
            serialized,
            prompts,
            run_id,
            parent_run_id,
            tags,
            metadata,
            name,
            extra,
        );

        self.start_trace_async(&mut llm_run).await;
        self.on_llm_start_async(&llm_run).await;

        llm_run
    }

    /// Handle new LLM token asynchronously.
    async fn handle_llm_new_token_async(
        &mut self,
        token: &str,
        run_id: Uuid,
        chunk: Option<&(dyn std::any::Any + Send + Sync)>,
        parent_run_id: Option<Uuid>,
    ) -> Result<Run, TracerError> {
        let llm_run = self.llm_run_with_token_event(
            token,
            run_id,
            chunk.map(|c| c as &dyn std::any::Any),
            parent_run_id,
        )?;
        self.on_llm_new_token_async(&llm_run, token, chunk).await;
        Ok(llm_run)
    }

    /// Handle retry event asynchronously.
    async fn handle_retry_async(
        &mut self,
        retry_state: &HashMap<String, Value>,
        run_id: Uuid,
    ) -> Result<Run, TracerError> {
        self.llm_run_with_retry_event(retry_state, run_id)
    }

    /// Handle LLM end asynchronously.
    async fn handle_llm_end_async(
        &mut self,
        response: &LLMResult,
        run_id: Uuid,
    ) -> Result<Run, TracerError> {
        let llm_run = self.complete_llm_run(response, run_id)?;

        self.on_llm_end_async(&llm_run).await;
        self.end_trace_async(&llm_run).await;

        Ok(llm_run)
    }

    /// Handle LLM error asynchronously.
    async fn handle_llm_error_async(
        &mut self,
        error: &(dyn std::error::Error + Send + Sync),
        run_id: Uuid,
        response: Option<&LLMResult>,
    ) -> Result<Run, TracerError> {
        let llm_run = self.errored_llm_run(error, run_id, response)?;

        self.on_llm_error_async(&llm_run).await;
        self.end_trace_async(&llm_run).await;

        Ok(llm_run)
    }

    /// Handle chain start asynchronously.
    #[allow(clippy::too_many_arguments)]
    async fn handle_chain_start_async(
        &mut self,
        serialized: HashMap<String, Value>,
        inputs: HashMap<String, Value>,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, Value>>,
        run_type: Option<String>,
        name: Option<String>,
        extra: HashMap<String, Value>,
    ) -> Run {
        let mut chain_run = self.create_chain_run(
            serialized,
            inputs,
            run_id,
            parent_run_id,
            tags,
            metadata,
            run_type,
            name,
            extra,
        );

        self.start_trace_async(&mut chain_run).await;
        self.on_chain_start_async(&chain_run).await;

        chain_run
    }

    /// Handle chain end asynchronously.
    async fn handle_chain_end_async(
        &mut self,
        outputs: HashMap<String, Value>,
        run_id: Uuid,
        inputs: Option<HashMap<String, Value>>,
    ) -> Result<Run, TracerError> {
        let chain_run = self.complete_chain_run(outputs, run_id, inputs)?;

        self.end_trace_async(&chain_run).await;
        self.on_chain_end_async(&chain_run).await;

        Ok(chain_run)
    }

    /// Handle chain error asynchronously.
    async fn handle_chain_error_async(
        &mut self,
        error: &(dyn std::error::Error + Send + Sync),
        run_id: Uuid,
        inputs: Option<HashMap<String, Value>>,
    ) -> Result<Run, TracerError> {
        let chain_run = self.errored_chain_run(error, run_id, inputs)?;

        self.end_trace_async(&chain_run).await;
        self.on_chain_error_async(&chain_run).await;

        Ok(chain_run)
    }

    /// Handle tool start asynchronously.
    #[allow(clippy::too_many_arguments)]
    async fn handle_tool_start_async(
        &mut self,
        serialized: HashMap<String, Value>,
        input_str: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, Value>>,
        name: Option<String>,
        inputs: Option<HashMap<String, Value>>,
        extra: HashMap<String, Value>,
    ) -> Run {
        let mut tool_run = self.create_tool_run(
            serialized,
            input_str,
            run_id,
            parent_run_id,
            tags,
            metadata,
            name,
            inputs,
            extra,
        );

        self.start_trace_async(&mut tool_run).await;
        self.on_tool_start_async(&tool_run).await;

        tool_run
    }

    /// Handle tool end asynchronously.
    async fn handle_tool_end_async(
        &mut self,
        output: Value,
        run_id: Uuid,
    ) -> Result<Run, TracerError> {
        let tool_run = self.complete_tool_run(output, run_id)?;

        self.end_trace_async(&tool_run).await;
        self.on_tool_end_async(&tool_run).await;

        Ok(tool_run)
    }

    /// Handle tool error asynchronously.
    async fn handle_tool_error_async(
        &mut self,
        error: &(dyn std::error::Error + Send + Sync),
        run_id: Uuid,
    ) -> Result<Run, TracerError> {
        let tool_run = self.errored_tool_run(error, run_id)?;

        self.end_trace_async(&tool_run).await;
        self.on_tool_error_async(&tool_run).await;

        Ok(tool_run)
    }

    /// Handle retriever start asynchronously.
    #[allow(clippy::too_many_arguments)]
    async fn handle_retriever_start_async(
        &mut self,
        serialized: HashMap<String, Value>,
        query: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, Value>>,
        name: Option<String>,
        extra: HashMap<String, Value>,
    ) -> Run {
        let mut retriever_run = self.create_retrieval_run(
            serialized,
            query,
            run_id,
            parent_run_id,
            tags,
            metadata,
            name,
            extra,
        );

        self.start_trace_async(&mut retriever_run).await;
        self.on_retriever_start_async(&retriever_run).await;

        retriever_run
    }

    /// Handle retriever end asynchronously.
    async fn handle_retriever_end_async(
        &mut self,
        documents: Vec<Value>,
        run_id: Uuid,
    ) -> Result<Run, TracerError> {
        let retrieval_run = self.complete_retrieval_run(documents, run_id)?;

        self.end_trace_async(&retrieval_run).await;
        self.on_retriever_end_async(&retrieval_run).await;

        Ok(retrieval_run)
    }

    /// Handle retriever error asynchronously.
    async fn handle_retriever_error_async(
        &mut self,
        error: &(dyn std::error::Error + Send + Sync),
        run_id: Uuid,
    ) -> Result<Run, TracerError> {
        let retrieval_run = self.errored_retrieval_run(error, run_id)?;

        self.end_trace_async(&retrieval_run).await;
        self.on_retriever_error_async(&retrieval_run).await;

        Ok(retrieval_run)
    }


    /// Called when a run is created (async).
    async fn on_run_create_async(&mut self, _run: &Run) {}

    /// Called when a run is updated (async).
    async fn on_run_update_async(&mut self, _run: &Run) {}

    /// Called when an LLM run starts (async).
    async fn on_llm_start_async(&mut self, _run: &Run) {}

    /// Called when a new LLM token is received (async).
    async fn on_llm_new_token_async(
        &mut self,
        _run: &Run,
        _token: &str,
        _chunk: Option<&(dyn std::any::Any + Send + Sync)>,
    ) {
    }

    /// Called when an LLM run ends (async).
    async fn on_llm_end_async(&mut self, _run: &Run) {}

    /// Called when an LLM run errors (async).
    async fn on_llm_error_async(&mut self, _run: &Run) {}

    /// Called when a chain run starts (async).
    async fn on_chain_start_async(&mut self, _run: &Run) {}

    /// Called when a chain run ends (async).
    async fn on_chain_end_async(&mut self, _run: &Run) {}

    /// Called when a chain run errors (async).
    async fn on_chain_error_async(&mut self, _run: &Run) {}

    /// Called when a tool run starts (async).
    async fn on_tool_start_async(&mut self, _run: &Run) {}

    /// Called when a tool run ends (async).
    async fn on_tool_end_async(&mut self, _run: &Run) {}

    /// Called when a tool run errors (async).
    async fn on_tool_error_async(&mut self, _run: &Run) {}

    /// Called when a chat model run starts (async).
    async fn on_chat_model_start_async(&mut self, _run: &Run) {}

    /// Called when a retriever run starts (async).
    async fn on_retriever_start_async(&mut self, _run: &Run) {}

    /// Called when a retriever run ends (async).
    async fn on_retriever_end_async(&mut self, _run: &Run) {}

    /// Called when a retriever run errors (async).
    async fn on_retriever_error_async(&mut self, _run: &Run) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracers::core::TracerCoreConfig;

    #[derive(Debug)]
    struct TestBaseTracer {
        config: TracerCoreConfig,
        run_map: HashMap<String, Run>,
        order_map: HashMap<Uuid, (Uuid, String)>,
        persisted_runs: Vec<Run>,
    }

    impl TestBaseTracer {
        fn new() -> Self {
            Self {
                config: TracerCoreConfig::default(),
                run_map: HashMap::new(),
                order_map: HashMap::new(),
                persisted_runs: Vec::new(),
            }
        }
    }

    impl TracerCore for TestBaseTracer {
        fn config(&self) -> &TracerCoreConfig {
            &self.config
        }

        fn config_mut(&mut self) -> &mut TracerCoreConfig {
            &mut self.config
        }

        fn run_map(&self) -> &HashMap<String, Run> {
            &self.run_map
        }

        fn run_map_mut(&mut self) -> &mut HashMap<String, Run> {
            &mut self.run_map
        }

        fn order_map(&self) -> &HashMap<Uuid, (Uuid, String)> {
            &self.order_map
        }

        fn order_map_mut(&mut self) -> &mut HashMap<Uuid, (Uuid, String)> {
            &mut self.order_map
        }

        fn persist_run(&mut self, _run: &Run) {}
    }

    impl BaseTracer for TestBaseTracer {
        fn persist_run_impl(&mut self, run: &Run) {
            self.persisted_runs.push(run.clone());
        }
    }

    #[test]
    fn test_base_tracer_chain_lifecycle() {
        let mut tracer = TestBaseTracer::new();

        let run = tracer.handle_chain_start(
            HashMap::new(),
            HashMap::new(),
            Uuid::new_v4(),
            None,
            None,
            None,
            None,
            Some("test_chain".to_string()),
            HashMap::new(),
        );

        assert_eq!(run.name, "test_chain");
        assert_eq!(run.run_type, "chain");
        assert!(tracer.run_map.contains_key(&run.id.to_string()));

        let run_id = run.id;
        let result = tracer.handle_chain_end(
            [("output".to_string(), Value::String("result".to_string()))]
                .into_iter()
                .collect(),
            run_id,
            None,
        );

        assert!(result.is_ok());
        let completed_run = result.unwrap();
        assert!(completed_run.end_time.is_some());
        assert!(!tracer.run_map.contains_key(&run_id.to_string()));
        assert_eq!(tracer.persisted_runs.len(), 1);
    }

    #[test]
    fn test_base_tracer_tool_lifecycle() {
        let mut tracer = TestBaseTracer::new();

        let run = tracer.handle_tool_start(
            HashMap::new(),
            "test input",
            Uuid::new_v4(),
            None,
            None,
            None,
            Some("test_tool".to_string()),
            None,
            HashMap::new(),
        );

        assert_eq!(run.name, "test_tool");
        assert_eq!(run.run_type, "tool");

        let result = tracer.handle_tool_end(Value::String("output".to_string()), run.id);

        assert!(result.is_ok());
    }

    #[test]
    fn test_base_tracer_error_handling() {
        let mut tracer = TestBaseTracer::new();

        let run = tracer.handle_chain_start(
            HashMap::new(),
            HashMap::new(),
            Uuid::new_v4(),
            None,
            None,
            None,
            None,
            None,
            HashMap::new(),
        );

        let error = std::io::Error::other("test error");
        let result = tracer.handle_chain_error(&error, run.id, None);

        assert!(result.is_ok());
        let errored_run = result.unwrap();
        assert!(errored_run.error.is_some());
        assert!(errored_run.end_time.is_some());
    }
}
