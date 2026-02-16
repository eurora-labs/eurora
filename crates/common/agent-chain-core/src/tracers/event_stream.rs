//! Internal tracer to power the event stream API.
//!
//! This module provides the callback handler and implementation functions for
//! the `astream_events()` API. It converts nested tracer run data into a flat
//! stream of typed events.
//!
//! Mirrors `langchain_core.tracers.event_stream`.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Mutex;

use futures::StreamExt;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::GenerationType;
use crate::callbacks::base::{
    BaseCallbackHandler, CallbackManagerMixin, ChainManagerMixin, LLMManagerMixin,
    RetrieverManagerMixin, RunManagerMixin, ToolManagerMixin,
};
use crate::messages::{AIMessageChunk, BaseMessage};
use crate::outputs::ChatResult;
use crate::outputs::{GenerationChunk, LLMResult};
use crate::runnables::schema::{CustomStreamEvent, EventData, StandardStreamEvent, StreamEvent};
use crate::runnables::utils::RootEventFilter;
use crate::tracers::memory_stream::{MemoryStream, ReceiveStream, SendStream};
use crate::tracers::streaming::StreamingCallbackHandler;

/// Information about a run.
///
/// This is used to keep track of the metadata associated with a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunInfo {
    /// The name of the run.
    pub name: String,
    /// The tags associated with the run.
    pub tags: Vec<String>,
    /// The metadata associated with the run.
    pub metadata: HashMap<String, Value>,
    /// The type of the run.
    pub run_type: String,
    /// The inputs to the run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Value>,
    /// The ID of the parent run.
    pub parent_run_id: Option<Uuid>,
}

/// Assign a name to a run.
fn assign_name(name: Option<&str>, serialized: Option<&HashMap<String, Value>>) -> String {
    if let Some(n) = name {
        return n.to_string();
    }
    if let Some(s) = serialized {
        if let Some(Value::String(n)) = s.get("name") {
            return n.clone();
        }
        if let Some(Value::Array(ids)) = s.get("id") {
            if let Some(Value::String(last)) = ids.last() {
                return last.clone();
            }
        }
    }
    "Unnamed".to_string()
}

/// Interior mutable state for the callback handler.
#[derive(Debug)]
struct HandlerState {
    /// Map of run ID to run info. Entries are cleaned up when each run ends.
    run_map: HashMap<Uuid, RunInfo>,
    /// Map of child run ID to parent run ID. Kept separately from run_map
    /// because parent end events may fire before child end events.
    parent_map: HashMap<Uuid, Option<Uuid>>,
    /// Track which runs have been tapped for streaming.
    is_tapped: HashMap<Uuid, bool>,
}

/// An implementation of a callback handler for astream events.
///
/// This handler tracks run metadata and sends stream events through a memory
/// stream. It is used internally by `astream_events()`.
///
/// Implements `BaseCallbackHandler` so it can be injected into
/// `RunnableConfig.callbacks` and receive callbacks during execution.
pub struct AstreamEventsCallbackHandler {
    /// Interior-mutable state (run_map, parent_map, is_tapped).
    state: Mutex<HandlerState>,
    /// Filter which events will be sent over the queue.
    root_event_filter: RootEventFilter,
    /// The send stream for events.
    send_stream: SendStream<StreamEvent>,
    /// The receive stream for events (can only be taken once).
    receive_stream: Mutex<Option<ReceiveStream<StreamEvent>>>,
}

impl AstreamEventsCallbackHandler {
    /// Create a new AstreamEventsCallbackHandler.
    pub fn new(
        include_names: Option<Vec<String>>,
        include_types: Option<Vec<String>>,
        include_tags: Option<Vec<String>>,
        exclude_names: Option<Vec<String>>,
        exclude_types: Option<Vec<String>>,
        exclude_tags: Option<Vec<String>>,
    ) -> Self {
        let stream: MemoryStream<StreamEvent> = MemoryStream::new();
        let send_stream = stream.get_send_stream();
        let receive_stream = stream.get_receive_stream();

        Self {
            state: Mutex::new(HandlerState {
                run_map: HashMap::new(),
                parent_map: HashMap::new(),
                is_tapped: HashMap::new(),
            }),
            root_event_filter: RootEventFilter {
                include_names,
                include_types,
                include_tags,
                exclude_names,
                exclude_types,
                exclude_tags,
            },
            send_stream,
            receive_stream: Mutex::new(Some(receive_stream)),
        }
    }

    /// Take the receive stream. Can only be called once.
    pub fn take_receive_stream(&self) -> Option<ReceiveStream<StreamEvent>> {
        self.receive_stream
            .lock()
            .expect("receive_stream lock poisoned")
            .take()
    }

    /// Get the send stream (clone).
    pub fn get_send_stream(&self) -> SendStream<StreamEvent> {
        self.send_stream.clone()
    }

    /// Get the parent IDs of a run (non-recursively) cast to strings.
    ///
    /// Returns parent IDs in order from root to immediate parent.
    fn get_parent_ids(parent_map: &HashMap<Uuid, Option<Uuid>>, mut run_id: Uuid) -> Vec<String> {
        let mut parent_ids = Vec::new();

        while let Some(Some(parent_id)) = parent_map.get(&run_id) {
            let str_parent_id = parent_id.to_string();
            assert!(
                !parent_ids.contains(&str_parent_id),
                "Parent ID {} is already in the parent_ids list. This should never happen.",
                parent_id
            );
            parent_ids.push(str_parent_id);
            run_id = *parent_id;
        }

        // Return in reverse order: root first, immediate parent last.
        parent_ids.reverse();
        parent_ids
    }

