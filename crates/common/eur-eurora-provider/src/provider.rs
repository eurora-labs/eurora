//! gRPC provider implementations.

use std::pin::Pin;

use async_trait::async_trait;
use eur_secret::secret;
use ferrous_llm_core::traits::{ChatProvider, StreamingProvider};
use futures::Stream;
use tonic::{
    Request, Status, Streaming,
    service::{Interceptor, interceptor::InterceptedService},
    transport::{Channel, ClientTlsConfig, Endpoint},
};
use tracing::info;

use crate::{
    config::EuroraConfig,
    error::EuroraError,
    proto::chat::{proto_chat_service_client::ProtoChatServiceClient, *},
};

#[derive(Clone)]
struct AuthInterceptor;

impl Interceptor for AuthInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        let access_token = secret::retrieve("AUTH_ACCESS_TOKEN", secret::Namespace::Global)
            .map_err(|e| Status::internal(format!("Failed to retrieve access token: {}", e)))?
            .ok_or_else(|| Status::unauthenticated("AUTH_ACCESS_TOKEN not found"))?;
        request.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", access_token.0).parse().unwrap(),
        );
        Ok(request)
    }
}

type EuroraGrpcClient = ProtoChatServiceClient<InterceptedService<Channel, AuthInterceptor>>;

/// Eurora-based chat provider.
#[derive(Debug, Clone)]
pub struct EuroraChatProvider {
    client: EuroraGrpcClient,
}

impl EuroraChatProvider {
    /// Create a new gRPC chat provider with the given configuration.
    pub async fn new(config: EuroraConfig) -> Result<Self, EuroraError> {
        use ferrous_llm_core::config::ProviderConfig;
        config
            .validate()
            .map_err(|e| EuroraError::InvalidConfig(e.to_string()))?;

        let client = Self::create_client(&config).await?;

        Ok(Self { client })
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
            .user_agent(config.user_agent.as_deref().unwrap_or("eurora-grpc"))?;

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
        let mut client = ProtoChatServiceClient::with_interceptor(channel, AuthInterceptor);

        // Configure message size limits
        if let Some(max_request_size) = config.max_request_size {
            client = client.max_encoding_message_size(max_request_size);
        }

        if let Some(max_response_size) = config.max_response_size {
            client = client.max_decoding_message_size(max_response_size);
        }

        Ok(client)
    }
}

#[async_trait]
impl ChatProvider for EuroraChatProvider {
    type Config = EuroraConfig;
    type Response = ProtoChatResponse;
    type Error = EuroraError;

    async fn chat(
        &self,
        request: ferrous_llm_core::types::ChatRequest,
    ) -> Result<Self::Response, Self::Error> {
        let proto_request = request.into();
        let mut client = self.client.clone();

        let grpc_request = Request::new(proto_request);

        let response = client.chat(grpc_request).await?;
        let proto_response = response.into_inner();

        Ok(proto_response)
    }
}

/// Eurora-based streaming provider.
#[derive(Debug, Clone)]
pub struct EuroraStreamingProvider {
    inner: EuroraChatProvider,
}

impl EuroraStreamingProvider {
    /// Create a new Eurora streaming provider with the given configuration.
    pub async fn new(config: EuroraConfig) -> Result<Self, EuroraError> {
        let inner = EuroraChatProvider::new(config).await?;
        Ok(Self { inner })
    }
}

#[async_trait]
impl ChatProvider for EuroraStreamingProvider {
    type Config = EuroraConfig;
    type Response = ProtoChatResponse;
    type Error = EuroraError;

    async fn chat(
        &self,
        request: ferrous_llm_core::types::ChatRequest,
    ) -> Result<Self::Response, Self::Error> {
        self.inner.chat(request).await
    }
}

#[async_trait]
impl StreamingProvider for EuroraStreamingProvider {
    type StreamItem = ProtoChatStreamResponse;
    type Stream =
        Pin<Box<dyn Stream<Item = Result<Self::StreamItem, Self::Error>> + Send + 'static>>;

    async fn chat_stream(
        &self,
        request: ferrous_llm_core::types::ChatRequest,
    ) -> Result<Self::Stream, Self::Error> {
        info!("Sending chat stream");
        let proto_request = request.into();
        let mut client = self.inner.client.clone();

        let grpc_request = Request::new(proto_request);

        let response = client.chat_stream(grpc_request).await?;
        let stream = response.into_inner();

        let converted_stream = Self::convert_stream(stream);
        Ok(Box::pin(converted_stream))
    }
}

impl EuroraStreamingProvider {
    /// Convert the gRPC stream to our stream type.
    fn convert_stream(
        mut stream: Streaming<ProtoChatStreamResponse>,
    ) -> impl Stream<Item = Result<ProtoChatStreamResponse, EuroraError>> + Send + 'static {
        async_stream::stream! {
            while let Some(result) = stream.message().await.transpose() {
                match result {
                    Ok(proto_response) => yield Ok(proto_response),
                    Err(e) => yield Err(EuroraError::Status(e)),
                }
            }
        }
    }
}
