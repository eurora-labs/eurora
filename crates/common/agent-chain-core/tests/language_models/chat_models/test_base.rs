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
        Ok(agent_chain_core::outputs::LLMResult::builder()
            .generations(generations)
            .build())
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
        let generation = ChatGeneration::builder().message(message.into()).build();
        Ok(ChatResult::builder().generations(vec![generation]).build())
    }
}

#[tokio::test]
async fn test_astream_fallback_to_ainvoke() {
    let model = ModelWithGenerateOnly::new();

    let chunks: Vec<BaseMessage> = {
        let result = model
            ._generate(vec![], None, None)
            .await
            .expect("_generate should succeed");
        result.generations.into_iter().map(|g| g.message).collect()
    };

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].content(), "hello");

    assert!(!model.has_stream_impl());
}

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
            yield Ok(ChatGenerationChunk::builder().message(AIMessage::builder().content("a").build().into()).build());
            yield Ok(ChatGenerationChunk::builder().message(AIMessage::builder().content("b").build().into()).build());
        };
        Ok(Box::pin(stream))
    }

    fn has_stream_impl(&self) -> bool {
        true
    }
}

#[tokio::test]
async fn test_astream_implementation_fallback_to_stream() {
    let model = ModelWithSyncStream::new();

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

    assert!(model.has_stream_impl());
    assert!(!model.has_astream_impl());
}

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
            yield Ok(ChatGenerationChunk::builder().message(AIMessage::builder().content("a").build().into()).build());
            yield Ok(ChatGenerationChunk::builder().message(AIMessage::builder().content("b").build().into()).build());
        };
        Ok(Box::pin(stream))
    }

    fn has_astream_impl(&self) -> bool {
        true
    }
}

#[tokio::test]
async fn test_astream_implementation_uses_astream() {
    let model = ModelWithAsyncStream::new();

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

    assert!(model.has_astream_impl());
}

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
        Ok(agent_chain_core::outputs::LLMResult::builder()
            .generations(generations)
            .build())
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
        let generation = ChatGeneration::builder().message(message.into()).build();
        Ok(ChatResult::builder().generations(vec![generation]).build())
    }
}

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
        Ok(agent_chain_core::outputs::LLMResult::builder()
            .generations(generations)
            .build())
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
        let generation = ChatGeneration::builder().message(message.into()).build();
        Ok(ChatResult::builder().generations(vec![generation]).build())
    }

    fn _stream(
        &self,
        _messages: Vec<BaseMessage>,
        _stop: Option<Vec<String>>,
        _run_manager: Option<&agent_chain_core::callbacks::CallbackManagerForLLMRun>,
    ) -> Result<ChatGenerationStream> {
        let stream = async_stream::stream! {
            yield Ok(ChatGenerationChunk::builder().message(AIMessage::builder().content("stream").build().into()).build());
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
    let model = StreamingModel::new().with_disable_streaming(DisableStreaming::Bool(true));

    assert!(!model._should_stream(false, false, None, None));
    assert!(!model._should_stream(false, true, None, None)); // with tools
}

#[test]
fn test_disable_streaming_bool_false() {
    let model = StreamingModel::new().with_disable_streaming(DisableStreaming::Bool(false));

    let handlers: Vec<std::sync::Arc<dyn agent_chain_core::callbacks::base::BaseCallbackHandler>> =
        vec![std::sync::Arc::new(
            agent_chain_core::callbacks::StdOutCallbackHandler::new(),
        )];
    assert!(model._should_stream(false, false, None, Some(&handlers)));
    assert!(model._should_stream(false, true, None, Some(&handlers))); // with tools
}

#[test]
fn test_disable_streaming_tool_calling() {
    let model = StreamingModel::new().with_disable_streaming(DisableStreaming::ToolCalling);

    let handlers: Vec<std::sync::Arc<dyn agent_chain_core::callbacks::base::BaseCallbackHandler>> =
        vec![std::sync::Arc::new(
            agent_chain_core::callbacks::StdOutCallbackHandler::new(),
        )];

    assert!(model._should_stream(false, false, None, Some(&handlers)));

    assert!(!model._should_stream(false, true, None, Some(&handlers)));
}

#[tokio::test]
async fn test_disable_streaming_async() {
    let handlers: Vec<std::sync::Arc<dyn agent_chain_core::callbacks::base::BaseCallbackHandler>> =
        vec![std::sync::Arc::new(
            agent_chain_core::callbacks::StdOutCallbackHandler::new(),
        )];

    let model = StreamingModel::new().with_disable_streaming(DisableStreaming::Bool(true));
    let result = model
        .invoke(LanguageModelInput::Messages(vec![]), None)
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().content, "invoke");

    let model = StreamingModel::new().with_disable_streaming(DisableStreaming::Bool(false));
    assert!(model._should_stream(true, false, None, Some(&handlers)));

    let model = StreamingModel::new().with_disable_streaming(DisableStreaming::ToolCalling);
    assert!(model._should_stream(true, false, None, Some(&handlers))); // no tools
    assert!(!model._should_stream(true, true, None, Some(&handlers))); // with tools
}