    /// Send an event to the stream if it passes the filter.
    fn send(&self, event: StreamEvent, event_type: &str) {
        let (event_name, event_tags) = match &event {
            StreamEvent::Standard(e) => (e.name.as_str(), e.base.tags.as_slice()),
            StreamEvent::Custom(e) => (e.name.as_str(), e.base.tags.as_slice()),
        };
        if self
            .root_event_filter
            .include_event(event_name, event_tags, event_type)
        {
            if let Err(error) = self.send_stream.send(event) {
                tracing::warn!("Failed to send stream event: {}", error);
            }
        }
    }

    /// Record the start of a run.
    fn write_run_start_info(
        state: &mut HandlerState,
        run_id: Uuid,
        tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, Value>>,
        parent_run_id: Option<Uuid>,
        name: String,
        run_type: String,
        inputs: Option<Value>,
    ) {
        let info = RunInfo {
            tags: tags.unwrap_or_default(),
            metadata: metadata.unwrap_or_default(),
            name,
            run_type,
            inputs,
            parent_run_id,
        };

        state.run_map.insert(run_id, info);
        state.parent_map.insert(run_id, parent_run_id);
    }

    /// Handle a chat model start event.
    fn handle_chat_model_start(
        &self,
        serialized: &HashMap<String, Value>,
        messages: &[Vec<BaseMessage>],
        run_id: Uuid,
        tags: Option<Vec<String>>,
        parent_run_id: Option<Uuid>,
        metadata: Option<HashMap<String, Value>>,
        name: Option<&str>,
    ) {
        let name_ = assign_name(name, Some(serialized));
        let run_type = "chat_model";

        let parent_ids = {
            let mut state = self.state.lock().expect("state lock poisoned");
            Self::write_run_start_info(
                &mut state,
                run_id,
                tags.clone(),
                metadata.clone(),
                parent_run_id,
                name_.clone(),
                run_type.to_string(),
                Some(serde_json::to_value(messages).unwrap_or_default()),
            );
            Self::get_parent_ids(&state.parent_map, run_id)
        };

        let event = StandardStreamEvent::new("on_chat_model_start", &run_id.to_string(), &name_)
            .with_tags(tags.unwrap_or_default())
            .with_metadata(metadata.unwrap_or_default())
            .with_parent_ids(parent_ids)
            .with_data(
                EventData::new().with_input(
                    serde_json::to_value(&HashMap::from([(
                        "messages",
                        serde_json::to_value(messages).unwrap_or_default(),
                    )]))
                    .unwrap_or_default(),
                ),
            );

        self.send(StreamEvent::Standard(event), run_type);
    }

    /// Handle an LLM start event.
    fn handle_llm_start(
        &self,
        serialized: &HashMap<String, Value>,
        prompts: &[String],
        run_id: Uuid,
        tags: Option<Vec<String>>,
        parent_run_id: Option<Uuid>,
        metadata: Option<HashMap<String, Value>>,
        name: Option<&str>,
    ) {
        let name_ = assign_name(name, Some(serialized));
        let run_type = "llm";

        let parent_ids = {
            let mut state = self.state.lock().expect("state lock poisoned");
            Self::write_run_start_info(
                &mut state,
                run_id,
                tags.clone(),
                metadata.clone(),
                parent_run_id,
                name_.clone(),
                run_type.to_string(),
                Some(
                    serde_json::to_value(&HashMap::from([("prompts", prompts)]))
                        .unwrap_or_default(),
                ),
            );
            Self::get_parent_ids(&state.parent_map, run_id)
        };

        let event = StandardStreamEvent::new("on_llm_start", &run_id.to_string(), &name_)
            .with_tags(tags.unwrap_or_default())
            .with_metadata(metadata.unwrap_or_default())
            .with_parent_ids(parent_ids)
            .with_data(
                EventData::new().with_input(
                    serde_json::to_value(&HashMap::from([(
                        "prompts",
                        serde_json::to_value(prompts).unwrap_or_default(),
                    )]))
                    .unwrap_or_default(),
                ),
            );

        self.send(StreamEvent::Standard(event), run_type);
    }

    /// Handle a custom event.
    fn handle_custom_event(
        &self,
        name: &str,
        data: Value,
        run_id: Uuid,
        tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, Value>>,
    ) {
        let parent_ids = {
            let state = self.state.lock().expect("state lock poisoned");
            Self::get_parent_ids(&state.parent_map, run_id)
        };

        let event = CustomStreamEvent::new(name, &run_id.to_string(), data.clone())
            .with_tags(tags.unwrap_or_default())
            .with_metadata(metadata.unwrap_or_default())
            .with_parent_ids(parent_ids);

        self.send(StreamEvent::Custom(event), name);
    }

