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
use serde_json::Value;

use super::base::{BaseLanguageModel, LanguageModelConfig, LanguageModelInput};
use super::chat_models::{
    BaseChatModel, ChatGenerationStream, ChatModelConfig, ChatResult, ChatResultMetadata,
};
use crate::caches::BaseCache;
use crate::callbacks::{CallbackManagerForLLMRun, Callbacks};
use crate::error::Result;
use crate::messages::{AIMessage, BaseMessage};
use crate::outputs::{ChatGeneration, ChatGenerationChunk, GenerationType, LLMResult};

/// Fake chat model that returns messages from a list.
///
/// Cycles through responses in order.
#[derive(Debug)]
pub struct FakeMessagesListChatModel {
    /// List of responses to cycle through.
    responses: Vec<BaseMessage>,
    /// Sleep time between responses.
    sleep: Option<Duration>,
    /// Current index.
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

    /// Get the next response.
    fn get_next_response(&self) -> BaseMessage {
        let i = self.index.load(Ordering::SeqCst);
        let response = self
            .responses
            .get(i)
            .cloned()
            .unwrap_or_else(|| BaseMessage::AI(AIMessage::new("")));

        let next_i = if i + 1 < self.responses.len() {
            i + 1
        } else {
            0
        };
        self.index.store(next_i, Ordering::SeqCst);

        response
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
            let result = self.generate(messages, stop.clone(), None).await?;
            let generation = ChatGeneration::new(result.message.into());
            generations.push(vec![GenerationType::ChatGeneration(generation)]);
        }

        Ok(LLMResult::new(generations))
    }
}

#[async_trait]
impl BaseChatModel for FakeMessagesListChatModel {
    fn chat_config(&self) -> &ChatModelConfig {
        &self.config
    }

    async fn generate(
        &self,
        _messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        if let Some(duration) = self.sleep {
            tokio::time::sleep(duration).await;
        }

        let response = self.get_next_response();

        // Convert response to AIMessage
        let ai_message = match response {
            BaseMessage::AI(m) => m,
            other => AIMessage::new(other.content()),
        };

        Ok(ChatResult {
            message: ai_message,
            metadata: ChatResultMetadata::default(),
        })
    }
}

/// Error raised by FakeListChatModel during streaming.
#[derive(Debug, Clone, thiserror::Error)]
#[error("FakeListChatModel error on chunk {0}")]
pub struct FakeListChatModelError(pub usize);

/// Fake chat model that returns string responses from a list.
#[derive(Debug)]
pub struct FakeListChatModel {
    /// List of string responses to cycle through.
    responses: Vec<String>,
    /// Sleep time between responses.
    sleep: Option<Duration>,
    /// Current index.
    index: AtomicUsize,
    /// If set, raise an error on the specified chunk during streaming.
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

    /// Get the current index.
    pub fn current_index(&self) -> usize {
        self.index.load(Ordering::SeqCst)
    }

    /// Reset the index.
    pub fn reset(&self) {
        self.index.store(0, Ordering::SeqCst);
    }

    /// Get the next response.
    fn get_next_response(&self) -> String {
        let i = self.index.load(Ordering::SeqCst);
        let response = self.responses.get(i).cloned().unwrap_or_default();

        let next_i = if i + 1 < self.responses.len() {
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
            let result = self.generate(messages, stop.clone(), None).await?;
            let generation = ChatGeneration::new(result.message.into());
            generations.push(vec![GenerationType::ChatGeneration(generation)]);
        }

        Ok(LLMResult::new(generations))
    }

    fn identifying_params(&self) -> HashMap<String, Value> {
        let mut params = HashMap::new();
        params.insert(
            "_type".to_string(),
            Value::String("fake-list-chat-model".to_string()),
        );
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

    async fn generate(
        &self,
        _messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        if let Some(duration) = self.sleep {
            tokio::time::sleep(duration).await;
        }

        let response = self.get_next_response();

        Ok(ChatResult {
            message: AIMessage::new(&response),
            metadata: ChatResultMetadata::default(),
        })
    }

    async fn stream(
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
            for (i, c) in response.chars().enumerate() {
                if let Some(error_chunk) = error_on_chunk {
                    if i == error_chunk {
                        yield Err(crate::error::Error::Other(
                            format!("FakeListChatModel error on chunk {}", i)
                        ));
                        return;
                    }
                }

                if let Some(duration) = sleep {
                    tokio::time::sleep(duration).await;
                }

                let is_last = i == response_len - 1;
                let ai_message = AIMessage::new(&c.to_string());
                let chunk = ChatGenerationChunk::new(ai_message.into());
                // Note: chunk_position would need to be set separately if needed
                let _ = is_last; // suppress unused warning

                yield Ok(chunk);
            }
        };

        Ok(Box::pin(stream))
    }
}

