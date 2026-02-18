//! Utilities for the tracer core.
//!
//! This module provides the TracerCore trait with common methods for tracers.
//! Mirrors `langchain_core.tracers.core`.

use std::collections::HashMap;
use std::fmt::Debug;

use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use crate::messages::BaseMessage;
use crate::outputs::{ChatGenerationChunk, GenerationChunk, LLMResult};
use crate::tracers::schemas::{Run, RunEvent};

/// Schema format type for tracers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SchemaFormat {
    /// Original format used by all current tracers.
    #[default]
    Original,
    /// Streaming events format for internal usage.
    StreamingEvents,
    /// Original format with chat model support.
    OriginalChat,
}

/// Configuration for TracerCore.
#[derive(Debug, Clone)]
pub struct TracerCoreConfig {
    /// The schema format to use.
    pub schema_format: SchemaFormat,
    /// Whether to log missing parent warnings.
    pub log_missing_parent: bool,
}

impl Default for TracerCoreConfig {
    fn default() -> Self {
        Self {
            schema_format: SchemaFormat::Original,
            log_missing_parent: true,
        }
    }
}

/// Abstract base trait for tracers.
///
/// This trait provides common methods and reusable methods for tracers.
pub trait TracerCore: Send + Sync + Debug {
    /// Get the configuration for this tracer.
    fn config(&self) -> &TracerCoreConfig;

    /// Get the mutable configuration for this tracer.
    fn config_mut(&mut self) -> &mut TracerCoreConfig;

    /// Get the run map.
    fn run_map(&self) -> &HashMap<String, Run>;

    /// Get the mutable run map.
    fn run_map_mut(&mut self) -> &mut HashMap<String, Run>;

    /// Get the order map (run_id -> (trace_id, dotted_order)).
    fn order_map(&self) -> &HashMap<Uuid, (Uuid, String)>;

    /// Get the mutable order map.
    fn order_map_mut(&mut self) -> &mut HashMap<Uuid, (Uuid, String)>;

    /// Persist a run.
    fn persist_run(&mut self, run: &Run);

    /// Add a child run to a parent run.
    fn add_child_run(&mut self, parent_run: &mut Run, child_run: Run) {
        parent_run.child_runs.push(child_run);
    }

    /// Get the stacktrace of an error.
    fn get_stacktrace(error: &dyn std::error::Error) -> String {
        error.to_string()
    }

    /// Start a trace for a run.
    fn start_trace(&mut self, run: &mut Run) {
        let current_dotted_order =
            format!("{}{}", run.start_time.format("%Y%m%dT%H%M%S%fZ"), run.id);

        if let Some(parent_run_id) = run.parent_run_id {
            if let Some((trace_id, parent_dotted_order)) =
                self.order_map().get(&parent_run_id).cloned()
            {
                run.trace_id = Some(trace_id);
                run.dotted_order =
                    Some(format!("{}.{}", parent_dotted_order, current_dotted_order));

                if let Some(parent_run) = self.run_map_mut().get_mut(&parent_run_id.to_string()) {
                    let child_clone = run.clone();
                    parent_run.child_runs.push(child_clone);
                }
            } else {
                if self.config().log_missing_parent {
                    tracing::debug!(
                        "Parent run {} not found for run {}. Treating as a root run.",
                        parent_run_id,
                        run.id
                    );
                }
                run.parent_run_id = None;
                run.trace_id = Some(run.id);
                run.dotted_order = Some(current_dotted_order.clone());
            }
        } else {
            run.trace_id = Some(run.id);
            run.dotted_order = Some(current_dotted_order.clone());
        }

        let trace_id = run.trace_id.unwrap_or(run.id);
        let dotted_order = run.dotted_order.clone().unwrap_or(current_dotted_order);

        self.order_map_mut()
            .insert(run.id, (trace_id, dotted_order));
        self.run_map_mut().insert(run.id.to_string(), run.clone());
    }

    /// End a trace for a run.
    fn end_trace(&mut self, run: &Run) {
        self.run_map_mut().remove(&run.id.to_string());
    }

    /// Get a run by ID.
    fn get_run(&self, run_id: Uuid, run_type: Option<&[&str]>) -> Result<Run, TracerError> {
        let run = self
            .run_map()
            .get(&run_id.to_string())
            .cloned()
            .ok_or(TracerError::RunNotFound(run_id))?;

        if let Some(expected_types) = run_type
            && !expected_types.contains(&run.run_type.as_str())
        {
            return Err(TracerError::WrongRunType {
                run_id,
                expected: expected_types.iter().map(|s| s.to_string()).collect(),
                actual: run.run_type.clone(),
            });
        }

        Ok(run)
    }