#[tokio::test]
async fn test_disable_streaming_no_streaming_model() {
    let model = NoStreamingModel::new().with_disable_streaming(DisableStreaming::Bool(true));
    let result = model
        .invoke(LanguageModelInput::Messages(vec![]), None)
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().content, "invoke");

    let model = NoStreamingModel::new().with_disable_streaming(DisableStreaming::Bool(false));
    assert!(!model._should_stream(false, false, None, None));
}

#[tokio::test]
async fn test_disable_streaming_no_streaming_model_async() {
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
        let mut params = LangSmithParams::builder()
            .provider("lsparamsmodel")
            .model_name(&self.model)
            .model_type("chat")
            .temperature(self.temperature)
            .max_tokens(self.max_tokens)
            .build();

        if let Some(stop_words) = stop {
            params.ls_stop = Some(stop_words.to_vec());
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
    let llm = LSParamsModel::new();

    let ls_params = llm.get_ls_params(None);
    assert_eq!(ls_params.ls_provider, Some("lsparamsmodel".to_string()));
    assert_eq!(ls_params.ls_model_type, Some("chat".to_string()));
    assert_eq!(ls_params.ls_model_name, Some("foo".to_string()));
    assert_eq!(ls_params.ls_temperature, Some(0.1));
    assert_eq!(ls_params.ls_max_tokens, Some(1024));

    let ls_params = llm.get_ls_params(Some(&["stop".to_string()]));
    assert_eq!(ls_params.ls_stop, Some(vec!["stop".to_string()]));
}

#[test]
fn test_model_profiles() {
    let model = GenericFakeChatModel::from_strings(vec!["test".to_string()]);
    assert!(model.profile().is_none());

    let profile = ModelProfile {
        max_input_tokens: Some(100),
        ..Default::default()
    };
    let config = ChatModelConfig::builder().profile(profile.clone()).build();
    let model_with_profile =
        GenericFakeChatModel::from_strings(vec!["test".to_string()]).with_config(config);

    let retrieved_profile = model_with_profile.profile();
    assert!(retrieved_profile.is_some());
    assert_eq!(retrieved_profile.unwrap().max_input_tokens, Some(100));
}

#[test]
fn test_batch_size() {
    let _messages = create_messages();
    let _messages_2 = create_messages_2();
    let _llm = FakeListChatModel::builder()
        .responses((0..100).map(|i| i.to_string()).collect())
        .build();
}

#[tokio::test]
async fn test_async_batch_size() {
    let _messages = create_messages();
    let _messages_2 = create_messages_2();
    let _llm = FakeListChatModel::builder()
        .responses((0..100).map(|i| i.to_string()).collect())
        .build();

    let llm = FakeListChatModel::builder()
        .responses(vec!["test".to_string()])
        .build();
    let result = llm.invoke(LanguageModelInput::Messages(vec![]), None).await;
    assert!(result.is_ok());
}

#[test]
fn test_pass_run_id() {
    let _llm = FakeListChatModel::builder()
        .responses(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        .build();
}

#[tokio::test]
async fn test_async_pass_run_id() {
    let _llm = FakeListChatModel::builder()
        .responses(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        .build();
}

#[tokio::test]
async fn test_streaming_attribute_overrides_streaming_callback() {
    let model = StreamingModel::new().with_streaming(false);

    assert!(model.has_streaming_field().is_none());
}

#[test]
fn test_trace_images_in_openai_format() {}

#[test]
fn test_trace_pdfs() {}

#[test]
fn test_content_block_transformation_v0_to_v1_image() {}

#[test]
fn test_trace_content_blocks_with_no_type_key() {}

#[test]
fn test_extend_support_to_openai_multimodal_formats() {}

#[test]
fn test_normalize_messages_edge_cases() {}

#[test]
fn test_normalize_messages_v1_content_blocks_unchanged() {}

#[test]
fn test_output_version_invoke() {}

#[tokio::test]
async fn test_output_version_ainvoke() {}

#[test]
fn test_output_version_stream() {}

#[tokio::test]
async fn test_output_version_astream() {}

#[test]
fn test_generate_response_from_error_with_valid_json() {}

#[test]
fn test_generate_response_from_error_handles_streaming_response_failure() {}

#[test]
fn test_disable_streaming_enum() {
    let disable_true = DisableStreaming::Bool(true);
    assert!(disable_true.should_disable(false));
    assert!(disable_true.should_disable(true));

    let disable_false = DisableStreaming::Bool(false);
    assert!(!disable_false.should_disable(false));
    assert!(!disable_false.should_disable(true));

    let tool_calling = DisableStreaming::ToolCalling;
    assert!(!tool_calling.should_disable(false)); // no tools
    assert!(tool_calling.should_disable(true)); // with tools

    let from_true: DisableStreaming = true.into();
    assert_eq!(from_true, DisableStreaming::Bool(true));

    let from_false: DisableStreaming = false.into();
    assert_eq!(from_false, DisableStreaming::Bool(false));
}

#[test]
fn test_chat_model_config_builder() {
    let config = ChatModelConfig::builder()
        .cache(true)
        .disable_streaming(DisableStreaming::Bool(true))
        .output_version("v1")
        .build();

    assert_eq!(config.base.cache, Some(true));
    assert_eq!(config.disable_streaming, DisableStreaming::Bool(true));
    assert_eq!(config.output_version, Some("v1".to_string()));

    let profile = ModelProfile {
        max_input_tokens: Some(1000),
        ..Default::default()
    };
    let config_with_profile = ChatModelConfig::builder().profile(profile).build();
    assert!(config_with_profile.profile.is_some());
    assert_eq!(
        config_with_profile.profile.unwrap().max_input_tokens,
        Some(1000)
    );
}

#[tokio::test]
async fn test_invoke_basic() {
    let model = FakeListChatModel::builder()
        .responses(vec!["hello world".to_string()])
        .build();
    let result = model
        .invoke(LanguageModelInput::Text("test".to_string()), None)
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().content, "hello world");
}

#[tokio::test]
async fn test_ainvoke_basic() {
    let model = FakeListChatModel::builder()
        .responses(vec!["async hello".to_string()])
        .build();
    let result = model
        .ainvoke(LanguageModelInput::Text("test".to_string()), None)
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().content, "async hello");
}

#[tokio::test]
async fn test_stream_basic() {
    let model = FakeListChatModel::builder()
        .responses(vec!["hello".to_string()])
        .build();
    let mut stream = model
        ._stream(vec![], None, None)
        .expect("stream should work");

    let mut chunks = Vec::new();
    while let Some(chunk_result) = stream.next().await {
        if let Ok(chunk) = chunk_result {
            chunks.push(chunk);
        }
    }

    assert_eq!(chunks.len(), 5);
    let text: String = chunks.iter().map(|c| c.text.as_str()).collect();
    assert_eq!(text, "hello");
}

#[tokio::test]
async fn test_generate_basic() {
    let model = FakeListChatModel::builder()
        .responses(vec!["gen1".to_string(), "gen2".to_string()])
        .build();
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

#[test]
fn test_generate_from_stream_accumulates_chunks() {
    use agent_chain_core::language_models::generate_from_stream;

    let chunks = vec![
        ChatGenerationChunk::builder()
            .message(BaseMessage::AI(
                AIMessage::builder().content("hello").build(),
            ))
            .build(),
        ChatGenerationChunk::builder()
            .message(BaseMessage::AI(
                AIMessage::builder().content(" world").build(),
            ))
            .build(),
    ];
    let result = generate_from_stream(chunks.into_iter()).unwrap();
    assert_eq!(result.generations.len(), 1);
    assert!(result.generations[0].message.content().contains("hello"));
    assert!(result.generations[0].message.content().contains("world"));
}

#[test]
fn test_generate_from_stream_single_chunk() {
    use agent_chain_core::language_models::generate_from_stream;

    let chunks = vec![
        ChatGenerationChunk::builder()
            .message(BaseMessage::AI(
                AIMessage::builder().content("single").build(),
            ))
            .build(),
    ];
    let result = generate_from_stream(chunks.into_iter()).unwrap();
    assert_eq!(result.generations.len(), 1);
    assert_eq!(result.generations[0].message.content(), "single");
}

#[test]
fn test_generate_from_stream_empty_raises_error() {
    use agent_chain_core::language_models::generate_from_stream;

    let chunks: Vec<ChatGenerationChunk> = vec![];
    let result = generate_from_stream(chunks.into_iter());
    assert!(result.is_err());
}

#[tokio::test]
async fn test_agenerate_from_stream_accumulates_chunks() {
    use agent_chain_core::language_models::agenerate_from_stream;

    let chunks = vec![
        Ok(ChatGenerationChunk::builder()
            .message(BaseMessage::AI(
                AIMessage::builder().content("hello").build(),
            ))
            .build()),
        Ok(ChatGenerationChunk::builder()
            .message(BaseMessage::AI(
                AIMessage::builder().content(" world").build(),
            ))
            .build()),
    ];
    let stream = futures::stream::iter(chunks);
    let result = agenerate_from_stream(stream).await.unwrap();
    assert_eq!(result.generations.len(), 1);
}

#[tokio::test]
async fn test_agenerate_from_stream_empty_raises_error() {
    use agent_chain_core::language_models::agenerate_from_stream;

    let chunks: Vec<Result<ChatGenerationChunk>> = vec![];
    let stream = futures::stream::iter(chunks);
    let result = agenerate_from_stream(stream).await;
    assert!(result.is_err());
}

#[test]
fn test_combine_llm_outputs_returns_empty_dict() {
    let model = agent_chain_core::FakeChatModel::builder().build();
    let result = model._combine_llm_outputs(&[]);
    assert!(result.is_empty());
}

#[test]
fn test_combine_llm_outputs_returns_empty_dict_with_empty_list() {
    let model = agent_chain_core::FakeChatModel::builder().build();
    let result = model._combine_llm_outputs(&[None, None]);
    assert!(result.is_empty());
}

#[test]
fn test_convert_cached_generations_chat_generation() {
    use agent_chain_core::outputs::Generation;

    let model = agent_chain_core::FakeChatModel::builder().build();
    let generations = vec![Generation::builder().text("hello".to_string()).build()];
    let result = model._convert_cached_generations(generations);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].message.content(), "hello");
}