/// Generic fake chat model that can be used to test the chat model interface.
///
/// Can be used in both sync and async tests, invokes callbacks for new tokens,
/// and breaks messages into chunks for streaming.
pub struct GenericFakeChatModel {
    /// Iterator over messages to return.
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
        Self::new(messages.into_iter().map(|s| AIMessage::new(&s)))
    }

    /// Set the configuration.
    pub fn with_config(mut self, config: ChatModelConfig) -> Self {
        self.config = config;
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
            let result = self.generate(messages, stop.clone(), None).await?;
            let generation = ChatGeneration::new(result.message.into());
            generations.push(vec![GenerationType::ChatGeneration(generation)]);
        }

        Ok(LLMResult::new(generations))
    }
}

#[async_trait]
impl BaseChatModel for GenericFakeChatModel {
    fn chat_config(&self) -> &ChatModelConfig {
        &self.config
    }

    async fn generate(
        &self,
        _messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        let message = {
            let mut guard = self.messages.lock().unwrap();
            guard.next().unwrap_or_else(|| AIMessage::new(""))
        };

        Ok(ChatResult {
            message,
            metadata: ChatResultMetadata::default(),
        })
    }

    async fn stream(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatGenerationStream> {
        let result = self.generate(messages, stop, run_manager).await?;
        let content = result.message.content().to_string();

        // Split content by whitespace, preserving whitespace
        let stream = async_stream::stream! {
            let mut chars = content.chars().peekable();
            let mut current_token = String::new();

            while let Some(c) = chars.next() {
                current_token.push(c);

                // Yield when we hit whitespace or end of string
                if c.is_whitespace() || chars.peek().is_none() {
                    let ai_message = AIMessage::new(&current_token);
                    let chunk = ChatGenerationChunk::new(ai_message.into());
                    yield Ok(chunk);
                    current_token.clear();
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

/// Parrot fake chat model that returns the last message.
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
            let result = self.generate(messages, stop.clone(), None).await?;
            let generation = ChatGeneration::new(result.message.into());
            generations.push(vec![GenerationType::ChatGeneration(generation)]);
        }

        Ok(LLMResult::new(generations))
    }
}

#[async_trait]
impl BaseChatModel for ParrotFakeChatModel {
    fn chat_config(&self) -> &ChatModelConfig {
        &self.config
    }

    async fn generate(
        &self,
        messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        // Return the last message content as an AI message
        let last_content = messages
            .last()
            .map(|m| m.content().to_string())
            .unwrap_or_default();

        Ok(ChatResult {
            message: AIMessage::new(&last_content),
            metadata: ChatResultMetadata::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::HumanMessage;

    #[tokio::test]
    async fn test_fake_messages_list_chat_model() {
        let llm = FakeMessagesListChatModel::new(vec![
            BaseMessage::AI(AIMessage::new("Response 1")),
            BaseMessage::AI(AIMessage::new("Response 2")),
        ]);

        let result = llm.generate(vec![], None, None).await.unwrap();
        assert_eq!(result.message.content(), "Response 1");

        let result = llm.generate(vec![], None, None).await.unwrap();
        assert_eq!(result.message.content(), "Response 2");

        // Cycles back
        let result = llm.generate(vec![], None, None).await.unwrap();
        assert_eq!(result.message.content(), "Response 1");
    }

    #[tokio::test]
    async fn test_fake_list_chat_model() {
        let llm = FakeListChatModel::new(vec!["Response 1".to_string(), "Response 2".to_string()]);

        let result = llm.generate(vec![], None, None).await.unwrap();
        assert_eq!(result.message.content(), "Response 1");

        let result = llm.generate(vec![], None, None).await.unwrap();
        assert_eq!(result.message.content(), "Response 2");
    }

    #[tokio::test]
    async fn test_fake_list_chat_model_stream() {
        use futures::StreamExt;

        let llm = FakeListChatModel::new(vec!["Hello".to_string()]);

        let mut stream = llm.stream(vec![], None, None).await.unwrap();

        let mut result = String::new();
        while let Some(chunk) = stream.next().await {
            let text = chunk.unwrap().text.clone();
            result.push_str(&text);
        }

        assert_eq!(result, "Hello");
    }

    #[tokio::test]
    async fn test_generic_fake_chat_model() {
        let llm = GenericFakeChatModel::from_strings(vec![
            "First response".to_string(),
            "Second response".to_string(),
        ]);

        let result = llm.generate(vec![], None, None).await.unwrap();
        assert_eq!(result.message.content(), "First response");

        let result = llm.generate(vec![], None, None).await.unwrap();
        assert_eq!(result.message.content(), "Second response");
    }

    #[tokio::test]
    async fn test_parrot_fake_chat_model() {
        let llm = ParrotFakeChatModel::new();

        let messages = vec![BaseMessage::Human(HumanMessage::new("Hello, parrot!"))];

        let result = llm.generate(messages, None, None).await.unwrap();
        assert_eq!(result.message.content(), "Hello, parrot!");
    }

    #[test]
    fn test_fake_list_chat_model_identifying_params() {
        let llm = FakeListChatModel::new(vec!["Response".to_string()]);
        let params = llm.identifying_params();

        assert_eq!(params.get("_type").unwrap(), "fake-list-chat-model");
        assert!(params.contains_key("responses"));
    }
}
