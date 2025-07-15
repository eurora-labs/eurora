//! gRPC provider implementations.

use crate::config::GrpcConfig;
use crate::error::GrpcError;
use crate::proto::chat::{
    proto_chat_service_client::ProtoChatServiceClient, proto_content_part::ProtoPartType,
    proto_message_content::ProtoContentType, *,
};
use crate::types::*;
use async_trait::async_trait;
use ferrous_llm_core::ChatRequest;
use ferrous_llm_core::traits::{ChatProvider, StreamingProvider};
use futures::Stream;
use std::pin::Pin;
use tonic::transport::{Channel, Endpoint};
use tonic::{Request, Streaming};

/// gRPC-based chat provider.
#[derive(Debug, Clone)]
pub struct GrpcChatProvider {
    client: ProtoChatServiceClient<Channel>,
    config: GrpcConfig,
}

impl GrpcChatProvider {
    /// Create a new gRPC chat provider with the given configuration.
    pub async fn new(config: GrpcConfig) -> Result<Self, GrpcError> {
        use ferrous_llm_core::config::ProviderConfig;
        config
            .validate()
            .map_err(|e| GrpcError::InvalidConfig(e.to_string()))?;

        let client = Self::create_client(&config).await?;

        Ok(Self { client, config })
    }

    /// Create a gRPC client from the configuration.
    async fn create_client(
        config: &GrpcConfig,
    ) -> Result<ProtoChatServiceClient<Channel>, GrpcError> {
        // Convert URL to URI
        let uri = config
            .endpoint
            .to_string()
            .parse::<tonic::transport::Uri>()
            .map_err(|e| GrpcError::InvalidConfig(format!("Invalid endpoint URI: {}", e)))?;

        let mut endpoint = Endpoint::from(uri)
            .user_agent(config.user_agent.as_deref().unwrap_or("ferrous-llm-grpc"))?;

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
            let tls_config = if let Some(domain) = &config.tls_domain {
                tonic::transport::ClientTlsConfig::new().domain_name(domain)
            } else {
                tonic::transport::ClientTlsConfig::new()
            };
            endpoint = endpoint.tls_config(tls_config)?;
        }

        let channel = endpoint.connect().await?;
        let mut client = ProtoChatServiceClient::new(channel);

        // Configure message size limits
        if let Some(max_request_size) = config.max_request_size {
            client = client.max_encoding_message_size(max_request_size);
        }

        if let Some(max_response_size) = config.max_response_size {
            client = client.max_decoding_message_size(max_response_size);
        }

        Ok(client)
    }

    /// Convert a core ChatRequest to a proto ChatRequest.
    fn convert_request(&self, request: ChatRequest) -> Result<ProtoChatRequest, GrpcError> {
        let messages = request
            .messages
            .into_iter()
            .map(|msg| self.convert_message(msg))
            .collect::<Result<Vec<_>, _>>()?;

        let parameters = Some(self.convert_parameters(request.parameters));
        let metadata = Some(self.convert_metadata(request.metadata));

        Ok(ProtoChatRequest {
            messages,
            parameters,
            metadata,
        })
    }

    /// Convert a core Message to a proto Message.
    fn convert_message(
        &self,
        message: ferrous_llm_core::types::Message,
    ) -> Result<ProtoMessage, GrpcError> {
        let role: ProtoRole = message.role.into();
        let content = Some(self.convert_message_content(message.content)?);

        Ok(ProtoMessage {
            // Double into here because prost treats all defined enums as i32 when used asa properties
            role: role.into(),
            content,
        })
    }

    /// Convert core MessageContent to proto MessageContent.
    fn convert_message_content(
        &self,
        content: ferrous_llm_core::types::MessageContent,
    ) -> Result<ProtoMessageContent, GrpcError> {
        let content_type = match content {
            ferrous_llm_core::types::MessageContent::Text(text) => ProtoContentType::Text(text),
            ferrous_llm_core::types::MessageContent::Multimodal(parts) => {
                let proto_parts = parts
                    .into_iter()
                    .map(|part| self.convert_content_part(part))
                    .collect::<Result<Vec<_>, _>>()?;

                ProtoContentType::Multimodal(ProtoMultimodalContent { parts: proto_parts })
            }
            ferrous_llm_core::types::MessageContent::Tool(tool_content) => {
                let tool_calls = tool_content
                    .tool_calls
                    .unwrap_or_default()
                    .into_iter()
                    .map(|call| self.convert_tool_call(call))
                    .collect::<Result<Vec<_>, _>>()?;

                ProtoContentType::Tool(ProtoToolContent {
                    tool_calls,
                    tool_call_id: tool_content.tool_call_id,
                    text: tool_content.text,
                })
            }
        };

        Ok(ProtoMessageContent {
            proto_content_type: Some(content_type),
        })
    }

    /// Convert core ContentPart to proto ContentPart.
    fn convert_content_part(
        &self,
        part: ferrous_llm_core::types::ContentPart,
    ) -> Result<ProtoContentPart, GrpcError> {
        let part_type = match part {
            ferrous_llm_core::types::ContentPart::Text { text } => {
                ProtoPartType::Text(ProtoTextPart { text })
            }
            ferrous_llm_core::types::ContentPart::Image {
                image_source,
                detail,
            } => {
                let source = image_source.into();

                ProtoPartType::Image(ProtoImagePart {
                    image_source: Some(source),
                    detail,
                })
            }
            ferrous_llm_core::types::ContentPart::Audio { audio_url, format } => {
                ProtoPartType::Audio(ProtoAudioPart { audio_url, format })
            }
        };

        Ok(ProtoContentPart {
            proto_part_type: Some(part_type),
        })
    }

    /// Convert core ToolCall to proto ToolCall.
    fn convert_tool_call(
        &self,
        call: ferrous_llm_core::types::ToolCall,
    ) -> Result<ProtoToolCall, GrpcError> {
        Ok(ProtoToolCall {
            id: call.id,
            call_type: call.call_type,
            function: Some(ProtoFunctionCall {
                name: call.function.name,
                arguments: call.function.arguments,
            }),
        })
    }

    /// Convert core Parameters to proto Parameters.
    fn convert_parameters(&self, params: ferrous_llm_core::types::Parameters) -> ProtoParameters {
        ProtoParameters {
            temperature: params.temperature,
            max_tokens: params.max_tokens,
            top_p: params.top_p,
            top_k: params.top_k,
            stop_sequences: params.stop_sequences,
            frequency_penalty: params.frequency_penalty,
            presence_penalty: params.presence_penalty,
        }
    }

    /// Convert core Metadata to proto Metadata.
    fn convert_metadata(&self, metadata: ferrous_llm_core::types::Metadata) -> ProtoMetadata {
        ProtoMetadata {
            extensions: Some(hashmap_to_proto_struct(&metadata.extensions)),
            request_id: metadata.request_id,
            user_id: metadata.user_id,
            created_at: Some(datetime_to_proto_timestamp(&metadata.created_at)),
        }
    }
}

