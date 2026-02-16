//! Fake chat models for testing purposes.
//!
//! This module provides fake chat model implementations that can be used
//! for testing without making actual API calls.
//! Mirrors `langchain_core.language_models.fake_chat_models`.

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

use super::base::{BaseLanguageModel, LanguageModelConfig, LanguageModelInput};
use super::chat_models::{BaseChatModel, ChatGenerationStream, ChatModelConfig};
use crate::caches::BaseCache;
use crate::callbacks::{CallbackManagerForLLMRun, Callbacks};
use crate::error::Result;
use crate::messages::{AIMessage, AIMessageChunk, BaseMessage, ChunkPosition};
use crate::outputs::{ChatGeneration, ChatGenerationChunk, ChatResult, GenerationType, LLMResult};

/// Fake chat model for testing purposes.
///
/// Cycles through BaseMessage responses in order.
#[derive(Debug)]
pub struct FakeMessagesListChatModel {
    /// List of responses to cycle through.
    responses: Vec<BaseMessage>,
    /// Sleep time in seconds between responses.
    sleep: Option<Duration>,
    /// Internally incremented after every model invocation.
    index: AtomicUsize,
    /// Chat model configuration.
    config: ChatModelConfig,
}

impl Clone for FakeMessagesListChatModel {
    fn clone(&self) -> Self {
        Self {
            responses: self.responses.clone(),
            sleep: self.sleep,
            index: AtomicUsize::new(self.index.load(Ordering::SeqCst)),
            config: self.config.clone(),
        }
    }
}

impl FakeMessagesListChatModel {
    /// Create a new FakeMessagesListChatModel with the given responses.
    pub fn new(responses: Vec<BaseMessage>) -> Self {
        Self {
            responses,
            sleep: None,
            index: AtomicUsize::new(0),
            config: ChatModelConfig::default(),
        }
    }

    /// Set the sleep duration.
    pub fn with_sleep(mut self, duration: Duration) -> Self {
        self.sleep = Some(duration);
        self
    }

    /// Set the configuration.
    pub fn with_config(mut self, config: ChatModelConfig) -> Self {
        self.config = config;
        self
    }

    /// Get the current index.
    pub fn current_index(&self) -> usize {
        self.index.load(Ordering::SeqCst)
    }

    /// Reset the index.
    pub fn reset(&self) {
        self.index.store(0, Ordering::SeqCst);
    }
}

#[async_trait]
impl BaseLanguageModel for FakeMessagesListChatModel {
    fn llm_type(&self) -> &str {
        "fake-messages-list-chat-model"
    }

    fn model_name(&self) -> &str {
        "fake-messages-list"
    }

    fn config(&self) -> &LanguageModelConfig {
        &self.config.base
    }

    fn cache(&self) -> Option<&dyn BaseCache> {
        None
    }

    fn callbacks(&self) -> Option<&Callbacks> {
        None
    }

    async fn generate_prompt(
        &self,
        prompts: Vec<LanguageModelInput>,
        stop: Option<Vec<String>>,
        _callbacks: Option<Callbacks>,
    ) -> Result<LLMResult> {
        let mut generations = Vec::new();

        for prompt in prompts {
            let messages = prompt.to_messages();
            let result = self._generate(messages, stop.clone(), None).await?;
            generations.push(
                result
                    .generations
                    .into_iter()
                    .map(GenerationType::ChatGeneration)
                    .collect(),
            );
        }

        Ok(LLMResult::new(generations))
    }
}

#[async_trait]
impl BaseChatModel for FakeMessagesListChatModel {
    fn chat_config(&self) -> &ChatModelConfig {
        &self.config
    }

    async fn _generate(
        &self,
        _messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        if let Some(duration) = self.sleep {
            tokio::time::sleep(duration).await;
        }

        let i = self.index.load(Ordering::SeqCst);
        let response = self
            .responses
            .get(i)
            .cloned()
            .unwrap_or_else(|| BaseMessage::AI(AIMessage::builder().content("").build()));

        // Update index
        let next_i = if i < self.responses.len() - 1 {
            i + 1
        } else {
            0
        };
        self.index.store(next_i, Ordering::SeqCst);

        // Create ChatGeneration with the original message type preserved
        let generation = ChatGeneration::new(response);
        Ok(ChatResult::new(vec![generation]))
    }
}

/// Fake error for testing purposes.
#[derive(Debug, Clone, thiserror::Error)]
#[error("FakeListChatModelError")]
pub struct FakeListChatModelError;

