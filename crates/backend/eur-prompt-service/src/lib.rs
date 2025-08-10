use std::pin::Pin;

use anyhow::{Result, anyhow};
use eur_auth::{Claims, JwtConfig, validate_access_token};
use eur_eurora_provider::proto::chat::{
    ProtoChatRequest, ProtoChatResponse, ProtoChatStreamResponse, ProtoFinishReason,
    proto_chat_service_server::{ProtoChatService, ProtoChatServiceServer},
};
use ferrous_llm::{
    ChatRequest, StreamingProvider,
    openai::{OpenAIConfig, OpenAIProvider},
};
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status};
use tracing::info;

/// Extract and validate JWT token from request metadata
pub fn authenticate_request<T>(request: &Request<T>, jwt_config: &JwtConfig) -> Result<Claims> {
    // Get authorization header
    let auth_header = request
        .metadata()
        .get("authorization")
        .ok_or_else(|| anyhow!("Missing authorization header"))?;

    // Convert to string
    let auth_str = auth_header
        .to_str()
        .map_err(|_| anyhow!("Invalid authorization header format"))?;

    // Extract Bearer token
    if !auth_str.starts_with("Bearer ") {
        return Err(anyhow!("Authorization header must start with 'Bearer '"));
    }

    let token = &auth_str[7..]; // Remove "Bearer " prefix

    // Validate access token using shared function
    validate_access_token(token, jwt_config)
}

pub fn get_service(prompt_service: PromptService) -> ProtoChatServiceServer<PromptService> {
    ProtoChatServiceServer::new(prompt_service)
}

#[derive(Debug)]
pub struct PromptService {
    provider: OpenAIProvider,
    jwt_config: JwtConfig,
}

impl PromptService {
    pub fn new(jwt_config: Option<JwtConfig>) -> Self {
        let mut config = OpenAIConfig::new(
            std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            // "gpt-4o-2024-08-06",
            // "meta-llama/Llama-4-Maverick-17B-128E-Instruct",
            std::env::var("OPENAI_MODEL").unwrap_or_default(),
            // "Llama Maverick",
            // "deepseek-ai/DeepSeek-V3-0324",
        );
        config.base_url = Some("https://api.chat.nebul.io/v1".parse().unwrap());
        Self {
            provider: OpenAIProvider::new(config).expect("Failed to create OpenAI provider"),
            jwt_config: jwt_config.unwrap_or_default(),
        }
    }
}

type ChatResult<T> = Result<Response<T>, Status>;
type ChatStreamResult = Pin<Box<dyn Stream<Item = Result<ProtoChatStreamResponse, Status>> + Send>>;

#[tonic::async_trait]
impl ProtoChatService for PromptService {
    type ChatStreamStream = ChatStreamResult;

    async fn chat(&self, request: Request<ProtoChatRequest>) -> ChatResult<ProtoChatResponse> {
        authenticate_request(&request, &self.jwt_config)
            .map_err(|e| Status::unauthenticated(e.to_string()))?;
        info!("Received send_prompt request");
        // Return a single response
        Ok(Response::new(ProtoChatResponse {
            content: "Hello, world!".to_string(),
            usage: None,
            finish_reason: Some(ProtoFinishReason::FinishReasonStop.into()),
            metadata: None,
            tool_calls: vec![],
        }))

        // let request_inner = request.into_inner();

        // let messages = request_inner
        //     .messages
        //     .iter()
        //     .map(|msg| msg.clone().into())
        //     .collect();

        // let stream = self
        //     .provider
        //     .chat_stream(ChatRequest {
        //         messages,
        //         parameters: Default::default(),
        //         metadata: Default::default(),
        //     })
        //     .await
        //     .map_err(|e| Status::internal(e.to_string()))?;

        // Ok(Response::new(stream))
        // unimplemented!()
    }

    async fn chat_stream(
        &self,
        request: Request<ProtoChatRequest>,
    ) -> ChatResult<Self::ChatStreamStream> {
        authenticate_request(&request, &self.jwt_config)
            .map_err(|e| Status::unauthenticated(e.to_string()))?;
        info!("Received chat_stream request");
        let request_inner = request.into_inner();

        let messages = request_inner
            .messages
            .iter()
            .map(|msg| msg.clone().into())
            .collect();

        let chat_request = ChatRequest {
            messages,
            parameters: Default::default(),
            metadata: Default::default(),
        };

        let openai_stream = self
            .provider
            .chat_stream(chat_request)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let output_stream = openai_stream.map(|result| {
            match result {
                Ok(content) => {
                    // Check if this is the final chunk (empty content typically indicates end)
                    let is_final = content.is_empty();

                    Ok(ProtoChatStreamResponse {
                        content,
                        is_final,
                        usage: None, // Usage info typically only available in final chunk
                        finish_reason: if is_final {
                            Some(ProtoFinishReason::FinishReasonStop.into())
                        } else {
                            None
                        },
                        metadata: None,
                        tool_calls: vec![],
                    })
                }
                Err(e) => Err(Status::internal(e.to_string())),
            }
        });

        Ok(Response::new(
            Box::pin(output_stream) as Self::ChatStreamStream
        ))
    }
}
