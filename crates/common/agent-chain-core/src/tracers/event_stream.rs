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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunInfo {
    pub name: String,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, Value>,
    pub run_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Value>,
    pub parent_run_id: Option<Uuid>,
}

fn assign_name(name: Option<&str>, serialized: Option<&HashMap<String, Value>>) -> String {
    if let Some(n) = name {
        return n.to_string();
    }
    if let Some(s) = serialized {
        if let Some(Value::String(n)) = s.get("name") {
            return n.clone();
        }
        if let Some(Value::Array(ids)) = s.get("id")
            && let Some(Value::String(last)) = ids.last()
        {
            return last.clone();
        }
    }
    "Unnamed".to_string()
}

#[derive(Debug)]
struct HandlerState {
    run_map: HashMap<Uuid, RunInfo>,
    parent_map: HashMap<Uuid, Option<Uuid>>,
    is_tapped: HashMap<Uuid, bool>,
}

pub struct AstreamEventsCallbackHandler {
    state: Mutex<HandlerState>,
    root_event_filter: RootEventFilter,
    send_stream: SendStream<StreamEvent>,
    receive_stream: Mutex<Option<ReceiveStream<StreamEvent>>>,
}

impl AstreamEventsCallbackHandler {
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

    pub fn take_receive_stream(&self) -> Option<ReceiveStream<StreamEvent>> {
        self.receive_stream
            .lock()
            .expect("receive_stream lock poisoned")
            .take()
    }

    pub fn get_send_stream(&self) -> SendStream<StreamEvent> {
        self.send_stream.clone()
    }

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