    /// Get a mutable run by ID.
    fn get_run_mut(&mut self, run_id: Uuid) -> Option<&mut Run> {
        self.run_map_mut().get_mut(&run_id.to_string())
    }

    /// Create a chat model run.
    #[allow(clippy::too_many_arguments)]
    fn create_chat_model_run(
        &self,
        serialized: HashMap<String, Value>,
        messages: &[Vec<BaseMessage>],
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, Value>>,
        name: Option<String>,
        extra: HashMap<String, Value>,
    ) -> Result<Run, TracerError> {
        let schema_format = self.config().schema_format;
        if schema_format != SchemaFormat::StreamingEvents
            && schema_format != SchemaFormat::OriginalChat
        {
            return Err(TracerError::UnsupportedSchemaFormat(
                "Chat model tracing is not supported in original format".to_string(),
            ));
        }

        let start_time = Utc::now();
        let mut run_extra = extra;
        if let Some(meta) = metadata {
            run_extra.insert(
                "metadata".to_string(),
                serde_json::to_value(meta).unwrap_or_default(),
            );
        }

        let inputs: HashMap<String, Value> = [(
            "messages".to_string(),
            serde_json::to_value(
                messages
                    .iter()
                    .map(|batch| {
                        batch
                            .iter()
                            .map(|msg| serde_json::to_value(msg).unwrap_or_default())
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_default(),
        )]
        .into_iter()
        .collect();

        let run = Run {
            id: run_id,
            name: name.unwrap_or_else(|| "ChatModel".to_string()),
            run_type: "chat_model".to_string(),
            parent_run_id,
            trace_id: None,
            dotted_order: None,
            start_time,
            end_time: None,
            inputs,
            outputs: None,
            error: None,
            serialized,
            extra: run_extra,
            events: vec![RunEvent::with_time("start", start_time)],
            tags,
            child_runs: Vec::new(),
            session_name: None,
            reference_example_id: None,
        };

        Ok(run)
    }

    /// Create an LLM run.
    #[allow(clippy::too_many_arguments)]
    fn create_llm_run(
        &self,
        serialized: HashMap<String, Value>,
        prompts: &[String],
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, Value>>,
        name: Option<String>,
        extra: HashMap<String, Value>,
    ) -> Run {
        let start_time = Utc::now();
        let mut run_extra = extra;
        if let Some(meta) = metadata {
            run_extra.insert(
                "metadata".to_string(),
                serde_json::to_value(meta).unwrap_or_default(),
            );
        }

        let inputs: HashMap<String, Value> = [(
            "prompts".to_string(),
            serde_json::to_value(prompts).unwrap_or_default(),
        )]
        .into_iter()
        .collect();

        Run {
            id: run_id,
            name: name.unwrap_or_else(|| "LLM".to_string()),
            run_type: "llm".to_string(),
            parent_run_id,
            trace_id: None,
            dotted_order: None,
            start_time,
            end_time: None,
            inputs,
            outputs: None,
            error: None,
            serialized,
            extra: run_extra,
            events: vec![RunEvent::with_time("start", start_time)],
            tags: Some(tags.unwrap_or_default()),
            child_runs: Vec::new(),
            session_name: None,
            reference_example_id: None,
        }
    }

    /// Process an LLM run with a new token event.
    fn llm_run_with_token_event(
        &mut self,
        token: &str,
        run_id: Uuid,
        chunk: Option<&dyn std::any::Any>,
        _parent_run_id: Option<Uuid>,
    ) -> Result<Run, TracerError> {
        let run = self
            .run_map_mut()
            .get_mut(&run_id.to_string())
            .ok_or(TracerError::RunNotFound(run_id))?;

        if run.run_type != "llm" && run.run_type != "chat_model" {
            return Err(TracerError::WrongRunType {
                run_id,
                expected: vec!["llm".to_string(), "chat_model".to_string()],
                actual: run.run_type.clone(),
            });
        }

        let mut event_kwargs: HashMap<String, Value> = HashMap::new();
        event_kwargs.insert("token".to_string(), Value::String(token.to_string()));

        if let Some(chunk_any) = chunk {
            if let Some(gen_chunk) = chunk_any.downcast_ref::<GenerationChunk>() {
                event_kwargs.insert(
                    "chunk".to_string(),
                    serde_json::to_value(gen_chunk).unwrap_or_default(),
                );
            } else if let Some(chat_chunk) = chunk_any.downcast_ref::<ChatGenerationChunk>() {
                event_kwargs.insert(
                    "chunk".to_string(),
                    serde_json::to_value(chat_chunk).unwrap_or_default(),
                );
            }
        }

        run.events
            .push(RunEvent::with_kwargs("new_token", event_kwargs));

        Ok(run.clone())
    }

    /// Process an LLM run with a retry event.
    fn llm_run_with_retry_event(
        &mut self,
        retry_state: &HashMap<String, Value>,
        run_id: Uuid,
    ) -> Result<Run, TracerError> {
        let run = self
            .run_map_mut()
            .get_mut(&run_id.to_string())
            .ok_or(TracerError::RunNotFound(run_id))?;

        run.events
            .push(RunEvent::with_kwargs("retry", retry_state.clone()));

        Ok(run.clone())
    }

    /// Complete an LLM run.
    fn complete_llm_run(&mut self, response: &LLMResult, run_id: Uuid) -> Result<Run, TracerError> {
        let run = self
            .run_map_mut()
            .get_mut(&run_id.to_string())
            .ok_or(TracerError::RunNotFound(run_id))?;

        if run.run_type != "llm" && run.run_type != "chat_model" {
            return Err(TracerError::WrongRunType {
                run_id,
                expected: vec!["llm".to_string(), "chat_model".to_string()],
                actual: run.run_type.clone(),
            });
        }

        if run.outputs.is_none() {
            run.outputs = Some(HashMap::new());
        }

        let omit_outputs = run
            .extra
            .get("__omit_auto_outputs")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !omit_outputs
            && let Some(outputs) = &mut run.outputs
            && let Ok(Value::Object(map)) = serde_json::to_value(response)
        {
            for (k, v) in map {
                outputs.insert(k, v);
            }
        }

        run.end_time = Some(Utc::now());
        run.events.push(RunEvent::with_time(
            "end",
            run.end_time.expect("end_time set before on_end"),
        ));

        Ok(run.clone())
    }

    /// Mark an LLM run as errored.
    fn errored_llm_run(
        &mut self,
        error: &dyn std::error::Error,
        run_id: Uuid,
        response: Option<&LLMResult>,
    ) -> Result<Run, TracerError> {
        let run = self
            .run_map_mut()
            .get_mut(&run_id.to_string())
            .ok_or(TracerError::RunNotFound(run_id))?;

        if run.run_type != "llm" && run.run_type != "chat_model" {
            return Err(TracerError::WrongRunType {
                run_id,
                expected: vec!["llm".to_string(), "chat_model".to_string()],
                actual: run.run_type.clone(),
            });
        }

        run.error = Some(Self::get_stacktrace(error));

        if let Some(resp) = response {
            if run.outputs.is_none() {
                run.outputs = Some(HashMap::new());
            }

            let omit_outputs = run
                .extra
                .get("__omit_auto_outputs")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if !omit_outputs
                && let Some(outputs) = &mut run.outputs
                && let Ok(Value::Object(map)) = serde_json::to_value(resp)
            {
                for (k, v) in map {
                    outputs.insert(k, v);
                }
            }
        }

        run.end_time = Some(Utc::now());
        run.events.push(RunEvent::with_time(
            "error",
            run.end_time.expect("end_time set before on_end"),
        ));

        Ok(run.clone())
    }

    /// Create a chain run.
    #[allow(clippy::too_many_arguments)]
    fn create_chain_run(
        &self,
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
        let start_time = Utc::now();
        let mut run_extra = extra;
        if let Some(meta) = metadata {
            run_extra.insert(
                "metadata".to_string(),
                serde_json::to_value(meta).unwrap_or_default(),
            );
        }

        let processed_inputs = self.get_chain_inputs(inputs);

        Run {
            id: run_id,
            name: name.unwrap_or_else(|| "Chain".to_string()),
            run_type: run_type.unwrap_or_else(|| "chain".to_string()),
            parent_run_id,
            trace_id: None,
            dotted_order: None,
            start_time,
            end_time: None,
            inputs: processed_inputs,
            outputs: None,
            error: None,
            serialized,
            extra: run_extra,
            events: vec![RunEvent::with_time("start", start_time)],
            tags: Some(tags.unwrap_or_default()),
            child_runs: Vec::new(),
            session_name: None,
            reference_example_id: None,
        }
    }

    /// Get chain inputs based on schema format.
    fn get_chain_inputs(&self, inputs: HashMap<String, Value>) -> HashMap<String, Value> {
        match self.config().schema_format {
            SchemaFormat::Original | SchemaFormat::OriginalChat => inputs,
            SchemaFormat::StreamingEvents => [(
                "input".to_string(),
                serde_json::to_value(inputs).unwrap_or_default(),
            )]
            .into_iter()
            .collect(),
        }
    }

    /// Get chain outputs based on schema format.
    fn get_chain_outputs(&self, outputs: HashMap<String, Value>) -> HashMap<String, Value> {
        match self.config().schema_format {
            SchemaFormat::Original | SchemaFormat::OriginalChat => outputs,
            SchemaFormat::StreamingEvents => [(
                "output".to_string(),
                serde_json::to_value(outputs).unwrap_or_default(),
            )]
            .into_iter()
            .collect(),
        }
    }

    /// Complete a chain run.
    fn complete_chain_run(
        &mut self,
        outputs: HashMap<String, Value>,
        run_id: Uuid,
        inputs: Option<HashMap<String, Value>>,
    ) -> Result<Run, TracerError> {
        let processed_outputs = self.get_chain_outputs(outputs);
        let processed_inputs = inputs.map(|i| self.get_chain_inputs(i));

        let run = self
            .run_map_mut()
            .get_mut(&run_id.to_string())
            .ok_or(TracerError::RunNotFound(run_id))?;

        if run.outputs.is_none() {
            run.outputs = Some(HashMap::new());
        }

        let omit_outputs = run
            .extra
            .get("__omit_auto_outputs")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !omit_outputs && let Some(outputs) = &mut run.outputs {
            outputs.extend(processed_outputs);
        }

        run.end_time = Some(Utc::now());
        run.events.push(RunEvent::with_time(
            "end",
            run.end_time.expect("end_time set before on_end"),
        ));

        if let Some(inputs) = processed_inputs {
            run.inputs = inputs;
        }

        Ok(run.clone())
    }

    /// Mark a chain run as errored.
    fn errored_chain_run(
        &mut self,
        error: &dyn std::error::Error,
        run_id: Uuid,
        inputs: Option<HashMap<String, Value>>,
    ) -> Result<Run, TracerError> {
        let processed_inputs = inputs.map(|i| self.get_chain_inputs(i));

        let run = self
            .run_map_mut()
            .get_mut(&run_id.to_string())
            .ok_or(TracerError::RunNotFound(run_id))?;

        run.error = Some(Self::get_stacktrace(error));
        run.end_time = Some(Utc::now());
        run.events.push(RunEvent::with_time(
            "error",
            run.end_time.expect("end_time set before on_end"),
        ));

        if let Some(inputs) = processed_inputs {
            run.inputs = inputs;
        }

        Ok(run.clone())
    }

    /// Create a tool run.
    #[allow(clippy::too_many_arguments)]
    fn create_tool_run(
        &self,
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
        let start_time = Utc::now();
        let mut run_extra = extra;
        if let Some(meta) = metadata {
            run_extra.insert(
                "metadata".to_string(),
                serde_json::to_value(meta).unwrap_or_default(),
            );
        }

        let processed_inputs = match self.config().schema_format {
            SchemaFormat::Original | SchemaFormat::OriginalChat => {
                [("input".to_string(), Value::String(input_str.to_string()))]
                    .into_iter()
                    .collect()
            }
            SchemaFormat::StreamingEvents => [(
                "input".to_string(),
                serde_json::to_value(inputs).unwrap_or_default(),
            )]
            .into_iter()
            .collect(),
        };

        Run {
            id: run_id,
            name: name.unwrap_or_else(|| "Tool".to_string()),
            run_type: "tool".to_string(),
            parent_run_id,
            trace_id: None,
            dotted_order: None,
            start_time,
            end_time: None,
            inputs: processed_inputs,
            outputs: None,
            error: None,
            serialized,
            extra: run_extra,
            events: vec![RunEvent::with_time("start", start_time)],
            tags: Some(tags.unwrap_or_default()),
            child_runs: Vec::new(),
            session_name: None,
            reference_example_id: None,
        }
    }

    /// Complete a tool run.
    fn complete_tool_run(&mut self, output: Value, run_id: Uuid) -> Result<Run, TracerError> {
        let run = self
            .run_map_mut()
            .get_mut(&run_id.to_string())
            .ok_or(TracerError::RunNotFound(run_id))?;

        if run.run_type != "tool" {
            return Err(TracerError::WrongRunType {
                run_id,
                expected: vec!["tool".to_string()],
                actual: run.run_type.clone(),
            });
        }

        if run.outputs.is_none() {
            run.outputs = Some(HashMap::new());
        }

        let omit_outputs = run
            .extra
            .get("__omit_auto_outputs")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !omit_outputs && let Some(outputs) = &mut run.outputs {
            outputs.insert("output".to_string(), output);
        }

        run.end_time = Some(Utc::now());
        run.events.push(RunEvent::with_time(
            "end",
            run.end_time.expect("end_time set before on_end"),
        ));

        Ok(run.clone())
    }

    /// Mark a tool run as errored.
    fn errored_tool_run(
        &mut self,
        error: &dyn std::error::Error,
        run_id: Uuid,
    ) -> Result<Run, TracerError> {
        let run = self
            .run_map_mut()
            .get_mut(&run_id.to_string())
            .ok_or(TracerError::RunNotFound(run_id))?;

        if run.run_type != "tool" {
            return Err(TracerError::WrongRunType {
                run_id,
                expected: vec!["tool".to_string()],
                actual: run.run_type.clone(),
            });
        }

        run.error = Some(Self::get_stacktrace(error));
        run.end_time = Some(Utc::now());
        run.events.push(RunEvent::with_time(
            "error",
            run.end_time.expect("end_time set before on_end"),
        ));

        Ok(run.clone())
    }

    /// Create a retrieval run.
    #[allow(clippy::too_many_arguments)]
    fn create_retrieval_run(
        &self,
        serialized: HashMap<String, Value>,
        query: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, Value>>,
        name: Option<String>,
        extra: HashMap<String, Value>,
    ) -> Run {
        let start_time = Utc::now();
        let mut run_extra = extra;
        if let Some(meta) = metadata {
            run_extra.insert(
                "metadata".to_string(),
                serde_json::to_value(meta).unwrap_or_default(),
            );
        }

        let inputs: HashMap<String, Value> =
            [("query".to_string(), Value::String(query.to_string()))]
                .into_iter()
                .collect();

        Run {
            id: run_id,
            name: name.unwrap_or_else(|| "Retriever".to_string()),
            run_type: "retriever".to_string(),
            parent_run_id,
            trace_id: None,
            dotted_order: None,
            start_time,
            end_time: None,
            inputs,
            outputs: None,
            error: None,
            serialized,
            extra: run_extra,
            events: vec![RunEvent::with_time("start", start_time)],
            tags,
            child_runs: Vec::new(),
            session_name: None,
            reference_example_id: None,
        }
    }

    /// Complete a retrieval run.
    fn complete_retrieval_run(
        &mut self,
        documents: Vec<Value>,
        run_id: Uuid,
    ) -> Result<Run, TracerError> {
        let run = self
            .run_map_mut()
            .get_mut(&run_id.to_string())
            .ok_or(TracerError::RunNotFound(run_id))?;

        if run.run_type != "retriever" {
            return Err(TracerError::WrongRunType {
                run_id,
                expected: vec!["retriever".to_string()],
                actual: run.run_type.clone(),
            });
        }

        if run.outputs.is_none() {
            run.outputs = Some(HashMap::new());
        }

        let omit_outputs = run
            .extra
            .get("__omit_auto_outputs")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !omit_outputs && let Some(outputs) = &mut run.outputs {
            outputs.insert("documents".to_string(), Value::Array(documents));
        }

        run.end_time = Some(Utc::now());
        run.events.push(RunEvent::with_time(
            "end",
            run.end_time.expect("end_time set before on_end"),
        ));

        Ok(run.clone())
    }

    /// Mark a retrieval run as errored.
    fn errored_retrieval_run(
        &mut self,
        error: &dyn std::error::Error,
        run_id: Uuid,
    ) -> Result<Run, TracerError> {
        let run = self
            .run_map_mut()
            .get_mut(&run_id.to_string())
            .ok_or(TracerError::RunNotFound(run_id))?;

        if run.run_type != "retriever" {
            return Err(TracerError::WrongRunType {
                run_id,
                expected: vec!["retriever".to_string()],
                actual: run.run_type.clone(),
            });
        }

        run.error = Some(Self::get_stacktrace(error));
        run.end_time = Some(Utc::now());
        run.events.push(RunEvent::with_time(
            "error",
            run.end_time.expect("end_time set before on_end"),
        ));

        Ok(run.clone())
    }

    /// Called when a run is created.
    fn on_run_create(&mut self, _run: &Run) {}

    /// Called when a run is updated.
    fn on_run_update(&mut self, _run: &Run) {}

    /// Called when an LLM run starts.
    fn on_llm_start(&mut self, _run: &Run) {}

    /// Called when a new LLM token is received.
    fn on_llm_new_token(&mut self, _run: &Run, _token: &str, _chunk: Option<&dyn std::any::Any>) {}

    /// Called when an LLM run ends.
    fn on_llm_end(&mut self, _run: &Run) {}

    /// Called when an LLM run errors.
    fn on_llm_error(&mut self, _run: &Run) {}

    /// Called when a chain run starts.
    fn on_chain_start(&mut self, _run: &Run) {}

    /// Called when a chain run ends.
    fn on_chain_end(&mut self, _run: &Run) {}

    /// Called when a chain run errors.
    fn on_chain_error(&mut self, _run: &Run) {}

    /// Called when a tool run starts.
    fn on_tool_start(&mut self, _run: &Run) {}

    /// Called when a tool run ends.
    fn on_tool_end(&mut self, _run: &Run) {}

    /// Called when a tool run errors.
    fn on_tool_error(&mut self, _run: &Run) {}

    /// Called when a chat model run starts.
    fn on_chat_model_start(&mut self, _run: &Run) {}

    /// Called when a retriever run starts.
    fn on_retriever_start(&mut self, _run: &Run) {}

    /// Called when a retriever run ends.
    fn on_retriever_end(&mut self, _run: &Run) {}

    /// Called when a retriever run errors.
    fn on_retriever_error(&mut self, _run: &Run) {}
}

/// Error type for tracer operations.
#[derive(Debug, Clone)]
pub enum TracerError {
    /// Run not found.
    RunNotFound(Uuid),
    /// Wrong run type.
    WrongRunType {
        run_id: Uuid,
        expected: Vec<String>,
        actual: String,
    },
    /// Unsupported schema format.
    UnsupportedSchemaFormat(String),
}

impl std::fmt::Display for TracerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TracerError::RunNotFound(id) => write!(f, "No indexed run ID {}", id),
            TracerError::WrongRunType {
                run_id,
                expected,
                actual,
            } => write!(
                f,
                "Found {} run at ID {}, but expected {:?} run",
                actual, run_id, expected
            ),
            TracerError::UnsupportedSchemaFormat(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for TracerError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestTracer {
        config: TracerCoreConfig,
        run_map: HashMap<String, Run>,
        order_map: HashMap<Uuid, (Uuid, String)>,
    }

    impl TestTracer {
        fn new() -> Self {
            Self {
                config: TracerCoreConfig::default(),
                run_map: HashMap::new(),
                order_map: HashMap::new(),
            }
        }
    }

    impl TracerCore for TestTracer {
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

    #[test]
    fn test_create_chain_run() {
        let tracer = TestTracer::new();
        let run = tracer.create_chain_run(
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

        assert_eq!(run.run_type, "chain");
        assert!(run.end_time.is_none());
        assert!(!run.events.is_empty());
    }

    #[test]
    fn test_start_trace() {
        let mut tracer = TestTracer::new();
        let mut run = tracer.create_chain_run(
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

        tracer.start_trace(&mut run);

        assert!(run.trace_id.is_some());
        assert!(run.dotted_order.is_some());
        assert!(tracer.run_map.contains_key(&run.id.to_string()));
    }

    #[test]
    fn test_complete_chain_run() {
        let mut tracer = TestTracer::new();
        let mut run = tracer.create_chain_run(
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

        tracer.start_trace(&mut run);
        let run_id = run.id;

        let result = tracer.complete_chain_run(
            [("result".to_string(), Value::String("success".to_string()))]
                .into_iter()
                .collect(),
            run_id,
            None,
        );

        assert!(result.is_ok());
        let completed_run = result.unwrap();
        assert!(completed_run.end_time.is_some());
        assert!(completed_run.outputs.is_some());
    }

    #[test]
    fn test_get_run_not_found() {
        let tracer = TestTracer::new();
        let result = tracer.get_run(Uuid::new_v4(), None);
        assert!(result.is_err());
    }
}
