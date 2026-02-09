//! Base language model class.
//!
//! This module provides the foundational abstractions for language models,
//! mirroring `langchain_core.language_models.base`.

use std::collections::HashMap;
use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::caches::BaseCache;
use crate::callbacks::Callbacks;
use crate::error::Result;
use crate::messages::{AIMessage, BaseMessage};
use crate::outputs::LLMResult;

/// Parameters for LangSmith tracing.
///
/// These parameters are used for tracing and monitoring language model calls.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LangSmithParams {
    /// Provider of the model (e.g., "anthropic", "openai").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_provider: Option<String>,

    /// Name of the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_model_name: Option<String>,

    /// Type of the model. Should be "chat" or "llm".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_model_type: Option<String>,

    /// Temperature for generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_temperature: Option<f64>,

    /// Max tokens for generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_max_tokens: Option<u32>,

    /// Stop words for generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_stop: Option<Vec<String>>,
}

impl LangSmithParams {
    /// Create new LangSmith params with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the provider.
    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.ls_provider = Some(provider.into());
        self
    }

    /// Set the model name.
    pub fn with_model_name(mut self, model_name: impl Into<String>) -> Self {
        self.ls_model_name = Some(model_name.into());
        self
    }

    /// Set the model type.
    pub fn with_model_type(mut self, model_type: impl Into<String>) -> Self {
        self.ls_model_type = Some(model_type.into());
        self
    }

    /// Set the temperature.
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.ls_temperature = Some(temperature);
        self
    }

    /// Set the max tokens.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.ls_max_tokens = Some(max_tokens);
        self
    }

    /// Set the stop sequences.
    pub fn with_stop(mut self, stop: Vec<String>) -> Self {
        self.ls_stop = Some(stop);
        self
    }
}

use crate::prompt_values::{ChatPromptValue, ImagePromptValue, StringPromptValue};

/// Input to a language model.
///
/// Can be a string, a prompt value, or a sequence of messages.
#[derive(Debug, Clone)]
pub enum LanguageModelInput {
    /// A simple string input.
    Text(String),
    /// A string prompt value.
    StringPrompt(StringPromptValue),
    /// A chat prompt value (messages).
    ChatPrompt(ChatPromptValue),
    /// An image prompt value.
    ImagePrompt(ImagePromptValue),
    /// A sequence of messages.
    Messages(Vec<BaseMessage>),
}

impl From<String> for LanguageModelInput {
    fn from(s: String) -> Self {
        LanguageModelInput::Text(s)
    }
}

impl From<&str> for LanguageModelInput {
    fn from(s: &str) -> Self {
        LanguageModelInput::Text(s.to_string())
    }
}

impl From<StringPromptValue> for LanguageModelInput {
    fn from(p: StringPromptValue) -> Self {
        LanguageModelInput::StringPrompt(p)
    }
}

impl From<ChatPromptValue> for LanguageModelInput {
    fn from(p: ChatPromptValue) -> Self {
        LanguageModelInput::ChatPrompt(p)
    }
}

impl From<ImagePromptValue> for LanguageModelInput {
    fn from(p: ImagePromptValue) -> Self {
        LanguageModelInput::ImagePrompt(p)
    }
}

impl From<Vec<BaseMessage>> for LanguageModelInput {
    fn from(m: Vec<BaseMessage>) -> Self {
        LanguageModelInput::Messages(m)
    }
}

impl LanguageModelInput {
    /// Convert the input to messages.
    pub fn to_messages(&self) -> Vec<BaseMessage> {
        use crate::prompt_values::PromptValue;
        match self {
            LanguageModelInput::Text(s) => {
                vec![BaseMessage::Human(
                    crate::messages::HumanMessage::builder()
                        .content(s.as_str())
                        .build(),
                )]
            }
            LanguageModelInput::StringPrompt(p) => p.to_messages(),
            LanguageModelInput::ChatPrompt(p) => p.to_messages(),
            LanguageModelInput::ImagePrompt(p) => p.to_messages(),
            LanguageModelInput::Messages(m) => m.clone(),
        }
    }
}

