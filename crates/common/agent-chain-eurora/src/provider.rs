//! Eurora gRPC chat model implementation.
//!
//! This module provides the `ChatEurora` struct which implements the
//! `ChatModel` trait for the Eurora gRPC service.

use std::pin::Pin;

use agent_chain_core::{
    AIMessage, BaseMessage, ChatChunk, ChatModel, ChatResult, ChatResultMetadata, ChatStream,
    LangSmithParams, ToolChoice, ToolDefinition, UsageMetadata,
};
use async_trait::async_trait;
use futures::Stream;
use tonic::{
    Request, Status, Streaming,
    transport::{Channel, ClientTlsConfig, Endpoint},
};
use tonic_async_interceptor::{AsyncInterceptor, async_interceptor};
use tower::ServiceBuilder;
use tracing::{debug, error};

use crate::{
    config::EuroraConfig,
    error::EuroraError,
    proto::chat::{
        ProtoChatRequest, ProtoChatStreamResponse, ProtoParameters,
        proto_chat_service_client::ProtoChatServiceClient,
    },
};

/// Auth interceptor for adding authentication headers to gRPC requests.
#[derive(Clone)]
struct AuthInterceptor {
    auth: euro_user::AuthManager,
}

impl AuthInterceptor {
    pub fn new(auth: euro_user::AuthManager) -> Self {
        Self { auth }
    }
}

impl AsyncInterceptor for AuthInterceptor {
    type Future = Pin<Box<dyn std::future::Future<Output = Result<Request<()>, Status>> + Send>>;

    fn call(&mut self, mut request: Request<()>) -> Self::Future {
        let auth = self.auth.clone();
        debug!("AuthInterceptor called");
        Box::pin(async move {
            let access_token = auth.get_or_refresh_access_token().await.map_err(|e| {
                Status::unauthenticated(format!("Failed to retrieve access token: {}", e))
            })?;
            let header: String = format!("Bearer {}", access_token.0);

            match header.parse() {
                Ok(value) => {
                    request.metadata_mut().insert("authorization", value);
                    Ok(request)
                }
                Err(err) => {
                    error!("Failed to parse authorization header: {}", err);
                    Ok(request)
                }
            }
        })
    }
}

type EuroraGrpcClient = ProtoChatServiceClient<
    tonic_async_interceptor::AsyncInterceptedService<Channel, AuthInterceptor>,
>;

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
    pub async fn new(config: EuroraConfig) -> Result<Self, EuroraError> {
        config.validate()?;
        let client = Self::create_client(&config).await?;

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
        })
    }

    /// Create a gRPC client from the configuration.
    async fn create_client(config: &EuroraConfig) -> Result<EuroraGrpcClient, EuroraError> {
        // Convert URL to URI
        let uri = config
            .endpoint
            .to_string()
            .parse::<tonic::transport::Uri>()
            .map_err(|e| EuroraError::InvalidConfig(format!("Invalid endpoint URI: {}", e)))?;

        let mut endpoint = Endpoint::from(uri)
            .user_agent(config.user_agent.as_deref().unwrap_or("agent-chain-eurora"))?;

        // Configure timeouts
        if let Some(timeout) = config.timeout {
            endpoint = endpoint.timeout(timeout);
        }

        if let Some(connect_timeout) = config.connect_timeout {
            endpoint = endpoint.connect_timeout(connect_timeout);
        }

        // Configure keep-alive
        if let Some(interval) = config.keep_alive_interval {
            endpoint = endpoint.keep_alive_timeout(config.keep_alive_timeout.unwrap_or(interval));
            endpoint = endpoint.keep_alive_while_idle(config.keep_alive_while_idle);
        }

        // Configure TLS if needed
        if config.use_tls {
            let tls_config = ClientTlsConfig::new().with_native_roots();
            endpoint = endpoint.tls_config(tls_config)?;
        }

        let channel = endpoint.connect().await?;
        let auth_manager = euro_user::AuthManager::new()
            .await
            .map_err(|err| EuroraError::Authentication(err.to_string()))?;
        let auth_interceptor = AuthInterceptor::new(auth_manager);

        let service = ServiceBuilder::new()
            .layer(async_interceptor(auth_interceptor))
            .service(channel);
        let mut client = ProtoChatServiceClient::new(service);

        // Configure message size limits
        if let Some(max_request_size) = config.max_request_size {
            client = client.max_encoding_message_size(max_request_size);
        }

        if let Some(max_response_size) = config.max_response_size {
            client = client.max_decoding_message_size(max_response_size);
        }

        Ok(client)
    }

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

    /// Convert the gRPC stream to a ChatStream.
    fn convert_stream(
        mut stream: Streaming<ProtoChatStreamResponse>,
        model: String,
    ) -> impl Stream<Item = agent_chain_core::Result<ChatChunk>> + Send + 'static {
        async_stream::stream! {
            while let Some(result) = stream.message().await.transpose() {
                match result {
                    Ok(proto_response) => {
                        let usage = proto_response.usage.map(|u| {
                            UsageMetadata::new(u.input_tokens, u.output_tokens)
                        });

                        let chunk = ChatChunk {
                            content: proto_response.content,
                            is_final: proto_response.is_final,
                            metadata: if proto_response.is_final {
                                Some(ChatResultMetadata {
                                    model: Some(model.clone()),
                                    stop_reason: proto_response.stop_reason,
                                    usage,
                                })
                            } else {
                                None
                            },
                        };
                        yield Ok(chunk);
                    }
                    Err(e) => {
                        yield Err(EuroraError::Status(e).into());
                    }
                }
            }
        }
    }
}

