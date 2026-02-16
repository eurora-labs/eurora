//! Internal tracer to power the event stream API.
//!
//! This module provides the callback handler and implementation functions for
//! the `astream_events()` API. It converts nested tracer run data into a flat
//! stream of typed events.
//!
//! Mirrors `langchain_core.tracers.event_stream`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::GenerationType;
use crate::messages::{AIMessageChunk, BaseMessage};
use crate::outputs::{GenerationChunk, LLMResult};
use crate::runnables::schema::{CustomStreamEvent, EventData, StandardStreamEvent, StreamEvent};
use crate::runnables::utils::RootEventFilter;
use crate::tracers::memory_stream::{MemoryStream, ReceiveStream, SendStream};

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

/// An implementation of a callback handler for astream events.
///
/// This handler tracks run metadata and sends stream events through a memory
/// stream. It is used internally by `astream_events()`.
pub struct AstreamEventsCallbackHandler {
    /// Map of run ID to run info. Entries are cleaned up when each run ends.
    pub run_map: HashMap<Uuid, RunInfo>,
    /// Map of child run ID to parent run ID. Kept separately from run_map
    /// because parent end events may fire before child end events.
    pub parent_map: HashMap<Uuid, Option<Uuid>>,
    /// Track which runs have been tapped for streaming.
    pub is_tapped: HashMap<Uuid, bool>,
    /// Filter which events will be sent over the queue.
    pub root_event_filter: RootEventFilter,
    /// The send stream for events.
    pub send_stream: SendStream<StreamEvent>,
    /// The receive stream for events.
    pub receive_stream: Option<ReceiveStream<StreamEvent>>,
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
            run_map: HashMap::new(),
            parent_map: HashMap::new(),
            is_tapped: HashMap::new(),
            root_event_filter: RootEventFilter {
                include_names,
                include_types,
                include_tags,
                exclude_names,
                exclude_types,
                exclude_tags,
            },
            send_stream,
            receive_stream: Some(receive_stream),
        }
    }

    /// Take the receive stream. Can only be called once.
    pub fn take_receive_stream(&mut self) -> Option<ReceiveStream<StreamEvent>> {
        self.receive_stream.take()
    }

    /// Get the parent IDs of a run (non-recursively) cast to strings.
    ///
    /// Returns parent IDs in order from root to immediate parent.
    pub fn get_parent_ids(&self, mut run_id: Uuid) -> Vec<String> {
        let mut parent_ids = Vec::new();

        while let Some(Some(parent_id)) = self.parent_map.get(&run_id) {
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
        &mut self,
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

        self.run_map.insert(run_id, info);
        self.parent_map.insert(run_id, parent_run_id);
    }

    /// Handle a chat model start event.
    pub fn on_chat_model_start(
        &mut self,
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

        self.write_run_start_info(
            run_id,
            tags.clone(),
            metadata.clone(),
            parent_run_id,
            name_.clone(),
            run_type.to_string(),
            Some(serde_json::to_value(messages).unwrap_or_default()),
        );

        let event = StandardStreamEvent::new("on_chat_model_start", &run_id.to_string(), &name_)
            .with_tags(tags.unwrap_or_default())
            .with_metadata(metadata.unwrap_or_default())
            .with_parent_ids(self.get_parent_ids(run_id))
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
    pub fn on_llm_start(
        &mut self,
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

        self.write_run_start_info(
            run_id,
            tags.clone(),
            metadata.clone(),
            parent_run_id,
            name_.clone(),
            run_type.to_string(),
            Some(serde_json::to_value(&HashMap::from([("prompts", prompts)])).unwrap_or_default()),
        );

        let event = StandardStreamEvent::new("on_llm_start", &run_id.to_string(), &name_)
            .with_tags(tags.unwrap_or_default())
            .with_metadata(metadata.unwrap_or_default())
            .with_parent_ids(self.get_parent_ids(run_id))
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
    pub fn on_custom_event(
        &mut self,
        name: &str,
        data: Value,
        run_id: Uuid,
        tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, Value>>,
    ) {
        let event = CustomStreamEvent::new(name, &run_id.to_string(), data.clone())
            .with_tags(tags.unwrap_or_default())
            .with_metadata(metadata.unwrap_or_default())
            .with_parent_ids(self.get_parent_ids(run_id));

        self.send(StreamEvent::Custom(event), name);
    }

    /// Handle a new LLM token event.
    ///
    /// For both chat models and non-chat models (legacy LLMs).
    pub fn on_llm_new_token(
        &mut self,
        token: &str,
        chunk: Option<Value>,
        run_id: Uuid,
    ) -> Result<(), String> {
        let run_info = self
            .run_map
            .get(&run_id)
            .ok_or_else(|| format!("Run ID {} not found in run map.", run_id))?;

        if self.is_tapped.get(&run_id).is_some() {
            return Ok(());
        }

        let (event_name, chunk_value) = if run_info.run_type == "chat_model" {
            let chunk_value = if let Some(c) = chunk {
                // Extract the message from the ChatGenerationChunk
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

        let event = StandardStreamEvent::new(event_name, &run_id.to_string(), &run_info.name)
            .with_tags(run_info.tags.clone())
            .with_metadata(run_info.metadata.clone())
            .with_parent_ids(self.get_parent_ids(run_id))
            .with_data(EventData::new().with_chunk(chunk_value));

        self.send(StreamEvent::Standard(event), &run_info.run_type);
        Ok(())
    }

    /// Handle an LLM end event.
    ///
    /// For both chat models and non-chat models (legacy LLMs).
    pub fn on_llm_end(&mut self, response: &LLMResult, run_id: Uuid) -> Result<(), String> {
        let run_info = self
            .run_map
            .remove(&run_id)
            .ok_or_else(|| format!("Run ID {} not found in run map.", run_id))?;
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
            .with_parent_ids(self.get_parent_ids(run_id))
            .with_data(data);

        self.send(StreamEvent::Standard(event), &run_info.run_type);
        Ok(())
    }

    /// Handle a chain start event.
    pub fn on_chain_start(
        &mut self,
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

        self.write_run_start_info(
            run_id,
            tags.clone(),
            metadata.clone(),
            parent_run_id,
            name_.clone(),
            run_type_.to_string(),
            stored_inputs,
        );

        let event = StandardStreamEvent::new(
            &format!("on_{}_start", run_type_),
            &run_id.to_string(),
            &name_,
        )
        .with_tags(tags.unwrap_or_default())
        .with_metadata(metadata.unwrap_or_default())
        .with_parent_ids(self.get_parent_ids(run_id))
        .with_data(data);

        self.send(StreamEvent::Standard(event), run_type_);
    }

    /// Handle a chain end event.
    pub fn on_chain_end(
        &mut self,
        outputs: &HashMap<String, Value>,
        run_id: Uuid,
        inputs: Option<&HashMap<String, Value>>,
    ) -> Result<(), String> {
        let run_info = self
            .run_map
            .remove(&run_id)
            .ok_or_else(|| format!("Run ID {} not found in run map.", run_id))?;
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
            .with_parent_ids(self.get_parent_ids(run_id))
            .with_data(data);

        self.send(StreamEvent::Standard(event), run_type);
        Ok(())
    }

    /// Handle a tool start event.
    pub fn on_tool_start(
        &mut self,
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

        self.write_run_start_info(
            run_id,
            tags.clone(),
            metadata.clone(),
            parent_run_id,
            name_.clone(),
            "tool".to_string(),
            inputs_value.clone(),
        );

        let event = StandardStreamEvent::new("on_tool_start", &run_id.to_string(), &name_)
            .with_tags(tags.unwrap_or_default())
            .with_metadata(metadata.unwrap_or_default())
            .with_parent_ids(self.get_parent_ids(run_id))
            .with_data(
                EventData::new()
                    .with_input(inputs_value.unwrap_or(Value::Object(Default::default()))),
            );

        self.send(StreamEvent::Standard(event), "tool");
    }

    /// Get run info for a tool, extracting inputs with validation.
    fn get_tool_run_info_with_inputs(&mut self, run_id: Uuid) -> Result<(RunInfo, Value), String> {
        let run_info = self
            .run_map
            .remove(&run_id)
            .ok_or_else(|| format!("Run ID {} not found in run map.", run_id))?;

        let inputs = run_info.inputs.clone().ok_or_else(|| {
            format!(
                "Run ID {} is a tool call and is expected to have inputs associated with it.",
                run_id
            )
        })?;

        Ok((run_info, inputs))
    }

    /// Handle a tool error event.
    pub fn on_tool_error(&mut self, error: &str, run_id: Uuid) -> Result<(), String> {
        let (run_info, inputs) = self.get_tool_run_info_with_inputs(run_id)?;

        let data = EventData::new().with_error(error).with_input(inputs);

        let event = StandardStreamEvent::new("on_tool_error", &run_id.to_string(), &run_info.name)
            .with_tags(run_info.tags.clone())
            .with_metadata(run_info.metadata.clone())
            .with_parent_ids(self.get_parent_ids(run_id))
            .with_data(data);

        self.send(StreamEvent::Standard(event), "tool");
        Ok(())
    }

    /// Handle a tool end event.
    pub fn on_tool_end(&mut self, output: Value, run_id: Uuid) -> Result<(), String> {
        let (run_info, inputs) = self.get_tool_run_info_with_inputs(run_id)?;

        let data = EventData::new().with_output(output).with_input(inputs);

        let event = StandardStreamEvent::new("on_tool_end", &run_id.to_string(), &run_info.name)
            .with_tags(run_info.tags.clone())
            .with_metadata(run_info.metadata.clone())
            .with_parent_ids(self.get_parent_ids(run_id))
            .with_data(data);

        self.send(StreamEvent::Standard(event), "tool");
        Ok(())
    }

    /// Handle a retriever start event.
    pub fn on_retriever_start(
        &mut self,
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

        self.write_run_start_info(
            run_id,
            tags.clone(),
            metadata.clone(),
            parent_run_id,
            name_.clone(),
            run_type.to_string(),
            Some(serde_json::json!({"query": query})),
        );

        let event = StandardStreamEvent::new("on_retriever_start", &run_id.to_string(), &name_)
            .with_tags(tags.unwrap_or_default())
            .with_metadata(metadata.unwrap_or_default())
            .with_parent_ids(self.get_parent_ids(run_id))
            .with_data(EventData::new().with_input(serde_json::json!({"query": query})));

        self.send(StreamEvent::Standard(event), run_type);
    }

    /// Handle a retriever end event.
    pub fn on_retriever_end(&mut self, documents: Value, run_id: Uuid) -> Result<(), String> {
        let run_info = self
            .run_map
            .remove(&run_id)
            .ok_or_else(|| format!("Run ID {} not found in run map.", run_id))?;

        let mut data = EventData::new().with_output(documents);
        if let Some(inputs) = run_info.inputs.clone() {
            data = data.with_input(inputs);
        }

        let event =
            StandardStreamEvent::new("on_retriever_end", &run_id.to_string(), &run_info.name)
                .with_tags(run_info.tags.clone())
                .with_metadata(run_info.metadata.clone())
                .with_parent_ids(self.get_parent_ids(run_id))
                .with_data(data);

        self.send(StreamEvent::Standard(event), &run_info.run_type);
        Ok(())
    }
}

impl std::fmt::Debug for AstreamEventsCallbackHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AstreamEventsCallbackHandler")
            .field("run_map_len", &self.run_map.len())
            .field("parent_map_len", &self.parent_map.len())
            .field("root_event_filter", &"RootEventFilter { .. }")
            .finish()
    }
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
        assert!(handler.run_map.is_empty());
        assert!(handler.parent_map.is_empty());
        assert!(handler.is_tapped.is_empty());
    }

    #[test]
    fn test_get_parent_ids() {
        let mut handler = AstreamEventsCallbackHandler::new(None, None, None, None, None, None);

        let root_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();
        let grandchild_id = Uuid::new_v4();

        handler.parent_map.insert(root_id, None);
        handler.parent_map.insert(child_id, Some(root_id));
        handler.parent_map.insert(grandchild_id, Some(child_id));

        let parent_ids = handler.get_parent_ids(grandchild_id);
        assert_eq!(parent_ids.len(), 2);
        // Root first, immediate parent last
        assert_eq!(parent_ids[0], root_id.to_string());
        assert_eq!(parent_ids[1], child_id.to_string());
    }

    #[test]
    fn test_get_parent_ids_root() {
        let mut handler = AstreamEventsCallbackHandler::new(None, None, None, None, None, None);

        let root_id = Uuid::new_v4();
        handler.parent_map.insert(root_id, None);

        let parent_ids = handler.get_parent_ids(root_id);
        assert!(parent_ids.is_empty());
    }

    #[test]
    fn test_write_run_start_info() {
        let mut handler = AstreamEventsCallbackHandler::new(None, None, None, None, None, None);

        let run_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();

        handler.write_run_start_info(
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

        assert!(handler.run_map.contains_key(&run_id));
        let info = &handler.run_map[&run_id];
        assert_eq!(info.name, "test_chain");
        assert_eq!(info.run_type, "chain");
        assert_eq!(info.tags, vec!["tag1"]);
        assert_eq!(info.parent_run_id, Some(parent_id));
        assert!(info.inputs.is_some());

        assert_eq!(handler.parent_map[&run_id], Some(parent_id));
    }

    #[test]
    fn test_on_chain_start_end() {
        let mut handler = AstreamEventsCallbackHandler::new(None, None, None, None, None, None);

        let run_id = Uuid::new_v4();
        let serialized =
            HashMap::from([("name".to_string(), Value::String("TestChain".to_string()))]);

        let inputs = HashMap::from([("input".to_string(), serde_json::json!("hello"))]);

        handler.on_chain_start(&serialized, &inputs, run_id, None, None, None, None, None);

        assert!(handler.run_map.contains_key(&run_id));
        assert_eq!(handler.run_map[&run_id].name, "TestChain");

        let outputs = HashMap::from([("output".to_string(), serde_json::json!("world"))]);
        let result = handler.on_chain_end(&outputs, run_id, None);
        assert!(result.is_ok());
        assert!(!handler.run_map.contains_key(&run_id));
    }

    #[test]
    fn test_on_tool_start_end() {
        let mut handler = AstreamEventsCallbackHandler::new(None, None, None, None, None, None);

        let run_id = Uuid::new_v4();
        let serialized = HashMap::new();
        let inputs = HashMap::from([("arg".to_string(), serde_json::json!("value"))]);

        handler.on_tool_start(
            &serialized,
            "",
            run_id,
            None,
            None,
            None,
            Some("MyTool"),
            Some(&inputs),
        );

        assert!(handler.run_map.contains_key(&run_id));

        let result = handler.on_tool_end(serde_json::json!("tool result"), run_id);
        assert!(result.is_ok());
        assert!(!handler.run_map.contains_key(&run_id));
    }

    #[test]
    fn test_on_tool_error() {
        let mut handler = AstreamEventsCallbackHandler::new(None, None, None, None, None, None);

        let run_id = Uuid::new_v4();
        let serialized = HashMap::new();
        let inputs = HashMap::from([("arg".to_string(), serde_json::json!("value"))]);

        handler.on_tool_start(
            &serialized,
            "",
            run_id,
            None,
            None,
            None,
            Some("FailTool"),
            Some(&inputs),
        );

        let result = handler.on_tool_error("something went wrong", run_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_on_retriever_start_end() {
        let mut handler = AstreamEventsCallbackHandler::new(None, None, None, None, None, None);

        let run_id = Uuid::new_v4();
        let serialized =
            HashMap::from([("name".to_string(), Value::String("MyRetriever".to_string()))]);

        handler.on_retriever_start(&serialized, "search query", run_id, None, None, None, None);

        assert!(handler.run_map.contains_key(&run_id));

        let docs = serde_json::json!([{"page_content": "result", "metadata": {}}]);
        let result = handler.on_retriever_end(docs, run_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_take_receive_stream() {
        let mut handler = AstreamEventsCallbackHandler::new(None, None, None, None, None, None);

        // First take should succeed
        assert!(handler.take_receive_stream().is_some());

        // Second take should return None
        assert!(handler.take_receive_stream().is_none());
    }
}
