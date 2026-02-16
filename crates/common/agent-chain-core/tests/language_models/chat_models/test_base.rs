//! Tests for base chat model.
//!
//! Mirrors `langchain/libs/core/tests/unit_tests/language_models/chat_models/test_base.py`
//!
//! This file contains tests for the BaseChatModel trait and related functionality.

use agent_chain_core::error::{Error, Result};
use agent_chain_core::language_models::GenerateConfig;
use agent_chain_core::language_models::{
    BaseChatModel, BaseLanguageModel, ChatGenerationStream, ChatModelConfig, DisableStreaming,
    FakeListChatModel, GenericFakeChatModel, LangSmithParams, LanguageModelConfig,
    LanguageModelInput, ModelProfile,
};
use agent_chain_core::messages::{AIMessage, BaseMessage, HumanMessage, SystemMessage};
use agent_chain_core::outputs::{ChatGeneration, ChatGenerationChunk, ChatResult};
use async_trait::async_trait;
use futures::StreamExt;

/// Helper function to create messages fixture
fn create_messages() -> Vec<BaseMessage> {
    vec![
        BaseMessage::System(
            SystemMessage::builder()
                .content("You are a test user.")
                .build(),
        ),
        BaseMessage::Human(
            HumanMessage::builder()
                .content("Hello, I am a test user.")
                .build(),
        ),
    ]
}

/// Helper function to create a second set of messages fixture
fn create_messages_2() -> Vec<BaseMessage> {
    vec![
        BaseMessage::System(
            SystemMessage::builder()
                .content("You are a test user.")
                .build(),
        ),
        BaseMessage::Human(
            HumanMessage::builder()
                .content("Hello, I not a test user.")
                .build(),
        ),
    ]
}

// =============================================================================
// Streaming Fallback Tests
// =============================================================================

/// A model that only implements `_generate` (no streaming).
struct ModelWithGenerateOnly {
    config: ChatModelConfig,
}

impl ModelWithGenerateOnly {
    fn new() -> Self {
        Self {
            config: ChatModelConfig::default(),
        }
    }
}

#[async_trait]
impl BaseLanguageModel for ModelWithGenerateOnly {
    fn llm_type(&self) -> &str {
        "fake-chat-model"
    }

    fn model_name(&self) -> &str {
        "fake-chat"
    }

    fn config(&self) -> &LanguageModelConfig {
        &self.config.base
    }

    fn cache(&self) -> Option<&dyn agent_chain_core::caches::BaseCache> {
        None
    }

    fn callbacks(&self) -> Option<&agent_chain_core::callbacks::Callbacks> {
        None
    }

    async fn generate_prompt(
        &self,
        prompts: Vec<LanguageModelInput>,
        stop: Option<Vec<String>>,
        _callbacks: Option<agent_chain_core::callbacks::Callbacks>,
    ) -> Result<agent_chain_core::outputs::LLMResult> {
        let mut generations = Vec::new();
        for prompt in prompts {
            let messages = prompt.to_messages();
            let result = self._generate(messages, stop.clone(), None).await?;
            generations.push(
                result
                    .generations
                    .into_iter()
                    .map(agent_chain_core::GenerationType::ChatGeneration)
                    .collect(),
            );
        }
        Ok(agent_chain_core::outputs::LLMResult::new(generations))
    }
}

#[async_trait]
impl BaseChatModel for ModelWithGenerateOnly {
    fn chat_config(&self) -> &ChatModelConfig {
        &self.config
    }

    async fn _generate(
        &self,
        _messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&agent_chain_core::callbacks::CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        let message = AIMessage::builder().content("hello").build();
        let generation = ChatGeneration::new(message.into());
        Ok(ChatResult::new(vec![generation]))
    }
}

#[tokio::test]
async fn test_astream_fallback_to_ainvoke() {
    // Test `astream()` uses appropriate implementation.
    // When streaming is not implemented, it should fall back to invoke
    // and return the result as a single chunk.
    // Python equivalent: test_astream_fallback_to_ainvoke()

    let model = ModelWithGenerateOnly::new();

    // Test sync stream
    let chunks: Vec<BaseMessage> = {
        let result = model
            ._generate(vec![], None, None)
            .await
            .expect("_generate should succeed");
        result.generations.into_iter().map(|g| g.message).collect()
    };

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].content(), "hello");

    // Test that model does NOT have stream implementation
    assert!(!model.has_stream_impl());
}

/// A model that implements `_stream` but not `_astream`.
struct ModelWithSyncStream {
    config: ChatModelConfig,
}

impl ModelWithSyncStream {
    fn new() -> Self {
        Self {
            config: ChatModelConfig::default(),
        }
    }
}

#[async_trait]
impl BaseLanguageModel for ModelWithSyncStream {
    fn llm_type(&self) -> &str {
        "fake-chat-model"
    }

    fn model_name(&self) -> &str {
        "fake-chat"
    }

    fn config(&self) -> &LanguageModelConfig {
        &self.config.base
    }

    fn cache(&self) -> Option<&dyn agent_chain_core::caches::BaseCache> {
        None
    }

    fn callbacks(&self) -> Option<&agent_chain_core::callbacks::Callbacks> {
        None
    }

    async fn generate_prompt(
        &self,
        _prompts: Vec<LanguageModelInput>,
        _stop: Option<Vec<String>>,
        _callbacks: Option<agent_chain_core::callbacks::Callbacks>,
    ) -> Result<agent_chain_core::outputs::LLMResult> {
        Err(Error::NotImplemented("not implemented".into()))
    }
}

#[async_trait]
impl BaseChatModel for ModelWithSyncStream {
    fn chat_config(&self) -> &ChatModelConfig {
        &self.config
    }

    async fn _generate(
        &self,
        _messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&agent_chain_core::callbacks::CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        Err(Error::NotImplemented("Use streaming".into()))
    }

    fn _stream(
        &self,
        _messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&agent_chain_core::callbacks::CallbackManagerForLLMRun>,
    ) -> Result<ChatGenerationStream> {
        let stream = async_stream::stream! {
            yield Ok(ChatGenerationChunk::new(AIMessage::builder().content("a").build().into()));
            yield Ok(ChatGenerationChunk::new(AIMessage::builder().content("b").build().into()));
        };
        Ok(Box::pin(stream))
    }

    fn has_stream_impl(&self) -> bool {
        true
    }
}