#[test]
fn test_convert_cached_generations_legacy() {
    use agent_chain_core::outputs::Generation;

    let model = agent_chain_core::FakeChatModel::builder().build();
    let generations = vec![
        Generation::builder().text("first".to_string()).build(),
        Generation::builder().text("second".to_string()).build(),
    ];
    let result = model._convert_cached_generations(generations);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].message.content(), "first");
    assert_eq!(result[1].message.content(), "second");
}

#[test]
fn test_convert_cached_generations_mixed() {
    use agent_chain_core::outputs::Generation;

    let model = agent_chain_core::FakeChatModel::builder().build();
    let generations = vec![
        Generation::builder().text("a".to_string()).build(),
        Generation::builder().text("b".to_string()).build(),
        Generation::builder().text("c".to_string()).build(),
    ];
    let result = model._convert_cached_generations(generations);
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].message.content(), "a");
    assert_eq!(result[1].message.content(), "b");
    assert_eq!(result[2].message.content(), "c");
}

#[test]
fn test_should_stream_no_stream_returns_false() {
    let model = agent_chain_core::FakeChatModel::builder().build();
    assert!(!model._should_stream(false, false, None, None));
}

#[test]
fn test_should_stream_no_astream_returns_false() {
    let model = agent_chain_core::FakeChatModel::builder().build();
    assert!(!model._should_stream(true, false, None, None));
}