    /// Handle a new LLM token event.
    fn handle_llm_new_token(
        &self,
        token: &str,
        chunk: Option<Value>,
        run_id: Uuid,
    ) -> Result<(), String> {
        let state = self.state.lock().expect("state lock poisoned");

        let run_info = state
            .run_map
            .get(&run_id)
            .ok_or_else(|| format!("Run ID {} not found in run map.", run_id))?;

        if state.is_tapped.get(&run_id).is_some() {
            return Ok(());
        }

        let (event_name, chunk_value) = if run_info.run_type == "chat_model" {
            let chunk_value = if let Some(c) = chunk {
                c.get("message").cloned().unwrap_or_else(|| {
                    serde_json::to_value(AIMessageChunk::builder().content(token).build())
                        .unwrap_or_default()
                })
            } else {
                serde_json::to_value(AIMessageChunk::builder().content(token).build())
                    .unwrap_or_default()
            };
            ("on_chat_model_stream", chunk_value)
        } else if run_info.run_type == "llm" {
            let chunk_value = chunk.unwrap_or_else(|| {
                serde_json::to_value(GenerationChunk {
                    text: token.to_string(),
                    generation_info: None,
                    generation_type: "GenerationChunk".to_string(),
                })
                .unwrap_or_default()
            });
            ("on_llm_stream", chunk_value)
        } else {
            return Err(format!("Unexpected run type: {}", run_info.run_type));
        };

        let parent_ids = Self::get_parent_ids(&state.parent_map, run_id);

        let event = StandardStreamEvent::new(event_name, &run_id.to_string(), &run_info.name)
            .with_tags(run_info.tags.clone())
            .with_metadata(run_info.metadata.clone())
            .with_parent_ids(parent_ids)
            .with_data(EventData::new().with_chunk(chunk_value));

        // Release lock before sending (send only needs &self)
        let run_type = run_info.run_type.clone();
        drop(state);

        self.send(StreamEvent::Standard(event), &run_type);
        Ok(())
    }

    /// Handle an LLM end event.
    fn handle_llm_end(&self, response: &LLMResult, run_id: Uuid) -> Result<(), String> {
        let (run_info, parent_ids) = {
            let mut state = self.state.lock().expect("state lock poisoned");
            let run_info = state
                .run_map
                .remove(&run_id)
                .ok_or_else(|| format!("Run ID {} not found in run map.", run_id))?;
            let parent_ids = Self::get_parent_ids(&state.parent_map, run_id);
            (run_info, parent_ids)
        };
        let inputs = run_info.inputs.clone();

        let (event_name, output) = if run_info.run_type == "chat_model" {
            let mut output: Value = Value::Null;
            'outer: for generation_list in &response.generations {
                for generation in generation_list {
                    output = match generation {
                        GenerationType::ChatGeneration(cg) => {
                            serde_json::to_value(&cg.message).unwrap_or(Value::Null)
                        }
                        GenerationType::ChatGenerationChunk(cgc) => {
                            serde_json::to_value(&cgc.message).unwrap_or(Value::Null)
                        }
                        _ => Value::Null,
                    };
                    break 'outer;
                }
            }
            ("on_chat_model_end", output)
        } else if run_info.run_type == "llm" {
            let generations_value: Vec<Vec<Value>> = response
                .generations
                .iter()
                .map(|generation_list| {
                    generation_list
                        .iter()
                        .map(|generation| match generation {
                            GenerationType::Generation(g) => serde_json::json!({
                                "text": g.text,
                                "generation_info": g.generation_info,
                                "type": g.generation_type,
                            }),
                            GenerationType::GenerationChunk(gc) => serde_json::json!({
                                "text": gc.text,
                                "generation_info": gc.generation_info,
                                "type": gc.generation_type,
                            }),
                            GenerationType::ChatGeneration(cg) => serde_json::json!({
                                "text": cg.text,
                                "generation_info": cg.generation_info,
                                "type": "ChatGeneration",
                            }),
                            GenerationType::ChatGenerationChunk(cgc) => serde_json::json!({
                                "text": cgc.text,
                                "generation_info": cgc.generation_info,
                                "type": "ChatGenerationChunk",
                            }),
                        })
                        .collect()
                })
                .collect();

            let output = serde_json::json!({
                "generations": generations_value,
                "llm_output": response.llm_output,
            });
            ("on_llm_end", output)
        } else {
            return Err(format!("Unexpected run type: {}", run_info.run_type));
        };

        let mut data = EventData::new().with_output(output);
        if let Some(inp) = inputs {
            data = data.with_input(inp);
        }

        let event = StandardStreamEvent::new(event_name, &run_id.to_string(), &run_info.name)
            .with_tags(run_info.tags.clone())
            .with_metadata(run_info.metadata.clone())
            .with_parent_ids(parent_ids)
            .with_data(data);