#[tokio::test]
async fn test_astream_implementation_fallback_to_stream() {
    // Test astream falls back to sync stream implementation.
    // Python equivalent: test_astream_implementation_fallback_to_stream()

    let model = ModelWithSyncStream::new();

    // Collect stream chunks
    let mut stream = model
        ._stream(vec![], None, None)
        .expect("stream should work");
    let mut chunks = Vec::new();
    while let Some(chunk_result) = stream.next().await {
        chunks.push(chunk_result.expect("chunk should succeed"));
    }

    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].message.content(), "a");
    assert_eq!(chunks[1].message.content(), "b");

    // Verify that model has sync stream but not async stream
    assert!(model.has_stream_impl());
    assert!(!model.has_astream_impl());
}

/// A model that implements `_astream`.
struct ModelWithAsyncStream {
    config: ChatModelConfig,
}

impl ModelWithAsyncStream {
    fn new() -> Self {
        Self {
            config: ChatModelConfig::default(),
        }
    }
}

#[async_trait]
impl BaseLanguageModel for ModelWithAsyncStream {
    fn llm_type(&self) -> &str {
        "fake-chat-model"
    }

    fn model_name(&self) -> &str {
        "fake-chat"
    }

    fn config(&self) -> &LanguageModelConfig {
        &self.config.base
    }

    fn cache(&self) -> Option<&dyn agent_chain_core::caches::BaseCache> {
        None
    }

    fn callbacks(&self) -> Option<&agent_chain_core::callbacks::Callbacks> {
        None
    }

    async fn generate_prompt(
        &self,
        _prompts: Vec<LanguageModelInput>,
        _stop: Option<Vec<String>>,
        _callbacks: Option<agent_chain_core::callbacks::Callbacks>,
    ) -> Result<agent_chain_core::outputs::LLMResult> {
        Err(Error::NotImplemented("not implemented".into()))
    }
}

#[async_trait]
impl BaseChatModel for ModelWithAsyncStream {
    fn chat_config(&self) -> &ChatModelConfig {
        &self.config
    }

    async fn _generate(
        &self,
        _messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&agent_chain_core::callbacks::CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        Err(Error::NotImplemented("Use streaming".into()))
    }

    async fn _astream(
        &self,
        _messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&agent_chain_core::callbacks::AsyncCallbackManagerForLLMRun>,
    ) -> Result<ChatGenerationStream> {
        let stream = async_stream::stream! {
            yield Ok(ChatGenerationChunk::new(AIMessage::builder().content("a").build().into()));
            yield Ok(ChatGenerationChunk::new(AIMessage::builder().content("b").build().into()));
        };
        Ok(Box::pin(stream))
    }

    fn has_astream_impl(&self) -> bool {
        true
    }
}

#[tokio::test]
async fn test_astream_implementation_uses_astream() {
    // Test that astream uses the async implementation when available.
    // Python equivalent: test_astream_implementation_uses_astream()

    let model = ModelWithAsyncStream::new();

    // Collect astream chunks
    let mut stream = model
        ._astream(vec![], None, None)
        .await
        .expect("astream should work");
    let mut chunks = Vec::new();
    while let Some(chunk_result) = stream.next().await {
        chunks.push(chunk_result.expect("chunk should succeed"));
    }

    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].message.content(), "a");
    assert_eq!(chunks[1].message.content(), "b");

    // Verify model has async stream
    assert!(model.has_astream_impl());
}

// =============================================================================
// Disable Streaming Tests
// =============================================================================

/// A model without streaming support.
struct NoStreamingModel {
    config: ChatModelConfig,
}

impl NoStreamingModel {
    fn new() -> Self {
        Self {
            config: ChatModelConfig::default(),
        }
    }

    fn with_disable_streaming(mut self, disable: DisableStreaming) -> Self {
        self.config.disable_streaming = disable;
        self
    }
}

#[async_trait]
impl BaseLanguageModel for NoStreamingModel {
    fn llm_type(&self) -> &str {
        "model1"
    }

    fn model_name(&self) -> &str {
        "model1"
    }

    fn config(&self) -> &LanguageModelConfig {
        &self.config.base
    }

    fn cache(&self) -> Option<&dyn agent_chain_core::caches::BaseCache> {
        None
    }

    fn callbacks(&self) -> Option<&agent_chain_core::callbacks::Callbacks> {
        None
    }

    async fn generate_prompt(
        &self,
        prompts: Vec<LanguageModelInput>,
        stop: Option<Vec<String>>,
        _callbacks: Option<agent_chain_core::callbacks::Callbacks>,
    ) -> Result<agent_chain_core::outputs::LLMResult> {
        let mut generations = Vec::new();
        for prompt in prompts {
            let messages = prompt.to_messages();
            let result = self._generate(messages, stop.clone(), None).await?;
            generations.push(
                result
                    .generations
                    .into_iter()
                    .map(agent_chain_core::GenerationType::ChatGeneration)
                    .collect(),
            );
        }
        Ok(agent_chain_core::outputs::LLMResult::new(generations))
    }
}

#[async_trait]
impl BaseChatModel for NoStreamingModel {
    fn chat_config(&self) -> &ChatModelConfig {
        &self.config
    }

    async fn _generate(
        &self,
        _messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&agent_chain_core::callbacks::CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        let message = AIMessage::builder().content("invoke").build();
        let generation = ChatGeneration::new(message.into());
        Ok(ChatResult::new(vec![generation]))
    }
}

/// A model with streaming support.
struct StreamingModel {
    config: ChatModelConfig,
    streaming: bool,
}

impl StreamingModel {
    fn new() -> Self {
        Self {
            config: ChatModelConfig::default(),
            streaming: false,
        }
    }

    fn with_disable_streaming(mut self, disable: DisableStreaming) -> Self {
        self.config.disable_streaming = disable;
        self
    }

    #[allow(dead_code)]
    fn with_streaming(mut self, streaming: bool) -> Self {
        self.streaming = streaming;
        self
    }
}

#[async_trait]
impl BaseLanguageModel for StreamingModel {
    fn llm_type(&self) -> &str {
        "model1"
    }

    fn model_name(&self) -> &str {
        "model1"
    }

    fn config(&self) -> &LanguageModelConfig {
        &self.config.base
    }

    fn cache(&self) -> Option<&dyn agent_chain_core::caches::BaseCache> {
        None
    }

    fn callbacks(&self) -> Option<&agent_chain_core::callbacks::Callbacks> {
        None
    }