/// Fake chat model for testing purposes.
///
/// Cycles through string responses in order.
#[derive(Debug)]
pub struct FakeListChatModel {
    /// List of string responses to cycle through.
    responses: Vec<String>,
    /// Sleep time in seconds between responses.
    sleep: Option<Duration>,
    /// Internally incremented after every model invocation.
    index: AtomicUsize,
    /// If set, raise an error on the specified chunk number during streaming.
    error_on_chunk_number: Option<usize>,
    /// Chat model configuration.
    config: ChatModelConfig,
}

impl Clone for FakeListChatModel {
    fn clone(&self) -> Self {
        Self {
            responses: self.responses.clone(),
            sleep: self.sleep,
            index: AtomicUsize::new(self.index.load(Ordering::SeqCst)),
            error_on_chunk_number: self.error_on_chunk_number,
            config: self.config.clone(),
        }
    }
}

impl FakeListChatModel {
    /// Create a new FakeListChatModel with the given responses.
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses,
            sleep: None,
            index: AtomicUsize::new(0),
            error_on_chunk_number: None,
            config: ChatModelConfig::default(),
        }
    }

    /// Set the sleep duration.
    pub fn with_sleep(mut self, duration: Duration) -> Self {
        self.sleep = Some(duration);
        self
    }

    /// Set the configuration.
    pub fn with_config(mut self, config: ChatModelConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the chunk number to error on during streaming.
    pub fn with_error_on_chunk(mut self, chunk_number: usize) -> Self {
        self.error_on_chunk_number = Some(chunk_number);
        self
    }

    /// Set a local cache instance.
    pub fn with_cache_instance(
        mut self,
        cache: std::sync::Arc<dyn crate::caches::BaseCache>,
    ) -> Self {
        self.config.cache_instance = Some(cache);
        self
    }

    /// Disable caching.
    pub fn with_cache_disabled(mut self) -> Self {
        self.config.base.cache = Some(false);
        self
    }

    /// Enable caching (use global cache).
    pub fn with_cache_enabled(mut self) -> Self {
        self.config.base.cache = Some(true);
        self
    }

    /// Get the current index.
    pub fn current_index(&self) -> usize {
        self.index.load(Ordering::SeqCst)
    }

    /// Reset the index.
    pub fn reset(&self) {
        self.index.store(0, Ordering::SeqCst);
    }

    /// Get the next response and update index.
    fn get_next_response(&self) -> String {
        let i = self.index.load(Ordering::SeqCst);
        let response = self.responses.get(i).cloned().unwrap_or_default();

        let next_i = if i < self.responses.len() - 1 {
            i + 1
        } else {
            0
        };
        self.index.store(next_i, Ordering::SeqCst);

        response
    }
}

#[async_trait]
impl BaseLanguageModel for FakeListChatModel {
    fn llm_type(&self) -> &str {
        "fake-list-chat-model"
    }

    fn model_name(&self) -> &str {
        "fake-list-chat"
    }

    fn config(&self) -> &LanguageModelConfig {
        &self.config.base
    }

    fn cache(&self) -> Option<&dyn BaseCache> {
        None
    }

    fn callbacks(&self) -> Option<&Callbacks> {
        None
    }

    async fn generate_prompt(
        &self,
        prompts: Vec<LanguageModelInput>,
        stop: Option<Vec<String>>,
        _callbacks: Option<Callbacks>,
    ) -> Result<LLMResult> {
        let mut generations = Vec::new();

        for prompt in prompts {
            let messages = prompt.to_messages();
            let result = self._generate(messages, stop.clone(), None).await?;
            generations.push(
                result
                    .generations
                    .into_iter()
                    .map(GenerationType::ChatGeneration)
                    .collect(),
            );
        }

        Ok(LLMResult::new(generations))
    }

    fn identifying_params(&self) -> HashMap<String, Value> {
        let mut params = HashMap::new();
        params.insert(
            "responses".to_string(),
            serde_json::to_value(&self.responses).unwrap_or_default(),
        );
        params
    }
}

#[async_trait]
impl BaseChatModel for FakeListChatModel {
    fn chat_config(&self) -> &ChatModelConfig {
        &self.config
    }

    async fn _generate(
        &self,
        _messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        if let Some(duration) = self.sleep {
            tokio::time::sleep(duration).await;
        }

        let response = self.get_next_response();
        let message = AIMessage::builder().content(&response).build();
        let generation = ChatGeneration::new(message.into());
        Ok(ChatResult::new(vec![generation]))
    }