        parent_ids.reverse();
        parent_ids
    }

    fn send(&self, event: StreamEvent, event_type: &str) {
        let (event_name, event_tags) = match &event {
            StreamEvent::Standard(e) => (e.name.as_str(), e.base.tags.as_slice()),
            StreamEvent::Custom(e) => (e.name.as_str(), e.base.tags.as_slice()),
        };
        if self
            .root_event_filter
            .include_event(event_name, event_tags, event_type)
            && let Err(error) = self.send_stream.send(event)
        {
            tracing::warn!("Failed to send stream event: {}", error);
        }
    }

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

        let event = StandardStreamEvent::builder()
            .event("on_chat_model_start")
            .run_id(run_id.to_string())
            .name(&name_)
            .tags(tags.unwrap_or_default())
            .metadata(metadata.unwrap_or_default())
            .parent_ids(parent_ids)
            .data(
                EventData::builder()
                    .input(
                        serde_json::to_value(HashMap::from([(
                            "messages",
                            serde_json::to_value(messages).unwrap_or_default(),
                        )]))
                        .unwrap_or_default(),
                    )
                    .build(),
            )
            .build();

        self.send(StreamEvent::Standard(event), run_type);
    }

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
                    serde_json::to_value(HashMap::from([("prompts", prompts)])).unwrap_or_default(),
                ),
            );
            Self::get_parent_ids(&state.parent_map, run_id)
        };

        let event = StandardStreamEvent::builder()
            .event("on_llm_start")
            .run_id(run_id.to_string())
            .name(&name_)
            .tags(tags.unwrap_or_default())
            .metadata(metadata.unwrap_or_default())
            .parent_ids(parent_ids)
            .data(
                EventData::builder()
                    .input(
                        serde_json::to_value(HashMap::from([(
                            "prompts",
                            serde_json::to_value(prompts).unwrap_or_default(),
                        )]))
                        .unwrap_or_default(),
                    )
                    .build(),
            )
            .build();

        self.send(StreamEvent::Standard(event), run_type);
    }

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

        let event = CustomStreamEvent::builder()
            .run_id(name)
            .name(run_id.to_string())
            .data(data.clone())
            .tags(tags.unwrap_or_default())
            .metadata(metadata.unwrap_or_default())
            .parent_ids(parent_ids)
            .build();

        self.send(StreamEvent::Custom(event), name);
    }

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

        if state.is_tapped.contains_key(&run_id) {
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

        let event = StandardStreamEvent::builder()
            .event(event_name)
            .run_id(run_id.to_string())
            .name(&run_info.name)
            .tags(run_info.tags.clone())
            .metadata(run_info.metadata.clone())
            .parent_ids(parent_ids)
            .data(EventData::builder().chunk(chunk_value).build())
            .build();

        let run_type = run_info.run_type.clone();
        drop(state);

        self.send(StreamEvent::Standard(event), &run_type);
        Ok(())
    }

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
            let output = response
                .generations
                .iter()
                .flatten()
                .next()
                .map(|generation| match generation {
                    GenerationType::ChatGeneration(cg) => {
                        serde_json::to_value(&cg.message).unwrap_or(Value::Null)
                    }
                    GenerationType::ChatGenerationChunk(cgc) => {
                        serde_json::to_value(&cgc.message).unwrap_or(Value::Null)
                    }
                    _ => Value::Null,
                })
                .unwrap_or(Value::Null);
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

        let mut data = EventData::builder().output(output).build();
        if let Some(inp) = inputs {
            data.input = Some(inp);
        }

        let event = StandardStreamEvent::builder()
            .event(event_name)
            .run_id(run_id.to_string())
            .name(&run_info.name)
            .tags(run_info.tags.clone())
            .metadata(run_info.metadata.clone())
            .parent_ids(parent_ids)
            .data(data)
            .build();

        self.send(StreamEvent::Standard(event), &run_info.run_type);
        Ok(())
    }

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

        let mut data = EventData::builder().build();
        let mut stored_inputs = None;

        let is_empty_placeholder =
            inputs.len() == 1 && inputs.get("input") == Some(&Value::String(String::new()));
        if !is_empty_placeholder {
            data.input = Some(serde_json::to_value(inputs).unwrap_or_default());
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

        let event = StandardStreamEvent::builder()
            .event(format!("on_{}_start", run_type_))
            .run_id(run_id.to_string())
            .name(&name_)
            .tags(tags.unwrap_or_default())
            .metadata(metadata.unwrap_or_default())
            .parent_ids(parent_ids)
            .data(data)
            .build();

        self.send(StreamEvent::Standard(event), run_type_);
    }

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

        let data = EventData::builder()
            .output(serde_json::to_value(outputs).unwrap_or_default())
            .input(resolved_inputs)
            .build();

        let event = StandardStreamEvent::builder()
            .event(&event_name)
            .run_id(run_id.to_string())
            .name(&run_info.name)
            .tags(run_info.tags.clone())
            .metadata(run_info.metadata.clone())
            .parent_ids(parent_ids)
            .data(data)
            .build();

        self.send(StreamEvent::Standard(event), run_type);
        Ok(())
    }

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

        let event = StandardStreamEvent::builder()
            .event("on_tool_start")
            .run_id(run_id.to_string())
            .name(&name_)
            .tags(tags.unwrap_or_default())
            .metadata(metadata.unwrap_or_default())
            .parent_ids(parent_ids)
            .data(
                EventData::builder()
                    .input(inputs_value.unwrap_or(Value::Object(Default::default())))
                    .build(),
            )
            .build();

        self.send(StreamEvent::Standard(event), "tool");
    }

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

    fn handle_tool_error(&self, error: &str, run_id: Uuid) -> Result<(), String> {
        let (run_info, inputs, parent_ids) = {
            let mut state = self.state.lock().expect("state lock poisoned");
            Self::remove_tool_run_info_with_inputs(&mut state, run_id)?
        };

        let data = EventData::builder().error(error).input(inputs).build();

        let event = StandardStreamEvent::builder()
            .event("on_tool_error")
            .run_id(run_id.to_string())
            .name(&run_info.name)
            .tags(run_info.tags.clone())
            .metadata(run_info.metadata.clone())
            .parent_ids(parent_ids)
            .data(data)
            .build();

        self.send(StreamEvent::Standard(event), "tool");
        Ok(())
    }

    fn handle_tool_end(&self, output: Value, run_id: Uuid) -> Result<(), String> {
        let (run_info, inputs, parent_ids) = {
            let mut state = self.state.lock().expect("state lock poisoned");
            Self::remove_tool_run_info_with_inputs(&mut state, run_id)?
        };

        let data = EventData::builder().output(output).input(inputs).build();

        let event = StandardStreamEvent::builder()
            .event("on_tool_end")
            .run_id(run_id.to_string())
            .name(&run_info.name)
            .tags(run_info.tags.clone())
            .metadata(run_info.metadata.clone())
            .parent_ids(parent_ids)
            .data(data)
            .build();

        self.send(StreamEvent::Standard(event), "tool");
        Ok(())
    }

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

        let event = StandardStreamEvent::builder()
            .event("on_retriever_start")
            .run_id(run_id.to_string())
            .name(&name_)
            .tags(tags.unwrap_or_default())
            .metadata(metadata.unwrap_or_default())
            .parent_ids(parent_ids)
            .data(
                EventData::builder()
                    .input(serde_json::json!({"query": query}))
                    .build(),
            )
            .build();

        self.send(StreamEvent::Standard(event), run_type);
    }

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

        let mut data = EventData::builder().output(documents).build();
        if let Some(inputs) = run_info.inputs.clone() {
            data.input = Some(inputs);
        }

        let event = StandardStreamEvent::builder()
            .event("on_retriever_end")
            .run_id(run_id.to_string())
            .name(&run_info.name)
            .tags(run_info.tags.clone())
            .metadata(run_info.metadata.clone())
            .parent_ids(parent_ids)
            .data(data)
            .build();

        self.send(StreamEvent::Standard(event), &run_info.run_type);
        Ok(())
    }
}

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
        let _name = name;
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