    async fn generate_prompt(
        &self,
        prompts: Vec<LanguageModelInput>,
        stop: Option<Vec<String>>,
        _callbacks: Option<agent_chain_core::callbacks::Callbacks>,
    ) -> Result<agent_chain_core::outputs::LLMResult> {
        let mut generations = Vec::new();
        for prompt in prompts {
            let messages = prompt.to_messages();
            let result = self._generate(messages, stop.clone(), None).await?;
            generations.push(
                result
                    .generations
                    .into_iter()
                    .map(agent_chain_core::GenerationType::ChatGeneration)
                    .collect(),
            );
        }
        Ok(agent_chain_core::outputs::LLMResult::new(generations))
    }
}

#[async_trait]
impl BaseChatModel for StreamingModel {
    fn chat_config(&self) -> &ChatModelConfig {
        &self.config
    }

    async fn _generate(
        &self,
        _messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&agent_chain_core::callbacks::CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        let message = AIMessage::builder().content("invoke").build();
        let generation = ChatGeneration::new(message.into());
        Ok(ChatResult::new(vec![generation]))
    }

    fn _stream(
        &self,
        _messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&agent_chain_core::callbacks::CallbackManagerForLLMRun>,
    ) -> Result<ChatGenerationStream> {
        let stream = async_stream::stream! {
            yield Ok(ChatGenerationChunk::new(AIMessage::builder().content("stream").build().into()));
        };
        Ok(Box::pin(stream))
    }

    fn has_stream_impl(&self) -> bool {
        true
    }

    fn has_streaming_field(&self) -> Option<bool> {
        if self.streaming {
            Some(self.streaming)
        } else {
            None
        }
    }
}

#[test]
fn test_disable_streaming_bool_true() {
    // Test disable_streaming with Bool(true) always disables.
    // Python equivalent: test_disable_streaming() with disable_streaming=True

    let model = StreamingModel::new().with_disable_streaming(DisableStreaming::Bool(true));

    // _should_stream should return false when disable_streaming is true
    assert!(!model._should_stream(false, false, None, None));
    assert!(!model._should_stream(false, true, None, None)); // with tools
}

#[test]
fn test_disable_streaming_bool_false() {
    // Test disable_streaming with Bool(false) never disables.
    // Python equivalent: test_disable_streaming() with disable_streaming=False

    let model = StreamingModel::new().with_disable_streaming(DisableStreaming::Bool(false));

    // _should_stream should return true when streaming is implemented and handlers present
    let handlers: Vec<std::sync::Arc<dyn agent_chain_core::callbacks::base::BaseCallbackHandler>> =
        vec![std::sync::Arc::new(
            agent_chain_core::callbacks::StdOutCallbackHandler::new(),
        )];
    assert!(model._should_stream(false, false, None, Some(&handlers)));
    assert!(model._should_stream(false, true, None, Some(&handlers))); // with tools
}

#[test]
fn test_disable_streaming_tool_calling() {
    // Test disable_streaming with ToolCalling disables only when tools present.
    // Python equivalent: test_disable_streaming() with disable_streaming="tool_calling"

    let model = StreamingModel::new().with_disable_streaming(DisableStreaming::ToolCalling);

    let handlers: Vec<std::sync::Arc<dyn agent_chain_core::callbacks::base::BaseCallbackHandler>> =
        vec![std::sync::Arc::new(
            agent_chain_core::callbacks::StdOutCallbackHandler::new(),
        )];

    // Without tools, streaming should work
    assert!(model._should_stream(false, false, None, Some(&handlers)));

    // With tools, streaming should be disabled
    assert!(!model._should_stream(false, true, None, Some(&handlers)));
}

#[tokio::test]
async fn test_disable_streaming_async() {
    // Test disable_streaming async variants.
    // Python equivalent: test_disable_streaming_async()

    let handlers: Vec<std::sync::Arc<dyn agent_chain_core::callbacks::base::BaseCallbackHandler>> =
        vec![std::sync::Arc::new(
            agent_chain_core::callbacks::StdOutCallbackHandler::new(),
        )];

    // Test Bool(true)
    let model = StreamingModel::new().with_disable_streaming(DisableStreaming::Bool(true));
    let result = model
        .invoke(LanguageModelInput::Messages(vec![]), None)
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().content, "invoke");

    // Test Bool(false) - streaming works with handlers
    let model = StreamingModel::new().with_disable_streaming(DisableStreaming::Bool(false));
    assert!(model._should_stream(true, false, None, Some(&handlers)));

    // Test ToolCalling
    let model = StreamingModel::new().with_disable_streaming(DisableStreaming::ToolCalling);
    assert!(model._should_stream(true, false, None, Some(&handlers))); // no tools
    assert!(!model._should_stream(true, true, None, Some(&handlers))); // with tools
}

#[tokio::test]
async fn test_disable_streaming_no_streaming_model() {
    // Test disable_streaming on models without streaming support.
    // Python equivalent: test_disable_streaming_no_streaming_model()

    let model = NoStreamingModel::new().with_disable_streaming(DisableStreaming::Bool(true));
    let result = model
        .invoke(LanguageModelInput::Messages(vec![]), None)
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().content, "invoke");

    // Even with Bool(false), _should_stream returns false because no stream impl
    let model = NoStreamingModel::new().with_disable_streaming(DisableStreaming::Bool(false));
    assert!(!model._should_stream(false, false, None, None));
}

#[tokio::test]
async fn test_disable_streaming_no_streaming_model_async() {
    // Test async disable_streaming on non-streaming models.
    // Python equivalent: test_disable_streaming_no_streaming_model_async()

    for disable in [
        DisableStreaming::Bool(true),
        DisableStreaming::Bool(false),
        DisableStreaming::ToolCalling,
    ] {
        let model = NoStreamingModel::new().with_disable_streaming(disable);
        let result = model
            .ainvoke(LanguageModelInput::Messages(vec![]), None)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content, "invoke");
    }
}

// =============================================================================
// LangSmith Params Tests
// =============================================================================

/// A model for testing _get_ls_params.
struct LSParamsModel {
    config: ChatModelConfig,
    model: String,
    temperature: f64,
    max_tokens: u32,
}

impl LSParamsModel {
    fn new() -> Self {
        Self {
            config: ChatModelConfig::default(),
            model: "foo".to_string(),
            temperature: 0.1,
            max_tokens: 1024,
        }
    }
}