    fn has_stream_impl(&self) -> bool {
        true
    }

    fn _stream(
        &self,
        _messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatGenerationStream> {
        let response = self.get_next_response();
        let sleep = self.sleep;
        let error_on_chunk = self.error_on_chunk_number;
        let response_len = response.len();

        let stream = async_stream::stream! {
            for (i_c, c) in response.chars().enumerate() {
                // Sleep first if configured
                if let Some(duration) = sleep {
                    tokio::time::sleep(duration).await;
                }

                // Check if we should error on this chunk
                if let Some(error_chunk) = error_on_chunk
                    && i_c == error_chunk
                {
                    yield Err(crate::error::Error::Other(
                        "FakeListChatModelError".to_string()
                    ));
                    return;
                }

                // Create chunk with proper chunk_position
                let chunk_position = if i_c == response_len - 1 {
                    Some(ChunkPosition::Last)
                } else {
                    None
                };

                let mut ai_chunk = AIMessageChunk::builder().content(c.to_string()).build();
                ai_chunk.set_chunk_position(chunk_position);

                let chunk = ChatGenerationChunk::new(ai_chunk.to_message().into());
                yield Ok(chunk);
            }
        };

        Ok(Box::pin(stream))
    }
}

/// Fake Chat Model wrapper for testing purposes.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct FakeChatModel {
    /// Chat model configuration.
    config: ChatModelConfig,
}

#[allow(dead_code)]
impl FakeChatModel {
    /// Create a new FakeChatModel.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the configuration.
    pub fn with_config(mut self, config: ChatModelConfig) -> Self {
        self.config = config;
        self
    }
}

#[async_trait]
impl BaseLanguageModel for FakeChatModel {
    fn llm_type(&self) -> &str {
        "fake-chat-model"
    }

    fn model_name(&self) -> &str {
        "fake-chat"
    }

    fn config(&self) -> &LanguageModelConfig {
        &self.config.base
    }

    fn cache(&self) -> Option<&dyn BaseCache> {
        None
    }

    fn callbacks(&self) -> Option<&Callbacks> {
        None
    }

    async fn generate_prompt(
        &self,
        prompts: Vec<LanguageModelInput>,
        stop: Option<Vec<String>>,
        _callbacks: Option<Callbacks>,
    ) -> Result<LLMResult> {
        let mut generations = Vec::new();

        for prompt in prompts {
            let messages = prompt.to_messages();
            let result = self._generate(messages, stop.clone(), None).await?;
            generations.push(
                result
                    .generations
                    .into_iter()
                    .map(GenerationType::ChatGeneration)
                    .collect(),
            );
        }

        Ok(LLMResult::new(generations))
    }

    fn identifying_params(&self) -> HashMap<String, Value> {
        let mut params = HashMap::new();
        params.insert("key".to_string(), Value::String("fake".to_string()));
        params
    }
}

#[async_trait]
impl BaseChatModel for FakeChatModel {
    fn chat_config(&self) -> &ChatModelConfig {
        &self.config
    }

    async fn _generate(
        &self,
        _messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        let message = AIMessage::builder().content("fake response").build();
        let generation = ChatGeneration::new(message.into());
        Ok(ChatResult::new(vec![generation]))
    }
}

/// Generic fake chat model that can be used to test the chat model interface.
///
/// * Chat model should be usable in both sync and async tests
/// * Invokes `on_llm_new_token` to allow for testing of callback related code for new tokens.
/// * Includes logic to break messages into message chunks to facilitate testing of streaming.
pub struct GenericFakeChatModel {
    /// Iterator over messages to return.
    ///
    /// If you want to pass a list, you can use `into_iter()` to convert it to an iterator.
    messages: std::sync::Mutex<Box<dyn Iterator<Item = AIMessage> + Send>>,
    /// Chat model configuration.
    config: ChatModelConfig,
}

impl fmt::Debug for GenericFakeChatModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GenericFakeChatModel")
            .field("messages", &"<iterator>")
            .field("config", &self.config)
            .finish()
    }
}

impl GenericFakeChatModel {
    /// Create a new GenericFakeChatModel with an iterator of messages.
    pub fn new<I>(messages: I) -> Self
    where
        I: Iterator<Item = AIMessage> + Send + 'static,
    {
        Self {
            messages: std::sync::Mutex::new(Box::new(messages)),
            config: ChatModelConfig::default(),
        }
    }

    /// Create from a vector of messages.
    pub fn from_vec(messages: Vec<AIMessage>) -> Self {
        Self::new(messages.into_iter())
    }

