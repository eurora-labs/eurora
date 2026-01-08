//! Eurora gRPC chat model implementation.
//!
//! This module provides the `ChatEurora` struct which implements the
//! `ChatModel` trait for the Eurora gRPC service.

use agent_chain_core::callbacks::{
    AsyncCallbackManagerForLLMRun, CallbackManagerForLLMRun, Callbacks,
};
use agent_chain_core::language_models::{
    BaseLanguageModel, ChatGenerationStream, LanguageModelConfig, LanguageModelInput,
};
use agent_chain_core::outputs::{ChatGeneration, ChatGenerationChunk, ChatResult, LLMResult};
use agent_chain_core::{
    AIMessage, BaseChatModel, BaseMessage, ChatModelConfig, LangSmithParams, ToolCall, ToolChoice,
    ToolDefinition,
};
use anyhow::Result;
use async_trait::async_trait;
use euro_auth::AuthedChannel;
use futures::StreamExt;
use tonic::Request;

use agent_chain_grpc::proto::{
    ProtoChatRequest, ProtoParameters, proto_chat_service_client::ProtoChatServiceClient,
};

// /// Auth interceptor for adding authentication headers to gRPC requests.
// #[derive(Clone)]
// struct AuthInterceptor {
//     auth: euro_auth::AuthManager,
// }

// impl AuthInterceptor {
//     pub fn new(auth: euro_auth::AuthManager) -> Self {
//         Self { auth }
//     }
// }

// impl AsyncInterceptor for AuthInterceptor {
//     type Future = Pin<Box<dyn std::future::Future<Output = Result<Request<()>, Status>> + Send>>;

//     fn call(&mut self, mut request: Request<()>) -> Self::Future {
//         let auth = self.auth.clone();
//         Box::pin(async move {
//             let access_token = auth.get_or_refresh_access_token().await.map_err(|e| {
//                 Status::unauthenticated(format!("Failed to retrieve access token: {}", e))
//             })?;
//             let header: String = format!("Bearer {}", access_token.0);

//             match header.parse() {
//                 Ok(value) => {
//                     request.metadata_mut().insert("authorization", value);
//                     Ok(request)
//                 }
//                 Err(err) => {
//                     error!("Failed to parse authorization header: {}", err);
//                     Ok(request)
//                 }
//             }
//         })
//     }
// }

type EuroraGrpcClient = ProtoChatServiceClient<AuthedChannel>;

/// Eurora gRPC chat model.
///
/// This struct implements the `ChatModel` trait for the Eurora gRPC service,
/// providing a LangChain-compatible interface for chat completion.
///
/// # Example
///
/// ```ignore
/// use agent_chain_eurora::{ChatEurora, EuroraConfig};
/// use agent_chain_core::{ChatModel, HumanMessage};
/// use url::Url;
///
/// let config = EuroraConfig::new(Url::parse("https://api.eurora.com").unwrap());
/// let model = ChatEurora::new(config).await?;
///
/// let messages = vec![HumanMessage::new("Hello!").into()];
/// let response = model.generate(messages, None).await?;
/// ```
#[derive(Debug, Clone)]
pub struct ChatEurora {
    /// gRPC client for the chat service
    client: EuroraGrpcClient,
    /// Model name/identifier
    model: String,
    /// Temperature for generation (0.0 - 2.0)
    temperature: Option<f32>,
    /// Maximum tokens to generate
    max_tokens: Option<u32>,
    /// Top-p (nucleus) sampling parameter
    top_p: Option<f32>,
    /// Top-k sampling parameter
    top_k: Option<u32>,
    /// Stop sequences
    stop_sequences: Vec<String>,
    /// Frequency penalty
    frequency_penalty: Option<f32>,
    /// Presence penalty
    presence_penalty: Option<f32>,
    /// Chat model configuration
    chat_model_config: ChatModelConfig,
    /// Language model configuration
    language_model_config: LanguageModelConfig,
}

impl ChatEurora {
    /// Create a new ChatEurora instance with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The Eurora configuration.
    ///
    /// # Returns
    ///
    /// A new `ChatEurora` instance, or an error if connection fails.
    pub async fn new() -> Result<Self> {
        let channel = euro_auth::get_authed_channel().await;

        let client = ProtoChatServiceClient::new(channel);

        Ok(Self {
            client,
            model: "eurora".to_string(),
            temperature: None,
            max_tokens: None,
            top_p: None,
            top_k: None,
            stop_sequences: Vec::new(),
            frequency_penalty: None,
            presence_penalty: None,
            chat_model_config: ChatModelConfig::new(),
            language_model_config: LanguageModelConfig::new(),
        })
    }

    // /// Create a gRPC client from the configuration.
    // async fn create_client(config: &EuroraConfig) -> Result<EuroraGrpcClient, EuroraError> {
    //     // Convert URL to URI
    //     let uri = config
    //         .endpoint
    //         .to_string()
    //         .parse::<tonic::transport::Uri>()
    //         .map_err(|e| EuroraError::InvalidConfig(format!("Invalid endpoint URI: {}", e)))?;

