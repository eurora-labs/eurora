//! Chat models for conversational AI.
//!
//! This module provides the base abstraction for chat models,
//! following the LangChain pattern of having a common interface
//! for different providers.
//!
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
use crate::callbacks::{
    AsyncCallbackManagerForLLMRun, BaseCallbackHandler, CallbackManagerForLLMRun, Callbacks,
};
use crate::error::{Error, Result};
use crate::messages::{AIMessage, AIMessageChunk, BaseMessage, ChunkPosition, UsageMetadata};
use crate::output_parsers::JsonOutputKeyToolsParser;
use crate::outputs::{ChatGeneration, ChatGenerationChunk, ChatResult, Generation, LLMResult};
use crate::rate_limiters::BaseRateLimiter;
use crate::runnables::base::{Runnable, pipe};
use crate::runnables::config::RunnableConfig;
use crate::tools::{BaseTool, ToolDefinition};
use crate::utils::function_calling::convert_to_openai_tool;

/// Type alias for streaming output.
pub type ChatStream = Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>;

/// Type alias for a streaming chat generation output.
pub type ChatGenerationStream = Pin<Box<dyn Stream<Item = Result<ChatGenerationChunk>> + Send>>;

/// Type alias for streaming AIMessageChunk output.
pub type AIMessageChunkStream = Pin<Box<dyn Stream<Item = Result<AIMessageChunk>> + Send>>;

/// Configuration for `generate()` and `agenerate()` calls.
///
/// Wraps the optional parameters that Python passes as keyword arguments.
/// Use the builder pattern for clean construction:
///
/// ```ignore
/// let config = GenerateConfig::builder()
///     .callbacks(my_callbacks)
///     .tags(vec!["tag1".into()])
///     .build();
/// model.generate(messages, config).await?;
/// ```
#[derive(Debug, Clone, Default, bon::Builder)]
pub struct GenerateConfig {
    /// Stop words to use when generating.
    #[builder(into)]
    pub stop: Option<Vec<String>>,
    /// Callbacks to pass through.
    pub callbacks: Option<Callbacks>,
    /// Tags to apply to the run.
    #[builder(into)]
    pub tags: Option<Vec<String>>,
    /// Metadata to apply to the run.
    #[builder(into)]
    pub metadata: Option<HashMap<String, Value>>,
    /// Name for the run (used in tracing).
    #[builder(into)]
    pub run_name: Option<String>,
    /// ID for the run (used in tracing).
    pub run_id: Option<uuid::Uuid>,
}

impl GenerateConfig {
    /// Create a GenerateConfig from a RunnableConfig.
    pub fn from_runnable_config(config: &RunnableConfig) -> Self {
        Self {
            stop: None,
            callbacks: config.callbacks.clone(),
            tags: Some(config.tags.clone()).filter(|t| !t.is_empty()),
            metadata: Some(config.metadata.clone()).filter(|m| !m.is_empty()),
            run_name: config.run_name.clone(),
            run_id: config.run_id,
        }
    }
}

/// A chunk of output from streaming.
///
/// This struct carries content deltas during streaming, along with optional
/// metadata that is typically attached to the final chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunk {
    /// The content delta.
    pub content: String,
    /// Whether this is the final chunk.
    pub is_final: bool,
    /// Usage metadata (token counts) - typically present on the final chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<UsageMetadata>,
    /// The reason the model stopped generating (e.g., "stop", "length", "tool_calls").
    /// Typically present on the final chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    /// Accumulated tool calls - typically present on the final chunk when finish_reason is "tool_calls".
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<crate::messages::ToolCall>,
}

impl ChatChunk {
    /// Create a new content chunk (non-final).
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_final: false,
            usage_metadata: None,
            finish_reason: None,
            tool_calls: Vec::new(),
        }
    }

    /// Create a final chunk with optional metadata.
    pub fn final_chunk(
        usage_metadata: Option<UsageMetadata>,
        finish_reason: Option<String>,
    ) -> Self {
        Self {
            content: String::new(),
            is_final: true,
            usage_metadata,
            finish_reason,
            tool_calls: Vec::new(),
        }
    }

    /// Set usage metadata on this chunk.
    pub fn with_usage_metadata(mut self, usage: UsageMetadata) -> Self {
        self.usage_metadata = Some(usage);
        self
    }

    /// Set finish reason on this chunk.
    pub fn with_finish_reason(mut self, reason: impl Into<String>) -> Self {
        self.finish_reason = Some(reason.into());
        self
    }
}

/// Represents a tool-like object that can be bound to a chat model.
///
/// Mirrors Python's polymorphic parameter type for `bind_tools`:
/// `Sequence[Dict | type | Callable | BaseTool]`. In Rust, we support
/// `BaseTool` trait objects and raw JSON schema values.
#[derive(Debug, Clone)]
pub enum ToolLike {
    /// A concrete tool implementing the BaseTool trait.
    Tool(Arc<dyn BaseTool>),
    /// A JSON schema describing a tool (OpenAI tool format or JSON Schema).
    Schema(Value),
}

impl ToolLike {
    /// Convert to a ToolDefinition.
    pub fn to_definition(&self) -> ToolDefinition {
        match self {
            ToolLike::Tool(tool) => tool.definition(),
            ToolLike::Schema(schema) => {
                let openai_tool = convert_to_openai_tool(schema, None);
                let function = openai_tool.get("function").cloned().unwrap_or_default();
                ToolDefinition {
                    name: function
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    description: function
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("")
                        .to_string(),
                    parameters: function
                        .get("parameters")
                        .cloned()
                        .unwrap_or(Value::Object(Default::default())),
                }
            }
        }
    }
}

impl From<Arc<dyn BaseTool>> for ToolLike {
    fn from(tool: Arc<dyn BaseTool>) -> Self {
        ToolLike::Tool(tool)
    }
}

impl From<Value> for ToolLike {
    fn from(schema: Value) -> Self {
        ToolLike::Schema(schema)
    }
}

/// Configuration for tool choice.
///
/// Mirrors Python's tool_choice parameter patterns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ToolChoice {
    /// String value like "auto", "any", "none", or a specific tool name.
    String(String),
    /// Structured tool choice with type and optional name.
    Structured {
        /// Type of tool choice.
        #[serde(rename = "type")]
        choice_type: String,
        /// Optional tool name.
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
}

impl ToolChoice {
    /// Create an "auto" tool choice - let the model decide.
    pub fn auto() -> Self {
        ToolChoice::String("auto".to_string())
    }

    /// Create an "any" tool choice - model must use at least one tool.
    pub fn any() -> Self {
        ToolChoice::String("any".to_string())
    }

    /// Create a "none" tool choice - model should not use any tools.
    pub fn none() -> Self {
        ToolChoice::String("none".to_string())
    }

    /// Create a tool choice for a specific tool by name.
    pub fn tool(name: impl Into<String>) -> Self {
        ToolChoice::Structured {
            choice_type: "tool".to_string(),
            name: Some(name.into()),
        }
    }
}