        self.send(StreamEvent::Standard(event), &run_info.run_type);
        Ok(())
    }

    /// Handle a chain start event.
    #[allow(clippy::too_many_arguments)]
    fn handle_chain_start(
        &self,
        serialized: &HashMap<String, Value>,
        inputs: &HashMap<String, Value>,
        run_id: Uuid,
        tags: Option<Vec<String>>,
        parent_run_id: Option<Uuid>,
        metadata: Option<HashMap<String, Value>>,
        run_type: Option<&str>,
        name: Option<&str>,
    ) {
        let name_ = assign_name(name, Some(serialized));
        let run_type_ = run_type.unwrap_or("chain");

        let mut data = EventData::new();
        let mut stored_inputs = None;

        // Work-around: Runnable core code sometimes doesn't send input.
        let is_empty_placeholder =
            inputs.len() == 1 && inputs.get("input") == Some(&Value::String(String::new()));
        if !is_empty_placeholder {
            data = data.with_input(serde_json::to_value(inputs).unwrap_or_default());
            stored_inputs = Some(serde_json::to_value(inputs).unwrap_or_default());
        }

        let parent_ids = {
            let mut state = self.state.lock().expect("state lock poisoned");
            Self::write_run_start_info(
                &mut state,
                run_id,
                tags.clone(),
                metadata.clone(),
                parent_run_id,
                name_.clone(),
                run_type_.to_string(),
                stored_inputs,
            );
            Self::get_parent_ids(&state.parent_map, run_id)
        };

        let event = StandardStreamEvent::new(
            &format!("on_{}_start", run_type_),
            &run_id.to_string(),
            &name_,
        )
        .with_tags(tags.unwrap_or_default())
        .with_metadata(metadata.unwrap_or_default())
        .with_parent_ids(parent_ids)
        .with_data(data);

        self.send(StreamEvent::Standard(event), run_type_);
    }

    /// Handle a chain end event.
    fn handle_chain_end(
        &self,
        outputs: &HashMap<String, Value>,
        run_id: Uuid,
        inputs: Option<&HashMap<String, Value>>,
    ) -> Result<(), String> {
        let (run_info, parent_ids) = {
            let mut state = self.state.lock().expect("state lock poisoned");
            let run_info = state
                .run_map
                .remove(&run_id)
                .ok_or_else(|| format!("Run ID {} not found in run map.", run_id))?;
            let parent_ids = Self::get_parent_ids(&state.parent_map, run_id);
            (run_info, parent_ids)
        };
        let run_type = &run_info.run_type;

        let event_name = format!("on_{}_end", run_type);

        let resolved_inputs = inputs
            .map(|i| serde_json::to_value(i).unwrap_or_default())
            .or(run_info.inputs.clone())
            .unwrap_or(Value::Object(Default::default()));

        let data = EventData::new()
            .with_output(serde_json::to_value(outputs).unwrap_or_default())
            .with_input(resolved_inputs);

        let event = StandardStreamEvent::new(&event_name, &run_id.to_string(), &run_info.name)
            .with_tags(run_info.tags.clone())
            .with_metadata(run_info.metadata.clone())
            .with_parent_ids(parent_ids)
            .with_data(data);

        self.send(StreamEvent::Standard(event), run_type);
        Ok(())
    }

    /// Handle a tool start event.
    #[allow(clippy::too_many_arguments)]
    fn handle_tool_start(
        &self,
        serialized: &HashMap<String, Value>,
        _input_str: &str,
        run_id: Uuid,
        tags: Option<Vec<String>>,
        parent_run_id: Option<Uuid>,
        metadata: Option<HashMap<String, Value>>,
        name: Option<&str>,
        inputs: Option<&HashMap<String, Value>>,
    ) {
        let name_ = assign_name(name, Some(serialized));

        let inputs_value = inputs.map(|i| serde_json::to_value(i).unwrap_or_default());

        let parent_ids = {
            let mut state = self.state.lock().expect("state lock poisoned");
            Self::write_run_start_info(
                &mut state,
                run_id,
                tags.clone(),
                metadata.clone(),
                parent_run_id,
                name_.clone(),
                "tool".to_string(),
                inputs_value.clone(),
            );
            Self::get_parent_ids(&state.parent_map, run_id)
        };

        let event = StandardStreamEvent::new("on_tool_start", &run_id.to_string(), &name_)
            .with_tags(tags.unwrap_or_default())
            .with_metadata(metadata.unwrap_or_default())
            .with_parent_ids(parent_ids)
            .with_data(
                EventData::new()
                    .with_input(inputs_value.unwrap_or(Value::Object(Default::default()))),
            );

        self.send(StreamEvent::Standard(event), "tool");
    }

    /// Get run info for a tool, extracting inputs with validation.
    fn remove_tool_run_info_with_inputs(
        state: &mut HandlerState,
        run_id: Uuid,
    ) -> Result<(RunInfo, Value, Vec<String>), String> {
        let run_info = state
            .run_map
            .remove(&run_id)
            .ok_or_else(|| format!("Run ID {} not found in run map.", run_id))?;

        let inputs = run_info.inputs.clone().ok_or_else(|| {
            format!(
                "Run ID {} is a tool call and is expected to have inputs associated with it.",
                run_id
            )
        })?;

        let parent_ids = Self::get_parent_ids(&state.parent_map, run_id);
        Ok((run_info, inputs, parent_ids))
    }

    /// Handle a tool error event.
    fn handle_tool_error(&self, error: &str, run_id: Uuid) -> Result<(), String> {
        let (run_info, inputs, parent_ids) = {
            let mut state = self.state.lock().expect("state lock poisoned");
            Self::remove_tool_run_info_with_inputs(&mut state, run_id)?
        };

        let data = EventData::new().with_error(error).with_input(inputs);

        let event = StandardStreamEvent::new("on_tool_error", &run_id.to_string(), &run_info.name)
            .with_tags(run_info.tags.clone())
            .with_metadata(run_info.metadata.clone())
            .with_parent_ids(parent_ids)
            .with_data(data);

        self.send(StreamEvent::Standard(event), "tool");
        Ok(())
    }

    /// Handle a tool end event.
    fn handle_tool_end(&self, output: Value, run_id: Uuid) -> Result<(), String> {
        let (run_info, inputs, parent_ids) = {
            let mut state = self.state.lock().expect("state lock poisoned");
            Self::remove_tool_run_info_with_inputs(&mut state, run_id)?
        };

        let data = EventData::new().with_output(output).with_input(inputs);

        let event = StandardStreamEvent::new("on_tool_end", &run_id.to_string(), &run_info.name)
            .with_tags(run_info.tags.clone())
            .with_metadata(run_info.metadata.clone())
            .with_parent_ids(parent_ids)
            .with_data(data);

        self.send(StreamEvent::Standard(event), "tool");
        Ok(())
    }

    /// Handle a retriever start event.
    #[allow(clippy::too_many_arguments)]
    fn handle_retriever_start(
        &self,
        serialized: &HashMap<String, Value>,
        query: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, Value>>,
        name: Option<&str>,
    ) {
        let name_ = assign_name(name, Some(serialized));
        let run_type = "retriever";

        let parent_ids = {
            let mut state = self.state.lock().expect("state lock poisoned");
            Self::write_run_start_info(
                &mut state,
                run_id,
                tags.clone(),
                metadata.clone(),
                parent_run_id,
                name_.clone(),
                run_type.to_string(),
                Some(serde_json::json!({"query": query})),
            );
            Self::get_parent_ids(&state.parent_map, run_id)
        };

        let event = StandardStreamEvent::new("on_retriever_start", &run_id.to_string(), &name_)
            .with_tags(tags.unwrap_or_default())
            .with_metadata(metadata.unwrap_or_default())
            .with_parent_ids(parent_ids)
            .with_data(EventData::new().with_input(serde_json::json!({"query": query})));

        self.send(StreamEvent::Standard(event), run_type);
    }

    /// Handle a retriever end event.
    fn handle_retriever_end(&self, documents: Value, run_id: Uuid) -> Result<(), String> {
        let (run_info, parent_ids) = {
            let mut state = self.state.lock().expect("state lock poisoned");
            let run_info = state
                .run_map
                .remove(&run_id)
                .ok_or_else(|| format!("Run ID {} not found in run map.", run_id))?;
            let parent_ids = Self::get_parent_ids(&state.parent_map, run_id);
            (run_info, parent_ids)
        };

        let mut data = EventData::new().with_output(documents);
        if let Some(inputs) = run_info.inputs.clone() {
            data = data.with_input(inputs);
        }

        let event =
            StandardStreamEvent::new("on_retriever_end", &run_id.to_string(), &run_info.name)
                .with_tags(run_info.tags.clone())
                .with_metadata(run_info.metadata.clone())
                .with_parent_ids(parent_ids)
                .with_data(data);

        self.send(StreamEvent::Standard(event), &run_info.run_type);
        Ok(())
    }
}