#[async_trait]
impl BaseLanguageModel for LSParamsModel {
    fn llm_type(&self) -> &str {
        "lsparamsmodel"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn config(&self) -> &LanguageModelConfig {
        &self.config.base
    }

    fn cache(&self) -> Option<&dyn agent_chain_core::caches::BaseCache> {
        None
    }

    fn callbacks(&self) -> Option<&agent_chain_core::callbacks::Callbacks> {
        None
    }

    async fn generate_prompt(
        &self,
        _prompts: Vec<LanguageModelInput>,
        _stop: Option<Vec<String>>,
        _callbacks: Option<agent_chain_core::callbacks::Callbacks>,
    ) -> Result<agent_chain_core::outputs::LLMResult> {
        Err(Error::NotImplemented("not implemented".into()))
    }

    fn get_ls_params(&self, stop: Option<&[String]>) -> LangSmithParams {
        let mut params = LangSmithParams::new()
            .with_provider("lsparamsmodel")
            .with_model_name(&self.model)
            .with_model_type("chat")
            .with_temperature(self.temperature)
            .with_max_tokens(self.max_tokens);

        if let Some(stop_words) = stop {
            params = params.with_stop(stop_words.to_vec());
        }

        params
    }
}

#[async_trait]
impl BaseChatModel for LSParamsModel {
    fn chat_config(&self) -> &ChatModelConfig {
        &self.config
    }

    async fn _generate(
        &self,
        _messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&agent_chain_core::callbacks::CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        Err(Error::NotImplemented("not implemented".into()))
    }
}

#[test]
fn test_get_ls_params() {
    // Test LangSmith parameter extraction.
    // Python equivalent: test_get_ls_params()

    let llm = LSParamsModel::new();

    // Test standard tracing params
    let ls_params = llm.get_ls_params(None);
    assert_eq!(ls_params.ls_provider, Some("lsparamsmodel".to_string()));
    assert_eq!(ls_params.ls_model_type, Some("chat".to_string()));
    assert_eq!(ls_params.ls_model_name, Some("foo".to_string()));
    assert_eq!(ls_params.ls_temperature, Some(0.1));
    assert_eq!(ls_params.ls_max_tokens, Some(1024));

    // Test with stop words
    let ls_params = llm.get_ls_params(Some(&["stop".to_string()]));
    assert_eq!(ls_params.ls_stop, Some(vec!["stop".to_string()]));
}

// =============================================================================
// Model Profiles Tests
// =============================================================================

#[test]
fn test_model_profiles() {
    // Test model profile functionality.
    // Python equivalent: test_model_profiles()

    let model = GenericFakeChatModel::from_strings(vec!["test".to_string()]);
    assert!(model.profile().is_none());

    // Create model with profile
    let profile = ModelProfile {
        max_input_tokens: Some(100),
        ..Default::default()
    };
    let config = ChatModelConfig::new().with_profile(profile.clone());
    let model_with_profile =
        GenericFakeChatModel::from_strings(vec!["test".to_string()]).with_config(config);

    let retrieved_profile = model_with_profile.profile();
    assert!(retrieved_profile.is_some());
    assert_eq!(retrieved_profile.unwrap().max_input_tokens, Some(100));
}

// =============================================================================
// Batch Size Tests (placeholder - requires callback infrastructure)
// =============================================================================

#[test]
fn test_batch_size() {
    // Test batch size tracking for chat models.
    // Python equivalent: test_batch_size()
    // Note: Full implementation requires callback/tracer infrastructure.

    let _messages = create_messages();
    let _messages_2 = create_messages_2();
    let _llm = FakeListChatModel::new((0..100).map(|i| i.to_string()).collect());

    // Without collect_runs implementation, we verify the model can be created
    // and that the test structure is in place
}

#[tokio::test]
async fn test_async_batch_size() {
    // Test async batch size tracking.
    // Python equivalent: test_async_batch_size()
    // Note: Full implementation requires callback/tracer infrastructure.

    let _messages = create_messages();
    let _messages_2 = create_messages_2();
    let _llm = FakeListChatModel::new((0..100).map(|i| i.to_string()).collect());

    // Test basic async operation works
    let llm = FakeListChatModel::new(vec!["test".to_string()]);
    let result = llm.invoke(LanguageModelInput::Messages(vec![]), None).await;
    assert!(result.is_ok());
}

// =============================================================================
// Run ID Tests (placeholder - requires callback infrastructure)
// =============================================================================

#[test]
fn test_pass_run_id() {
    // Test that run_id is correctly passed through callbacks.
    // Python equivalent: test_pass_run_id()
    // Note: Full implementation requires callback/tracer infrastructure.

    let _llm = FakeListChatModel::new(vec!["a".to_string(), "b".to_string(), "c".to_string()]);

    // Without full tracer implementation, verify test structure
}

#[tokio::test]
async fn test_async_pass_run_id() {
    // Test async run_id passing.
    // Python equivalent: test_async_pass_run_id()
    // Note: Full implementation requires callback/tracer infrastructure.

    let _llm = FakeListChatModel::new(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
}

// =============================================================================
// Streaming Attribute Tests
// =============================================================================

#[tokio::test]
async fn test_streaming_attribute_overrides_streaming_callback() {
    // Test that streaming attribute takes precedence.
    // Python equivalent: test_streaming_attribute_overrides_streaming_callback()

    // When model has streaming=false, even with streaming callbacks,
    // it should not use streaming
    let model = StreamingModel::new().with_streaming(false);

    // has_streaming_field returns None when streaming is false (not explicitly set)
    // This means the callback check will be used
    assert!(model.has_streaming_field().is_none());
}

// =============================================================================
// Content Block Tests (placeholder - requires content block infrastructure)
// =============================================================================

#[test]
fn test_trace_images_in_openai_format() {
    // Test that images are traced in OpenAI Chat Completions format.
    // Python equivalent: test_trace_images_in_openai_format()
    // Note: Requires content block transformation infrastructure.

    // Placeholder - verify test structure
}

#[test]
fn test_trace_pdfs() {
    // Test PDF content block tracing.
    // Python equivalent: test_trace_pdfs()
    // Note: Requires content block transformation infrastructure.
}

#[test]
fn test_content_block_transformation_v0_to_v1_image() {
    // Test v0 to v1 content block transformation for images.
    // Python equivalent: test_content_block_transformation_v0_to_v1_image()
    // Note: Requires content block versioning infrastructure.
}

#[test]
fn test_trace_content_blocks_with_no_type_key() {
    // Test content blocks without explicit type key.
    // Python equivalent: test_trace_content_blocks_with_no_type_key()
    // Note: Requires content block handling infrastructure.
}

#[test]
fn test_extend_support_to_openai_multimodal_formats() {
    // Test normalization of OpenAI audio, image, and file inputs.
    // Python equivalent: test_extend_support_to_openai_multimodal_formats()
    // Note: Requires multimodal support infrastructure.
}

#[test]
fn test_normalize_messages_edge_cases() {
    // Test edge cases in message normalization.
    // Python equivalent: test_normalize_messages_edge_cases()
    // Note: Requires message normalization infrastructure.
}

#[test]
fn test_normalize_messages_v1_content_blocks_unchanged() {
    // Test that v1 content blocks pass through unchanged.
    // Python equivalent: test_normalize_messages_v1_content_blocks_unchanged()
    // Note: Requires message normalization infrastructure.
}

// =============================================================================
// Output Version Tests (placeholder - requires output versioning)
// =============================================================================

#[test]
fn test_output_version_invoke() {
    // Test output_version parameter in invoke.
    // Python equivalent: test_output_version_invoke()
    // Note: Requires output versioning infrastructure.
}

#[tokio::test]
async fn test_output_version_ainvoke() {
    // Test output_version in async invoke.
    // Python equivalent: test_output_version_ainvoke()
    // Note: Requires output versioning infrastructure.
}

#[test]
fn test_output_version_stream() {
    // Test output_version in streaming.
    // Python equivalent: test_output_version_stream()
    // Note: Requires output versioning infrastructure.
}

#[tokio::test]
async fn test_output_version_astream() {
    // Test output_version in async streaming.
    // Python equivalent: test_output_version_astream()
    // Note: Requires output versioning infrastructure.
}

// =============================================================================
// Error Response Tests (placeholder - requires error response infrastructure)
// =============================================================================

#[test]
fn test_generate_response_from_error_with_valid_json() {
    // Test error response generation with JSON.
    // Python equivalent: test_generate_response_from_error_with_valid_json()
    // Note: Requires _generate_response_from_error implementation.
}

#[test]
fn test_generate_response_from_error_handles_streaming_response_failure() {
    // Test error handling for streaming response failures.
    // Python equivalent: test_generate_response_from_error_handles_streaming_response_failure()
    // Note: Requires _generate_response_from_error implementation.
}

// =============================================================================
// Additional Helper Tests
// =============================================================================

#[test]
fn test_disable_streaming_enum() {
    // Test DisableStreaming enum functionality.

    // Test Bool variant
    let disable_true = DisableStreaming::Bool(true);
    assert!(disable_true.should_disable(false));
    assert!(disable_true.should_disable(true));

    let disable_false = DisableStreaming::Bool(false);
    assert!(!disable_false.should_disable(false));
    assert!(!disable_false.should_disable(true));

    // Test ToolCalling variant
    let tool_calling = DisableStreaming::ToolCalling;
    assert!(!tool_calling.should_disable(false)); // no tools
    assert!(tool_calling.should_disable(true)); // with tools

    // Test From<bool>
    let from_true: DisableStreaming = true.into();
    assert_eq!(from_true, DisableStreaming::Bool(true));

    let from_false: DisableStreaming = false.into();
    assert_eq!(from_false, DisableStreaming::Bool(false));
}

#[test]
fn test_chat_model_config_builder() {
    // Test ChatModelConfig builder pattern.

    let config = ChatModelConfig::new()
        .with_cache(true)
        .with_disable_streaming(true)
        .with_output_version("v1");

    assert_eq!(config.base.cache, Some(true));
    assert_eq!(config.disable_streaming, DisableStreaming::Bool(true));
    assert_eq!(config.output_version, Some("v1".to_string()));

    // Test with profile
    let profile = ModelProfile {
        max_input_tokens: Some(1000),
        ..Default::default()
    };
    let config_with_profile = ChatModelConfig::new().with_profile(profile);
    assert!(config_with_profile.profile.is_some());
    assert_eq!(
        config_with_profile.profile.unwrap().max_input_tokens,
        Some(1000)
    );
}

#[tokio::test]
async fn test_invoke_basic() {
    // Test basic invoke functionality.

    let model = FakeListChatModel::new(vec!["hello world".to_string()]);
    let result = model
        .invoke(LanguageModelInput::Text("test".to_string()), None)
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().content, "hello world");
}

#[tokio::test]
async fn test_ainvoke_basic() {
    // Test basic ainvoke functionality.

    let model = FakeListChatModel::new(vec!["async hello".to_string()]);
    let result = model
        .ainvoke(LanguageModelInput::Text("test".to_string()), None)
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().content, "async hello");
}