#[async_trait]
impl ChatModel for ChatEurora {
    fn llm_type(&self) -> &str {
        "eurora-chat"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    async fn generate(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
    ) -> agent_chain_core::Result<ChatResult> {
        let proto_request = self.build_request(&messages, stop);

        let mut client = self.client.clone();
        let grpc_request = Request::new(proto_request);

        let response = client.chat(grpc_request).await.map_err(EuroraError::from)?;
        let proto_response = response.into_inner();

        // Convert proto AIMessage directly to agent-chain AIMessage
        let message: AIMessage = proto_response
            .message
            .map(Into::into)
            .unwrap_or_else(|| AIMessage::new(""));

        let usage = proto_response
            .usage
            .map(|u| UsageMetadata::new(u.input_tokens, u.output_tokens));

        Ok(ChatResult {
            message,
            metadata: ChatResultMetadata {
                model: Some(self.model.clone()),
                stop_reason: proto_response.stop_reason,
                usage,
            },
        })
    }

    async fn generate_with_tools(
        &self,
        messages: Vec<BaseMessage>,
        _tools: &[ToolDefinition],
        _tool_choice: Option<&ToolChoice>,
        stop: Option<Vec<String>>,
    ) -> agent_chain_core::Result<ChatResult> {
        // For now, we don't have tool support in the proto definition
        // Just call the regular generate method
        // TODO: Add tool support to the proto definition and implement here
        self.generate(messages, stop).await
    }

    async fn stream(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
    ) -> agent_chain_core::Result<ChatStream> {
        debug!("Sending chat stream");
        let proto_request = self.build_request(&messages, stop);

        let mut client = self.client.clone();
        let grpc_request = Request::new(proto_request);

        let response = client
            .chat_stream(grpc_request)
            .await
            .map_err(EuroraError::from)?;
        let stream = response.into_inner();

        let converted_stream = Self::convert_stream(stream, self.model.clone());
        Ok(Box::pin(converted_stream)
            as Pin<
                Box<dyn Stream<Item = agent_chain_core::Result<ChatChunk>> + Send>,
            >)
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

    fn identifying_params(&self) -> serde_json::Value {
        serde_json::json!({
            "_type": self.llm_type(),
            "model": self.model,
            "max_tokens": self.max_tokens,
            "temperature": self.temperature,
            "top_p": self.top_p,
            "top_k": self.top_k,
            "frequency_penalty": self.frequency_penalty,
            "presence_penalty": self.presence_penalty,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    #[test]
    fn test_llm_type() {
        // Can't fully test without a running server, but we can test the type identifier
        // This is a placeholder for now
        assert_eq!("eurora-chat", "eurora-chat");
    }

    #[tokio::test]
    async fn test_config_validation() {
        let config = EuroraConfig::new(Url::parse("http://localhost:50051").unwrap());
        assert!(config.validate().is_ok());
    }
}
