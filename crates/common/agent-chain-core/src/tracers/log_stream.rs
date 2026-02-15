//! Tracer that streams run logs to a stream.
//!
//! This module provides a tracer that streams run logs using JSON patches.
//! Mirrors `langchain_core.tracers.log_stream`.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::tracers::base::BaseTracer;
use crate::tracers::core::{SchemaFormat, TracerCore, TracerCoreConfig};
use crate::tracers::memory_stream::{MemoryStream, ReceiveStream, SendStream};
use crate::tracers::schemas::Run;
use crate::tracers::streaming::StreamingCallbackHandler;

/// A single entry in the run log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// ID of the sub-run.
    pub id: String,
    /// Name of the object being run.
    pub name: String,
    /// Type of the object being run, eg. prompt, chain, llm, etc.
    #[serde(rename = "type")]
    pub run_type: String,
    /// List of tags for the run.
    pub tags: Vec<String>,
    /// Key-value pairs of metadata for the run.
    pub metadata: HashMap<String, Value>,
    /// ISO-8601 timestamp of when the run started.
    pub start_time: String,
    /// List of LLM tokens streamed by this run, if applicable.
    pub streamed_output_str: Vec<String>,
    /// List of output chunks streamed by this run, if available.
    pub streamed_output: Vec<Value>,
    /// Inputs to this run. Not available currently via astream_log.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Value>,
    /// Final output of this run. Only available after the run has finished successfully.
    pub final_output: Option<Value>,
    /// ISO-8601 timestamp of when the run ended. Only available after the run has finished.
    pub end_time: Option<String>,
}

impl LogEntry {
    /// Create a new log entry.
    pub fn new(
        id: String,
        name: String,
        run_type: String,
        tags: Vec<String>,
        metadata: HashMap<String, Value>,
        start_time: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            run_type,
            tags,
            metadata,
            start_time: start_time.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
            streamed_output_str: Vec::new(),
            streamed_output: Vec::new(),
            inputs: None,
            final_output: None,
            end_time: None,
        }
    }
}

/// State of the run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunState {
    /// ID of the run.
    pub id: String,
    /// List of output chunks streamed by Runnable.stream()
    pub streamed_output: Vec<Value>,
    /// Final output of the run, usually the result of aggregating streamed_output.
    pub final_output: Option<Value>,
    /// Name of the object being run.
    pub name: String,
    /// Type of the object being run, eg. prompt, chain, llm, etc.
    #[serde(rename = "type")]
    pub run_type: String,
    /// Map of run names to sub-runs.
    pub logs: HashMap<String, LogEntry>,
}

impl RunState {
    /// Create a new run state.
    pub fn new(id: String, name: String, run_type: String) -> Self {
        Self {
            id,
            streamed_output: Vec::new(),
            final_output: None,
            name,
            run_type,
            logs: HashMap::new(),
        }
    }
}

/// A JSON patch operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonPatchOp {
    /// The operation type (add, replace, remove, etc.)
    pub op: String,
    /// The path to apply the operation to.
    pub path: String,
    /// The value for the operation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
}

impl JsonPatchOp {
    /// Create a new add operation.
    pub fn add(path: impl Into<String>, value: Value) -> Self {
        Self {
            op: "add".to_string(),
            path: path.into(),
            value: Some(value),
        }
    }

    /// Create a new replace operation.
    pub fn replace(path: impl Into<String>, value: Value) -> Self {
        Self {
            op: "replace".to_string(),
            path: path.into(),
            value: Some(value),
        }
    }

    /// Create a new remove operation.
    pub fn remove(path: impl Into<String>) -> Self {
        Self {
            op: "remove".to_string(),
            path: path.into(),
            value: None,
        }
    }
}

/// Patch to the run log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunLogPatch {
    /// List of JSONPatch operations.
    pub ops: Vec<JsonPatchOp>,
}

impl RunLogPatch {
    /// Create a new run log patch.
    pub fn new(ops: Vec<JsonPatchOp>) -> Self {
        Self { ops }
    }

    /// Create a patch from a single operation.
    pub fn from_op(op: JsonPatchOp) -> Self {
        Self { ops: vec![op] }
    }