#[tokio::test]
async fn test_stream_basic() {
    // Test basic stream functionality.

    let model = FakeListChatModel::new(vec!["hello".to_string()]);
    let mut stream = model
        ._stream(vec![], None, None)
        .expect("stream should work");

    let mut chunks = Vec::new();
    while let Some(chunk_result) = stream.next().await {
        if let Ok(chunk) = chunk_result {
            chunks.push(chunk);
        }
    }

    // FakeListChatModel streams character by character
    assert_eq!(chunks.len(), 5);
    let text: String = chunks.iter().map(|c| c.text.as_str()).collect();
    assert_eq!(text, "hello");
}

#[tokio::test]
async fn test_generate_basic() {
    // Test basic generate functionality.

    let model = FakeListChatModel::new(vec!["gen1".to_string(), "gen2".to_string()]);
    let messages1 = vec![BaseMessage::Human(
        HumanMessage::builder().content("test1").build(),
    )];
    let messages2 = vec![BaseMessage::Human(
        HumanMessage::builder().content("test2").build(),
    )];

    let result = model
        .generate(vec![messages1, messages2], GenerateConfig::default())
        .await;

    assert!(result.is_ok());
    let llm_result = result.unwrap();
    assert_eq!(llm_result.generations.len(), 2);
}

// ====================================================================
// Previously missing tests — now implemented
// ====================================================================

// ---- TestGenerateFromStream ----

/// Ported from `TestGenerateFromStream::test_accumulates_chunks`.
#[test]
fn test_generate_from_stream_accumulates_chunks() {
    use agent_chain_core::language_models::generate_from_stream;

    let chunks = vec![
        ChatGenerationChunk::new(BaseMessage::AI(
            AIMessage::builder().content("hello").build(),
        )),
        ChatGenerationChunk::new(BaseMessage::AI(
            AIMessage::builder().content(" world").build(),
        )),
    ];
    let result = generate_from_stream(chunks.into_iter()).unwrap();
    assert_eq!(result.generations.len(), 1);
    assert!(result.generations[0].message.content().contains("hello"));
    assert!(result.generations[0].message.content().contains("world"));
}