// =============================================================================
// BaseCallbackHandler implementation
// =============================================================================

// Bridge from the BaseCallbackHandler mixin trait signatures to the handler's
// internal methods. The mixin traits use `&self` while the handler uses
// interior mutability via Mutex.

impl LLMManagerMixin for AstreamEventsCallbackHandler {
    fn on_llm_new_token(
        &self,
        token: &str,
        run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        chunk: Option<&serde_json::Value>,
    ) {
        if let Err(e) = self.handle_llm_new_token(token, chunk.cloned(), run_id) {
            tracing::warn!("AstreamEventsCallbackHandler on_llm_new_token error: {}", e);
        }
    }

    fn on_llm_end(&self, response: &ChatResult, run_id: Uuid, _parent_run_id: Option<Uuid>) {
        // Convert ChatResult to LLMResult for the internal handler
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
        if let Err(e) = self.handle_llm_end(&llm_result, run_id) {
            tracing::warn!("AstreamEventsCallbackHandler on_llm_end error: {}", e);
        }
    }
}

impl ChainManagerMixin for AstreamEventsCallbackHandler {
    fn on_chain_end(
        &self,
        outputs: &HashMap<String, serde_json::Value>,
        run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        if let Err(e) = self.handle_chain_end(outputs, run_id, None) {
            tracing::warn!("AstreamEventsCallbackHandler on_chain_end error: {}", e);
        }
    }
}

impl ToolManagerMixin for AstreamEventsCallbackHandler {
    fn on_tool_end(
        &self,
        output: &str,
        run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _color: Option<&str>,
        _observation_prefix: Option<&str>,
        _llm_prefix: Option<&str>,
    ) {
        if let Err(e) = self.handle_tool_end(serde_json::json!(output), run_id) {
            tracing::warn!("AstreamEventsCallbackHandler on_tool_end error: {}", e);
        }
    }

    fn on_tool_error(
        &self,
        error: &dyn std::error::Error,
        run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        if let Err(e) = self.handle_tool_error(&error.to_string(), run_id) {
            tracing::warn!("AstreamEventsCallbackHandler on_tool_error error: {}", e);
        }
    }
}

impl RetrieverManagerMixin for AstreamEventsCallbackHandler {
    fn on_retriever_end(
        &self,
        documents: &[serde_json::Value],
        run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        if let Err(e) = self.handle_retriever_end(serde_json::json!(documents), run_id) {
            tracing::warn!("AstreamEventsCallbackHandler on_retriever_end error: {}", e);
        }
    }
}