impl std::fmt::Display for LanguageModelInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::prompt_values::PromptValue;
        match self {
            LanguageModelInput::Text(s) => write!(f, "{}", s),
            LanguageModelInput::StringPrompt(p) => write!(f, "{}", PromptValue::to_string(p)),
            LanguageModelInput::ChatPrompt(p) => write!(f, "{}", PromptValue::to_string(p)),
            LanguageModelInput::ImagePrompt(p) => write!(f, "{}", PromptValue::to_string(p)),
            LanguageModelInput::Messages(m) => {
                let joined = m
                    .iter()
                    .map(|msg| format!("{}: {}", msg.message_type(), msg.content()))
                    .collect::<Vec<_>>()
                    .join("\n");
                write!(f, "{}", joined)
            }
        }
    }
}

/// Output from a language model.
///
/// Can be either a message (from chat models) or a string (from LLMs).
#[derive(Debug, Clone)]
pub enum LanguageModelOutput {
    /// A message output (from chat models).
    Message(Box<AIMessage>),
    /// A string output (from LLMs).
    Text(String),
}

impl From<AIMessage> for LanguageModelOutput {
    fn from(m: AIMessage) -> Self {
        LanguageModelOutput::Message(Box::new(m))
    }
}

impl From<String> for LanguageModelOutput {
    fn from(s: String) -> Self {
        LanguageModelOutput::Text(s)
    }
}

impl LanguageModelOutput {
    /// Get the text content of the output.
    pub fn text(&self) -> &str {
        match self {
            LanguageModelOutput::Message(m) => m.content(),
            LanguageModelOutput::Text(s) => s,
        }
    }

    /// Convert to string, consuming the output.
    pub fn into_text(self) -> String {
        match self {
            LanguageModelOutput::Message(m) => m.content().to_string(),
            LanguageModelOutput::Text(s) => s,
        }
    }

    /// Create a Message variant from an AIMessage.
    pub fn message(m: AIMessage) -> Self {
        LanguageModelOutput::Message(Box::new(m))
    }
}

/// Custom tokenizer function type.
///
/// This is used when a custom tokenizer is provided for get_token_ids.
pub type CustomGetTokenIds = fn(&str) -> Vec<u32>;

/// Configuration for a language model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LanguageModelConfig {
    /// Whether to cache the response.
    ///
    /// - If `true`, will use the global cache.
    /// - If `false`, will not use a cache.
    /// - If not set (`None`), will use the global cache if it's set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<bool>,

    /// Tags to add to the run trace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,

    /// Metadata to add to the run trace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,

    /// Custom function for tokenizing text.
    ///
    /// If provided, this function will be used instead of the default tokenizer.
    #[serde(skip)]
    pub custom_get_token_ids: Option<CustomGetTokenIds>,
}

impl LanguageModelConfig {
    /// Create a new configuration with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable caching.
    pub fn with_cache(mut self, cache: bool) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Set tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: HashMap<String, Value>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Set a custom tokenizer function.
    ///
    /// If provided, this function will be used instead of the default tokenizer
    /// when calling `get_token_ids` on a model using this configuration.
    pub fn with_custom_get_token_ids(mut self, tokenizer: CustomGetTokenIds) -> Self {
        self.custom_get_token_ids = Some(tokenizer);
        self
    }
}

/// Abstract base trait for interfacing with language models.
///
/// All language model wrappers inherit from `BaseLanguageModel`.
/// This trait provides common functionality for both chat models and traditional LLMs.
#[async_trait]
pub trait BaseLanguageModel: Send + Sync {
    /// Return the type identifier for this language model.
    ///
    /// This is used for logging and tracing purposes.
    fn llm_type(&self) -> &str;

    /// Get the model name/identifier.
    fn model_name(&self) -> &str;

    /// Get the configuration for this model.
    fn config(&self) -> &LanguageModelConfig;

    /// Get the cache for this model, if any.
    fn cache(&self) -> Option<&dyn BaseCache> {
        None
    }

    /// Get the callbacks for this model.
    fn callbacks(&self) -> Option<&Callbacks> {
        None
    }

    /// Pass a sequence of prompts to the model and return model generations.
    ///
    /// This method should make use of batched calls for models that expose a batched API.
    ///
    /// # Arguments
    ///
    /// * `prompts` - List of `PromptValue` objects.
    /// * `stop` - Stop words to use when generating.
    /// * `callbacks` - Callbacks to pass through.
    ///
    /// # Returns
    ///
    /// An `LLMResult`, which contains a list of candidate `Generation` objects.
    async fn generate_prompt(
        &self,
        prompts: Vec<LanguageModelInput>,
        stop: Option<Vec<String>>,
        callbacks: Option<Callbacks>,
    ) -> Result<LLMResult>;