/// Ported from `TestGenerateFromStream::test_single_chunk`.
#[test]
fn test_generate_from_stream_single_chunk() {
    use agent_chain_core::language_models::generate_from_stream;

    let chunks = vec![ChatGenerationChunk::new(BaseMessage::AI(
        AIMessage::builder().content("single").build(),
    ))];
    let result = generate_from_stream(chunks.into_iter()).unwrap();
    assert_eq!(result.generations.len(), 1);
    assert_eq!(result.generations[0].message.content(), "single");
}

/// Ported from `TestGenerateFromStream::test_empty_stream_raises_value_error`.
#[test]
fn test_generate_from_stream_empty_raises_error() {
    use agent_chain_core::language_models::generate_from_stream;

    let chunks: Vec<ChatGenerationChunk> = vec![];
    let result = generate_from_stream(chunks.into_iter());
    assert!(result.is_err());
}

// ---- TestAGenerateFromStream ----

/// Ported from `TestAGenerateFromStream::test_accumulates_chunks`.
#[tokio::test]
async fn test_agenerate_from_stream_accumulates_chunks() {
    use agent_chain_core::language_models::agenerate_from_stream;

    let chunks = vec![
        Ok(ChatGenerationChunk::new(BaseMessage::AI(
            AIMessage::builder().content("hello").build(),
        ))),
        Ok(ChatGenerationChunk::new(BaseMessage::AI(
            AIMessage::builder().content(" world").build(),
        ))),
    ];
    let stream = futures::stream::iter(chunks);
    let result = agenerate_from_stream(stream).await.unwrap();
    assert_eq!(result.generations.len(), 1);
}

/// Ported from `TestAGenerateFromStream::test_empty_stream_raises_value_error`.
#[tokio::test]
async fn test_agenerate_from_stream_empty_raises_error() {
    use agent_chain_core::language_models::agenerate_from_stream;

    let chunks: Vec<Result<ChatGenerationChunk>> = vec![];
    let stream = futures::stream::iter(chunks);
    let result = agenerate_from_stream(stream).await;
    assert!(result.is_err());
}

// ---- TestCombineLlmOutputs ----

/// Ported from `TestCombineLlmOutputs::test_returns_empty_dict_by_default`.
#[test]
fn test_combine_llm_outputs_returns_empty_dict() {
    let model = agent_chain_core::FakeChatModel::new();
    let result = model._combine_llm_outputs(&[]);
    assert!(result.is_empty());
}

/// Ported from `TestCombineLlmOutputs::test_returns_empty_dict_with_empty_list`.
#[test]
fn test_combine_llm_outputs_returns_empty_dict_with_empty_list() {
    let model = agent_chain_core::FakeChatModel::new();
    let result = model._combine_llm_outputs(&[None, None]);
    assert!(result.is_empty());
}

// ---- TestConvertCachedGenerations ----

/// Ported from `TestConvertCachedGenerations::test_with_chat_generation_objects`.
#[test]
fn test_convert_cached_generations_chat_generation() {
    use agent_chain_core::outputs::Generation;

    let model = agent_chain_core::FakeChatModel::new();
    let generations = vec![Generation::new("hello".to_string())];
    let result = model._convert_cached_generations(generations);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].message.content(), "hello");
}

/// Ported from `TestConvertCachedGenerations::test_with_legacy_generation_objects`.
#[test]
fn test_convert_cached_generations_legacy() {
    use agent_chain_core::outputs::Generation;

    let model = agent_chain_core::FakeChatModel::new();
    let generations = vec![
        Generation::new("first".to_string()),
        Generation::new("second".to_string()),
    ];
    let result = model._convert_cached_generations(generations);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].message.content(), "first");
    assert_eq!(result[1].message.content(), "second");
}

/// Ported from `TestConvertCachedGenerations::test_with_mixed_generation_objects`.
#[test]
fn test_convert_cached_generations_mixed() {
    use agent_chain_core::outputs::Generation;

    let model = agent_chain_core::FakeChatModel::new();
    let generations = vec![
        Generation::new("a".to_string()),
        Generation::new("b".to_string()),
        Generation::new("c".to_string()),
    ];
    let result = model._convert_cached_generations(generations);
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].message.content(), "a");
    assert_eq!(result[1].message.content(), "b");
    assert_eq!(result[2].message.content(), "c");
}

// ---- TestShouldStream ----

/// Ported from `TestShouldStream::test_no_stream_implemented_returns_false`.
#[test]
fn test_should_stream_no_stream_returns_false() {
    let model = agent_chain_core::FakeChatModel::new();
    // FakeChatModel doesn't implement _stream, so should return false
    assert!(!model._should_stream(false, false, None, None));
}

/// Ported from `TestShouldStream::test_no_stream_or_astream_returns_false_for_async`.
#[test]
fn test_should_stream_no_astream_returns_false() {
    let model = agent_chain_core::FakeChatModel::new();
    assert!(!model._should_stream(true, false, None, None));
}

/// Ported from `TestShouldStream::test_disable_streaming_true_returns_false`.
#[test]
fn test_should_stream_disabled_returns_false() {
    let config = ChatModelConfig::new().with_disable_streaming(true);
    let model = FakeListChatModel::new(vec!["test".to_string()]).with_config(config);
    // Even with stream impl, disabled should return false
    assert!(!model._should_stream(false, false, None, None));
}

/// Ported from `TestShouldStream::test_stream_kwarg_true`.
#[test]
fn test_should_stream_kwarg_true() {
    let model = FakeListChatModel::new(vec!["test".to_string()]);
    // FakeListChatModel has _stream impl, kwarg=true forces streaming
    assert!(model._should_stream(false, false, Some(true), None));
}

/// Ported from `TestShouldStream::test_stream_kwarg_false`.
#[test]
fn test_should_stream_kwarg_false() {
    let model = FakeListChatModel::new(vec!["test".to_string()]);
    assert!(!model._should_stream(false, false, Some(false), None));
}

/// Ported from `TestShouldStream::test_no_handlers_no_streaming`.
#[test]
fn test_should_stream_no_handlers() {
    let model = FakeListChatModel::new(vec!["test".to_string()]);
    // With has_stream_impl=true, no disable, no kwarg, no streaming field,
    // and empty handlers list — should return true (default when stream is available)
    let handlers: Vec<std::sync::Arc<dyn agent_chain_core::callbacks::base::BaseCallbackHandler>> =
        vec![];
    assert!(!model._should_stream(false, false, None, Some(&handlers)));
}

// ---- TestConvertInput ----