/// Disable streaming options.
///
/// Mirrors Python's `disable_streaming: bool | Literal["tool_calling"]` field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DisableStreaming {
    /// Boolean value: true = always disable, false = never disable.
    Bool(bool),
    /// Literal "tool_calling": disable only when tools are present.
    ToolCalling,
}

impl Default for DisableStreaming {
    fn default() -> Self {
        DisableStreaming::Bool(false)
    }
}

impl DisableStreaming {
    /// Check if streaming should be bypassed.
    ///
    /// # Arguments
    ///
    /// * `has_tools` - Whether tools are present in the current call.
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

    /// Optional local cache instance for this chat model.
    /// When set, this cache is used instead of the global cache.
    pub cache_instance: Option<Arc<dyn crate::caches::BaseCache>>,
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
            .field(
                "cache_instance",
                &self.cache_instance.as_ref().map(|_| "..."),
            )
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

    /// Set a local cache instance for this chat model.
    pub fn with_cache_instance(mut self, cache: Arc<dyn crate::caches::BaseCache>) -> Self {
        self.cache_instance = Some(cache);
        self
    }

    /// Disable caching for this chat model.
    pub fn with_cache_disabled(mut self) -> Self {
        self.base.cache = Some(false);
        self
    }

    /// Enable caching (use global cache).
    pub fn with_cache_enabled(mut self) -> Self {
        self.base.cache = Some(true);
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
/// | Method/Property           | Description                                        | Required |
/// |--------------------------|----------------------------------------------------|---------:|
/// | `_generate`              | Use to generate a chat result from messages        | Required |
/// | `_llm_type` (property)   | Used to uniquely identify the type of the model    | Required |
/// | `_identifying_params`    | Represent model parameterization for tracing       | Optional |
/// | `_stream`                | Use to implement streaming                         | Optional |
/// | `_agenerate`             | Use to implement a native async method             | Optional |
/// | `_astream`               | Use to implement async version of `_stream`        | Optional |
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
    ///
    /// # Arguments
    ///
    /// * `messages` - The messages to generate from.
    /// * `stop` - Optional list of stop words to use when generating.
    /// * `run_manager` - Optional callback manager to use for this call.
    ///
    /// # Returns
    ///
    /// The output chat result containing generations.
    async fn _generate(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatResult>;

    /// Async version of `_generate`.
    ///
    /// Default implementation calls `_generate`.
    async fn _agenerate(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        run_manager: Option<&AsyncCallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        let sync_manager = run_manager.map(|m| m.get_sync());
        self._generate(messages, stop, sync_manager.as_ref()).await
    }

    /// Stream the output of the model.
    ///
    /// Default implementation raises NotImplementedError.
    ///
    /// # Arguments
    ///
    /// * `messages` - The messages to generate from.
    /// * `stop` - Optional list of stop words to use when generating.
    /// * `run_manager` - Optional callback manager to use for this call.
    ///
    /// # Yields
    ///
    /// The chat generation chunks.
    fn _stream(
        &self,
        _messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatGenerationStream> {
        Err(Error::NotImplemented("Streaming not implemented".into()))
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
        let sync_manager = run_manager.map(|m| m.get_sync());
        self._stream(messages, stop, sync_manager.as_ref())
    }

    /// Get the first AI message from a chat result.
    ///
    /// Helper method to extract the first generation's message as an AIMessage.
    fn get_first_message(&self, result: &ChatResult) -> Result<AIMessage> {
        if result.generations.is_empty() {
            return Err(Error::Other("No generations returned".into()));
        }

        match result.generations[0].message.clone() {
            BaseMessage::AI(message) => Ok(message),
            other => Ok(AIMessage::builder().content(other.text()).build()),
        }
    }

    /// Combine LLM outputs from multiple results.
    ///
    /// This method is called after generating results from multiple prompts
    /// to combine any LLM-specific output information.
    ///
    /// Default implementation returns an empty HashMap.
    /// Subclasses can override to combine provider-specific output data.
    fn _combine_llm_outputs(
        &self,
        _llm_outputs: &[Option<HashMap<String, Value>>],
    ) -> HashMap<String, Value> {
        HashMap::new()
    }

    /// Convert cached Generation objects to ChatGeneration objects.
    ///
    /// Handle case where cache contains Generation objects instead of
    /// ChatGeneration objects. This can happen due to serialization/deserialization
    /// issues or legacy cache data.
    fn _convert_cached_generations(&self, cache_val: Vec<Generation>) -> Vec<ChatGeneration> {
        cache_val
            .into_iter()
            .map(|cached_gen| {
                // Convert Generation to ChatGeneration by creating AIMessage from text
                let message = AIMessage::builder().content(&cached_gen.text).build();
                match cached_gen.generation_info {
                    Some(info) => ChatGeneration::with_info(message.into(), info),
                    None => ChatGeneration::new(message.into()),
                }
            })
            .collect()
    }

    /// Get invocation parameters for tracing.
    ///
    /// Returns a HashMap containing the model configuration and stop sequences.
    fn _get_invocation_params(
        &self,
        stop: Option<&[String]>,
        kwargs: Option<&HashMap<String, Value>>,
    ) -> HashMap<String, Value> {
        let mut params = self.get_identifying_params();
        if let Some(stop) = stop {
            params.insert(
                "stop".to_string(),
                Value::Array(stop.iter().map(|s| Value::String(s.clone())).collect()),
            );
        }
        if let Some(kw) = kwargs {
            params.extend(kw.clone());
        }
        params
    }

    /// Get the LLM string for cache key generation.
    ///
    /// This string uniquely identifies the model configuration for caching purposes.
    fn _get_llm_string(
        &self,
        stop: Option<&[String]>,
        kwargs: Option<&HashMap<String, Value>>,
    ) -> String {
        let params = self._get_invocation_params(stop, kwargs);

        // Sort params for deterministic key
        let mut sorted_items: Vec<_> = params.iter().collect();
        sorted_items.sort_by_key(|(k, _)| *k);

        format!("{:?}", sorted_items)
    }

    /// Check if `_stream` is implemented (not the default).
    ///
    /// This is used by `_should_stream` to determine if streaming is available.
    /// Implementations that override `_stream` should also override this to return `true`.
    fn has_stream_impl(&self) -> bool {
        false
    }

    /// Check if `_astream` is implemented (not the default).
    ///
    /// This is used by `_should_stream` to determine if async streaming is available.
    /// Implementations that override `_astream` should also override this to return `true`.
    fn has_astream_impl(&self) -> bool {
        false
    }

    /// Check if streaming is enabled via a model field.
    ///
    /// Override this if the model has a `streaming` field that should be checked.
    fn has_streaming_field(&self) -> Option<bool> {
        None
    }

    /// Determine if a given model call should hit the streaming API.
    ///
    /// This method mirrors Python's `_should_stream` behavior:
    /// 1. Check if streaming is implemented (either sync or async)
    /// 2. Check if streaming has been disabled on this instance
    /// 3. Check if streaming is disabled for tool calling and tools are present
    /// 4. Check if streaming field is set on the model
    /// 5. Check if any streaming callback handlers are present
    ///
    /// # Arguments
    ///
    /// * `async_api` - Whether this is an async API call
    /// * `has_tools` - Whether tools are present in the call
    /// * `stream_kwarg` - Optional explicit stream kwarg from caller
    /// * `run_manager` - Optional callback manager for checking streaming handlers
    ///
    /// # Returns
    ///
    /// `true` if streaming should be used, `false` otherwise.
    fn _should_stream(
        &self,
        async_api: bool,
        has_tools: bool,
        stream_kwarg: Option<bool>,
        run_manager: Option<&[Arc<dyn BaseCallbackHandler>]>,
    ) -> bool {
        // Check if streaming is implemented
        let sync_not_implemented = !self.has_stream_impl();
        let async_not_implemented = !self.has_astream_impl();

        // Check if streaming is implemented
        if !async_api && sync_not_implemented {
            return false;
        }
        // Note: since async falls back to sync, we check both here
        if async_api && async_not_implemented && sync_not_implemented {
            return false;
        }

        // Check if streaming has been disabled on this instance
        if self
            .chat_config()
            .disable_streaming
            .should_disable(has_tools)
        {
            return false;
        }

        // Check if a runtime streaming flag has been passed in
        if let Some(stream) = stream_kwarg {
            return stream;
        }

        // Check if streaming field is set on the model
        if let Some(streaming) = self.has_streaming_field() {
            return streaming;
        }

        // TODO: Python checks for `_StreamingCallbackHandler` marker trait here,
        // but we can't easily check for a marker trait in Rust. This returns true
        // if ANY handlers are present, which is more permissive than the Python
        // implementation. Consider adding a `is_streaming_handler()` method to
        // `BaseCallbackHandler` to allow filtering.
        if let Some(handlers) = run_manager
            && !handlers.is_empty()
        {
            return true;
        }

        // Default: no streaming without explicit request or callback handlers
        false
    }

    /// Generate from a batch of message lists.
    ///
    /// This method should make use of batched calls for models that expose a batched API.
    ///
    /// Use this method when you want to:
    /// 1. Take advantage of batched calls
    /// 2. Need more output from the model than just the top generated value
    /// 3. Are building chains that are agnostic to the underlying language model type
    ///
    /// # Arguments
    ///
    /// * `messages` - List of message lists.
    /// * `stop` - Stop words to use when generating.
    /// * `callbacks` - Callbacks to pass through.
    /// * `tags` - Tags to apply.
    /// * `metadata` - Metadata to apply.
    /// * `run_name` - Name of the run.
    /// * `run_id` - ID of the run.
    ///
    /// # Returns
    ///
    /// An `LLMResult` containing a list of candidate `ChatGeneration` objects.
    async fn generate(
        &self,
        messages: Vec<Vec<BaseMessage>>,
        config: GenerateConfig,
    ) -> Result<LLMResult> {
        use crate::callbacks::CallbackManager;
        use crate::outputs::RunInfo;

        let GenerateConfig {
            stop,
            callbacks,
            tags,
            metadata,
            run_name: _run_name,
            run_id,
        } = config;

        // Get invocation params and options
        let params = self._get_invocation_params(stop.as_deref(), None);
        let _options = {
            let mut opts = HashMap::new();
            if let Some(ref s) = stop {
                opts.insert(
                    "stop".to_string(),
                    Value::Array(s.iter().map(|x| Value::String(x.clone())).collect()),
                );
            }
            opts
        };

        // Get inheritable metadata including LangSmith params
        let mut inheritable_metadata = metadata.clone().unwrap_or_default();
        let ls_params = self.get_chat_ls_params(stop.as_deref());
        if let Some(provider) = ls_params.ls_provider {
            inheritable_metadata.insert("ls_provider".to_string(), Value::String(provider));
        }
        if let Some(model_name) = ls_params.ls_model_name {
            inheritable_metadata.insert("ls_model_name".to_string(), Value::String(model_name));
        }
        if let Some(model_type) = ls_params.ls_model_type {
            inheritable_metadata.insert("ls_model_type".to_string(), Value::String(model_type));
        }

        // Configure callback manager
        let callback_manager = CallbackManager::configure(
            callbacks,
            self.callbacks().cloned(),
            false,
            tags,
            self.config().tags.clone(),
            Some(inheritable_metadata),
            self.config().metadata.clone(),
        );

        // Start chat model runs
        let run_managers = callback_manager.on_chat_model_start(&params, &messages, run_id);

        // Process each message list
        let mut results = Vec::new();
        for (i, message_list) in messages.iter().enumerate() {
            let run_manager = run_managers.get(i);

            match self
                ._generate_with_cache(message_list.clone(), stop.clone(), run_manager)
                .await
            {
                Ok(result) => {
                    results.push(result);
                }
                Err(e) => {
                    // Report error to run manager
                    if let Some(rm) = run_manager {
                        rm.on_llm_error(&e);
                    }
                    return Err(e);
                }
            }
        }

        // Create flattened outputs for on_llm_end callbacks
        let flattened_outputs: Vec<LLMResult> = results
            .iter()
            .map(|res| LLMResult {
                generations: vec![res.generations.iter().cloned().map(|g| g.into()).collect()],
                llm_output: res.llm_output.clone(),
                run: None,
                result_type: "LLMResult".to_string(),
            })
            .collect();

        // Combine LLM outputs
        let llm_outputs: Vec<Option<HashMap<String, Value>>> =
            results.iter().map(|res| res.llm_output.clone()).collect();
        let combined_llm_output = self._combine_llm_outputs(&llm_outputs);

        // Collect all generations
        let generations: Vec<Vec<GenerationType>> = results
            .into_iter()
            .map(|res| res.generations.into_iter().map(|g| g.into()).collect())
            .collect();

        // Create final output
        let mut output = LLMResult {
            generations,
            llm_output: if combined_llm_output.is_empty() {
                None
            } else {
                Some(combined_llm_output)
            },
            run: None,
            result_type: "LLMResult".to_string(),
        };

        // Call on_llm_end for each run manager and collect run info
        let mut run_infos = Vec::new();
        for (run_manager, flattened_output) in run_managers.iter().zip(flattened_outputs.iter()) {
            // Convert flattened_output to ChatResult for on_llm_end
            // Extract the first generation if available
            if let Some(gen_list) = flattened_output.generations.first()
                && let Some(generation) = gen_list.first()
                && let GenerationType::ChatGeneration(chat_gen) = generation
            {
                let chat_result = crate::outputs::ChatResult::new(vec![chat_gen.clone()]);
                run_manager.on_llm_end(&chat_result);
            }
            run_infos.push(RunInfo::new(run_manager.run_id()));
        }

        // Attach run info to output
        if !run_infos.is_empty() {
            output.run = Some(run_infos);
        }

        Ok(output)
    }

    /// Async version of `generate`.
    ///
    /// # Arguments
    ///
    /// * `messages` - List of message lists.
    /// * `config` - Generation configuration (stop, callbacks, tags, metadata, etc.).
    ///
    /// # Returns
    ///
    /// An `LLMResult` containing a list of candidate `ChatGeneration` objects.
    async fn agenerate(
        &self,
        messages: Vec<Vec<BaseMessage>>,
        config: GenerateConfig,
    ) -> Result<LLMResult> {
        use crate::callbacks::AsyncCallbackManager;
        use crate::outputs::RunInfo;

        let GenerateConfig {
            stop,
            callbacks,
            tags,
            metadata,
            run_name: _run_name,
            run_id,
        } = config;

        // Get invocation params and options
        let params = self._get_invocation_params(stop.as_deref(), None);
        let _options = {
            let mut opts = HashMap::new();
            if let Some(ref s) = stop {
                opts.insert(
                    "stop".to_string(),
                    Value::Array(s.iter().map(|x| Value::String(x.clone())).collect()),
                );
            }
            opts
        };

        // Get inheritable metadata including LangSmith params
        let mut inheritable_metadata = metadata.clone().unwrap_or_default();
        let ls_params = self.get_chat_ls_params(stop.as_deref());
        if let Some(provider) = ls_params.ls_provider {
            inheritable_metadata.insert("ls_provider".to_string(), Value::String(provider));
        }
        if let Some(model_name) = ls_params.ls_model_name {
            inheritable_metadata.insert("ls_model_name".to_string(), Value::String(model_name));
        }
        if let Some(model_type) = ls_params.ls_model_type {
            inheritable_metadata.insert("ls_model_type".to_string(), Value::String(model_type));
        }

        // Configure async callback manager
        let callback_manager = AsyncCallbackManager::configure(
            callbacks,
            self.callbacks().cloned(),
            false,
            tags,
            self.config().tags.clone(),
            Some(inheritable_metadata),
            self.config().metadata.clone(),
        );

        // Start chat model runs
        let run_managers = callback_manager
            .on_chat_model_start(&params, &messages, run_id)
            .await;

        // Process each message list concurrently (matches Python's asyncio.gather)
        let futures: Vec<_> = messages
            .iter()
            .enumerate()
            .map(|(i, message_list)| {
                let message_list = message_list.clone();
                let stop = stop.clone();
                let run_manager = run_managers.get(i);
                async move {
                    let result = self
                        ._agenerate_with_cache(message_list, stop, run_manager)
                        .await;
                    (i, result)
                }
            })
            .collect();

        let settled = futures::future::join_all(futures).await;

        let mut results = Vec::with_capacity(settled.len());
        for (i, result) in settled {
            match result {
                Ok(chat_result) => {
                    results.push(chat_result);
                }
                Err(e) => {
                    if let Some(rm) = run_managers.get(i) {
                        rm.get_sync().on_llm_error(&e);
                    }
                    return Err(e);
                }
            }
        }

        // Create flattened outputs for on_llm_end callbacks
        let flattened_outputs: Vec<LLMResult> = results
            .iter()
            .map(|res| LLMResult {
                generations: vec![res.generations.iter().cloned().map(|g| g.into()).collect()],
                llm_output: res.llm_output.clone(),
                run: None,
                result_type: "LLMResult".to_string(),
            })
            .collect();

        // Combine LLM outputs
        let llm_outputs: Vec<Option<HashMap<String, Value>>> =
            results.iter().map(|res| res.llm_output.clone()).collect();
        let combined_llm_output = self._combine_llm_outputs(&llm_outputs);

        // Collect all generations
        let generations: Vec<Vec<GenerationType>> = results
            .into_iter()
            .map(|res| res.generations.into_iter().map(|g| g.into()).collect())
            .collect();

        // Create final output
        let mut output = LLMResult {
            generations,
            llm_output: if combined_llm_output.is_empty() {
                None
            } else {
                Some(combined_llm_output)
            },
            run: None,
            result_type: "LLMResult".to_string(),
        };

        // Call on_llm_end for each run manager and collect run info
        let mut run_infos = Vec::new();
        for (run_manager, flattened_output) in run_managers.iter().zip(flattened_outputs.iter()) {
            // Extract the first generation if available
            if let Some(gen_list) = flattened_output.generations.first()
                && let Some(generation) = gen_list.first()
                && let GenerationType::ChatGeneration(chat_gen) = generation
            {
                let chat_result = crate::outputs::ChatResult::new(vec![chat_gen.clone()]);
                run_manager.on_llm_end(&chat_result).await;
            }
            run_infos.push(RunInfo::new(run_manager.run_id()));
        }

        // Attach run info to output
        if !run_infos.is_empty() {
            output.run = Some(run_infos);
        }

        Ok(output)
    }

    /// Generate with cache support.
    ///
    /// This method checks the cache before calling `_generate` and caches the result.
    /// It also handles streaming if appropriate.
    async fn _generate_with_cache(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<crate::outputs::ChatResult> {
        // Resolve cache: local instance > global, respect cache=false
        let cache_config = self.chat_config().base.cache;
        let cache_instance = self.chat_config().cache_instance.clone();

        let resolved_cache: Option<std::sync::Arc<dyn crate::caches::BaseCache>> =
            if let Some(instance) = cache_instance {
                Some(instance)
            } else if cache_config == Some(false) {
                None
            } else if cache_config == Some(true) {
                let global = crate::globals::get_llm_cache();
                if global.is_none() {
                    return Err(Error::Other(
                        "Asked to cache, but no cache found at global cache.".to_string(),
                    ));
                }
                global
            } else {
                // cache is None — use global if available
                crate::globals::get_llm_cache()
            };

        // Check cache before rate limiting
        if let Some(ref cache) = resolved_cache {
            let llm_string = self._get_llm_string(stop.as_deref(), None);
            let prompt_key = serde_json::to_string(&messages).unwrap_or_default();
            if let Some(cached) = cache.lookup(&prompt_key, &llm_string) {
                let generations = self._convert_cached_generations(cached);
                return Ok(crate::outputs::ChatResult::new(generations));
            }
        }

        // Apply rate limiter after cache check
        if let Some(ref rate_limiter) = self.chat_config().rate_limiter {
            rate_limiter.acquire(true);
        }

        // Check if streaming should be used
        if self._should_stream(false, false, None, run_manager.map(|rm| rm.handlers())) {
            // Use streaming
            let stream_result = self._stream(messages.clone(), stop.clone(), run_manager);
            match stream_result {
                Ok(stream) => {
                    // Collect stream and merge chunks
                    let chat_result = agenerate_from_stream(stream).await?;
                    return Ok(chat_result);
                }
                Err(Error::NotImplemented(_)) => {
                    // Fall through to non-streaming
                }
                Err(e) => return Err(e),
            }
        }

        // Non-streaming path
        let result = self
            ._generate(messages.clone(), stop.clone(), run_manager)
            .await?;

        // Update cache with new result
        if let Some(ref cache) = resolved_cache {
            let llm_string = self._get_llm_string(stop.as_deref(), None);
            let prompt_key = serde_json::to_string(&messages).unwrap_or_default();
            let generations: Vec<crate::outputs::Generation> = result
                .generations
                .iter()
                .map(|generation| crate::outputs::Generation::new(generation.message.text()))
                .collect();
            cache.update(&prompt_key, &llm_string, generations);
        }

        Ok(result)
    }

    /// Async generate with cache support.
    ///
    /// This method checks the cache before calling `_agenerate` and caches the result.
    /// It also handles streaming if appropriate.
    async fn _agenerate_with_cache(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        run_manager: Option<&AsyncCallbackManagerForLLMRun>,
    ) -> Result<crate::outputs::ChatResult> {
        // Resolve cache (same logic as sync version)
        let cache_config = self.chat_config().base.cache;
        let cache_instance = self.chat_config().cache_instance.clone();

        let resolved_cache: Option<std::sync::Arc<dyn crate::caches::BaseCache>> =
            if let Some(instance) = cache_instance {
                Some(instance)
            } else if cache_config == Some(false) {
                None
            } else if cache_config == Some(true) {
                let global = crate::globals::get_llm_cache();
                if global.is_none() {
                    return Err(Error::Other(
                        "Asked to cache, but no cache found at global cache.".to_string(),
                    ));
                }
                global
            } else {
                crate::globals::get_llm_cache()
            };

        // Check cache before rate limiting
        if let Some(ref cache) = resolved_cache {
            let llm_string = self._get_llm_string(stop.as_deref(), None);
            let prompt_key = serde_json::to_string(&messages).unwrap_or_default();
            if let Some(cached) = cache.alookup(&prompt_key, &llm_string).await {
                let generations = self._convert_cached_generations(cached);
                return Ok(crate::outputs::ChatResult::new(generations));
            }
        }
        // Apply rate limiter after cache check
        if let Some(ref rate_limiter) = self.chat_config().rate_limiter {
            rate_limiter.aacquire(true).await;
        }

        // Check if streaming should be used
        if self._should_stream(true, false, None, run_manager.map(|rm| rm.handlers())) {
            // Use async streaming
            let stream_result = self
                ._astream(messages.clone(), stop.clone(), run_manager)
                .await;
            match stream_result {
                Ok(stream) => {
                    // Collect stream and merge chunks
                    let chat_result = agenerate_from_stream(stream).await?;
                    return Ok(chat_result);
                }
                Err(Error::NotImplemented(_)) => {
                    // Fall through to non-streaming
                }
                Err(e) => return Err(e),
            }
        }

        // Non-streaming path
        let result = self
            ._agenerate(messages.clone(), stop.clone(), run_manager)
            .await?;

        // Update cache with new result
        if let Some(ref cache) = resolved_cache {
            let llm_string = self._get_llm_string(stop.as_deref(), None);
            let prompt_key = serde_json::to_string(&messages).unwrap_or_default();
            let generations: Vec<crate::outputs::Generation> = result
                .generations
                .iter()
                .map(|generation| crate::outputs::Generation::new(generation.message.text()))
                .collect();
            cache.aupdate(&prompt_key, &llm_string, generations).await;
        }

        Ok(result)
    }

    /// Async call helper.
    ///
    /// This is a convenience method that wraps `agenerate` for single-message calls.
    async fn _call_async(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        callbacks: Option<Callbacks>,
    ) -> Result<BaseMessage> {
        let result = self
            .agenerate(
                vec![messages],
                GenerateConfig::builder()
                    .maybe_stop(stop)
                    .maybe_callbacks(callbacks)
                    .build(),
            )
            .await?;

        if result.generations.is_empty() || result.generations[0].is_empty() {
            return Err(Error::Other("No generations returned".into()));
        }

        match &result.generations[0][0] {
            GenerationType::ChatGeneration(chat_gen) => Ok(chat_gen.message.clone()),
            _ => Err(Error::Other("Unexpected generation type".into())),
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
    /// An `AIMessage` containing the generated response.
    async fn generate_with_tools(
        &self,
        messages: Vec<BaseMessage>,
        _tools: &[ToolDefinition],
        _tool_choice: Option<&ToolChoice>,
        stop: Option<Vec<String>>,
    ) -> Result<AIMessage> {
        let result = self._generate(messages, stop, None).await?;

        if result.generations.is_empty() {
            return Err(Error::Other("No generations returned".into()));
        }

        match result.generations[0].message.clone() {
            BaseMessage::AI(message) => Ok(message),
            _ => Err(Error::Other("Unexpected message type".into())),
        }
    }

    /// Convert input to messages.
    fn convert_input(&self, input: LanguageModelInput) -> Result<Vec<BaseMessage>> {
        Ok(input.to_messages())
    }

    /// Invoke the model with input.
    ///
    /// Routes through `generate()` to ensure the full callback pipeline
    /// (on_chat_model_start, on_llm_end, on_llm_error) is triggered.
    async fn invoke(
        &self,
        input: LanguageModelInput,
        config: Option<&RunnableConfig>,
    ) -> Result<AIMessage> {
        let messages = self.convert_input(input)?;

        let (callbacks, tags, metadata, run_name, run_id) = if let Some(cfg) = config {
            (
                cfg.callbacks.clone(),
                Some(cfg.tags.clone()).filter(|t| !t.is_empty()),
                Some(cfg.metadata.clone()).filter(|m| !m.is_empty()),
                cfg.run_name.clone(),
                cfg.run_id,
            )
        } else {
            (None, None, None, None, None)
        };

        let result = self
            .generate(
                vec![messages],
                GenerateConfig::builder()
                    .maybe_callbacks(callbacks)
                    .maybe_tags(tags)
                    .maybe_metadata(metadata)
                    .maybe_run_name(run_name)
                    .maybe_run_id(run_id)
                    .build(),
            )
            .await?;

        if result.generations.is_empty() || result.generations[0].is_empty() {
            return Err(Error::Other("No generations returned".into()));
        }

        match &result.generations[0][0] {
            GenerationType::ChatGeneration(chat_gen) => match &chat_gen.message {
                BaseMessage::AI(ai) => Ok(ai.clone()),
                other => Ok(AIMessage::builder().content(other.text()).build()),
            },
            _ => Err(Error::Other("Unexpected generation type".into())),
        }
    }

    /// Async invoke the model.
    ///
    /// Routes through `agenerate()` to ensure the full callback pipeline
    /// (on_chat_model_start, on_llm_end, on_llm_error) is triggered.
    async fn ainvoke(
        &self,
        input: LanguageModelInput,
        config: Option<&RunnableConfig>,
    ) -> Result<AIMessage> {
        let messages = self.convert_input(input)?;

        let (callbacks, tags, metadata, run_name, run_id) = if let Some(cfg) = config {
            (
                cfg.callbacks.clone(),
                Some(cfg.tags.clone()).filter(|t| !t.is_empty()),
                Some(cfg.metadata.clone()).filter(|m| !m.is_empty()),
                cfg.run_name.clone(),
                cfg.run_id,
            )
        } else {
            (None, None, None, None, None)
        };

        let result = self
            .agenerate(
                vec![messages],
                GenerateConfig::builder()
                    .maybe_callbacks(callbacks)
                    .maybe_tags(tags)
                    .maybe_metadata(metadata)
                    .maybe_run_name(run_name)
                    .maybe_run_id(run_id)
                    .build(),
            )
            .await?;

        if result.generations.is_empty() || result.generations[0].is_empty() {
            return Err(Error::Other("No generations returned".into()));
        }

        match &result.generations[0][0] {
            GenerationType::ChatGeneration(chat_gen) => match &chat_gen.message {
                BaseMessage::AI(ai) => Ok(ai.clone()),
                other => Ok(AIMessage::builder().content(other.text()).build()),
            },
            _ => Err(Error::Other("Unexpected generation type".into())),
        }
    }

    /// Bind tools to the model.
    ///
    /// Returns a model with the given tools bound. Provider implementations
    /// should override this method to return a configured model clone.
    ///
    /// Accepts `&[ToolLike]` to support both concrete `BaseTool` trait objects
    /// and raw JSON schema values (matching Python's polymorphic parameter).
    fn bind_tools(
        &self,
        _tools: &[ToolLike],
        _tool_choice: Option<ToolChoice>,
    ) -> Result<Box<dyn BaseChatModel>> {
        Err(Error::NotImplemented(
            "bind_tools is not implemented for this model".into(),
        ))
    }

    /// Get tool definitions from tools.
    ///
    /// Helper method to convert tool-like objects to their definitions.
    fn get_tool_definitions(&self, tools: &[ToolLike]) -> Vec<ToolDefinition> {
        tools.iter().map(|t| t.to_definition()).collect()
    }

    /// Generate a streaming response from the model.
    ///
    /// This is the main streaming API. It yields `AIMessageChunk`s.
    /// Providers should override `_stream` for native streaming support.
    ///
    /// Sets up the full callback pipeline: on_chat_model_start before
    /// streaming, on_llm_new_token for each chunk, on_llm_end at
    /// completion, and on_llm_error on failure.
    async fn stream(
        &self,
        input: LanguageModelInput,
        config: Option<&RunnableConfig>,
        stop: Option<Vec<String>>,
    ) -> Result<AIMessageChunkStream> {
        let messages = self.convert_input(input)?;
        let has_tools = false;

        // Check if streaming should be used
        if !self._should_stream(false, has_tools, Some(true), None) {
            // Fall back to invoke
            let result = self._generate(messages, stop, None).await?;
            let message = self.get_first_message(&result)?;
            let chunk = AIMessageChunk::builder()
                .content(message.content.clone())
                .build();
            return Ok(Box::pin(futures::stream::once(async move { Ok(chunk) })));
        }

        // Extract config fields
        let (callbacks, tags, metadata, _run_name, run_id) = if let Some(cfg) = config {
            (
                cfg.callbacks.clone(),
                Some(cfg.tags.clone()).filter(|t| !t.is_empty()),
                Some(cfg.metadata.clone()).filter(|m| !m.is_empty()),
                cfg.run_name.clone(),
                cfg.run_id,
            )
        } else {
            (None, None, None, None, None)
        };

        // Build inheritable metadata with LangSmith params
        let mut inheritable_metadata = metadata.unwrap_or_default();
        let ls_params = self.get_chat_ls_params(stop.as_deref());
        if let Some(provider) = ls_params.ls_provider {
            inheritable_metadata.insert("ls_provider".to_string(), Value::String(provider));
        }
        if let Some(model_name) = ls_params.ls_model_name {
            inheritable_metadata.insert("ls_model_name".to_string(), Value::String(model_name));
        }
        if let Some(model_type) = ls_params.ls_model_type {
            inheritable_metadata.insert("ls_model_type".to_string(), Value::String(model_type));
        }

        // Configure callback manager
        let params = self._get_invocation_params(stop.as_deref(), None);
        let callback_manager = crate::callbacks::CallbackManager::configure(
            callbacks,
            self.callbacks().cloned(),
            false,
            tags,
            self.config().tags.clone(),
            Some(inheritable_metadata),
            self.config().metadata.clone(),
        );
        let run_managers =
            callback_manager.on_chat_model_start(&params, &[messages.clone()], run_id);
        let run_manager = run_managers.into_iter().next();

        // Acquire rate limiter if configured
        if let Some(ref rate_limiter) = self.chat_config().rate_limiter {
            rate_limiter.acquire(true);
        }

        // Use the _stream method with callback run_manager
        let generation_stream = self._stream(messages, stop, run_manager.as_ref())?;

        let chunk_stream = async_stream::stream! {
            use futures::StreamExt;

            let mut pinned_stream = generation_stream;
            let mut chunks: Vec<ChatGenerationChunk> = Vec::new();
            let mut yielded = false;

            while let Some(result) = pinned_stream.next().await {
                match result {
                    Ok(generation_chunk) => {
                        let ai_chunk = match &generation_chunk.message {
                            BaseMessage::AI(ai_msg) => AIMessageChunk::builder().content(ai_msg.content.clone()).build(),
                            other => AIMessageChunk::builder().content(other.text()).build(),
                        };

                        // Fire on_llm_new_token callback
                        if let Some(ref rm) = run_manager {
                            rm.on_llm_new_token(ai_chunk.content.as_text_ref(), None);
                        }

                        chunks.push(generation_chunk);
                        yielded = true;
                        yield Ok(ai_chunk);
                    }
                    Err(e) => {
                        if let Some(ref rm) = run_manager {
                            rm.on_llm_error(&e);
                        }
                        yield Err(e);
                        return;
                    }
                }
            }

            // Yield a final empty chunk with chunk_position="last"
            if yielded {
                let mut final_chunk = AIMessageChunk::builder().content("").build();
                final_chunk.set_chunk_position(Some(ChunkPosition::Last));

                if let Some(ref rm) = run_manager {
                    rm.on_llm_new_token("", None);
                }

                yield Ok(final_chunk);
            }

            // Fire on_llm_end with merged generation
            if let Some(ref rm) = run_manager {
                if let Some(merged) = crate::outputs::merge_chat_generation_chunks(chunks) {
                    let chat_gen: ChatGeneration = merged.into();
                    let chat_result = ChatResult::new(vec![chat_gen]);
                    rm.on_llm_end(&chat_result);
                }
            }
        };

        Ok(Box::pin(chunk_stream))
    }

    /// Async stream the model output.
    ///
    /// This is the async version of `stream`. It yields `AIMessageChunk`s.
    /// Providers should override `_astream` for native async streaming support.
    ///
    /// Sets up the full async callback pipeline: on_chat_model_start before
    /// streaming, on_llm_new_token for each chunk, on_llm_end at
    /// completion, and on_llm_error on failure.
    async fn astream(
        &self,
        input: LanguageModelInput,
        config: Option<&RunnableConfig>,
        stop: Option<Vec<String>>,
    ) -> Result<AIMessageChunkStream> {
        let messages = self.convert_input(input)?;
        let has_tools = false;

        // Check if streaming should be used
        if !self._should_stream(true, has_tools, Some(true), None) {
            // No async or sync stream is implemented, fall back to ainvoke
            let result = self._agenerate(messages, stop, None).await?;
            let message = self.get_first_message(&result)?;
            let chunk = AIMessageChunk::builder()
                .content(message.content.clone())
                .build();
            return Ok(Box::pin(futures::stream::once(async move { Ok(chunk) })));
        }

        // Extract config fields
        let (callbacks, tags, metadata, _run_name, run_id) = if let Some(cfg) = config {
            (
                cfg.callbacks.clone(),
                Some(cfg.tags.clone()).filter(|t| !t.is_empty()),
                Some(cfg.metadata.clone()).filter(|m| !m.is_empty()),
                cfg.run_name.clone(),
                cfg.run_id,
            )
        } else {
            (None, None, None, None, None)
        };

        // Build inheritable metadata with LangSmith params
        let mut inheritable_metadata = metadata.unwrap_or_default();
        let ls_params = self.get_chat_ls_params(stop.as_deref());
        if let Some(provider) = ls_params.ls_provider {
            inheritable_metadata.insert("ls_provider".to_string(), Value::String(provider));
        }
        if let Some(model_name) = ls_params.ls_model_name {
            inheritable_metadata.insert("ls_model_name".to_string(), Value::String(model_name));
        }
        if let Some(model_type) = ls_params.ls_model_type {
            inheritable_metadata.insert("ls_model_type".to_string(), Value::String(model_type));
        }

        // Configure async callback manager
        let params = self._get_invocation_params(stop.as_deref(), None);
        let callback_manager = crate::callbacks::AsyncCallbackManager::configure(
            callbacks,
            self.callbacks().cloned(),
            false,
            tags,
            self.config().tags.clone(),
            Some(inheritable_metadata),
            self.config().metadata.clone(),
        );
        let run_managers = callback_manager
            .on_chat_model_start(&params, &[messages.clone()], run_id)
            .await;
        let run_manager = run_managers.into_iter().next();

        // Acquire rate limiter if configured
        if let Some(ref rate_limiter) = self.chat_config().rate_limiter {
            rate_limiter.aacquire(true).await;
        }

        // Use the _astream method with callback run_manager
        let generation_stream = self._astream(messages, stop, run_manager.as_ref()).await?;

        let chunk_stream = async_stream::stream! {
            use futures::StreamExt;

            let mut pinned_stream = generation_stream;
            let mut chunks: Vec<ChatGenerationChunk> = Vec::new();
            let mut yielded = false;

            while let Some(result) = pinned_stream.next().await {
                match result {
                    Ok(generation_chunk) => {
                        let ai_chunk = match &generation_chunk.message {
                            BaseMessage::AI(ai_msg) => AIMessageChunk::builder().content(ai_msg.content.clone()).build(),
                            other => AIMessageChunk::builder().content(other.text()).build(),
                        };

                        // Fire on_llm_new_token callback
                        if let Some(ref rm) = run_manager {
                            rm.on_llm_new_token(ai_chunk.content.as_text_ref(), None).await;
                        }

                        chunks.push(generation_chunk);
                        yielded = true;
                        yield Ok(ai_chunk);
                    }
                    Err(e) => {
                        if let Some(ref rm) = run_manager {
                            rm.get_sync().on_llm_error(&e);
                        }
                        yield Err(e);
                        return;
                    }
                }
            }

            // Yield a final empty chunk with chunk_position="last"
            if yielded {
                let mut final_chunk = AIMessageChunk::builder().content("").build();
                final_chunk.set_chunk_position(Some(ChunkPosition::Last));

                if let Some(ref rm) = run_manager {
                    rm.on_llm_new_token("", None).await;
                }

                yield Ok(final_chunk);
            }

            // Fire on_llm_end with merged generation
            if let Some(ref rm) = run_manager {
                if let Some(merged) = crate::outputs::merge_chat_generation_chunks(chunks) {
                    let chat_gen: ChatGeneration = merged.into();
                    let chat_result = ChatResult::new(vec![chat_gen]);
                    rm.on_llm_end(&chat_result).await;
                }
            }
        };

        Ok(Box::pin(chunk_stream))
    }

    /// Stream ChatGenerationChunk objects from the model.
    ///
    /// This is a lower-level streaming API that yields `ChatGenerationChunk`s directly.
    /// Most users should use `stream()` or `astream()` instead.
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
    async fn stream_generations(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatGenerationStream> {
        let has_tools = false;

        // Check if streaming should be used
        if !self._should_stream(false, has_tools, None, None) {
            // Fall back to non-streaming
            let result = self._generate(messages, stop, run_manager).await?;
            if result.generations.is_empty() {
                return Err(Error::Other("No generations returned".into()));
            }

            let message = result.generations[0].message.clone();
            let chunk = ChatGenerationChunk::new(message);
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

    /// Get a dictionary representation of the model.
    ///
    /// Returns identifying parameters plus the model type.
    fn to_dict(&self) -> HashMap<String, Value> {
        let mut result = self.get_identifying_params();
        result.insert(
            "_type".to_string(),
            Value::String(self.llm_type().to_string()),
        );
        result
    }

    /// Create a runnable that structures model output using a schema.
    ///
    /// Returns a `Runnable` that takes `LanguageModelInput` and produces
    /// parsed `Value` output. The chain is composed as `llm | output_parser`.
    ///
    /// This matches Python's `BaseChatModel.with_structured_output()` which
    /// returns `Runnable[LanguageModelInput, Dict | BaseModel]`.
    ///
    /// Provider implementations should override `bind_tools` first, as the
    /// default implementation uses `bind_tools` internally.
    fn with_structured_output(
        &self,
        schema: Value,
        _include_raw: bool,
    ) -> Result<Box<dyn Runnable<Input = LanguageModelInput, Output = Value> + Send + Sync>> {
        // Extract the tool name from the schema
        let tool_name = extract_tool_name_from_schema(&schema);

        // Bind tools to the model with the schema
        let tool_like = ToolLike::Schema(schema);
        let bound_model = self.bind_tools(&[tool_like], Some(ToolChoice::any()))?;

        // Create the output parser
        let output_parser = JsonOutputKeyToolsParser::new(&tool_name).with_first_tool_only(true);

        // Wrap the bound model as a Runnable and compose with the parser
        let model_runnable = ChatModelRunnable::new(Arc::from(bound_model));
        let chain = pipe(model_runnable, output_parser);
        Ok(Box::new(chain))
    }

    /// Get the identifying parameters for this model.
    ///
    /// Returns a map of parameters that uniquely identify this model instance.
    fn get_identifying_params(&self) -> HashMap<String, Value> {
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
}

/// Extract the tool name from a JSON schema.
///
/// Uses `convert_to_openai_tool` to normalize the schema, then extracts
/// the function name from the result.
pub fn extract_tool_name_from_schema(schema: &Value) -> String {
    let openai_tool = convert_to_openai_tool(schema, None);
    openai_tool
        .get("function")
        .and_then(|f| f.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Adapter that wraps a `BaseChatModel` as a `Runnable`.
///
/// This bridges the gap between `BaseChatModel` (which has its own invoke/stream
/// methods) and the `Runnable` trait (which uses associated types). This adapter
/// is needed for chain composition (e.g., `with_structured_output` which pipes
/// a chat model into an output parser).
///
/// Mirrors how Python's `BaseChatModel` inherits from `Runnable`.
pub struct ChatModelRunnable {
    model: Arc<dyn BaseChatModel>,
}

impl ChatModelRunnable {
    /// Create a new ChatModelRunnable wrapping a chat model.
    pub fn new(model: Arc<dyn BaseChatModel>) -> Self {
        Self { model }
    }
}

impl std::fmt::Debug for ChatModelRunnable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatModelRunnable")
            .field("model", &self.model.model_name())
            .finish()
    }
}

#[async_trait]
impl Runnable for ChatModelRunnable {
    type Input = LanguageModelInput;
    type Output = AIMessage;

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        let rt = tokio::runtime::Handle::current();
        let model = self.model.clone();
        rt.block_on(async move { model.invoke(input, config.as_ref()).await })
    }

    async fn ainvoke(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output> {
        self.model.ainvoke(input, config.as_ref()).await
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
    ) -> Result<ChatResult> {
        let output_str = self._call(messages, stop, run_manager).await?;
        let message = AIMessage::builder().content(output_str).build();
        let generation = ChatGeneration::new(message.into());
        Ok(ChatResult::new(vec![generation]))
    }
}

/// Generate from a stream of chunks.
///
/// Collects all chunks from the stream and generates a final ChatResult.
///
/// This corresponds to `generate_from_stream` in LangChain Python.
///
/// # Arguments
///
/// * `stream` - An iterator of `ChatGenerationChunk` objects.
///
/// # Returns
///
/// A `ChatResult` containing the merged generation.
///
/// # Errors
///
/// Returns an error if no generations are found in the stream.
pub fn generate_from_stream<I>(mut stream: I) -> Result<ChatResult>
where
    I: Iterator<Item = ChatGenerationChunk>,
{
    let first = stream.next();
    if first.is_none() {
        return Err(Error::Other("No generations found in stream.".into()));
    }

    let mut generation = first.unwrap();

    // Merge remaining chunks
    for chunk in stream {
        generation = generation + chunk;
    }

    // Convert ChatGenerationChunk to ChatGeneration
    let chat_generation: ChatGeneration = generation.into();
    Ok(ChatResult::new(vec![chat_generation]))
}

/// Async generate from a stream of chunks.
///
/// Collects all chunks from an async stream and generates a final ChatResult.
///
/// This corresponds to `agenerate_from_stream` in LangChain Python.
///
/// # Arguments
///
/// * `stream` - An async stream of `ChatGenerationChunk` objects.
///
/// # Returns
///
/// A `ChatResult` containing the merged generation.
///
/// # Errors
///
/// Returns an error if no generations are found in the stream.
pub async fn agenerate_from_stream(
    stream: impl futures::Stream<Item = Result<ChatGenerationChunk>> + Unpin,
) -> Result<ChatResult> {
    use futures::StreamExt;

    let mut chunks = Vec::new();
    futures::pin_mut!(stream);
    while let Some(result) = stream.next().await {
        chunks.push(result?);
    }

    if chunks.is_empty() {
        return Err(Error::Other("No generations found in stream.".into()));
    }

    generate_from_stream(chunks.into_iter())
}

/// Collect a stream of ChatGenerationChunks and merge them.
///
/// This is a convenience function that collects all chunks from a stream
/// and returns the merged result.
///
/// # Arguments
///
/// * `stream` - An async stream of `ChatGenerationChunk` results.
///
/// # Returns
///
/// The merged `ChatGenerationChunk`, or `None` if the stream was empty.
pub async fn collect_and_merge_stream(
    mut stream: impl futures::StreamExt<Item = Result<ChatGenerationChunk>> + Unpin,
) -> Result<Option<ChatGenerationChunk>> {
    let mut chunks = Vec::new();
    while let Some(chunk_result) = stream.next().await {
        chunks.push(chunk_result?);
    }

    if chunks.is_empty() {
        return Ok(None);
    }

    Ok(crate::outputs::merge_chat_generation_chunks(chunks))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_model_config_builder() {
        let config = ChatModelConfig::new()
            .with_cache(true)
            .with_disable_streaming(true)
            .with_output_version("v1");

        assert_eq!(config.base.cache, Some(true));
        assert_eq!(config.disable_streaming, DisableStreaming::Bool(true));
        assert_eq!(config.output_version, Some("v1".to_string()));
    }

    #[test]
    fn test_tool_choice_auto() {
        let choice = ToolChoice::auto();
        assert_eq!(choice, ToolChoice::String("auto".to_string()));
    }

    #[test]
    fn test_tool_choice_any() {
        let choice = ToolChoice::any();
        assert_eq!(choice, ToolChoice::String("any".to_string()));
    }

    #[test]
    fn test_tool_choice_none() {
        let choice = ToolChoice::none();
        assert_eq!(choice, ToolChoice::String("none".to_string()));
    }

    #[test]
    fn test_tool_choice_tool() {
        let choice = ToolChoice::tool("my_tool");
        assert_eq!(
            choice,
            ToolChoice::Structured {
                choice_type: "tool".to_string(),
                name: Some("my_tool".to_string()),
            }
        );
    }

    #[test]
    fn test_tool_choice_serialization() {
        let auto = ToolChoice::auto();
        let json = serde_json::to_string(&auto).unwrap();
        assert_eq!(json, "\"auto\"");

        let tool = ToolChoice::tool("my_tool");
        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("my_tool"));
        assert!(json.contains("tool"));
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

    #[test]
    fn test_generate_from_stream() {
        let chunks = vec![
            ChatGenerationChunk::new(AIMessage::builder().content("Hello, ").build().into()),
            ChatGenerationChunk::new(AIMessage::builder().content("world!").build().into()),
        ];

        let result = generate_from_stream(chunks.into_iter()).unwrap();
        assert_eq!(result.generations.len(), 1);
        assert_eq!(result.generations[0].message.content(), "Hello, world!");
    }

    #[test]
    fn test_generate_from_stream_empty() {
        let chunks: Vec<ChatGenerationChunk> = vec![];
        let result = generate_from_stream(chunks.into_iter());
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_agenerate_from_stream() {
        let chunks = vec![
            Ok(ChatGenerationChunk::new(
                AIMessage::builder().content("Hello, ").build().into(),
            )),
            Ok(ChatGenerationChunk::new(
                AIMessage::builder().content("world!").build().into(),
            )),
        ];

        let stream = futures::stream::iter(chunks);
        let result = agenerate_from_stream(stream).await.unwrap();
        assert_eq!(result.generations.len(), 1);
        assert_eq!(result.generations[0].message.content(), "Hello, world!");
    }

    #[tokio::test]
    async fn test_collect_and_merge_stream() {
        let chunks = vec![
            Ok(ChatGenerationChunk::new(
                AIMessage::builder().content("a").build().into(),
            )),
            Ok(ChatGenerationChunk::new(
                AIMessage::builder().content("b").build().into(),
            )),
            Ok(ChatGenerationChunk::new(
                AIMessage::builder().content("c").build().into(),
            )),
        ];

        let stream = futures::stream::iter(chunks);
        let merged = collect_and_merge_stream(stream).await.unwrap();

        assert!(merged.is_some());
        assert_eq!(merged.unwrap().text, "abc");
    }

    #[tokio::test]
    async fn test_collect_and_merge_stream_empty() {
        let chunks: Vec<Result<ChatGenerationChunk>> = vec![];
        let stream = futures::stream::iter(chunks);
        let merged = collect_and_merge_stream(stream).await.unwrap();
        assert!(merged.is_none());
    }
}
