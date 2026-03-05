use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use uuid::Uuid;

use crate::messages::BaseMessage;
use crate::outputs::ChatResult;

pub trait BaseCallbackHandler: Send + Sync + Debug {
    // -- LLM events --

    fn on_llm_new_token(
        &self,
        _token: &str,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _chunk: Option<&serde_json::Value>,
    ) {
    }

    fn on_llm_end(&self, _response: &ChatResult, _run_id: Uuid, _parent_run_id: Option<Uuid>) {}

    fn on_llm_error(
        &self,
        _error: &dyn std::error::Error,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
    }

    // -- Chain events --

    fn on_chain_end(
        &self,
        _outputs: &HashMap<String, serde_json::Value>,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
    }

    fn on_chain_error(
        &self,
        _error: &dyn std::error::Error,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
    }

    fn on_agent_action(
        &self,
        _action: &serde_json::Value,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _color: Option<&str>,
    ) {
    }

    fn on_agent_finish(
        &self,
        _finish: &serde_json::Value,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _color: Option<&str>,
    ) {
    }

    // -- Tool events --

    fn on_tool_end(
        &self,
        _output: &str,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _color: Option<&str>,
        _observation_prefix: Option<&str>,
        _llm_prefix: Option<&str>,
    ) {
    }

    fn on_tool_error(
        &self,
        _error: &dyn std::error::Error,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
    }

    // -- Retriever events --

    fn on_retriever_error(
        &self,
        _error: &dyn std::error::Error,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
    }

    fn on_retriever_end(
        &self,
        _documents: &[serde_json::Value],
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
    }

    // -- Lifecycle start events --

    #[allow(clippy::too_many_arguments)]
    fn on_llm_start(
        &self,
        _serialized: &HashMap<String, serde_json::Value>,
        _prompts: &[String],
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _tags: Option<&[String]>,
        _metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
    }

    #[allow(clippy::too_many_arguments)]
    fn on_chat_model_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        messages: &[Vec<BaseMessage>],
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
        _name: Option<&str>,
    ) {
        use crate::messages::utils::get_buffer_string;
        let message_strings: Vec<String> = messages
            .iter()
            .map(|m| get_buffer_string(m, "Human", "AI"))
            .collect();
        self.on_llm_start(
            serialized,
            &message_strings,
            run_id,
            parent_run_id,
            tags,
            metadata,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn on_retriever_start(
        &self,
        _serialized: &HashMap<String, serde_json::Value>,
        _query: &str,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _tags: Option<&[String]>,
        _metadata: Option<&HashMap<String, serde_json::Value>>,
        _name: Option<&str>,
    ) {
    }

    #[allow(clippy::too_many_arguments)]
    fn on_chain_start(
        &self,
        _serialized: &HashMap<String, serde_json::Value>,
        _inputs: &HashMap<String, serde_json::Value>,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _tags: Option<&[String]>,
        _metadata: Option<&HashMap<String, serde_json::Value>>,
        _name: Option<&str>,
    ) {
    }

    #[allow(clippy::too_many_arguments)]
    fn on_tool_start(
        &self,
        _serialized: &HashMap<String, serde_json::Value>,
        _input_str: &str,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _tags: Option<&[String]>,
        _metadata: Option<&HashMap<String, serde_json::Value>>,
        _inputs: Option<&HashMap<String, serde_json::Value>>,
    ) {
    }

    // -- Run events --

    fn on_text(
        &self,
        _text: &str,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _color: Option<&str>,
        _end: &str,
    ) {
    }

    fn on_retry(&self, _retry_state: &dyn Any, _run_id: Uuid, _parent_run_id: Option<Uuid>) {}

    fn on_custom_event(
        &self,
        _name: &str,
        _data: &dyn Any,
        _run_id: Uuid,
        _tags: Option<&[String]>,
        _metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
    }

    // -- Configuration --

    fn raise_error(&self) -> bool {
        false
    }

    fn run_inline(&self) -> bool {
        false
    }

    fn ignore_llm(&self) -> bool {
        false
    }

    fn ignore_retry(&self) -> bool {
        false
    }

    fn ignore_chain(&self) -> bool {
        false
    }

    fn ignore_agent(&self) -> bool {
        false
    }

    fn ignore_tool(&self) -> bool {
        false
    }

    fn ignore_retriever(&self) -> bool {
        false
    }

    fn ignore_chat_model(&self) -> bool {
        false
    }

    fn ignore_custom_event(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        "BaseCallbackHandler"
    }
}

pub fn resolve_chain_name<'a>(
    serialized: &'a HashMap<String, serde_json::Value>,
    name: Option<&'a str>,
) -> &'a str {
    name.or_else(|| {
        serialized.get("name").and_then(|v| v.as_str()).or_else(|| {
            serialized.get("id").and_then(|v| {
                v.as_array()
                    .and_then(|arr| arr.last())
                    .and_then(|v| v.as_str())
            })
        })
    })
    .unwrap_or("<unknown>")
}

pub type BoxedCallbackHandler = Box<dyn BaseCallbackHandler>;

pub type ArcCallbackHandler = Arc<dyn BaseCallbackHandler>;