impl CallbackManagerMixin for AstreamEventsCallbackHandler {
    fn on_llm_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        prompts: &[String],
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
        self.handle_llm_start(
            serialized,
            prompts,
            run_id,
            tags.map(|t| t.to_vec()),
            parent_run_id,
            metadata.cloned(),
            None,
        );
    }

    fn on_chat_model_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        messages: &[Vec<BaseMessage>],
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
        self.handle_chat_model_start(
            serialized,
            messages,
            run_id,
            tags.map(|t| t.to_vec()),
            parent_run_id,
            metadata.cloned(),
            None,
        );
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
        self.handle_chain_start(
            serialized,
            inputs,
            run_id,
            tags.map(|t| t.to_vec()),
            parent_run_id,
            metadata.cloned(),
            None,
            name,
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
        self.handle_tool_start(
            serialized,
            input_str,
            run_id,
            tags.map(|t| t.to_vec()),
            parent_run_id,
            metadata.cloned(),
            None,
            inputs,
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
        name: Option<&str>,
    ) {
        let _ = name;
        self.handle_retriever_start(
            serialized,
            query,
            run_id,
            parent_run_id,
            tags.map(|t| t.to_vec()),
            metadata.cloned(),
            None,
        );
    }
}

impl RunManagerMixin for AstreamEventsCallbackHandler {
    fn on_custom_event(
        &self,
        name: &str,
        data: &dyn std::any::Any,
        run_id: Uuid,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
        // Try to downcast to Value for serialization
        let value = if let Some(v) = data.downcast_ref::<serde_json::Value>() {
            v.clone()
        } else {
            serde_json::json!(null)
        };
        self.handle_custom_event(
            name,
            value,
            run_id,
            tags.map(|t| t.to_vec()),
            metadata.cloned(),
        );
    }
}

impl BaseCallbackHandler for AstreamEventsCallbackHandler {
    fn name(&self) -> &str {
        "AstreamEventsCallbackHandler"
    }

    fn run_inline(&self) -> bool {
        true
    }
}

impl std::fmt::Debug for AstreamEventsCallbackHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = self.state.lock().expect("state lock poisoned");
        f.debug_struct("AstreamEventsCallbackHandler")
            .field("run_map_len", &state.run_map.len())
            .field("parent_map_len", &state.parent_map.len())
            .field("root_event_filter", &"RootEventFilter { .. }")
            .finish()
    }
}

// =============================================================================
// StreamingCallbackHandler implementation
// =============================================================================

impl StreamingCallbackHandler<crate::error::Result<Value>> for AstreamEventsCallbackHandler {
    fn tap_output_aiter(
        &self,
        run_id: Uuid,
        output: Pin<Box<dyn futures::Stream<Item = crate::error::Result<Value>> + Send>>,
    ) -> Pin<Box<dyn futures::Stream<Item = crate::error::Result<Value>> + Send>> {
        // Atomic check-and-set: if already tapped, pass through
        {
            let mut state = self.state.lock().expect("state lock poisoned");
            if state.is_tapped.contains_key(&run_id) {
                return output;
            }
            state.is_tapped.insert(run_id, true);
        }

        let send_stream = self.send_stream.clone();
        let root_event_filter = self.root_event_filter.clone();

        // Snapshot the state we need for the stream closure
        let state_mutex = &self.state;
        let run_info_snapshot = {
            let state = state_mutex.lock().expect("state lock poisoned");
            state.run_map.get(&run_id).cloned()
        };

        // If there's no run info yet (run hasn't started), just pass through
        let Some(run_info) = run_info_snapshot else {
            return output;
        };

        let parent_ids = {
            let state = state_mutex.lock().expect("state lock poisoned");
            Self::get_parent_ids(&state.parent_map, run_id)
        };

        let event_name = format!("on_{}_stream", run_info.run_type);
        let run_type = run_info.run_type.clone();

        Box::pin(output.map(move |chunk| {
            if let Ok(ref value) = chunk {
                let event =
                    StandardStreamEvent::new(&event_name, &run_id.to_string(), &run_info.name)
                        .with_tags(run_info.tags.clone())
                        .with_metadata(run_info.metadata.clone())
                        .with_parent_ids(parent_ids.clone())
                        .with_data(EventData::new().with_chunk(value.clone()));

                let (name, tags) = (run_info.name.as_str(), run_info.tags.as_slice());
                if root_event_filter.include_event(name, tags, &run_type) {
                    if let Err(e) = send_stream.send(StreamEvent::Standard(event)) {
                        tracing::warn!("Failed to send stream event: {}", e);
                    }
                }
            }
            chunk
        }))
    }

    fn tap_output_iter(
        &self,
        run_id: Uuid,
        output: Box<dyn Iterator<Item = crate::error::Result<Value>> + Send>,
    ) -> Box<dyn Iterator<Item = crate::error::Result<Value>> + Send> {
        // Atomic check-and-set: if already tapped, pass through
        {
            let mut state = self.state.lock().expect("state lock poisoned");
            if state.is_tapped.contains_key(&run_id) {
                return output;
            }
            state.is_tapped.insert(run_id, true);
        }

        // Sync iteration — just pass through for now
        // (astream_events is an async API; sync tap is rarely used)
        output
    }
}

// =============================================================================
// astream_events_implementation (free function)
// =============================================================================