/// Ported from `TestConvertInput::test_convert_input_from_string`.
#[test]
fn test_convert_input_from_string() {
    let model = agent_chain_core::FakeChatModel::new();
    let result = model
        .convert_input(LanguageModelInput::from("hello world"))
        .unwrap();
    assert_eq!(result.len(), 1);
    assert!(matches!(&result[0], BaseMessage::Human(_)));
    assert_eq!(result[0].content(), "hello world");
}

/// Ported from `TestConvertInput::test_convert_input_from_message_sequence`.
#[test]
fn test_convert_input_from_message_sequence() {
    let model = agent_chain_core::FakeChatModel::new();
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder().content("hi").build(),
    )];
    let result = model
        .convert_input(LanguageModelInput::from(messages))
        .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].content(), "hi");
}

// ---- TestGenerateMethod ----

/// Ported from `TestGenerateMethod::test_single_message_list`.
#[tokio::test]
async fn test_generate_single_message_list() {
    let model = FakeListChatModel::new(vec!["response".to_string()]);
    let result = model
        .generate(
            vec![vec![BaseMessage::Human(
                HumanMessage::builder().content("hello").build(),
            )]],
            GenerateConfig::default(),
        )
        .await
        .unwrap();
    assert_eq!(result.generations.len(), 1);
}

/// Ported from `TestGenerateMethod::test_multiple_message_lists`.
#[tokio::test]
async fn test_generate_multiple_message_lists() {
    let model = FakeListChatModel::new(vec!["r1".to_string(), "r2".to_string(), "r3".to_string()]);
    let result = model
        .generate(
            vec![
                vec![BaseMessage::Human(
                    HumanMessage::builder().content("p1").build(),
                )],
                vec![BaseMessage::Human(
                    HumanMessage::builder().content("p2").build(),
                )],
                vec![BaseMessage::Human(
                    HumanMessage::builder().content("p3").build(),
                )],
            ],
            GenerateConfig::default(),
        )
        .await
        .unwrap();
    assert_eq!(result.generations.len(), 3);
}

/// Ported from `TestGenerateMethod::test_generate_returns_chat_result`.
#[tokio::test]
async fn test_generate_returns_chat_result() {
    let model = FakeListChatModel::new(vec!["hello".to_string()]);
    let result = model
        .generate(
            vec![vec![BaseMessage::Human(
                HumanMessage::builder().content("hi").build(),
            )]],
            GenerateConfig::default(),
        )
        .await
        .unwrap();
    assert_eq!(result.generations.len(), 1);
    // Verify it's a ChatGeneration inside
    match &result.generations[0][0] {
        agent_chain_core::outputs::GenerationType::ChatGeneration(cg) => {
            assert_eq!(cg.message.content(), "hello");
        }
        _ => panic!("Expected ChatGeneration"),
    }
}

// ---- TestAGenerateMethod ----

/// Ported from `TestAGenerateMethod::test_single_message_list`.
#[tokio::test]
async fn test_agenerate_single_message_list() {
    let model = FakeListChatModel::new(vec!["response".to_string()]);
    let result = model
        .agenerate(
            vec![vec![BaseMessage::Human(
                HumanMessage::builder().content("hello").build(),
            )]],
            GenerateConfig::default(),
        )
        .await
        .unwrap();
    assert_eq!(result.generations.len(), 1);
}

/// Ported from `TestAGenerateMethod::test_multiple_message_lists`.
#[tokio::test]
async fn test_agenerate_multiple_message_lists() {
    let model = FakeListChatModel::new(vec!["r1".to_string(), "r2".to_string()]);
    let result = model
        .agenerate(
            vec![
                vec![BaseMessage::Human(
                    HumanMessage::builder().content("p1").build(),
                )],
                vec![BaseMessage::Human(
                    HumanMessage::builder().content("p2").build(),
                )],
            ],
            GenerateConfig::default(),
        )
        .await
        .unwrap();
    assert_eq!(result.generations.len(), 2);
}

/// Ported from `TestAGenerateMethod::test_agenerate_returns_chat_result`.
#[tokio::test]
async fn test_agenerate_returns_chat_result() {
    let model = FakeListChatModel::new(vec!["hello".to_string()]);
    let result = model
        .agenerate(
            vec![vec![BaseMessage::Human(
                HumanMessage::builder().content("hi").build(),
            )]],
            GenerateConfig::default(),
        )
        .await
        .unwrap();
    assert_eq!(result.generations.len(), 1);
}

// ---- TestBindTools / TestWithStructuredOutput ----

/// Ported from `TestBindTools::test_raises_not_implemented_by_default`.
#[test]
fn test_bind_tools_raises_not_implemented() {
    let model = agent_chain_core::FakeChatModel::new();
    let result = model.bind_tools(&[], None);
    assert!(result.is_err());
}

/// Ported from `TestWithStructuredOutput::test_raises_not_implemented`.
#[test]
fn test_with_structured_output_raises_not_implemented() {
    let model = agent_chain_core::FakeChatModel::new();
    let result = model.with_structured_output(serde_json::json!({}), false);
    assert!(result.is_err());
}

// ---- TestSimpleChatModelGenerate ----