    /// Create from a vector of strings (converted to AIMessages).
    pub fn from_strings(messages: Vec<String>) -> Self {
        Self::new(
            messages
                .into_iter()
                .map(|s| AIMessage::builder().content(&s).build()),
        )
    }

    /// Set the configuration.
    pub fn with_config(mut self, config: ChatModelConfig) -> Self {
        self.config = config;
        self
    }

    /// Set a local cache instance.
    pub fn with_cache_instance(
        mut self,
        cache: std::sync::Arc<dyn crate::caches::BaseCache>,
    ) -> Self {
        self.config.cache_instance = Some(cache);
        self
    }

    /// Disable caching.
    pub fn with_cache_disabled(mut self) -> Self {
        self.config.base.cache = Some(false);
        self
    }

    /// Enable caching (use global cache).
    pub fn with_cache_enabled(mut self) -> Self {
        self.config.base.cache = Some(true);
        self
    }
}

#[async_trait]
impl BaseLanguageModel for GenericFakeChatModel {
    fn llm_type(&self) -> &str {
        "generic-fake-chat-model"
    }

    fn model_name(&self) -> &str {
        "generic-fake"
    }

    fn config(&self) -> &LanguageModelConfig {
        &self.config.base
    }

    fn cache(&self) -> Option<&dyn BaseCache> {
        None
    }

    fn callbacks(&self) -> Option<&Callbacks> {
        None
    }

    async fn generate_prompt(
        &self,
        prompts: Vec<LanguageModelInput>,
        stop: Option<Vec<String>>,
        _callbacks: Option<Callbacks>,
    ) -> Result<LLMResult> {
        let mut generations = Vec::new();

        for prompt in prompts {
            let messages = prompt.to_messages();
            let result = self._generate(messages, stop.clone(), None).await?;
            generations.push(
                result
                    .generations
                    .into_iter()
                    .map(GenerationType::ChatGeneration)
                    .collect(),
            );
        }

        Ok(LLMResult::new(generations))
    }
}

#[async_trait]
impl BaseChatModel for GenericFakeChatModel {
    fn chat_config(&self) -> &ChatModelConfig {
        &self.config
    }

    async fn _generate(
        &self,
        _messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        let message = {
            let mut guard = self
                .messages
                .lock()
                .map_err(|e| crate::error::Error::Other(format!("Lock poisoned: {}", e)))?;
            guard
                .next()
                .unwrap_or_else(|| AIMessage::builder().content("").build())
        };

        let generation = ChatGeneration::new(message.into());
        Ok(ChatResult::new(vec![generation]))
    }

    fn has_stream_impl(&self) -> bool {
        true
    }

