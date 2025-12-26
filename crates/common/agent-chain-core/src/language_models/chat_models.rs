//! Chat models for conversational AI.
//!
//! This module provides the base abstraction for chat models,
//! following the LangChain pattern of having a common interface
//! for different providers.
//! Mirrors `langchain_core.language_models.chat_models`.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::base::{BaseLanguageModel, LangSmithParams, LanguageModelConfig, LanguageModelInput};
use super::model_profile::ModelProfile;
use crate::GenerationType;
use crate::callbacks::{AsyncCallbackManagerForLLMRun, CallbackManagerForLLMRun, Callbacks};
use crate::error::{Error, Result};
use crate::messages::{AIMessage, BaseMessage};
use crate::outputs::{
    ChatGeneration, ChatGenerationChunk, ChatResult as OutputChatResult, LLMResult,
};
use crate::prompt_values::PromptValue;
use crate::rate_limiters::BaseRateLimiter;
use crate::tools::{Tool, ToolDefinition};

/// Output from a chat model generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResult {
    /// The generated message.
    pub message: AIMessage,
    /// Additional metadata from the model.
    #[serde(default)]
    pub metadata: ChatResultMetadata,
}

/// Metadata from a chat model generation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatResultMetadata {
    /// The model that was used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Stop reason from the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    /// Token usage information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageMetadata>,
}

/// Token usage metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageMetadata {
    /// Number of input tokens.
    pub input_tokens: u32,
    /// Number of output tokens.
    pub output_tokens: u32,
    /// Total tokens (input + output).
    pub total_tokens: u32,
}

impl UsageMetadata {
    /// Create a new usage metadata.
    pub fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
        }
    }
}

/// A chunk of output from streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunk {
    /// The content delta.
    pub content: String,
    /// Whether this is the final chunk.
    pub is_final: bool,
    /// Metadata (only present on final chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ChatResultMetadata>,
}

/// Type alias for streaming output.
pub type ChatStream = Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>;

/// Type alias for a streaming chat generation output.
pub type ChatGenerationStream = Pin<Box<dyn Stream<Item = Result<ChatGenerationChunk>> + Send>>;

/// Configuration for tool choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice {
    /// Let the model decide whether to use tools.
    Auto,
    /// Model must use at least one tool.
    Any,
    /// Model must use a specific tool.
    Tool {
        /// Name of the tool to use.
        name: String,
    },
    /// Model should not use any tools.
    None,
}

/// Disable streaming options.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DisableStreaming {
    /// Boolean value: true = always disable, false = never disable.
    Bool(bool),
    /// String "tool_calling": disable only when tools are present.
    ToolCalling,
}

impl Default for DisableStreaming {
    fn default() -> Self {
        DisableStreaming::Bool(false)
    }
}

impl DisableStreaming {
    /// Check if streaming should be bypassed.
    pub fn should_disable(&self, has_tools: bool) -> bool {
        match self {
            DisableStreaming::Bool(b) => *b,
            DisableStreaming::ToolCalling => has_tools,
        }
    }
}

impl From<bool> for DisableStreaming {
    fn from(b: bool) -> Self {
        DisableStreaming::Bool(b)
    }
}

/// Configuration specific to chat models.
#[derive(Clone, Default)]
pub struct ChatModelConfig {
    /// Base language model configuration.
    pub base: LanguageModelConfig,

    /// Rate limiter for limiting API requests.
    pub rate_limiter: Option<Arc<dyn BaseRateLimiter>>,

    /// Whether to disable streaming for this model.
    ///
    /// If streaming is bypassed, then `stream`/`astream` will defer to `invoke`/`ainvoke`.
    ///
    /// - If `Bool(true)`, will always bypass streaming case.
    /// - If `ToolCalling`, will bypass streaming case only when tools are present.
    /// - If `Bool(false)` (default), will always use streaming case if available.
    pub disable_streaming: DisableStreaming,

    /// Version of `AIMessage` output format.
    ///
    /// - `"v0"`: provider-specific format in content
    /// - `"v1"`: standardized format in content
    ///
    /// Can also be set via `LC_OUTPUT_VERSION` environment variable.
    pub output_version: Option<String>,

    /// Profile detailing model capabilities.
    pub profile: Option<ModelProfile>,
}

