use agent_chain_core::callbacks::BaseCallbackHandler;
use agent_chain_core::outputs::ChatResult;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug)]
struct LlmEndHandler;

impl BaseCallbackHandler for LlmEndHandler {
    fn name(&self) -> &str {
        "LlmEndHandler"
    }

    fn on_llm_end(&self, _response: &ChatResult, _run_id: Uuid, _parent_run_id: Option<Uuid>) {}
}

#[derive(Debug)]
struct ChatModelStartHandler;

impl BaseCallbackHandler for ChatModelStartHandler {
    fn name(&self) -> &str {
        "ChatModelStartHandler"
    }

    fn on_chat_model_start(
        &self,
        _serialized: &HashMap<String, serde_json::Value>,
        _messages: &[Vec<agent_chain_core::messages::BaseMessage>],
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _tags: Option<&[String]>,
        _metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
    }
}

#[derive(Debug)]
struct AsyncLlmEndHandler;

impl BaseCallbackHandler for AsyncLlmEndHandler {
    fn name(&self) -> &str {
        "AsyncLlmEndHandler"
    }
}

#[derive(Debug)]
struct AsyncChatModelStartHandler;

impl BaseCallbackHandler for AsyncChatModelStartHandler {
    fn name(&self) -> &str {
        "AsyncChatModelStartHandler"
    }
}

#[test]
fn test_on_llm_end_is_defined() {
    let handler = LlmEndHandler;
    assert!(!handler.ignore_llm());
}

#[test]
fn test_on_chat_model_start_is_defined() {
    let handler = ChatModelStartHandler;
    assert!(!handler.ignore_chat_model());
}

#[test]
fn test_async_on_llm_end_is_defined() {
    let handler = AsyncLlmEndHandler;
    assert!(!handler.ignore_llm());
}

#[test]
fn test_async_on_chat_model_start_is_defined() {
    let handler = AsyncChatModelStartHandler;
    assert!(!handler.ignore_chat_model());
}