impl StreamingCallbackHandler<crate::error::Result<Value>> for AstreamEventsCallbackHandler {
    fn tap_output_aiter(
        &self,
        run_id: Uuid,
        output: Pin<Box<dyn futures::Stream<Item = crate::error::Result<Value>> + Send>>,
    ) -> Pin<Box<dyn futures::Stream<Item = crate::error::Result<Value>> + Send>> {
        {
            let mut state = self.state.lock().expect("state lock poisoned");
            if state.is_tapped.contains_key(&run_id) {
                return output;
            }
            state.is_tapped.insert(run_id, true);
        }

        let send_stream = self.send_stream.clone();
        let root_event_filter = self.root_event_filter.clone();

        let state_mutex = &self.state;
        let run_info_snapshot = {
            let state = state_mutex.lock().expect("state lock poisoned");
            state.run_map.get(&run_id).cloned()
        };

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
                let event = StandardStreamEvent::builder()
                    .event(&event_name)
                    .run_id(run_id.to_string())
                    .name(&run_info.name)
                    .tags(run_info.tags.clone())
                    .metadata(run_info.metadata.clone())
                    .parent_ids(parent_ids.clone())
                    .data(EventData::builder().chunk(value.clone()).build())
                    .build();

                let (name, tags) = (run_info.name.as_str(), run_info.tags.as_slice());
                if root_event_filter.include_event(name, tags, &run_type)
                    && let Err(e) = send_stream.send(StreamEvent::Standard(event))
                {
                    tracing::warn!("Failed to send stream event: {}", e);
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
        {
            let mut state = self.state.lock().expect("state lock poisoned");
            if state.is_tapped.contains_key(&run_id) {
                return output;
            }
            state.is_tapped.insert(run_id, true);
        }

        output
    }
}

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

    let run_id = config.run_id.unwrap_or_else(|| uuid7(None));
    config.run_id = Some(run_id);

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

    let receive_stream = event_streamer
        .take_receive_stream()
        .expect("receive stream should be available");

    let send_stream = event_streamer.get_send_stream();

    Box::pin(async_stream::stream! {
        let mut astream = std::pin::pin!(runnable.astream(input, Some(config)));
        while let Some(_chunk) = astream.next().await {
        }
        if let Err(e) = send_stream.close() { tracing::warn!("Failed to close stream: {e}"); }

        let mut first_event_sent = false;
        let mut first_event_run_id: Option<String> = None;

        let mut event_stream = std::pin::pin!(receive_stream.into_stream());
        while let Some(mut event) = event_stream.next().await {
            if !first_event_sent {
                first_event_sent = true;
                first_event_run_id = match &event {
                    StreamEvent::Standard(e) => Some(e.base.run_id.clone()),
                    StreamEvent::Custom(e) => Some(e.base.run_id.clone()),
                };
                yield event;
                continue;
            }

            if let StreamEvent::Standard(ref mut e) = event
                && Some(&e.base.run_id) == first_event_run_id.as_ref()
                    && e.base.event.ends_with("_end")
                {
                    e.data.input = None;
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

        assert!(handler.take_receive_stream().is_some());

        assert!(handler.take_receive_stream().is_none());
    }

    #[test]
    fn test_handler_implements_base_callback_handler() {
        let handler = AstreamEventsCallbackHandler::new(None, None, None, None, None, None);
        let _handler_ref: &dyn BaseCallbackHandler = &handler;
        assert_eq!(handler.name(), "AstreamEventsCallbackHandler");
        assert!(handler.run_inline());
    }
}