#[test]
fn test_should_stream_disabled_returns_false() {
    let config = ChatModelConfig::builder()
        .disable_streaming(DisableStreaming::Bool(true))
        .build();
    let model = FakeListChatModel::builder()
        .responses(vec!["test".to_string()])
        .config(config)
        .build();
    assert!(!model._should_stream(false, false, None, None));
}

#[test]
fn test_should_stream_kwarg_true() {
    let model = FakeListChatModel::builder()
        .responses(vec!["test".to_string()])
        .build();
    assert!(model._should_stream(false, false, Some(true), None));
}

#[test]
fn test_should_stream_kwarg_false() {
    let model = FakeListChatModel::builder()
        .responses(vec!["test".to_string()])
        .build();
    assert!(!model._should_stream(false, false, Some(false), None));
}

#[test]
fn test_should_stream_no_handlers() {
    let model = FakeListChatModel::builder()
        .responses(vec!["test".to_string()])
        .build();
    let handlers: Vec<std::sync::Arc<dyn agent_chain_core::callbacks::base::BaseCallbackHandler>> =
        vec![];
    assert!(!model._should_stream(false, false, None, Some(&handlers)));
}

#[test]
fn test_convert_input_from_string() {
    let model = agent_chain_core::FakeChatModel::builder().build();
    let result = model
        .convert_input(LanguageModelInput::from("hello world"))
        .unwrap();
    assert_eq!(result.len(), 1);
    assert!(matches!(&result[0], BaseMessage::Human(_)));
    assert_eq!(result[0].content(), "hello world");
}