    fn _stream(
        &self,
        _messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatGenerationStream> {
        let message = {
            let mut guard = self
                .messages
                .lock()
                .map_err(|e| crate::error::Error::Other(format!("Lock poisoned: {}", e)))?;
            guard
                .next()
                .unwrap_or_else(|| AIMessage::builder().content("").build())
        };

        let content = message.text();
        let message_id = message.id;
        let additional_kwargs = message.additional_kwargs.clone();

        // Extract callback data from run_manager so we can call on_llm_new_token
        // inside the stream. We clone the Arc handlers (cheap) so the stream
        // can own them without borrowing run_manager.
        let callback_handlers: Vec<
            std::sync::Arc<dyn crate::callbacks::base::BaseCallbackHandler>,
        > = run_manager
            .map(|rm| rm.handlers().to_vec())
            .unwrap_or_default();
        let callback_run_id = run_manager.map(|rm| rm.run_id());
        let callback_parent_run_id = run_manager.and_then(|rm| rm.parent_run_id());

        let stream = async_stream::stream! {
            if !content.is_empty() {
                // Use a regular expression to split on whitespace with a capture group
                // so that we can preserve the whitespace in the output.
                let re = Regex::new(r"(\s)")
                    .map_err(|e| crate::error::Error::Other(format!("Regex error: {}", e)))?;

                // Split content preserving whitespace using regex
                let all_parts: Vec<String> = {
                    let mut parts = Vec::new();
                    let mut last = 0;
                    for m in re.find_iter(&content) {
                        if m.start() > last {
                            parts.push(content[last..m.start()].to_string());
                        }
                        parts.push(m.as_str().to_string());
                        last = m.end();
                    }
                    if last < content.len() {
                        parts.push(content[last..].to_string());
                    }
                    parts
                };

                let num_chunks = all_parts.len();

                for (idx, token) in all_parts.into_iter().enumerate() {
                    let mut chunk_msg = AIMessageChunk::builder().content(&token).build();

                    // Set message ID if available
                    if let Some(ref id) = message_id {
                        chunk_msg = AIMessageChunk::builder().id(id.clone()).content(&token).build();
                    }

                    // Set chunk_position on the last chunk if no additional_kwargs
                    if idx == num_chunks - 1 && additional_kwargs.is_empty() {
                        chunk_msg.set_chunk_position(Some(ChunkPosition::Last));
                    }

                    let chunk = ChatGenerationChunk::new(chunk_msg.to_message().into());

                    // Invoke on_llm_new_token on each handler
                    if let Some(run_id) = callback_run_id {
                        for handler in &callback_handlers {
                            handler.on_llm_new_token(&token, run_id, callback_parent_run_id, None);
                        }
                    }

                    yield Ok(chunk);
                }
            }

            // Handle additional_kwargs
            if !additional_kwargs.is_empty() {
                for (key, value) in additional_kwargs.iter() {
                    if key == "function_call" {
                        if let Some(obj) = value.as_object() {
                            for (fkey, fvalue) in obj.iter() {
                                if let Some(fvalue_str) = fvalue.as_str() {
                                    // Break function call by `,` preserving the delimiter
                                    // Python: re.split(r"(,)", fvalue) -> ["a", ",", "b"]
                                    let fvalue_parts: Vec<String> = {
                                        let mut parts = Vec::new();
                                        let segments: Vec<&str> = fvalue_str.split(',').collect();
                                        for (i, segment) in segments.iter().enumerate() {
                                            if !segment.is_empty() {
                                                parts.push(segment.to_string());
                                            }
                                            if i < segments.len() - 1 {
                                                parts.push(",".to_string());
                                            }
                                        }
                                        parts
                                    };

                                    for fvalue_chunk in &fvalue_parts {
                                        let mut fc: HashMap<String, Value> = HashMap::new();
                                        fc.insert(fkey.clone(), Value::String(fvalue_chunk.clone()));
                                        let mut ak: HashMap<String, Value> = HashMap::new();
                                        ak.insert("function_call".to_string(), Value::Object(fc.into_iter().collect()));

                                        let chunk_msg = AIMessageChunk::builder()
                                            .maybe_id(message_id.clone())
                                            .content("")
                                            .additional_kwargs(ak)
                                            .build();

                                        let chunk = ChatGenerationChunk::new(chunk_msg.to_message().into());

                                        if let Some(run_id) = callback_run_id {
                                            for handler in &callback_handlers {
                                                handler.on_llm_new_token("", run_id, callback_parent_run_id, None);
                                            }
                                        }

                                        yield Ok(chunk);
                                    }
                                } else {
                                    let mut fc: HashMap<String, Value> = HashMap::new();
                                    fc.insert(fkey.clone(), fvalue.clone());
                                    let mut ak: HashMap<String, Value> = HashMap::new();
                                    ak.insert("function_call".to_string(), Value::Object(fc.into_iter().collect()));

                                    let chunk_msg = AIMessageChunk::builder()
                                        .maybe_id(message_id.clone())
                                        .content("")
                                        .additional_kwargs(ak)
                                        .build();

                                    let chunk = ChatGenerationChunk::new(chunk_msg.to_message().into());

                                    if let Some(run_id) = callback_run_id {
                                        for handler in &callback_handlers {
                                            handler.on_llm_new_token("", run_id, callback_parent_run_id, None);
                                        }
                                    }

                                    yield Ok(chunk);
                                }
                            }
                        }
                    } else {
                        let mut ak: HashMap<String, Value> = HashMap::new();
                        ak.insert(key.clone(), value.clone());

                        let chunk_msg = AIMessageChunk::builder()
                            .maybe_id(message_id.clone())
                            .content("")
                            .additional_kwargs(ak)
                            .build();

                        let chunk = ChatGenerationChunk::new(chunk_msg.to_message().into());

                        if let Some(run_id) = callback_run_id {
                            for handler in &callback_handlers {
                                handler.on_llm_new_token("", run_id, callback_parent_run_id, None);
                            }
                        }

                        yield Ok(chunk);
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

/// Generic fake chat model that echoes the last message.
///
/// * Chat model should be usable in both sync and async tests
#[derive(Debug, Clone, Default)]
pub struct ParrotFakeChatModel {
    /// Chat model configuration.
    config: ChatModelConfig,
}

impl ParrotFakeChatModel {
    /// Create a new ParrotFakeChatModel.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the configuration.
    pub fn with_config(mut self, config: ChatModelConfig) -> Self {
        self.config = config;
        self
    }
}

#[async_trait]
impl BaseLanguageModel for ParrotFakeChatModel {
    fn llm_type(&self) -> &str {
        "parrot-fake-chat-model"
    }

    fn model_name(&self) -> &str {
        "parrot-fake"
    }

    fn config(&self) -> &LanguageModelConfig {
        &self.config.base
    }

    fn cache(&self) -> Option<&dyn BaseCache> {
        None
    }

    fn callbacks(&self) -> Option<&Callbacks> {
        None
    }

    async fn generate_prompt(
        &self,
        prompts: Vec<LanguageModelInput>,
        stop: Option<Vec<String>>,
        _callbacks: Option<Callbacks>,
    ) -> Result<LLMResult> {
        let mut generations = Vec::new();

        for prompt in prompts {
            let messages = prompt.to_messages();
            let result = self._generate(messages, stop.clone(), None).await?;
            generations.push(
                result
                    .generations
                    .into_iter()
                    .map(GenerationType::ChatGeneration)
                    .collect(),
            );
        }

        Ok(LLMResult::new(generations))
    }
}

#[async_trait]
impl BaseChatModel for ParrotFakeChatModel {
    fn chat_config(&self) -> &ChatModelConfig {
        &self.config
    }

    async fn _generate(
        &self,
        messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        // Return the last message as-is, preserving its type
        let last_message = messages
            .last()
            .cloned()
            .unwrap_or_else(|| BaseMessage::AI(AIMessage::builder().content("").build()));

        let generation = ChatGeneration::new(last_message);
        Ok(ChatResult::new(vec![generation]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::HumanMessage;

    #[tokio::test]
    async fn test_fake_messages_list_chat_model() {
        let llm = FakeMessagesListChatModel::new(vec![
            BaseMessage::AI(AIMessage::builder().content("Response 1").build()),
            BaseMessage::AI(AIMessage::builder().content("Response 2").build()),
        ]);

        let result = llm._generate(vec![], None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "Response 1");

        let result = llm._generate(vec![], None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "Response 2");

        // Cycles back
        let result = llm._generate(vec![], None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "Response 1");
    }

    #[tokio::test]
    async fn test_fake_list_chat_model() {
        let llm = FakeListChatModel::new(vec!["Response 1".to_string(), "Response 2".to_string()]);

        let result = llm._generate(vec![], None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "Response 1");

        let result = llm._generate(vec![], None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "Response 2");
    }

    #[tokio::test]
    async fn test_fake_list_chat_model_stream() {
        use futures::StreamExt;

        let llm = FakeListChatModel::new(vec!["Hello".to_string()]);

        let mut stream = llm._stream(vec![], None, None).unwrap();

        let mut result = String::new();
        while let Some(chunk) = stream.next().await {
            let text = chunk.unwrap().text.clone();
            result.push_str(&text);
        }

        assert_eq!(result, "Hello");
    }

    #[tokio::test]
    async fn test_fake_chat_model() {
        let llm = FakeChatModel::new();

        let result = llm._generate(vec![], None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "fake response");
    }

    #[tokio::test]
    async fn test_generic_fake_chat_model() {
        let llm = GenericFakeChatModel::from_strings(vec![
            "First response".to_string(),
            "Second response".to_string(),
        ]);

        let result = llm._generate(vec![], None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "First response");

        let result = llm._generate(vec![], None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "Second response");
    }

    #[tokio::test]
    async fn test_parrot_fake_chat_model() {
        let llm = ParrotFakeChatModel::new();

        let messages = vec![BaseMessage::Human(
            HumanMessage::builder().content("Hello, parrot!").build(),
        )];

        let result = llm._generate(messages, None, None).await.unwrap();
        // ParrotFakeChatModel should return the last message as-is
        assert_eq!(result.generations[0].message.content(), "Hello, parrot!");
    }

    #[test]
    fn test_fake_list_chat_model_identifying_params() {
        let llm = FakeListChatModel::new(vec!["Response".to_string()]);
        let params = llm.identifying_params();

        assert!(params.contains_key("responses"));
    }

    #[test]
    fn test_fake_chat_model_identifying_params() {
        let llm = FakeChatModel::new();
        let params = llm.identifying_params();

        assert_eq!(params.get("key").unwrap(), "fake");
    }
}