impl std::fmt::Debug for ChatModelConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatModelConfig")
            .field("base", &self.base)
            .field(
                "rate_limiter",
                &self.rate_limiter.as_ref().map(|_| "<rate_limiter>"),
            )
            .field("disable_streaming", &self.disable_streaming)
            .field("output_version", &self.output_version)
            .field("profile", &self.profile)
            .finish()
    }
}

impl ChatModelConfig {
    /// Create a new chat model configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the rate limiter.
    pub fn with_rate_limiter(mut self, rate_limiter: Arc<dyn BaseRateLimiter>) -> Self {
        self.rate_limiter = Some(rate_limiter);
        self
    }

    /// Set whether to disable streaming.
    pub fn with_disable_streaming(mut self, disable: impl Into<DisableStreaming>) -> Self {
        self.disable_streaming = disable.into();
        self
    }

    /// Set the output version.
    pub fn with_output_version(mut self, version: impl Into<String>) -> Self {
        self.output_version = Some(version.into());
        self
    }

    /// Set the model profile.
    pub fn with_profile(mut self, profile: ModelProfile) -> Self {
        self.profile = Some(profile);
        self
    }

    /// Enable caching.
    pub fn with_cache(mut self, cache: bool) -> Self {
        self.base.cache = Some(cache);
        self
    }

    /// Enable verbose mode.
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.base.verbose = verbose;
        self
    }

    /// Set tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.base.tags = Some(tags);
        self
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: HashMap<String, Value>) -> Self {
        self.base.metadata = Some(metadata);
        self
    }
}

/// Base trait for all chat models.
///
/// This trait follows the LangChain pattern where each provider implements
/// the core generation methods. The trait provides both sync-style (via async)
/// and streaming interfaces.
///
/// # Implementation Guide
///
/// Custom chat model implementations should override these methods:
///
/// | Method/Property         | Description                                        | Required |
/// |------------------------|----------------------------------------------------|---------:|
/// | `_generate`            | Use to generate a chat result from messages        | Required |
/// | `_llm_type` (property) | Used to uniquely identify the type of the model    | Required |
/// | `_stream`              | Use to implement streaming                         | Optional |
/// | `_agenerate`           | Use to implement a native async method             | Optional |
/// | `_astream`             | Use to implement async version of `_stream`        | Optional |
#[async_trait]
pub trait BaseChatModel: BaseLanguageModel {
    /// Get the chat model configuration.
    fn chat_config(&self) -> &ChatModelConfig;

    /// Get the model profile, if available.
    fn profile(&self) -> Option<&ModelProfile> {
        self.chat_config().profile.as_ref()
    }