    //     let mut endpoint = Endpoint::from(uri)
    //         .user_agent(config.user_agent.as_deref().unwrap_or("agent-chain-eurora"))?;

    //     // Configure timeouts
    //     if let Some(timeout) = config.timeout {
    //         endpoint = endpoint.timeout(timeout);
    //     }

    //     if let Some(connect_timeout) = config.connect_timeout {
    //         endpoint = endpoint.connect_timeout(connect_timeout);
    //     }

    //     // Configure keep-alive
    //     if let Some(interval) = config.keep_alive_interval {
    //         endpoint = endpoint.keep_alive_timeout(config.keep_alive_timeout.unwrap_or(interval));
    //         endpoint = endpoint.keep_alive_while_idle(config.keep_alive_while_idle);
    //     }

    //     // Configure TLS if needed
    //     if config.use_tls {
    //         let tls_config = ClientTlsConfig::new().with_native_roots();
    //         endpoint = endpoint.tls_config(tls_config)?;
    //     }

    //     let channel = endpoint.connect().await?;
    //     let auth_manager = euro_user::AuthManager::new()
    //         .await
    //         .map_err(|err| EuroraError::Authentication(err.to_string()))?;
    //     let auth_interceptor = AuthInterceptor::new(auth_manager);

    //     let service = ServiceBuilder::new()
    //         .layer(async_interceptor(auth_interceptor))
    //         .service(channel);
    //     let mut client = ProtoChatServiceClient::new(service);

    //     // Configure message size limits
    //     if let Some(max_request_size) = config.max_request_size {
    //         client = client.max_encoding_message_size(max_request_size);
    //     }

    //     if let Some(max_response_size) = config.max_response_size {
    //         client = client.max_decoding_message_size(max_response_size);
    //     }

    //     Ok(client)
    // }

    /// Set the model name.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the temperature.
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set the maximum tokens to generate.
    pub fn max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
        self
    }

    /// Set the top-p parameter.
    pub fn top_p(mut self, p: f32) -> Self {
        self.top_p = Some(p);
        self
    }

    /// Set the top-k parameter.
    pub fn top_k(mut self, k: u32) -> Self {
        self.top_k = Some(k);
        self
    }

    /// Set stop sequences.
    pub fn stop_sequences(mut self, sequences: Vec<String>) -> Self {
        self.stop_sequences = sequences;
        self
    }

    /// Set the frequency penalty.
    pub fn frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = Some(penalty);
        self
    }

    /// Set the presence penalty.
    pub fn presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = Some(penalty);
        self
    }

    /// Build a ProtoChatRequest from agent-chain messages.
    fn build_request(
        &self,
        messages: &[BaseMessage],
        stop: Option<Vec<String>>,
    ) -> ProtoChatRequest {
        let proto_messages = messages.iter().map(Into::into).collect();
        let stop_sequences = stop.unwrap_or_else(|| self.stop_sequences.clone());

        ProtoChatRequest {
            // TODO: change to actual conversation id
            conversation_id: "test".to_string(),
            messages: proto_messages,
            parameters: Some(ProtoParameters {
                temperature: self.temperature,
                max_tokens: self.max_tokens,
                top_p: self.top_p,
                top_k: self.top_k,
                stop_sequences,
                frequency_penalty: self.frequency_penalty,
                presence_penalty: self.presence_penalty,
            }),
        }
    }
}

#[async_trait]
impl BaseLanguageModel for ChatEurora {
    fn llm_type(&self) -> &str {
        "eurora-chat"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn config(&self) -> &LanguageModelConfig {
        &self.language_model_config
    }

    async fn generate_prompt(
        &self,
        prompts: Vec<LanguageModelInput>,
        stop: Option<Vec<String>>,
        _callbacks: Option<Callbacks>,
    ) -> agent_chain_core::Result<LLMResult> {
        // Convert prompts to message batches and generate
        let mut all_generations = Vec::new();
        for prompt in prompts {
            let messages = prompt.to_messages();
            let result = self
                ._generate_internal(messages, stop.clone(), None)
                .await?;
            all_generations.push(result.generations.into_iter().map(|g| g.into()).collect());
        }
        Ok(LLMResult::new(all_generations))
    }

    fn get_ls_params(&self, stop: Option<&[String]>) -> LangSmithParams {
        LangSmithParams {
            ls_provider: Some("eurora".to_string()),
            ls_model_name: Some(self.model.clone()),
            ls_model_type: Some("chat".to_string()),
            ls_temperature: self.temperature.map(|t| t as f64),
            ls_max_tokens: self.max_tokens,
            ls_stop: stop.map(|s| s.to_vec()),
        }
    }
}

#[async_trait]
impl BaseChatModel for ChatEurora {
    fn chat_config(&self) -> &ChatModelConfig {
        &self.chat_model_config
    }