#[async_trait]
impl ChatProvider for GrpcChatProvider {
    type Config = GrpcConfig;
    type Response = ProtoChatResponse;
    type Error = GrpcError;

    async fn chat(
        &self,
        request: ferrous_llm_core::types::ChatRequest,
    ) -> Result<Self::Response, Self::Error> {
        let proto_request = request.into();
        let mut client = self.client.clone();

        // Add authentication if configured
        let mut grpc_request = Request::new(proto_request);
        if let Some(token) = &self.config.auth_token {
            grpc_request.metadata_mut().insert(
                "authorization",
                format!("Bearer {}", token).parse().map_err(|_| {
                    GrpcError::Authentication("Invalid auth token format".to_string())
                })?,
            );
        }

        let response = client.chat(grpc_request).await?;
        let proto_response = response.into_inner();

        Ok(proto_response)
    }
}

/// gRPC-based streaming provider.
#[derive(Debug, Clone)]
pub struct GrpcStreamingProvider {
    inner: GrpcChatProvider,
}

impl GrpcStreamingProvider {
    /// Create a new gRPC streaming provider with the given configuration.
    pub async fn new(config: GrpcConfig) -> Result<Self, GrpcError> {
        let inner = GrpcChatProvider::new(config).await?;
        Ok(Self { inner })
    }
}

#[async_trait]
impl ChatProvider for GrpcStreamingProvider {
    type Config = GrpcConfig;
    type Response = ProtoChatResponse;
    type Error = GrpcError;

    async fn chat(
        &self,
        request: ferrous_llm_core::types::ChatRequest,
    ) -> Result<Self::Response, Self::Error> {
        self.inner.chat(request).await
    }
}

#[async_trait]
impl StreamingProvider for GrpcStreamingProvider {
    type StreamItem = ProtoChatStreamResponse;
    type Stream =
        Pin<Box<dyn Stream<Item = Result<Self::StreamItem, Self::Error>> + Send + 'static>>;

    async fn chat_stream(
        &self,
        request: ferrous_llm_core::types::ChatRequest,
    ) -> Result<Self::Stream, Self::Error> {
        let proto_request = self.inner.convert_request(request)?;
        let mut client = self.inner.client.clone();

        // Add authentication if configured
        let mut grpc_request = Request::new(proto_request);
        if let Some(token) = &self.inner.config.auth_token {
            grpc_request.metadata_mut().insert(
                "authorization",
                format!("Bearer {}", token).parse().map_err(|_| {
                    GrpcError::Authentication("Invalid auth token format".to_string())
                })?,
            );
        }

        let response = client.chat_stream(grpc_request).await?;
        let stream = response.into_inner();

        let converted_stream = Self::convert_stream(stream, self.inner.clone());
        Ok(Box::pin(converted_stream))
    }
}

impl GrpcStreamingProvider {
    /// Convert the gRPC stream to our stream type.
    fn convert_stream(
        mut stream: Streaming<ProtoChatStreamResponse>,
        provider: GrpcChatProvider,
    ) -> impl Stream<Item = Result<ProtoChatStreamResponse, GrpcError>> + Send + 'static {
        async_stream::stream! {
            while let Some(result) = stream.message().await.transpose() {
                match result {
                    Ok(proto_response) => yield Ok(proto_response),
                    Err(e) => yield Err(GrpcError::Status(e)),
                }
            }
        }
    }
}