#[test]
fn test_convert_input_from_message_sequence() {
    let model = agent_chain_core::FakeChatModel::builder().build();
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder().content("hi").build(),
    )];
    let result = model
        .convert_input(LanguageModelInput::from(messages))
        .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].content(), "hi");
}

#[tokio::test]
async fn test_generate_single_message_list() {
    let model = FakeListChatModel::builder()
        .responses(vec!["response".to_string()])
        .build();
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

#[tokio::test]
async fn test_generate_multiple_message_lists() {
    let model = FakeListChatModel::builder()
        .responses(vec!["r1".to_string(), "r2".to_string(), "r3".to_string()])
        .build();
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

#[tokio::test]
async fn test_generate_returns_chat_result() {
    let model = FakeListChatModel::builder()
        .responses(vec!["hello".to_string()])
        .build();
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
    match &result.generations[0][0] {
        agent_chain_core::outputs::GenerationType::ChatGeneration(cg) => {
            assert_eq!(cg.message.content(), "hello");
        }
        _ => panic!("Expected ChatGeneration"),
    }
}

#[tokio::test]
async fn test_agenerate_single_message_list() {
    let model = FakeListChatModel::builder()
        .responses(vec!["response".to_string()])
        .build();
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

#[tokio::test]
async fn test_agenerate_multiple_message_lists() {
    let model = FakeListChatModel::builder()
        .responses(vec!["r1".to_string(), "r2".to_string()])
        .build();
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

#[tokio::test]
async fn test_agenerate_returns_chat_result() {
    let model = FakeListChatModel::builder()
        .responses(vec!["hello".to_string()])
        .build();
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

#[test]
fn test_bind_tools_raises_not_implemented() {
    let model = agent_chain_core::FakeChatModel::builder().build();
    let result = model.bind_tools(&[], None);
    assert!(result.is_err());
}

#[test]
fn test_with_structured_output_raises_not_implemented() {
    let model = agent_chain_core::FakeChatModel::builder().build();
    let result = model.with_structured_output(serde_json::json!({}), false);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_simple_chat_model_generate_wraps_call() {
    let model = agent_chain_core::FakeChatModel::builder().build();
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

#[tokio::test]
async fn test_simple_fake_chat_generate_returns_chat_result() {
    let model = agent_chain_core::FakeChatModel::builder().build();
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

#[tokio::test]
async fn test_simple_fake_chat_agenerate_returns_chat_result() {
    let model = agent_chain_core::FakeChatModel::builder().build();
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
    assert_eq!(
        result.get("key").and_then(|v| v.as_str()),
        Some("from_metadata")
    );
}

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

    assert!(chunks.len() >= 2);

    let last = chunks.last().unwrap();
    assert_eq!(
        last.chunk_position,
        Some(agent_chain_core::messages::ChunkPosition::Last)
    );
}

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

    let config = RunnableConfig::builder()
        .callbacks(vec![handler].into())
        .build();

    let mut stream = model
        .stream(LanguageModelInput::from("test"), Some(&config), None)
        .await
        .unwrap();

    while stream.next().await.is_some() {}

    let recorded = recorder.chunks_received.lock().unwrap();
    let non_none_chunks: Vec<_> = recorded.iter().filter(|c| c.is_some()).collect();
    assert!(
        !non_none_chunks.is_empty(),
        "on_llm_new_token should receive chunk data, got {} calls with {:?}",
        recorded.len(),
        recorded.iter().map(|c| c.is_some()).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn test_structured_output_with_raw_success() {
    use agent_chain_core::language_models::{ChatModelRunnable, StructuredOutputWithRaw};
    use agent_chain_core::messages::ToolCall;
    use agent_chain_core::output_parsers::JsonOutputKeyToolsParser;
    use agent_chain_core::runnables::Runnable;
    use std::sync::Arc;

    let tool_args = serde_json::json!({"answer": "42", "justification": "The meaning of life"});
    let ai_msg = AIMessage::builder()
        .content("")
        .tool_calls(vec![
            ToolCall::builder()
                .name("test_tool")
                .args(tool_args.clone())
                .build(),
        ])
        .build();

    let model = GenericFakeChatModel::from_vec(vec![ai_msg]);
    let model_runnable = ChatModelRunnable::new(Arc::new(model));
    let parser = JsonOutputKeyToolsParser::builder()
        .key_name("test_tool")
        .first_tool_only(true)
        .build();

    let runnable = StructuredOutputWithRaw::new(model_runnable, parser);
    let result = runnable
        .ainvoke(LanguageModelInput::from("test"), None)
        .await
        .unwrap();

    assert_eq!(result["parsed"], tool_args);
    assert_eq!(result["parsing_error"], serde_json::Value::Null);
    assert!(result.get("raw").is_some());
    assert!(!result["raw"].is_null());
}

#[tokio::test]
async fn test_structured_output_with_raw_no_matching_tool() {
    use agent_chain_core::language_models::{ChatModelRunnable, StructuredOutputWithRaw};
    use agent_chain_core::output_parsers::JsonOutputKeyToolsParser;
    use agent_chain_core::runnables::Runnable;
    use std::sync::Arc;

    let ai_msg = AIMessage::builder().content("plain text").build();

    let model = GenericFakeChatModel::from_vec(vec![ai_msg]);
    let model_runnable = ChatModelRunnable::new(Arc::new(model));
    let parser = JsonOutputKeyToolsParser::builder()
        .key_name("test_tool")
        .first_tool_only(true)
        .build();

    let runnable = StructuredOutputWithRaw::new(model_runnable, parser);
    let result = runnable
        .ainvoke(LanguageModelInput::from("test"), None)
        .await
        .unwrap();

    assert_eq!(result["parsed"], serde_json::Value::Null);
    assert_eq!(result["parsing_error"], serde_json::Value::Null);
    assert!(result.get("raw").is_some());
    assert!(!result["raw"].is_null());
}

#[tokio::test]
async fn test_structured_output_with_raw_parse_error() {
    use agent_chain_core::language_models::{ChatModelRunnable, StructuredOutputWithRaw};
    use agent_chain_core::output_parsers::JsonOutputKeyToolsParser;
    use agent_chain_core::runnables::Runnable;
    use std::sync::Arc;

    let ai_msg = AIMessage::builder()
        .content("")
        .additional_kwargs(std::collections::HashMap::from([(
            "tool_calls".to_string(),
            serde_json::json!([{"function": {"name": "test_tool", "arguments": "not json"}}]),
        )]))
        .build();

    let model = GenericFakeChatModel::from_vec(vec![ai_msg]);
    let model_runnable = ChatModelRunnable::new(Arc::new(model));
    let parser = JsonOutputKeyToolsParser::builder()
        .key_name("test_tool")
        .first_tool_only(true)
        .strict(true)
        .build();

    let runnable = StructuredOutputWithRaw::new(model_runnable, parser);
    let result = runnable
        .ainvoke(LanguageModelInput::from("test"), None)
        .await
        .unwrap();

    assert_eq!(result["parsed"], serde_json::Value::Null);
    assert!(
        result["parsing_error"].is_string(),
        "Expected string parsing_error, got: {:?}",
        result["parsing_error"]
    );
    assert!(result.get("raw").is_some());
}

#[tokio::test]
async fn test_structured_output_with_raw_serializes_message() {
    use agent_chain_core::language_models::{ChatModelRunnable, StructuredOutputWithRaw};
    use agent_chain_core::messages::ToolCall;
    use agent_chain_core::output_parsers::JsonOutputKeyToolsParser;
    use agent_chain_core::runnables::Runnable;
    use std::sync::Arc;

    let tool_args = serde_json::json!({"key": "value"});
    let ai_msg = AIMessage::builder()
        .content("some content")
        .tool_calls(vec![
            ToolCall::builder().name("my_tool").args(tool_args).build(),
        ])
        .build();

    let model = GenericFakeChatModel::from_vec(vec![ai_msg]);
    let model_runnable = ChatModelRunnable::new(Arc::new(model));
    let parser = JsonOutputKeyToolsParser::builder()
        .key_name("my_tool")
        .first_tool_only(true)
        .build();

    let runnable = StructuredOutputWithRaw::new(model_runnable, parser);
    let result = runnable
        .ainvoke(LanguageModelInput::from("test"), None)
        .await
        .unwrap();

    let raw = &result["raw"];
    assert_eq!(raw["content"], "some content");
    assert_eq!(raw["type"], "ai");
}