    /// Get parameters for tracing/monitoring.
    fn get_ls_params(&self, stop: Option<&[String]>) -> LangSmithParams {
        let mut params = LangSmithParams::new();

        // Try to determine provider from class name
        let llm_type = self.llm_type();
        let provider = if llm_type.starts_with("Chat") {
            llm_type
                .strip_prefix("Chat")
                .unwrap_or(llm_type)
                .to_lowercase()
        } else if llm_type.ends_with("Chat") {
            llm_type
                .strip_suffix("Chat")
                .unwrap_or(llm_type)
                .to_lowercase()
        } else {
            llm_type.to_lowercase()
        };

        params.ls_provider = Some(provider);
        params.ls_model_name = Some(self.model_name().to_string());

        if let Some(stop) = stop {
            params.ls_stop = Some(stop.to_vec());
        }

        params
    }

    /// Get the identifying parameters for this model.
    fn identifying_params(&self) -> HashMap<String, Value> {
        let mut params = HashMap::new();
        params.insert(
            "_type".to_string(),
            Value::String(self.llm_type().to_string()),
        );
        params.insert(
            "model".to_string(),
            Value::String(self.model_name().to_string()),
        );
        params
    }

    /// Get the ordered IDs of tokens in a text.
    ///
    /// # Arguments
    ///
    /// * `text` - The string input to tokenize.
    ///
    /// # Returns
    ///
    /// A list of token IDs.
    fn get_token_ids(&self, text: &str) -> Vec<u32> {
        // Default implementation: rough estimate based on whitespace
        // Actual implementations should use proper tokenizers
        text.split_whitespace()
            .enumerate()
            .map(|(i, _)| i as u32)
            .collect()
    }

    /// Get the number of tokens present in the text.
    ///
    /// # Arguments
    ///
    /// * `text` - The string input to tokenize.
    ///
    /// # Returns
    ///
    /// The number of tokens in the text.
    fn get_num_tokens(&self, text: &str) -> usize {
        self.get_token_ids(text).len()
    }

    /// Get the number of tokens in the messages.
    ///
    /// # Arguments
    ///
    /// * `messages` - The message inputs to tokenize.
    ///
    /// # Returns
    ///
    /// The sum of the number of tokens across the messages.
    fn get_num_tokens_from_messages(&self, messages: &[BaseMessage]) -> usize {
        messages
            .iter()
            .map(|m| {
                // Add some tokens for the message role/type
                let role_tokens = 4; // Approximate overhead for role
                let content_tokens = self.get_num_tokens(m.content());
                role_tokens + content_tokens
            })
            .sum()
    }
}

/// Type alias for a boxed language model output stream.
#[allow(dead_code)]
pub type LanguageModelOutputStream =
    Pin<Box<dyn Stream<Item = Result<LanguageModelOutput>> + Send>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_langsmith_params_builder() {
        let params = LangSmithParams::new()
            .with_provider("openai")
            .with_model_name("gpt-4")
            .with_model_type("chat")
            .with_temperature(0.7)
            .with_max_tokens(1000)
            .with_stop(vec!["STOP".to_string()]);

        assert_eq!(params.ls_provider, Some("openai".to_string()));
        assert_eq!(params.ls_model_name, Some("gpt-4".to_string()));
        assert_eq!(params.ls_model_type, Some("chat".to_string()));
        assert_eq!(params.ls_temperature, Some(0.7));
        assert_eq!(params.ls_max_tokens, Some(1000));
        assert_eq!(params.ls_stop, Some(vec!["STOP".to_string()]));
    }

    #[test]
    fn test_language_model_input_from_str() {
        let input: LanguageModelInput = "Hello".into();
        match input {
            LanguageModelInput::Text(s) => assert_eq!(s, "Hello"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_language_model_output_text() {
        let output = LanguageModelOutput::Text("Hello".to_string());
        assert_eq!(output.text(), "Hello");
        assert_eq!(output.into_text(), "Hello");
    }

    #[test]
    fn test_language_model_config_builder() {
        let config = LanguageModelConfig::new()
            .with_cache(true)
            .with_tags(vec!["test".to_string()]);

        assert_eq!(config.cache, Some(true));
        assert_eq!(config.tags, Some(vec!["test".to_string()]));
    }
}