    /// Create a patch from multiple operations.
    pub fn from_ops(ops: impl IntoIterator<Item = JsonPatchOp>) -> Self {
        Self {
            ops: ops.into_iter().collect(),
        }
    }
}

impl fmt::Display for RunLogPatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RunLogPatch({:?})", self.ops)
    }
}

/// Run log with full state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunLog {
    /// List of JSONPatch operations.
    pub ops: Vec<JsonPatchOp>,
    /// Current state of the log.
    pub state: Option<RunState>,
}

impl RunLog {
    /// Create a new run log.
    pub fn new(ops: Vec<JsonPatchOp>, state: Option<RunState>) -> Self {
        Self { ops, state }
    }

    /// Apply a patch to the run log.
    pub fn apply_patch(&mut self, patch: RunLogPatch) {
        self.ops.extend(patch.ops.clone());

        // Apply the operations to the state
        if let Some(ref mut state) = self.state {
            for op in patch.ops {
                Self::apply_op_to_state(state, &op);
            }
        }
    }

    fn apply_op_to_state(state: &mut RunState, op: &JsonPatchOp) {
        let path_parts: Vec<&str> = op.path.split('/').filter(|s| !s.is_empty()).collect();

        match op.op.as_str() {
            "replace" => {
                if op.path.is_empty() || op.path == "/" {
                    // Replace entire state
                    if let Some(value) = &op.value
                        && let Ok(new_state) = serde_json::from_value::<RunState>(value.clone())
                    {
                        *state = new_state;
                    }
                } else if path_parts.first() == Some(&"final_output") {
                    state.final_output = op.value.clone();
                }
            }
            "add" => {
                if path_parts.len() >= 2 {
                    match path_parts[0] {
                        "logs" => {
                            if path_parts.len() == 2 {
                                // Adding a new log entry
                                if let Some(value) = &op.value
                                    && let Ok(entry) =
                                        serde_json::from_value::<LogEntry>(value.clone())
                                {
                                    state.logs.insert(path_parts[1].to_string(), entry);
                                }
                            } else if path_parts.len() >= 3 {
                                // Updating an existing log entry field
                                if let Some(entry) = state.logs.get_mut(path_parts[1]) {
                                    match path_parts[2] {
                                        "streamed_output"
                                            if path_parts.len() == 4 && path_parts[3] == "-" =>
                                        {
                                            if let Some(value) = &op.value {
                                                entry.streamed_output.push(value.clone());
                                            }
                                        }
                                        "streamed_output_str"
                                            if path_parts.len() == 4 && path_parts[3] == "-" =>
                                        {
                                            if let Some(value) = &op.value
                                                && let Some(s) = value.as_str()
                                            {
                                                entry.streamed_output_str.push(s.to_string());
                                            }
                                        }
                                        "final_output" => {
                                            entry.final_output = op.value.clone();
                                        }
                                        "end_time" => {
                                            entry.end_time = op
                                                .value
                                                .clone()
                                                .and_then(|v| v.as_str().map(String::from));
                                        }
                                        "inputs" => {
                                            entry.inputs = op.value.clone();
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        "streamed_output" if path_parts.len() == 2 && path_parts[1] == "-" => {
                            if let Some(value) = &op.value {
                                state.streamed_output.push(value.clone());
                            }
                        }
                        "final_output" => {
                            state.final_output = op.value.clone();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

impl fmt::Display for RunLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RunLog({:?})", self.state)
    }
}

/// Tracer that streams run logs to a stream.
pub struct LogStreamCallbackHandler {
    /// The tracer configuration.
    config: TracerCoreConfig,
    /// The run map.
    run_map: HashMap<String, Run>,
    /// The order map.
    order_map: HashMap<Uuid, (Uuid, String)>,
    /// Whether to auto-close the stream when the root run finishes.
    auto_close: bool,
    /// Only include runs from Runnables with matching names.
    include_names: Option<Vec<String>>,
    /// Only include runs from Runnables with matching types.
    include_types: Option<Vec<String>>,
    /// Only include runs from Runnables with matching tags.
    include_tags: Option<Vec<String>>,
    /// Exclude runs from Runnables with matching names.
    exclude_names: Option<Vec<String>>,
    /// Exclude runs from Runnables with matching types.
    exclude_types: Option<Vec<String>>,
    /// Exclude runs from Runnables with matching tags.
    exclude_tags: Option<Vec<String>>,
    /// The send stream for patches.
    send_stream: SendStream<RunLogPatch>,
    /// The receive stream for patches.
    receive_stream: Option<ReceiveStream<RunLogPatch>>,
    /// Map of run ID to key name.
    key_map_by_run_id: HashMap<Uuid, String>,
    /// Map of name to counter.
    counter_map_by_name: HashMap<String, usize>,
    /// The root run ID.
    root_id: Option<Uuid>,
    /// Lock for thread safety.
    lock: Arc<Mutex<()>>,
}

impl fmt::Debug for LogStreamCallbackHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LogStreamCallbackHandler")
            .field("config", &self.config)
            .field("auto_close", &self.auto_close)
            .field("include_names", &self.include_names)
            .field("include_types", &self.include_types)
            .field("include_tags", &self.include_tags)
            .field("exclude_names", &self.exclude_names)
            .field("exclude_types", &self.exclude_types)
            .field("exclude_tags", &self.exclude_tags)
            .field("root_id", &self.root_id)
            .finish()
    }
}

/// Configuration for LogStreamCallbackHandler.
#[derive(Debug, Clone, Default)]
pub struct LogStreamConfig {
    /// Whether to auto-close the stream when the root run finishes.
    pub auto_close: bool,
    /// Only include runs from Runnables with matching names.
    pub include_names: Option<Vec<String>>,
    /// Only include runs from Runnables with matching types.
    pub include_types: Option<Vec<String>>,
    /// Only include runs from Runnables with matching tags.
    pub include_tags: Option<Vec<String>>,
    /// Exclude runs from Runnables with matching names.
    pub exclude_names: Option<Vec<String>>,
    /// Exclude runs from Runnables with matching types.
    pub exclude_types: Option<Vec<String>>,
    /// Exclude runs from Runnables with matching tags.
    pub exclude_tags: Option<Vec<String>>,
    /// The schema format to use.
    pub schema_format: SchemaFormat,
}

impl LogStreamCallbackHandler {
    /// Create a new LogStreamCallbackHandler.
    pub fn new(config: LogStreamConfig) -> Self {
        let stream: MemoryStream<RunLogPatch> = MemoryStream::new();
        let send_stream = stream.get_send_stream();
        let receive_stream = stream.get_receive_stream();

        Self {
            config: TracerCoreConfig {
                schema_format: config.schema_format,
                log_missing_parent: true,
            },
            run_map: HashMap::new(),
            order_map: HashMap::new(),
            auto_close: config.auto_close,
            include_names: config.include_names,
            include_types: config.include_types,
            include_tags: config.include_tags,
            exclude_names: config.exclude_names,
            exclude_types: config.exclude_types,
            exclude_tags: config.exclude_tags,
            send_stream,
            receive_stream: Some(receive_stream),
            key_map_by_run_id: HashMap::new(),
            counter_map_by_name: HashMap::new(),
            root_id: None,
            lock: Arc::new(Mutex::new(())),
        }
    }

    /// Take the receive stream. Can only be called once.
    pub fn take_receive_stream(&mut self) -> Option<ReceiveStream<RunLogPatch>> {
        self.receive_stream.take()
    }

    /// Get the root run ID.
    pub fn root_id(&self) -> Option<Uuid> {
        self.root_id
    }

    /// Send patches to the stream.
    ///
    /// # Returns
    ///
    /// `true` if the patches were sent successfully, `false` otherwise.
    pub fn send(&self, ops: Vec<JsonPatchOp>) -> bool {
        self.send_stream.send(RunLogPatch::new(ops)).is_ok()
    }

    /// Check if a Run should be included in the log.
    pub fn include_run(&self, run: &Run) -> bool {
        if Some(run.id) == self.root_id {
            return false;
        }

        let run_tags = run.tags.clone().unwrap_or_default();

        let mut include = self.include_names.is_none()
            && self.include_types.is_none()
            && self.include_tags.is_none();

        if let Some(ref names) = self.include_names {
            include = include || names.contains(&run.name);
        }
        if let Some(ref types) = self.include_types {
            include = include || types.contains(&run.run_type);
        }
        if let Some(ref tags) = self.include_tags {
            include = include || run_tags.iter().any(|t| tags.contains(t));
        }

        if let Some(ref names) = self.exclude_names {
            include = include && !names.contains(&run.name);
        }
        if let Some(ref types) = self.exclude_types {
            include = include && !types.contains(&run.run_type);
        }
        if let Some(ref tags) = self.exclude_tags {
            include = include && !run_tags.iter().any(|t| tags.contains(t));
        }

        include
    }

    /// Get the standardized inputs for a run.
    fn get_standardized_inputs(&self, run: &Run) -> Option<Value> {
        match self.config.schema_format {
            SchemaFormat::Original | SchemaFormat::OriginalChat => {
                Some(serde_json::to_value(&run.inputs).unwrap_or_default())
            }
            SchemaFormat::StreamingEvents => {
                if run.run_type == "retriever"
                    || run.run_type == "llm"
                    || run.run_type == "chat_model"
                {
                    Some(serde_json::to_value(&run.inputs).unwrap_or_default())
                } else {
                    run.inputs.get("input").cloned()
                }
            }
        }
    }

    /// Get the standardized outputs for a run.
    fn get_standardized_outputs(&self, run: &Run) -> Option<Value> {
        let outputs = run.outputs.as_ref()?;

        match self.config.schema_format {
            SchemaFormat::Original | SchemaFormat::OriginalChat => {
                if run.run_type == "prompt" {
                    outputs.get("output").cloned()
                } else {
                    Some(serde_json::to_value(outputs).unwrap_or_default())
                }
            }
            SchemaFormat::StreamingEvents => {
                if run.run_type == "retriever"
                    || run.run_type == "llm"
                    || run.run_type == "chat_model"
                {
                    Some(serde_json::to_value(outputs).unwrap_or_default())
                } else {
                    outputs.get("output").cloned()
                }
            }
        }
    }
}

impl TracerCore for LogStreamCallbackHandler {
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

    fn persist_run(&mut self, _run: &Run) {
        // This is a legacy method only called once for an entire run tree
        // therefore not useful here
    }

    fn on_run_create(&mut self, run: &Run) {
        if self.root_id.is_none() {
            self.root_id = Some(run.id);
            let state = RunState::new(run.id.to_string(), run.name.clone(), run.run_type.clone());
            if !self.send(vec![JsonPatchOp::replace(
                "",
                serde_json::to_value(state).unwrap_or_default(),
            )]) {
                return;
            }
        }

        if !self.include_run(run) {
            return;
        }

        // Determine key name with counter
        let _lock = self.lock.lock().unwrap();
        let count = self
            .counter_map_by_name
            .entry(run.name.clone())
            .or_insert(0);
        *count += 1;
        let key = if *count == 1 {
            run.name.clone()
        } else {
            format!("{}:{}", run.name, count)
        };
        self.key_map_by_run_id.insert(run.id, key.clone());

        let metadata = run
            .extra
            .get("metadata")
            .and_then(|v| serde_json::from_value::<HashMap<String, Value>>(v.clone()).ok())
            .unwrap_or_default();

        let mut entry = LogEntry::new(
            run.id.to_string(),
            run.name.clone(),
            run.run_type.clone(),
            run.tags.clone().unwrap_or_default(),
            metadata,
            run.start_time,
        );

        if self.config.schema_format == SchemaFormat::StreamingEvents {
            entry.inputs = self.get_standardized_inputs(run);
        }

        self.send(vec![JsonPatchOp::add(
            format!("/logs/{}", key),
            serde_json::to_value(entry).unwrap_or_default(),
        )]);
    }

    fn on_run_update(&mut self, run: &Run) {
        let key = match self.key_map_by_run_id.get(&run.id) {
            Some(k) => k.clone(),
            None => {
                // Check if this is the root run ending
                if run.id == self.root_id.unwrap_or(Uuid::nil()) && self.auto_close {
                    let _ = self.send_stream.close();
                }
                return;
            }
        };

        let mut ops = Vec::new();

        if self.config.schema_format == SchemaFormat::StreamingEvents
            && let Some(inputs) = self.get_standardized_inputs(run)
        {
            ops.push(JsonPatchOp::replace(
                format!("/logs/{}/inputs", key),
                inputs,
            ));
        }

        if let Some(outputs) = self.get_standardized_outputs(run) {
            ops.push(JsonPatchOp::add(
                format!("/logs/{}/final_output", key),
                outputs,
            ));
        }

        if let Some(end_time) = run.end_time {
            ops.push(JsonPatchOp::add(
                format!("/logs/{}/end_time", key),
                Value::String(end_time.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()),
            ));
        }

        self.send(ops);

        if run.id == self.root_id.unwrap_or(Uuid::nil()) && self.auto_close {
            let _ = self.send_stream.close();
        }
    }

    fn on_llm_new_token(&mut self, run: &Run, token: &str, chunk: Option<&dyn std::any::Any>) {
        let key = match self.key_map_by_run_id.get(&run.id) {
            Some(k) => k.clone(),
            None => return,
        };

        let chunk_value = if let Some(chunk_any) = chunk {
            if let Some(gen_chunk) = chunk_any.downcast_ref::<crate::outputs::GenerationChunk>() {
                serde_json::to_value(gen_chunk).unwrap_or(Value::String(token.to_string()))
            } else if let Some(chat_chunk) =
                chunk_any.downcast_ref::<crate::outputs::ChatGenerationChunk>()
            {
                // For chat chunks, include the message
                serde_json::to_value(&chat_chunk.message)
                    .unwrap_or(Value::String(token.to_string()))
            } else {
                Value::String(token.to_string())
            }
        } else {
            Value::String(token.to_string())
        };

        self.send(vec![
            JsonPatchOp::add(
                format!("/logs/{}/streamed_output_str/-", key),
                Value::String(token.to_string()),
            ),
            JsonPatchOp::add(format!("/logs/{}/streamed_output/-", key), chunk_value),
        ]);
    }
}

impl BaseTracer for LogStreamCallbackHandler {
    fn persist_run_impl(&mut self, _run: &Run) {
        // This is a legacy method only called once for an entire run tree
        // therefore not useful here
    }
}

impl<T: Send + 'static> StreamingCallbackHandler<T> for LogStreamCallbackHandler {
    fn tap_output_aiter(
        &self,
        run_id: Uuid,
        output: std::pin::Pin<Box<dyn futures::Stream<Item = T> + Send>>,
    ) -> std::pin::Pin<Box<dyn futures::Stream<Item = T> + Send>> {
        use futures::StreamExt;

        let root_id = self.root_id;
        let key = self.key_map_by_run_id.get(&run_id).cloned();
        let send_stream = self.send_stream.clone();

        Box::pin(futures::stream::unfold(
            (output, run_id, root_id, key, send_stream),
            |(mut stream, run_id, root_id, key, sender)| async move {
                let item = stream.next().await?;

                // Root run is handled separately
                // If we can't find the run key, silently ignore
                if run_id != root_id.unwrap_or(Uuid::nil())
                    && let Some(ref k) = key
                {
                    // Note: We can't easily serialize generic T here
                    // This would need a more sophisticated implementation
                    // for real-world use with proper chunk serialization
                    let _ = sender.send(RunLogPatch::new(vec![JsonPatchOp::add(
                        format!("/logs/{}/streamed_output/-", k),
                        Value::Null, // Placeholder - real implementation would serialize the chunk
                    )]));
                }

                Some((item, (stream, run_id, root_id, key, sender)))
            },
        ))
    }

    fn tap_output_iter(
        &self,
        run_id: Uuid,
        output: Box<dyn Iterator<Item = T> + Send>,
    ) -> Box<dyn Iterator<Item = T> + Send> {
        let root_id = self.root_id;
        let key = self.key_map_by_run_id.get(&run_id).cloned();
        let send_stream = self.send_stream.clone();

        Box::new(TappedIterator {
            inner: output,
            run_id,
            root_id,
            key,
            send_stream,
        })
    }
}

struct TappedIterator<T> {
    inner: Box<dyn Iterator<Item = T> + Send>,
    run_id: Uuid,
    root_id: Option<Uuid>,
    key: Option<String>,
    send_stream: SendStream<RunLogPatch>,
}

impl<T> Iterator for TappedIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.inner.next()?;

        // Root run is handled separately
        if self.run_id != self.root_id.unwrap_or(Uuid::nil())
            && let Some(ref k) = self.key
        {
            let _ = self
                .send_stream
                .send(RunLogPatch::new(vec![JsonPatchOp::add(
                    format!("/logs/{}/streamed_output/-", k),
                    Value::Null, // Placeholder
                )]));
        }

        Some(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_entry_new() {
        let entry = LogEntry::new(
            "test-id".to_string(),
            "test".to_string(),
            "chain".to_string(),
            vec!["tag1".to_string()],
            HashMap::new(),
            Utc::now(),
        );

        assert_eq!(entry.id, "test-id");
        assert_eq!(entry.name, "test");
        assert_eq!(entry.run_type, "chain");
        assert!(entry.final_output.is_none());
    }

    #[test]
    fn test_run_state_new() {
        let state = RunState::new(
            "state-id".to_string(),
            "test".to_string(),
            "chain".to_string(),
        );

        assert_eq!(state.id, "state-id");
        assert!(state.logs.is_empty());
        assert!(state.final_output.is_none());
    }

    #[test]
    fn test_json_patch_ops() {
        let add_op = JsonPatchOp::add("/path", Value::String("value".to_string()));
        assert_eq!(add_op.op, "add");
        assert_eq!(add_op.path, "/path");

        let replace_op = JsonPatchOp::replace("/path", Value::Number(42.into()));
        assert_eq!(replace_op.op, "replace");

        let remove_op = JsonPatchOp::remove("/path");
        assert_eq!(remove_op.op, "remove");
        assert!(remove_op.value.is_none());
    }

    #[test]
    fn test_run_log_apply_patch() {
        let mut log = RunLog::new(
            vec![],
            Some(RunState::new(
                "id".to_string(),
                "test".to_string(),
                "chain".to_string(),
            )),
        );

        let patch = RunLogPatch::new(vec![JsonPatchOp::add(
            "/logs/entry1",
            serde_json::to_value(LogEntry::new(
                "entry1".to_string(),
                "sub".to_string(),
                "tool".to_string(),
                vec![],
                HashMap::new(),
                Utc::now(),
            ))
            .unwrap(),
        )]);

        log.apply_patch(patch);

        assert!(log.state.as_ref().unwrap().logs.contains_key("entry1"));
    }

    #[test]
    fn test_log_stream_handler_include_run() {
        let handler = LogStreamCallbackHandler::new(LogStreamConfig {
            include_names: Some(vec!["allowed".to_string()]),
            ..Default::default()
        });

        let run = Run {
            name: "allowed".to_string(),
            ..Default::default()
        };
        assert!(handler.include_run(&run));

        let run = Run {
            name: "not_allowed".to_string(),
            ..Default::default()
        };
        assert!(!handler.include_run(&run));
    }

    #[test]
    fn test_log_stream_handler_exclude_run() {
        let handler = LogStreamCallbackHandler::new(LogStreamConfig {
            exclude_names: Some(vec!["excluded".to_string()]),
            ..Default::default()
        });

        let run = Run {
            name: "excluded".to_string(),
            ..Default::default()
        };
        assert!(!handler.include_run(&run));

        let run = Run {
            name: "allowed".to_string(),
            ..Default::default()
        };
        assert!(handler.include_run(&run));
    }

    #[test]
    fn test_log_stream_handler_include_tags() {
        let handler = LogStreamCallbackHandler::new(LogStreamConfig {
            include_tags: Some(vec!["important".to_string()]),
            ..Default::default()
        });

        let run = Run {
            tags: Some(vec!["important".to_string(), "other".to_string()]),
            ..Default::default()
        };
        assert!(handler.include_run(&run));

        let run = Run {
            tags: Some(vec!["other".to_string()]),
            ..Default::default()
        };
        assert!(!handler.include_run(&run));
    }
}
