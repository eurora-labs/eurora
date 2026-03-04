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
        token: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        chunk: Option<&serde_json::Value>,
    ) {
        let _ = (token, run_id, parent_run_id, chunk);
    }

    fn on_llm_end(&self, response: &ChatResult, run_id: Uuid, parent_run_id: Option<Uuid>) {
        let _ = (response, run_id, parent_run_id);
    }

    fn on_llm_error(
        &self,
        error: &dyn std::error::Error,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
    ) {
        let _ = (error, run_id, parent_run_id);
    }

    // -- Chain events --

    fn on_chain_end(
        &self,
        outputs: &HashMap<String, serde_json::Value>,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
    ) {
        let _ = (outputs, run_id, parent_run_id);
    }

    fn on_chain_error(
        &self,
        error: &dyn std::error::Error,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
    ) {
        let _ = (error, run_id, parent_run_id);
    }

    fn on_agent_action(
        &self,
        action: &serde_json::Value,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        color: Option<&str>,
    ) {
        let _ = (action, run_id, parent_run_id, color);
    }

    fn on_agent_finish(
        &self,
        finish: &serde_json::Value,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        color: Option<&str>,
    ) {
        let _ = (finish, run_id, parent_run_id, color);
    }

    // -- Tool events --

    fn on_tool_end(
        &self,
        output: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        color: Option<&str>,
        observation_prefix: Option<&str>,
        llm_prefix: Option<&str>,
    ) {
        let _ = (
            output,
            run_id,
            parent_run_id,
            color,
            observation_prefix,
            llm_prefix,
        );
    }

    fn on_tool_error(
        &self,
        error: &dyn std::error::Error,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
    ) {
        let _ = (error, run_id, parent_run_id);
    }

    // -- Retriever events --

    fn on_retriever_error(
        &self,
        error: &dyn std::error::Error,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
    ) {
        let _ = (error, run_id, parent_run_id);
    }

    fn on_retriever_end(
        &self,
        documents: &[serde_json::Value],
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
    ) {
        let _ = (documents, run_id, parent_run_id);
    }

    // -- Lifecycle start events --

    #[allow(clippy::too_many_arguments)]
    fn on_llm_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        prompts: &[String],
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
        let _ = (serialized, prompts, run_id, parent_run_id, tags, metadata);
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
        serialized: &HashMap<String, serde_json::Value>,
        query: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
        name: Option<&str>,
    ) {
        let _ = (
            serialized,
            query,
            run_id,
            parent_run_id,
            tags,
            metadata,
            name,
        );
    }

    #[allow(clippy::too_many_arguments)]
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
        let _ = (
            serialized,
            inputs,
            run_id,
            parent_run_id,
            tags,
            metadata,
            name,
        );
    }

    #[allow(clippy::too_many_arguments)]
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
        let _ = (
            serialized,
            input_str,
            run_id,
            parent_run_id,
            tags,
            metadata,
            inputs,
        );
    }

    // -- Run events --

    fn on_text(
        &self,
        text: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        color: Option<&str>,
        end: &str,
    ) {
        let _ = (text, run_id, parent_run_id, color, end);
    }

    fn on_retry(&self, retry_state: &dyn Any, run_id: Uuid, parent_run_id: Option<Uuid>) {
        let _ = (retry_state, run_id, parent_run_id);
    }

    fn on_custom_event(
        &self,
        name: &str,
        data: &dyn Any,
        run_id: Uuid,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
        let _ = (name, data, run_id, tags, metadata);
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
