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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub run_type: String,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, Value>,
    pub start_time: String,
    pub streamed_output_str: Vec<String>,
    pub streamed_output: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Value>,
    pub final_output: Option<Value>,
    pub end_time: Option<String>,
}

impl LogEntry {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunState {
    pub id: String,
    pub streamed_output: Vec<Value>,
    pub final_output: Option<Value>,
    pub name: String,
    #[serde(rename = "type")]
    pub run_type: String,
    pub logs: HashMap<String, LogEntry>,
}

impl RunState {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonPatchOp {
    pub op: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
}

impl JsonPatchOp {
    pub fn add(path: impl Into<String>, value: Value) -> Self {
        Self {
            op: "add".to_string(),
            path: path.into(),
            value: Some(value),
        }
    }

    pub fn replace(path: impl Into<String>, value: Value) -> Self {
        Self {
            op: "replace".to_string(),
            path: path.into(),
            value: Some(value),
        }
    }

    pub fn remove(path: impl Into<String>) -> Self {
        Self {
            op: "remove".to_string(),
            path: path.into(),
            value: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunLogPatch {
    pub ops: Vec<JsonPatchOp>,
}

impl RunLogPatch {
    pub fn new(ops: Vec<JsonPatchOp>) -> Self {
        Self { ops }
    }

    pub fn from_op(op: JsonPatchOp) -> Self {
        Self { ops: vec![op] }
    }

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunLog {
    pub ops: Vec<JsonPatchOp>,
    pub state: Option<RunState>,
}

impl RunLog {
    pub fn new(ops: Vec<JsonPatchOp>, state: Option<RunState>) -> Self {
        Self { ops, state }
    }

    pub fn apply_patch(&mut self, patch: RunLogPatch) {
        self.ops.extend(patch.ops.clone());

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
                                if let Some(value) = &op.value
                                    && let Ok(entry) =
                                        serde_json::from_value::<LogEntry>(value.clone())
                                {
                                    state.logs.insert(path_parts[1].to_string(), entry);
                                }
                            } else if path_parts.len() >= 3
                                && let Some(entry) = state.logs.get_mut(path_parts[1])
                            {
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

pub struct LogStreamCallbackHandler {
    config: TracerCoreConfig,
    run_map: HashMap<String, Run>,
    order_map: HashMap<Uuid, (Uuid, String)>,
    auto_close: bool,
    include_names: Option<Vec<String>>,
    include_types: Option<Vec<String>>,
    include_tags: Option<Vec<String>>,
    exclude_names: Option<Vec<String>>,
    exclude_types: Option<Vec<String>>,
    exclude_tags: Option<Vec<String>>,
    send_stream: SendStream<RunLogPatch>,
    receive_stream: Option<ReceiveStream<RunLogPatch>>,
    key_map_by_run_id: HashMap<Uuid, String>,
    counter_map_by_name: HashMap<String, usize>,
    root_id: Option<Uuid>,
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

#[derive(Debug, Clone, Default)]
pub struct LogStreamConfig {
    pub auto_close: bool,
    pub include_names: Option<Vec<String>>,
    pub include_types: Option<Vec<String>>,
    pub include_tags: Option<Vec<String>>,
    pub exclude_names: Option<Vec<String>>,
    pub exclude_types: Option<Vec<String>>,
    pub exclude_tags: Option<Vec<String>>,
    pub schema_format: SchemaFormat,
}

impl LogStreamCallbackHandler {
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

    pub fn take_receive_stream(&mut self) -> Option<ReceiveStream<RunLogPatch>> {
        self.receive_stream.take()
    }

    pub fn root_id(&self) -> Option<Uuid> {
        self.root_id
    }

    pub fn send(&self, ops: Vec<JsonPatchOp>) -> bool {
        self.send_stream.send(RunLogPatch::new(ops)).is_ok()
    }

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

    fn persist_run(&mut self, _run: &Run) {}

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

        let _lock = self.lock.lock().expect("lock poisoned");
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
                if run.id == self.root_id.unwrap_or(Uuid::nil())
                    && self.auto_close
                    && let Err(error) = self.send_stream.close()
                {
                    tracing::warn!("Failed to close log stream: {}", error);
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

        if run.id == self.root_id.unwrap_or(Uuid::nil())
            && self.auto_close
            && let Err(error) = self.send_stream.close()
        {
            tracing::warn!("Failed to close log stream: {}", error);
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
    fn persist_run_impl(&mut self, _run: &Run) {}
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

                if run_id != root_id.unwrap_or(Uuid::nil())
                    && let Some(ref k) = key
                    && let Err(error) = sender.send(RunLogPatch::new(vec![JsonPatchOp::add(
                        format!("/logs/{}/streamed_output/-", k),
                        Value::Null, // Placeholder - real implementation would serialize the chunk
                    )]))
                {
                    tracing::warn!("Failed to send log stream patch: {}", error);
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

        if self.run_id != self.root_id.unwrap_or(Uuid::nil())
            && let Some(ref k) = self.key
            && let Err(error) = self
                .send_stream
                .send(RunLogPatch::new(vec![JsonPatchOp::add(
                    format!("/logs/{}/streamed_output/-", k),
                    Value::Null, // Placeholder
                )]))
        {
            tracing::warn!("Failed to send log stream patch: {}", error);
        }

        Some(item)
    }
}

pub struct LogStreamCallbackHandlerBridge {
    inner: Arc<Mutex<LogStreamCallbackHandler>>,
}

impl LogStreamCallbackHandlerBridge {
    pub fn new(handler: LogStreamCallbackHandler) -> Self {
        Self {
            inner: Arc::new(Mutex::new(handler)),
        }
    }

    pub fn take_receive_stream(&self) -> Option<ReceiveStream<RunLogPatch>> {
        self.inner
            .lock()
            .expect("lock poisoned")
            .take_receive_stream()
    }

    pub fn get_send_stream(&self) -> SendStream<RunLogPatch> {
        self.inner
            .lock()
            .expect("lock poisoned")
            .send_stream
            .clone()
    }
}

impl fmt::Debug for LogStreamCallbackHandlerBridge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LogStreamCallbackHandlerBridge").finish()
    }
}

impl crate::callbacks::base::LLMManagerMixin for LogStreamCallbackHandlerBridge {
    fn on_llm_new_token(
        &self,
        token: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        chunk: Option<&serde_json::Value>,
    ) {
        let chunk_any: Option<Box<dyn std::any::Any>> =
            chunk.map(|v| Box::new(v.clone()) as Box<dyn std::any::Any>);
        let chunk_ref = chunk_any.as_deref();
        let mut handler = self.inner.lock().expect("lock poisoned");
        if let Err(e) = handler.handle_llm_new_token(token, run_id, chunk_ref, parent_run_id) {
            tracing::warn!(
                "LogStreamCallbackHandlerBridge on_llm_new_token error: {:?}",
                e
            );
        }
    }

    fn on_llm_end(
        &self,
        response: &crate::outputs::ChatResult,
        run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        use crate::outputs::{GenerationType, LLMResult};
        let llm_result = LLMResult {
            generations: vec![
                response
                    .generations
                    .iter()
                    .map(|cg| GenerationType::ChatGeneration(cg.clone()))
                    .collect(),
            ],
            llm_output: response.llm_output.clone(),
            run: None,
            result_type: "LLMResult".to_string(),
        };
        let mut handler = self.inner.lock().expect("lock poisoned");
        if let Err(e) = handler.handle_llm_end(&llm_result, run_id) {
            tracing::warn!("LogStreamCallbackHandlerBridge on_llm_end error: {:?}", e);
        }
    }

    fn on_llm_error(
        &self,
        error: &dyn std::error::Error,
        run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        let mut handler = self.inner.lock().expect("lock poisoned");
        if let Err(e) = handler.handle_llm_error(error, run_id, None) {
            tracing::warn!("LogStreamCallbackHandlerBridge on_llm_error error: {:?}", e);
        }
    }
}

impl crate::callbacks::base::ChainManagerMixin for LogStreamCallbackHandlerBridge {
    fn on_chain_end(
        &self,
        outputs: &HashMap<String, serde_json::Value>,
        run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        let mut handler = self.inner.lock().expect("lock poisoned");
        if let Err(e) = handler.handle_chain_end(outputs.clone(), run_id, None) {
            tracing::warn!("LogStreamCallbackHandlerBridge on_chain_end error: {:?}", e);
        }
    }

    fn on_chain_error(
        &self,
        error: &dyn std::error::Error,
        run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        let mut handler = self.inner.lock().expect("lock poisoned");
        if let Err(e) = handler.handle_chain_error(error, run_id, None) {
            tracing::warn!(
                "LogStreamCallbackHandlerBridge on_chain_error error: {:?}",
                e
            );
        }
    }
}

impl crate::callbacks::base::ToolManagerMixin for LogStreamCallbackHandlerBridge {
    fn on_tool_end(
        &self,
        output: &str,
        run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _color: Option<&str>,
        _observation_prefix: Option<&str>,
        _llm_prefix: Option<&str>,
    ) {
        let mut handler = self.inner.lock().expect("lock poisoned");
        if let Err(e) = handler.handle_tool_end(serde_json::json!(output), run_id) {
            tracing::warn!("LogStreamCallbackHandlerBridge on_tool_end error: {:?}", e);
        }
    }

    fn on_tool_error(
        &self,
        error: &dyn std::error::Error,
        run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        let mut handler = self.inner.lock().expect("lock poisoned");
        if let Err(e) = handler.handle_tool_error(error, run_id) {
            tracing::warn!(
                "LogStreamCallbackHandlerBridge on_tool_error error: {:?}",
                e
            );
        }
    }
}

impl crate::callbacks::base::RetrieverManagerMixin for LogStreamCallbackHandlerBridge {
    fn on_retriever_end(
        &self,
        documents: &[serde_json::Value],
        run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        let mut handler = self.inner.lock().expect("lock poisoned");
        if let Err(e) = handler.handle_retriever_end(documents.to_vec(), run_id) {
            tracing::warn!(
                "LogStreamCallbackHandlerBridge on_retriever_end error: {:?}",
                e
            );
        }
    }

    fn on_retriever_error(
        &self,
        error: &dyn std::error::Error,
        run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        let mut handler = self.inner.lock().expect("lock poisoned");
        if let Err(e) = handler.handle_retriever_error(error, run_id) {
            tracing::warn!(
                "LogStreamCallbackHandlerBridge on_retriever_error error: {:?}",
                e
            );
        }
    }
}

impl crate::callbacks::base::CallbackManagerMixin for LogStreamCallbackHandlerBridge {
    fn on_llm_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        prompts: &[String],
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
        let mut handler = self.inner.lock().expect("lock poisoned");
        handler.handle_llm_start(
            serialized.clone(),
            prompts,
            run_id,
            parent_run_id,
            tags.map(|t| t.to_vec()),
            metadata.cloned(),
            None,
            HashMap::new(),
        );
    }

    fn on_chat_model_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        messages: &[Vec<crate::messages::BaseMessage>],
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
        let mut handler = self.inner.lock().expect("lock poisoned");
        if let Err(e) = handler.handle_chat_model_start(
            serialized.clone(),
            messages,
            run_id,
            parent_run_id,
            tags.map(|t| t.to_vec()),
            metadata.cloned(),
            None,
            HashMap::new(),
        ) {
            tracing::warn!(
                "LogStreamCallbackHandlerBridge on_chat_model_start error: {:?}",
                e
            );
        }
    }

    fn on_chain_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        inputs: &HashMap<String, serde_json::Value>,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
        name: Option<&str>,
    ) {
        let mut handler = self.inner.lock().expect("lock poisoned");
        handler.handle_chain_start(
            serialized.clone(),
            inputs.clone(),
            run_id,
            parent_run_id,
            tags.map(|t| t.to_vec()),
            metadata.cloned(),
            None,
            name.map(|n| n.to_string()),
            HashMap::new(),
        );
    }

    fn on_tool_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        input_str: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
        inputs: Option<&HashMap<String, serde_json::Value>>,
    ) {
        let mut handler = self.inner.lock().expect("lock poisoned");
        handler.handle_tool_start(
            serialized.clone(),
            input_str,
            run_id,
            parent_run_id,
            tags.map(|t| t.to_vec()),
            metadata.cloned(),
            None,
            inputs.cloned(),
            HashMap::new(),
        );
    }

    fn on_retriever_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        query: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
        _name: Option<&str>,
    ) {
        let mut handler = self.inner.lock().expect("lock poisoned");
        handler.handle_retriever_start(
            serialized.clone(),
            query,
            run_id,
            parent_run_id,
            tags.map(|t| t.to_vec()),
            metadata.cloned(),
            None,
            HashMap::new(),
        );
    }
}

impl crate::callbacks::base::RunManagerMixin for LogStreamCallbackHandlerBridge {}

impl crate::callbacks::base::BaseCallbackHandler for LogStreamCallbackHandlerBridge {
    fn name(&self) -> &str {
        "LogStreamCallbackHandler"
    }

    fn run_inline(&self) -> bool {
        true
    }
}

pub fn astream_log_implementation<'a, R>(
    runnable: &'a R,
    input: R::Input,
    config: Option<crate::runnables::config::RunnableConfig>,
    diff: bool,
    with_streamed_output_list: bool,
    include_names: Option<Vec<String>>,
    include_types: Option<Vec<String>>,
    include_tags: Option<Vec<String>>,
    exclude_names: Option<Vec<String>>,
    exclude_types: Option<Vec<String>>,
    exclude_tags: Option<Vec<String>>,
) -> futures::stream::BoxStream<'a, RunLogPatch>
where
    R: crate::runnables::base::Runnable + 'static,
    R::Output: serde::Serialize,
{
    use crate::callbacks::base::Callbacks;
    use crate::runnables::config::ensure_config;
    use futures::StreamExt;

    let handler = LogStreamCallbackHandler::new(LogStreamConfig {
        auto_close: false,
        include_names,
        include_types,
        include_tags,
        exclude_names,
        exclude_types,
        exclude_tags,
        ..Default::default()
    });

    let bridge = Arc::new(LogStreamCallbackHandlerBridge::new(handler));

    let mut config = ensure_config(config);

    let cb_handler: Arc<dyn crate::callbacks::base::BaseCallbackHandler> = bridge.clone();
    match &mut config.callbacks {
        None => {
            config.callbacks = Some(Callbacks::Handlers(vec![cb_handler]));
        }
        Some(Callbacks::Handlers(handlers)) => {
            handlers.push(cb_handler);
        }
        Some(Callbacks::Manager(manager)) => {
            manager.add_handler(cb_handler, true);
        }
    }

    let receive_stream = bridge
        .take_receive_stream()
        .expect("receive stream should be available");

    let send_stream = bridge.get_send_stream();

    Box::pin(async_stream::stream! {
        let mut astream = std::pin::pin!(runnable.astream(input, Some(config)));
        while let Some(chunk_result) = astream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    let serialized = serde_json::to_value(&chunk).unwrap_or_default();

                    let mut ops = Vec::new();
                    if with_streamed_output_list {
                        ops.push(JsonPatchOp::add(
                            "/streamed_output/-",
                            serialized.clone(),
                        ));
                    }

                    ops.push(JsonPatchOp::replace(
                        "/final_output",
                        serialized,
                    ));

                    if let Err(e) = send_stream.send(RunLogPatch::new(ops)) {
                        tracing::warn!("Failed to send log patch: {}", e);
                    }
                }
                Err(e) => {
                    tracing::warn!("astream_log chunk error: {}", e);
                }
            }
        }

        if let Err(e) = send_stream.close() { tracing::warn!("Failed to close stream: {e}"); }

        let mut event_stream = std::pin::pin!(receive_stream.into_stream());

        if diff {
            while let Some(patch) = event_stream.next().await {
                yield patch;
            }
        } else {
            let mut state = RunLog::new(vec![], None);
            while let Some(patch) = event_stream.next().await {
                state.apply_patch(patch);
                yield RunLogPatch::new(state.ops.clone());
            }
        }
    })
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

    #[test]
    fn test_log_stream_bridge_implements_base_callback_handler() {
        let handler = LogStreamCallbackHandler::new(LogStreamConfig::default());
        let bridge = LogStreamCallbackHandlerBridge::new(handler);
        let _handler_ref: &dyn crate::callbacks::base::BaseCallbackHandler = &bridge;
        assert_eq!(
            crate::callbacks::base::BaseCallbackHandler::name(&bridge),
            "LogStreamCallbackHandler"
        );
    }
}