    fn has_astream_impl(&self) -> bool {
        true
    }

    async fn _astream(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        _run_manager: Option<&AsyncCallbackManagerForLLMRun>,
    ) -> agent_chain_core::Result<ChatGenerationStream> {
        let proto_request = self.build_request(&messages, stop);

        let mut client = self.client.clone();
        let grpc_request = Request::new(proto_request);

        // Use streaming API
        let response = client
            .chat_stream(grpc_request)
            .await
            .expect("Failed to start streaming");
        let grpc_stream = response.into_inner();

        // Create an async stream that yields ChatGenerationChunk for each gRPC chunk
        let chunk_stream = async_stream::stream! {
            let mut stream = grpc_stream;

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(stream_response) => {
                        if let Some(chunk) = stream_response.chunk {
                            // Create an AIMessage from the chunk content
                            let ai_message = AIMessage::new(&chunk.content);
                            let generation_chunk = ChatGenerationChunk::new(ai_message.into());
                            yield Ok(generation_chunk);
                        }
                    }
                    Err(e) => {
                        yield Err(agent_chain_core::Error::Other(format!("Stream error: {}", e)));
                        return;
                    }
                }
            }
        };

        Ok(Box::pin(chunk_stream))
    }

    async fn _generate(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> agent_chain_core::Result<ChatResult> {
        self._generate_internal(messages, stop, None).await
    }

    async fn generate_with_tools(
        &self,
        messages: Vec<BaseMessage>,
        tools: &[ToolDefinition],
        tool_choice: Option<&ToolChoice>,
        stop: Option<Vec<String>>,
    ) -> agent_chain_core::Result<AIMessage> {
        self.generate_with_tools_internal(messages, tools, tool_choice, stop)
            .await
    }
}

impl ChatEurora {
    /// Internal generate implementation.
    async fn _generate_internal(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> agent_chain_core::Result<ChatResult> {
        let proto_request = self.build_request(&messages, stop);

        let mut client = self.client.clone();
        let grpc_request = Request::new(proto_request);

        // Use streaming API and accumulate chunks
        let response = client
            .chat_stream(grpc_request)
            .await
            .expect("Failed to start streaming");
        let mut stream = response.into_inner();

        // Accumulate content from all chunks
        let mut accumulated_content = String::new();
        let mut accumulated_tool_calls: Vec<ToolCall> = Vec::new();
        let mut message_id: Option<String> = None;

        while let Some(chunk_result) = stream.next().await {
            let stream_response = chunk_result.expect("Failed to receive chunk");

            if let Some(chunk) = stream_response.chunk {
                // Accumulate content
                accumulated_content.push_str(&chunk.content);

                // Capture message ID from first chunk that has it
                if message_id.is_none() && chunk.id.is_some() {
                    message_id = chunk.id;
                }

                // Accumulate tool calls (only from complete tool calls, not chunks)
                for proto_tool_call in chunk.tool_calls {
                    let args: serde_json::Value =
                        serde_json::from_str(&proto_tool_call.args).unwrap_or_default();
                    accumulated_tool_calls.push(ToolCall::with_id(
                        proto_tool_call.id,
                        proto_tool_call.name,
                        args,
                    ));
                }
            }
        }

        // Build the final AIMessage from accumulated data
        let message = match (message_id, accumulated_tool_calls.is_empty()) {
            (Some(id), true) => AIMessage::with_id(id, accumulated_content),
            (Some(id), false) => {
                AIMessage::with_id_and_tool_calls(id, accumulated_content, accumulated_tool_calls)
            }
            (None, true) => AIMessage::new(accumulated_content),
            (None, false) => {
                AIMessage::with_tool_calls(accumulated_content, accumulated_tool_calls)
            }
        };

        let generation = ChatGeneration::new(message.into());
        Ok(ChatResult::new(vec![generation]))
    }

    /// Internal generate with tools implementation.
    async fn generate_with_tools_internal(
        &self,
        messages: Vec<BaseMessage>,
        _tools: &[ToolDefinition],
        _tool_choice: Option<&ToolChoice>,
        stop: Option<Vec<String>>,
    ) -> agent_chain_core::Result<AIMessage> {
        // For now, we don't have tool support in the proto definition
        // Just call the regular _generate method
        // TODO: Add tool support to the proto definition and implement here
        let result = self._generate_internal(messages, stop, None).await?;
        Self::extract_ai_message(result)
    }

    /// Extract AIMessage from ChatResult.
    fn extract_ai_message(result: ChatResult) -> agent_chain_core::Result<AIMessage> {
        if result.generations.is_empty() {
            return Err(agent_chain_core::Error::Other(
                "No generations returned".into(),
            ));
        }
        match result.generations[0].message.clone() {
            BaseMessage::AI(msg) => Ok(msg),
            _ => Err(agent_chain_core::Error::Other("Expected AI message".into())),
        }
    }
}