/// Implementation of the astream_events API for V2 runnables.
///
/// This is a free function that mirrors Python's
/// `_astream_events_implementation_v2`. It creates the handler,
/// injects it into the config, consumes `astream()` while
/// forwarding events from the receive stream.
///
/// Mirrors `langchain_core.tracers.event_stream._astream_events_implementation_v2`.
pub fn astream_events_implementation<'a, R>(
    runnable: &'a R,
    input: R::Input,
    config: Option<crate::runnables::config::RunnableConfig>,
    include_names: Option<Vec<String>>,
    include_types: Option<Vec<String>>,
    include_tags: Option<Vec<String>>,
    exclude_names: Option<Vec<String>>,
    exclude_types: Option<Vec<String>>,
    exclude_tags: Option<Vec<String>>,
) -> BoxStream<'a, StreamEvent>
where
    R: crate::runnables::base::Runnable + 'static,
    R::Output: serde::Serialize,
{
    use crate::callbacks::base::Callbacks;
    use crate::runnables::config::ensure_config;
    use crate::utils::uuid::uuid7;
    use std::sync::Arc;

    let event_streamer = Arc::new(AstreamEventsCallbackHandler::new(
        include_names,
        include_types,
        include_tags,
        exclude_names,
        exclude_types,
        exclude_tags,
    ));

    let mut config = ensure_config(config);

    // Assign a run_id if not already set
    let run_id = config.run_id.unwrap_or_else(|| uuid7(None));
    config.run_id = Some(run_id);

    // Inject the event streamer into callbacks
    let handler: Arc<dyn BaseCallbackHandler> = event_streamer.clone();
    match &mut config.callbacks {
        None => {
            config.callbacks = Some(Callbacks::Handlers(vec![handler]));
        }
        Some(Callbacks::Handlers(handlers)) => {
            handlers.push(handler);
        }
        Some(Callbacks::Manager(manager)) => {
            manager.add_handler(handler, true);
        }
    }

    // Take the receive stream before starting
    let receive_stream = event_streamer
        .take_receive_stream()
        .expect("receive stream should be available");

    let send_stream = event_streamer.get_send_stream();

    Box::pin(async_stream::stream! {
        // Consume the astream output. Callbacks fire synchronously during
        // each poll of the stream, pushing events into the unbounded channel.
        // After the stream finishes, close the send stream so the receive
        // stream terminates.
        let mut astream = std::pin::pin!(runnable.astream(input, Some(config)));
        while let Some(_chunk) = astream.next().await {
            // Chunks are consumed to drive the runnable forward.
            // All events are produced via callbacks into the channel.
        }
        let _ = send_stream.close();

        // Now drain all buffered events from the receive stream.
        let mut first_event_sent = false;
        let mut first_event_run_id: Option<String> = None;

        let mut event_stream = std::pin::pin!(receive_stream.into_stream());
        while let Some(mut event) = event_stream.next().await {
            if !first_event_sent {
                first_event_sent = true;
                // Store the run_id of the first event
                first_event_run_id = match &event {
                    StreamEvent::Standard(e) => Some(e.base.run_id.clone()),
                    StreamEvent::Custom(e) => Some(e.base.run_id.clone()),
                };
                yield event;
                continue;
            }

            // If it's the end event corresponding to the root runnable,
            // remove the input from data since it was in the first event.
            if let StreamEvent::Standard(ref mut e) = event {
                if Some(&e.base.run_id) == first_event_run_id.as_ref()
                    && e.base.event.ends_with("_end")
                {
                    e.data.input = None;
                }
            }

            yield event;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assign_name_with_name() {
        assert_eq!(assign_name(Some("test"), None), "test");
    }

    #[test]
    fn test_assign_name_from_serialized_name() {
        let mut serialized = HashMap::new();
        serialized.insert("name".to_string(), Value::String("MyModel".to_string()));
        assert_eq!(assign_name(None, Some(&serialized)), "MyModel");
    }

    #[test]
    fn test_assign_name_from_serialized_id() {
        let mut serialized = HashMap::new();
        serialized.insert(
            "id".to_string(),
            serde_json::json!(["langchain", "llms", "ChatOpenAI"]),
        );
        assert_eq!(assign_name(None, Some(&serialized)), "ChatOpenAI");
    }

    #[test]
    fn test_assign_name_fallback() {
        assert_eq!(assign_name(None, None), "Unnamed");
    }

    #[test]
    fn test_handler_creation() {
        let handler = AstreamEventsCallbackHandler::new(
            Some(vec!["test".to_string()]),
            None,
            None,
            None,
            None,
            None,
        );
        let state = handler.state.lock().unwrap();
        assert!(state.run_map.is_empty());
        assert!(state.parent_map.is_empty());
        assert!(state.is_tapped.is_empty());
    }

    #[test]
    fn test_get_parent_ids() {
        let handler = AstreamEventsCallbackHandler::new(None, None, None, None, None, None);

        let root_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();
        let grandchild_id = Uuid::new_v4();

        {
            let mut state = handler.state.lock().unwrap();
            state.parent_map.insert(root_id, None);
            state.parent_map.insert(child_id, Some(root_id));
            state.parent_map.insert(grandchild_id, Some(child_id));
        }

        let state = handler.state.lock().unwrap();
        let parent_ids =
            AstreamEventsCallbackHandler::get_parent_ids(&state.parent_map, grandchild_id);
        assert_eq!(parent_ids.len(), 2);
        // Root first, immediate parent last
        assert_eq!(parent_ids[0], root_id.to_string());
        assert_eq!(parent_ids[1], child_id.to_string());
    }

    #[test]
    fn test_get_parent_ids_root() {
        let handler = AstreamEventsCallbackHandler::new(None, None, None, None, None, None);

        let root_id = Uuid::new_v4();
        {
            let mut state = handler.state.lock().unwrap();
            state.parent_map.insert(root_id, None);
        }

        let state = handler.state.lock().unwrap();
        let parent_ids = AstreamEventsCallbackHandler::get_parent_ids(&state.parent_map, root_id);
        assert!(parent_ids.is_empty());
    }

    #[test]
    fn test_write_run_start_info() {
        let handler = AstreamEventsCallbackHandler::new(None, None, None, None, None, None);

        let run_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();

        {
            let mut state = handler.state.lock().unwrap();
            AstreamEventsCallbackHandler::write_run_start_info(
                &mut state,
                run_id,
                Some(vec!["tag1".to_string()]),
                Some(HashMap::from([(
                    "key".to_string(),
                    Value::String("value".to_string()),
                )])),
                Some(parent_id),
                "test_chain".to_string(),
                "chain".to_string(),
                Some(serde_json::json!({"input": "hello"})),
            );
        }

        let state = handler.state.lock().unwrap();
        assert!(state.run_map.contains_key(&run_id));
        let info = &state.run_map[&run_id];
        assert_eq!(info.name, "test_chain");
        assert_eq!(info.run_type, "chain");
        assert_eq!(info.tags, vec!["tag1"]);
        assert_eq!(info.parent_run_id, Some(parent_id));
        assert!(info.inputs.is_some());
        assert_eq!(state.parent_map[&run_id], Some(parent_id));
    }

    #[test]
    fn test_on_chain_start_end() {
        let handler = AstreamEventsCallbackHandler::new(None, None, None, None, None, None);

        let run_id = Uuid::new_v4();
        let serialized =
            HashMap::from([("name".to_string(), Value::String("TestChain".to_string()))]);

        let inputs = HashMap::from([("input".to_string(), serde_json::json!("hello"))]);

        handler.handle_chain_start(&serialized, &inputs, run_id, None, None, None, None, None);

        {
            let state = handler.state.lock().unwrap();
            assert!(state.run_map.contains_key(&run_id));
            assert_eq!(state.run_map[&run_id].name, "TestChain");
        }

        let outputs = HashMap::from([("output".to_string(), serde_json::json!("world"))]);
        let result = handler.handle_chain_end(&outputs, run_id, None);
        assert!(result.is_ok());

        let state = handler.state.lock().unwrap();
        assert!(!state.run_map.contains_key(&run_id));
    }

    #[test]
    fn test_on_tool_start_end() {
        let handler = AstreamEventsCallbackHandler::new(None, None, None, None, None, None);

        let run_id = Uuid::new_v4();
        let serialized = HashMap::new();
        let inputs = HashMap::from([("arg".to_string(), serde_json::json!("value"))]);

        handler.handle_tool_start(
            &serialized,
            "",
            run_id,
            None,
            None,
            None,
            Some("MyTool"),
            Some(&inputs),
        );

        {
            let state = handler.state.lock().unwrap();
            assert!(state.run_map.contains_key(&run_id));
        }

        let result = handler.handle_tool_end(serde_json::json!("tool result"), run_id);
        assert!(result.is_ok());

        let state = handler.state.lock().unwrap();
        assert!(!state.run_map.contains_key(&run_id));
    }

    #[test]
    fn test_on_tool_error() {
        let handler = AstreamEventsCallbackHandler::new(None, None, None, None, None, None);

        let run_id = Uuid::new_v4();
        let serialized = HashMap::new();
        let inputs = HashMap::from([("arg".to_string(), serde_json::json!("value"))]);

        handler.handle_tool_start(
            &serialized,
            "",
            run_id,
            None,
            None,
            None,
            Some("FailTool"),
            Some(&inputs),
        );

        let result = handler.handle_tool_error("something went wrong", run_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_on_retriever_start_end() {
        let handler = AstreamEventsCallbackHandler::new(None, None, None, None, None, None);

        let run_id = Uuid::new_v4();
        let serialized =
            HashMap::from([("name".to_string(), Value::String("MyRetriever".to_string()))]);

        handler.handle_retriever_start(&serialized, "search query", run_id, None, None, None, None);

        {
            let state = handler.state.lock().unwrap();
            assert!(state.run_map.contains_key(&run_id));
        }

        let docs = serde_json::json!([{"page_content": "result", "metadata": {}}]);
        let result = handler.handle_retriever_end(docs, run_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_take_receive_stream() {
        let handler = AstreamEventsCallbackHandler::new(None, None, None, None, None, None);

        // First take should succeed
        assert!(handler.take_receive_stream().is_some());

        // Second take should return None
        assert!(handler.take_receive_stream().is_none());
    }

    #[test]
    fn test_handler_implements_base_callback_handler() {
        let handler = AstreamEventsCallbackHandler::new(None, None, None, None, None, None);
        // Verify it can be used as a BaseCallbackHandler
        let _handler_ref: &dyn BaseCallbackHandler = &handler;
        assert_eq!(handler.name(), "AstreamEventsCallbackHandler");
        assert!(handler.run_inline());
    }
}
