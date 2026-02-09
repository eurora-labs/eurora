//! Unit tests for BaseCallbackHandler and AsyncCallbackHandler.
//!
//! Ported from `langchain/libs/core/tests/unit_tests/callbacks/test_base.py`

use agent_chain_core::callbacks::base::{
    AsyncCallbackHandler, BaseCallbackHandler, CallbackManagerMixin, ChainManagerMixin,
    LLMManagerMixin, RetrieverManagerMixin, RunManagerMixin, ToolManagerMixin,
};
use agent_chain_core::outputs::ChatResult;
use async_trait::async_trait;
use std::collections::HashMap;
use uuid::Uuid;

// -- Handler that overrides on_llm_end --

#[derive(Debug)]
struct LlmEndHandler;

impl RetrieverManagerMixin for LlmEndHandler {}
impl ChainManagerMixin for LlmEndHandler {}
impl ToolManagerMixin for LlmEndHandler {}
impl CallbackManagerMixin for LlmEndHandler {}
impl RunManagerMixin for LlmEndHandler {}

impl LLMManagerMixin for LlmEndHandler {
    fn on_llm_end(&mut self, _response: &ChatResult, _run_id: Uuid, _parent_run_id: Option<Uuid>) {}
}

impl BaseCallbackHandler for LlmEndHandler {
    fn name(&self) -> &str {
        "LlmEndHandler"
    }
}

// -- Handler that overrides on_chat_model_start --

#[derive(Debug)]
struct ChatModelStartHandler;

impl RetrieverManagerMixin for ChatModelStartHandler {}
impl LLMManagerMixin for ChatModelStartHandler {}
impl ChainManagerMixin for ChatModelStartHandler {}
impl ToolManagerMixin for ChatModelStartHandler {}
impl RunManagerMixin for ChatModelStartHandler {}

impl CallbackManagerMixin for ChatModelStartHandler {
    fn on_chat_model_start(
        &mut self,
        _serialized: &HashMap<String, serde_json::Value>,
        _messages: &[Vec<agent_chain_core::messages::BaseMessage>],
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _tags: Option<&[String]>,
        _metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
    }
}

impl BaseCallbackHandler for ChatModelStartHandler {
    fn name(&self) -> &str {
        "ChatModelStartHandler"
    }
}

// -- Async handler that overrides on_llm_end_async --

#[derive(Debug)]
struct AsyncLlmEndHandler;

impl RetrieverManagerMixin for AsyncLlmEndHandler {}
impl LLMManagerMixin for AsyncLlmEndHandler {}
impl ChainManagerMixin for AsyncLlmEndHandler {}
impl ToolManagerMixin for AsyncLlmEndHandler {}
impl CallbackManagerMixin for AsyncLlmEndHandler {}
impl RunManagerMixin for AsyncLlmEndHandler {}

impl BaseCallbackHandler for AsyncLlmEndHandler {
    fn name(&self) -> &str {
        "AsyncLlmEndHandler"
    }
}

#[async_trait]
impl AsyncCallbackHandler for AsyncLlmEndHandler {
    async fn on_llm_end_async(
        &mut self,
        _response: &ChatResult,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _tags: Option<&[String]>,
    ) {
    }
}

// -- Async handler that overrides on_chat_model_start_async --

#[derive(Debug)]
struct AsyncChatModelStartHandler;

impl RetrieverManagerMixin for AsyncChatModelStartHandler {}
impl LLMManagerMixin for AsyncChatModelStartHandler {}
impl ChainManagerMixin for AsyncChatModelStartHandler {}
impl ToolManagerMixin for AsyncChatModelStartHandler {}
impl CallbackManagerMixin for AsyncChatModelStartHandler {}
impl RunManagerMixin for AsyncChatModelStartHandler {}

impl BaseCallbackHandler for AsyncChatModelStartHandler {
    fn name(&self) -> &str {
        "AsyncChatModelStartHandler"
    }
}

#[async_trait]
impl AsyncCallbackHandler for AsyncChatModelStartHandler {
    async fn on_chat_model_start_async(
        &mut self,
        _serialized: &HashMap<String, serde_json::Value>,
        _messages: &[Vec<agent_chain_core::messages::BaseMessage>],
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _tags: Option<&[String]>,
        _metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
    }
}

// -- Tests --

/// Ported from `test_on_llm_end_is_defined`.
///
/// When a handler overrides `on_llm_end`, `ignore_llm` should still be `false`.
#[test]
fn test_on_llm_end_is_defined() {
    let handler = LlmEndHandler;
    assert!(!handler.ignore_llm());
}

/// Ported from `test_on_chat_model_start_is_defined`.
///
/// When a handler overrides `on_chat_model_start`, `ignore_chat_model` should
/// still be `false`.
#[test]
fn test_on_chat_model_start_is_defined() {
    let handler = ChatModelStartHandler;
    assert!(!handler.ignore_chat_model());
}

/// Ported from `test_async_on_llm_end_is_defined`.
///
/// When an async handler overrides `on_llm_end_async`, `ignore_llm` should
/// still be `false`.
#[test]
fn test_async_on_llm_end_is_defined() {
    let handler = AsyncLlmEndHandler;
    assert!(!handler.ignore_llm());
}

/// Ported from `test_async_on_chat_model_start_is_defined`.
///
/// When an async handler overrides `on_chat_model_start_async`,
/// `ignore_chat_model` should still be `false`.
#[test]
fn test_async_on_chat_model_start_is_defined() {
    let handler = AsyncChatModelStartHandler;
    assert!(!handler.ignore_chat_model());
}