    /// Core abstract method to generate a chat result.
    ///
    /// Implementations must override this method.
    async fn _generate(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<OutputChatResult>;

    /// Async version of `_generate`.
    ///
    /// Default implementation calls `_generate`.
    async fn _agenerate(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        run_manager: Option<&AsyncCallbackManagerForLLMRun>,
    ) -> Result<OutputChatResult> {
        // Default: call sync version (suboptimal but provides fallback)
        self._generate(messages, stop, None).await
    }

    /// Stream the output of the model.
    ///
    /// Default implementation raises NotImplementedError.
    fn _stream(
        &self,
        _messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatGenerationStream> {
        Err(Error::Other("Streaming not implemented".into()))
    }

    /// Async stream the output of the model.
    ///
    /// Default implementation calls `_stream`.
    async fn _astream(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        run_manager: Option<&AsyncCallbackManagerForLLMRun>,
    ) -> Result<ChatGenerationStream> {
        // Default: call sync version
        self._stream(messages, stop, None)
    }

    /// Generate from a batch of message lists.
    ///
    /// This method should make use of batched calls for models that expose a batched API.
    ///
    /// # Arguments
    ///
    /// * `messages` - List of message lists.
    /// * `stop` - Stop words to use when generating.
    /// * `callbacks` - Callbacks to pass through.
    ///
    /// # Returns
    ///
    /// An `LLMResult` containing a list of candidate `ChatGeneration` objects.
    async fn generate(
        &self,
        messages: Vec<Vec<BaseMessage>>,
        stop: Option<Vec<String>>,
        callbacks: Option<Callbacks>,
    ) -> Result<LLMResult> {
        let mut all_generations: Vec<Vec<GenerationType>> = Vec::new();

        for message_list in messages {
            let result = self._generate(message_list, stop.clone(), None).await?;
            all_generations.push(result.generations.into_iter().map(|e| e.into()).collect());
        }

        Ok(LLMResult::new(all_generations))
    }

    /// Async version of `generate`.
    async fn agenerate(
        &self,
        messages: Vec<Vec<BaseMessage>>,
        stop: Option<Vec<String>>,
        callbacks: Option<Callbacks>,
    ) -> Result<LLMResult> {
        let mut all_generations: Vec<Vec<GenerationType>> = Vec::new();

        for message_list in messages {
            let result = self._agenerate(message_list, stop.clone(), None).await?;
            all_generations.push(result.generations.into_iter().map(|e| e.into()).collect());
        }

        Ok(LLMResult::new(all_generations))
    }

    /// Convert input to messages.
    fn convert_input(&self, input: LanguageModelInput) -> Result<Vec<BaseMessage>> {
        Ok(input.to_messages())
    }

    /// Invoke the model with input.
    async fn invoke(&self, input: LanguageModelInput) -> Result<AIMessage> {
        let messages = self.convert_input(input)?;
        let result = self._generate(messages, None, None).await?;

        if result.generations.is_empty() {
            return Err(Error::Other("No generations returned".into()));
        }

        match result.generations[0].message.clone() {
            BaseMessage::AI(message) => Ok(message),
            _ => Err(Error::Other("Unexpected message type".into())),
        }
    }

    /// Async invoke the model.
    async fn ainvoke(&self, input: LanguageModelInput) -> Result<AIMessage> {
        let messages = self.convert_input(input)?;
        let result = self._agenerate(messages, None, None).await?;

        if result.generations.is_empty() {
            return Err(Error::Other("No generations returned".into()));
        }

        match result.generations[0].message.clone() {
            BaseMessage::AI(message) => Ok(message),
            _ => Err(Error::Other("Unexpected message type".into())),
        }
    }

    /// Generate a response from the model with tools.
    ///
    /// This is the preferred method when tool calling is needed.
    /// Default implementation ignores tools and calls `_generate`.
    ///
    /// # Arguments
    ///
    /// * `messages` - The conversation history.
    /// * `tools` - Tool definitions for the model to use.
    /// * `tool_choice` - Optional configuration for tool selection.
    /// * `stop` - Optional stop sequences.
    ///
    /// # Returns
    ///
    /// A `ChatResult` containing the generated message and metadata.
    async fn generate_with_tools(
        &self,
        messages: Vec<BaseMessage>,
        tools: &[ToolDefinition],
        tool_choice: Option<&ToolChoice>,
        stop: Option<Vec<String>>,
    ) -> Result<ChatResult> {
        // Default implementation ignores tools
        let _ = tools;
        let _ = tool_choice;
        let result = self._generate(messages, stop, None).await?;

        if result.generations.is_empty() {
            return Err(Error::Other("No generations returned".into()));
        }

        let message = match result.generations[0].message.clone() {
            BaseMessage::AI(message) => Ok(message),
            _ => Err(Error::Other("Unexpected message type".into())),
        }?;

        Ok(ChatResult {
            message,
            metadata: ChatResultMetadata::default(),
        })
    }

    /// Generate a streaming response from the model.
    ///
    /// Default implementation calls `_generate` and wraps the result in a stream.
    /// Providers should override this for native streaming support.
    ///
    /// # Arguments
    ///
    /// * `messages` - The conversation history.
    /// * `stop` - Optional stop sequences.
    /// * `run_manager` - Optional callback manager for the run.
    ///
    /// # Returns
    ///
    /// A stream of `ChatGenerationChunk`s.
    async fn stream(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatGenerationStream> {
        // Check if streaming should be used
        let has_tools = false; // In real implementation, check kwargs for tools
        if self
            .chat_config()
            .disable_streaming
            .should_disable(has_tools)
        {
            // Fall back to non-streaming
            let result = self._generate(messages, stop, run_manager).await?;
            if result.generations.is_empty() {
                return Err(Error::Other("No generations returned".into()));
            }

            let message = result.generations[0].message.clone();
            let chunk = ChatGenerationChunk::new(message.into());
            return Ok(Box::pin(futures::stream::once(async move { Ok(chunk) })));
        }

        // Try to use streaming
        self._stream(messages, stop, run_manager)
    }

    /// Get standard params for tracing.
    fn get_chat_ls_params(&self, stop: Option<&[String]>) -> LangSmithParams {
        let mut params = self.get_ls_params(stop);
        params.ls_model_type = Some("chat".to_string());
        params
    }
}

/// Simplified implementation for a chat model to inherit from.
///
/// This implementation is primarily here for backwards compatibility.
/// For new implementations, please use `BaseChatModel` directly.
#[async_trait]
pub trait SimpleChatModel: BaseChatModel {
    /// Simple call method that takes messages and returns a string.
    ///
    /// Implementations should override this method.
    async fn _call(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<String>;
}

#[async_trait]
impl<T: SimpleChatModel> BaseChatModel for T {
    fn chat_config(&self) -> &ChatModelConfig {
        <T as BaseChatModel>::chat_config(self)
    }

    async fn _generate(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<OutputChatResult> {
        let output_str = self._call(messages, stop, run_manager).await?;
        let message = AIMessage::new(output_str);
        let generation = ChatGeneration::new(message.into());
        Ok(OutputChatResult::new(vec![generation]))
    }
}

/// A chat model that has been bound with tools (generic version).
///
/// This wraps an underlying chat model and includes tool definitions
/// that will be passed to the model on each invocation.
pub struct BoundChatModel<M: BaseChatModel> {
    /// The underlying chat model.
    model: M,
    /// Tools bound to this model.
    tools: Vec<Arc<dyn Tool + Send + Sync>>,
    /// Tool choice configuration.
    tool_choice: Option<ToolChoice>,
}

impl<M: BaseChatModel> BoundChatModel<M> {
    /// Create a new bound chat model.
    pub fn new(model: M, tools: Vec<Arc<dyn Tool + Send + Sync>>) -> Self {
        Self {
            model,
            tools,
            tool_choice: None,
        }
    }

    /// Set the tool choice.
    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    /// Get the tool definitions.
    pub fn tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.iter().map(|t| t.definition()).collect()
    }

    /// Get a reference to the underlying model.
    pub fn model(&self) -> &M {
        &self.model
    }

    /// Get the tools.
    pub fn tools(&self) -> &[Arc<dyn Tool + Send + Sync>] {
        &self.tools
    }

    /// Get the tool choice.
    pub fn tool_choice(&self) -> Option<&ToolChoice> {
        self.tool_choice.as_ref()
    }

    /// Invoke the model with messages.
    ///
    /// This generates a response using the bound tools.
    pub async fn invoke(&self, messages: Vec<BaseMessage>) -> BaseMessage {
        let tool_definitions = self.tool_definitions();
        match self
            .model
            .generate_with_tools(messages, &tool_definitions, self.tool_choice.as_ref(), None)
            .await
        {
            Ok(result) => result.message.into(),
            Err(e) => {
                // Return an error message
                AIMessage::new(format!("Error: {}", e)).into()
            }
        }
    }
}

impl<M: BaseChatModel + Clone> Clone for BoundChatModel<M> {
    fn clone(&self) -> Self {
        Self {
            model: self.model.clone(),
            tools: self.tools.clone(),
            tool_choice: self.tool_choice.clone(),
        }
    }
}

/// Extension trait for chat models to add tool binding.
pub trait ChatModelExt: BaseChatModel + Sized {
    /// Bind tools to this chat model.
    ///
    /// # Arguments
    ///
    /// * `tools` - The tools to bind.
    ///
    /// # Returns
    ///
    /// A `BoundChatModel` that includes the tools.
    fn bind_tools(self, tools: Vec<Arc<dyn Tool + Send + Sync>>) -> BoundChatModel<Self> {
        BoundChatModel::new(self, tools)
    }
}

// Implement ChatModelExt for all BaseChatModel implementations
impl<T: BaseChatModel + Sized> ChatModelExt for T {}

/// A dynamically-typed chat model bound with tools.
///
/// This is the dynamic dispatch version of `BoundChatModel`, useful when
/// working with `Arc<dyn BaseChatModel>` or boxed trait objects.
#[derive(Clone)]
pub struct DynBoundChatModel {
    /// The underlying chat model.
    model: Arc<dyn BaseChatModel>,
    /// Tools bound to this model.
    tools: Vec<Arc<dyn Tool + Send + Sync>>,
    /// Tool choice configuration.
    tool_choice: Option<ToolChoice>,
}

impl DynBoundChatModel {
    /// Create a new dynamically-typed bound chat model.
    pub fn new(model: Arc<dyn BaseChatModel>, tools: Vec<Arc<dyn Tool + Send + Sync>>) -> Self {
        Self {
            model,
            tools,
            tool_choice: None,
        }
    }

    /// Set the tool choice.
    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    /// Get the tool definitions.
    pub fn tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.iter().map(|t| t.definition()).collect()
    }

    /// Get a reference to the underlying model.
    pub fn model(&self) -> &Arc<dyn BaseChatModel> {
        &self.model
    }

    /// Get the tools.
    pub fn tools(&self) -> &[Arc<dyn Tool + Send + Sync>] {
        &self.tools
    }

    /// Get the tool choice.
    pub fn tool_choice(&self) -> Option<&ToolChoice> {
        self.tool_choice.as_ref()
    }

    /// Invoke the model with messages.
    ///
    /// This generates a response using the bound tools.
    pub async fn invoke(&self, messages: Vec<BaseMessage>) -> BaseMessage {
        let tool_definitions = self.tool_definitions();
        match self
            .model
            .generate_with_tools(messages, &tool_definitions, self.tool_choice.as_ref(), None)
            .await
        {
            Ok(result) => result.message.into(),
            Err(e) => {
                // Return an error message
                AIMessage::new(format!("Error: {}", e)).into()
            }
        }
    }
}

/// Extension methods for `Arc<dyn BaseChatModel>`.
pub trait DynChatModelExt {
    /// Bind tools to this chat model, returning a dynamically-typed bound model.
    fn bind_tools(self, tools: Vec<Arc<dyn Tool + Send + Sync>>) -> DynBoundChatModel;
}

impl DynChatModelExt for Arc<dyn BaseChatModel> {
    fn bind_tools(self, tools: Vec<Arc<dyn Tool + Send + Sync>>) -> DynBoundChatModel {
        DynBoundChatModel::new(self, tools)
    }
}

/// Generate from a stream of chunks.
///
/// Collects all chunks from the stream and generates a final ChatResult.
pub async fn generate_from_stream(
    mut stream: impl futures::StreamExt<Item = Result<ChatGenerationChunk>> + Unpin,
) -> Result<OutputChatResult> {
    use futures::StreamExt;

    let mut chunks = Vec::new();
    while let Some(chunk_result) = stream.next().await {
        chunks.push(chunk_result?);
    }

    if chunks.is_empty() {
        return Err(Error::Other("No generations found in stream.".into()));
    }

    // Merge all chunks
    let merged = crate::outputs::merge_chat_generation_chunks(chunks);

    match merged {
        Some(generation_chunk) => {
            // Convert ChatGenerationChunk to ChatGeneration
            let chat_generation: ChatGeneration = generation_chunk.into();
            Ok(OutputChatResult::new(vec![chat_generation]))
        }
        None => Err(Error::Other("Failed to merge chunks.".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_metadata() {
        let usage = UsageMetadata::new(100, 50);
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
    }

    #[test]
    fn test_chat_model_config_builder() {
        let config = ChatModelConfig::new()
            .with_cache(true)
            .with_verbose(true)
            .with_disable_streaming(true)
            .with_output_version("v1");

        assert_eq!(config.base.cache, Some(true));
        assert!(config.base.verbose);
        assert_eq!(config.disable_streaming, DisableStreaming::Bool(true));
        assert_eq!(config.output_version, Some("v1".to_string()));
    }

    #[test]
    fn test_tool_choice_serialization() {
        let auto = ToolChoice::Auto;
        let json = serde_json::to_string(&auto).unwrap();
        assert!(json.contains("auto"));

        let tool = ToolChoice::Tool {
            name: "my_tool".to_string(),
        };
        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("my_tool"));
    }

    #[test]
    fn test_chat_result_metadata_serialization() {
        let metadata = ChatResultMetadata {
            model: Some("gpt-4".to_string()),
            stop_reason: Some("stop".to_string()),
            usage: Some(UsageMetadata::new(100, 50)),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("gpt-4"));
        assert!(json.contains("stop"));
        assert!(json.contains("150")); // total_tokens
    }

    #[test]
    fn test_disable_streaming() {
        let bool_false = DisableStreaming::Bool(false);
        assert!(!bool_false.should_disable(true));
        assert!(!bool_false.should_disable(false));

        let bool_true = DisableStreaming::Bool(true);
        assert!(bool_true.should_disable(true));
        assert!(bool_true.should_disable(false));

        let tool_calling = DisableStreaming::ToolCalling;
        assert!(tool_calling.should_disable(true));
        assert!(!tool_calling.should_disable(false));
    }
}