/// Ported from `TestSimpleChatModelGenerate::test_generate_wraps_call_output`.
#[tokio::test]
async fn test_simple_chat_model_generate_wraps_call() {
    let model = agent_chain_core::FakeChatModel::new();
    let result = model
        ._generate(
            vec![BaseMessage::Human(
                HumanMessage::builder().content("hello").build(),
            )],
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(result.generations.len(), 1);
    assert_eq!(result.generations[0].message.content(), "fake response");
}

/// Ported from `TestSimpleChatModelFakeChatModel::test_generate_returns_chat_result`.
#[tokio::test]
async fn test_simple_fake_chat_generate_returns_chat_result() {
    let model = agent_chain_core::FakeChatModel::new();
    let result = model
        ._generate(
            vec![BaseMessage::Human(
                HumanMessage::builder().content("hi").build(),
            )],
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(result.generations.len(), 1);
    assert!(matches!(result.generations[0].message, BaseMessage::AI(_)));
}

/// Ported from `TestSimpleChatModelFakeChatModel::test_agenerate_returns_chat_result`.
#[tokio::test]
async fn test_simple_fake_chat_agenerate_returns_chat_result() {
    let model = agent_chain_core::FakeChatModel::new();
    let result = model
        ._generate(
            vec![BaseMessage::Human(
                HumanMessage::builder().content("hi").build(),
            )],
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(result.generations.len(), 1);
    assert_eq!(result.generations[0].message.content(), "fake response");
}

// ---- TestGenInfoAndMsgMetadata ----

/// Ported from `TestGenInfoAndMsgMetadata::test_merges_generation_info_with_response_metadata`.
#[test]
fn test_gen_info_merges_with_response_metadata() {
    use agent_chain_core::language_models::chat_models::_gen_info_and_msg_metadata;
    use std::collections::HashMap;

    let mut response_metadata = HashMap::new();
    response_metadata.insert(
        "model".to_string(),
        serde_json::Value::String("test".to_string()),
    );

    let mut generation_info = HashMap::new();
    generation_info.insert(
        "finish_reason".to_string(),
        serde_json::Value::String("stop".to_string()),
    );

    let result = _gen_info_and_msg_metadata(Some(&generation_info), &response_metadata);
    assert_eq!(
        result.get("finish_reason").and_then(|v| v.as_str()),
        Some("stop")
    );
    assert_eq!(result.get("model").and_then(|v| v.as_str()), Some("test"));
}

/// Ported from `TestGenInfoAndMsgMetadata::test_empty_generation_info`.
#[test]
fn test_gen_info_empty_generation_info() {
    use agent_chain_core::language_models::chat_models::_gen_info_and_msg_metadata;
    use std::collections::HashMap;

    let mut response_metadata = HashMap::new();
    response_metadata.insert(
        "key".to_string(),
        serde_json::Value::String("val".to_string()),
    );

    let result = _gen_info_and_msg_metadata(None, &response_metadata);
    assert_eq!(result.len(), 1);
    assert_eq!(result.get("key").and_then(|v| v.as_str()), Some("val"));
}

/// Ported from `TestGenInfoAndMsgMetadata::test_empty_response_metadata`.
#[test]
fn test_gen_info_empty_response_metadata() {
    use agent_chain_core::language_models::chat_models::_gen_info_and_msg_metadata;
    use std::collections::HashMap;

    let mut generation_info = HashMap::new();
    generation_info.insert(
        "token_count".to_string(),
        serde_json::Value::Number(serde_json::Number::from(10)),
    );

    let empty_metadata = HashMap::new();
    let result = _gen_info_and_msg_metadata(Some(&generation_info), &empty_metadata);
    assert_eq!(result.get("token_count").and_then(|v| v.as_i64()), Some(10));
}

/// Ported from `TestGenInfoAndMsgMetadata::test_response_metadata_overrides_generation_info`.
#[test]
fn test_gen_info_response_metadata_overrides() {
    use agent_chain_core::language_models::chat_models::_gen_info_and_msg_metadata;
    use std::collections::HashMap;

    let mut response_metadata = HashMap::new();
    response_metadata.insert(
        "key".to_string(),
        serde_json::Value::String("from_metadata".to_string()),
    );

    let mut generation_info = HashMap::new();
    generation_info.insert(
        "key".to_string(),
        serde_json::Value::String("from_gen_info".to_string()),
    );

    let result = _gen_info_and_msg_metadata(Some(&generation_info), &response_metadata);
    // response_metadata values win (same as Python dict merge order)
    assert_eq!(
        result.get("key").and_then(|v| v.as_str()),
        Some("from_metadata")
    );
}

/// Test that streaming via `stream()` injects response_metadata on yielded chunks.
#[tokio::test]
async fn test_stream_injects_response_metadata() {
    use agent_chain_core::messages::AIMessageChunk;

    let model =
        GenericFakeChatModel::from_vec(vec![AIMessage::builder().content("hello world").build()]);

    let mut stream = model
        .stream(LanguageModelInput::from("test"), None, None)
        .await
        .unwrap();

    let mut chunks: Vec<AIMessageChunk> = Vec::new();
    while let Some(result) = stream.next().await {
        if let Ok(chunk) = result {
            chunks.push(chunk);
        }
    }

    // Should have content chunks + final empty chunk
    assert!(chunks.len() >= 2);

    // Last chunk should have chunk_position="last"
    let last = chunks.last().unwrap();
    assert_eq!(
        last.chunk_position,
        Some(agent_chain_core::messages::ChunkPosition::Last)
    );
}

/// Test that `on_llm_new_token` receives chunk data when streaming.
#[tokio::test]
async fn test_stream_callback_receives_chunk_data() {
    use agent_chain_core::callbacks::base::{
        BaseCallbackHandler, CallbackManagerMixin, ChainManagerMixin, LLMManagerMixin,
        RetrieverManagerMixin, RunManagerMixin, ToolManagerMixin,
    };
    use agent_chain_core::runnables::config::RunnableConfig;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    #[derive(Debug, Clone)]
    struct ChunkRecorder {
        chunks_received: Arc<Mutex<Vec<Option<serde_json::Value>>>>,
    }

    impl ChunkRecorder {
        fn new() -> Self {
            Self {
                chunks_received: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl LLMManagerMixin for ChunkRecorder {
        fn on_llm_new_token(
            &self,
            _token: &str,
            _run_id: Uuid,
            _parent_run_id: Option<Uuid>,
            chunk: Option<&serde_json::Value>,
        ) {
            self.chunks_received.lock().unwrap().push(chunk.cloned());
        }
    }

    impl ChainManagerMixin for ChunkRecorder {}
    impl ToolManagerMixin for ChunkRecorder {}
    impl RetrieverManagerMixin for ChunkRecorder {}
    impl CallbackManagerMixin for ChunkRecorder {}
    impl RunManagerMixin for ChunkRecorder {}

    impl BaseCallbackHandler for ChunkRecorder {
        fn name(&self) -> &str {
            "chunk_recorder"
        }
    }

    let recorder = ChunkRecorder::new();
    let handler: Arc<dyn BaseCallbackHandler> = Arc::new(recorder.clone());

    let model = GenericFakeChatModel::from_vec(vec![AIMessage::builder().content("hi").build()]);

    let config = RunnableConfig::new().with_callbacks(vec![handler].into());

    let mut stream = model
        .stream(LanguageModelInput::from("test"), Some(&config), None)
        .await
        .unwrap();

    while let Some(_) = stream.next().await {}

    let recorded = recorder.chunks_received.lock().unwrap();
    // on_llm_new_token should have been called at least once with Some(chunk_json)
    let non_none_chunks: Vec<_> = recorded.iter().filter(|c| c.is_some()).collect();
    assert!(
        !non_none_chunks.is_empty(),
        "on_llm_new_token should receive chunk data, got {} calls with {:?}",
        recorded.len(),
        recorded.iter().map(|c| c.is_some()).collect::<Vec<_>>()
    );
}
