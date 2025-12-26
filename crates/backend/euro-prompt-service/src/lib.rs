use std::pin::Pin;

use agent_chain::providers::openai::ChatOpenAI;
use agent_chain_core::chat_models::ChatModel;
use agent_chain_core::messages::BaseMessage;
use agent_chain_eurora::proto::chat::{
    ProtoAiMessage, ProtoAiMessageChunk, ProtoChatRequest, ProtoChatResponse,
    ProtoChatStreamResponse,
    proto_chat_service_server::{ProtoChatService, ProtoChatServiceServer},
};
use anyhow::{Result, anyhow};
use euro_auth::{Claims, JwtConfig, validate_access_token};

use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status};
use tracing::debug;

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
    provider: ChatOpenAI,
    jwt_config: JwtConfig,
}

impl PromptService {
    pub fn new(jwt_config: Option<JwtConfig>) -> Self {
        let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());

        let provider = ChatOpenAI::new(&model).api_key(api_key);

        Self {
            provider,
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
        debug!("Received chat request");

        let request_inner = request.into_inner();

        // Convert ProtoBaseMessage to agent_chain_core::BaseMessage
        let messages: Vec<BaseMessage> = request_inner
            .messages
            .into_iter()
            .map(|msg| msg.into())
            .collect();

        // Call the provider to generate a response
        let ai_message = self.provider.invoke(messages.into()).await.map_err(|e| {
            debug!("Error in chat: {}", e);
            Status::internal(e.to_string())
        })?;

        // Convert AIMessage to ProtoAiMessage
        let tool_calls: Vec<_> = ai_message
            .tool_calls()
            .iter()
            .map(|tc| agent_chain_eurora::proto::chat::ProtoToolCall {
                id: tc.id().to_string(),
                name: tc.name().to_string(),
                args: tc.args().to_string(),
            })
            .collect();

        Ok(Response::new(ProtoChatResponse {
            message: Some(ProtoAiMessage {
                content: ai_message.content().to_string(),
                id: ai_message.id().map(|s| s.to_string()),
                name: ai_message.name().map(|s| s.to_string()),
                tool_calls,
                invalid_tool_calls: vec![],
                usage_metadata: None,
                additional_kwargs: None,
                response_metadata: None,
            }),
            usage: None,
            stop_reason: Some("stop".to_string()),
        }))
    }

    async fn chat_stream(
        &self,
        request: Request<ProtoChatRequest>,
    ) -> ChatResult<Self::ChatStreamStream> {
        authenticate_request(&request, &self.jwt_config)
            .map_err(|e| Status::unauthenticated(e.to_string()))?;
        debug!("Received chat_stream request");
        let request_inner = request.into_inner();

        // Convert ProtoBaseMessage to agent_chain_core::BaseMessage
        let messages: Vec<BaseMessage> = request_inner
            .messages
            .into_iter()
            .map(|msg| msg.into())
            .collect();

        let openai_stream = self
            .provider
            .astream(messages.into(), None)
            .await
            .map_err(|e| {
                debug!("Error in chat_stream: {}", e);
                Status::internal(e.to_string())
            })?;

        let output_stream = openai_stream.map(|result| match result {
            Ok(chunk) => {
                // AIMessageChunk has content() method for getting the text content
                // We determine finality by empty content or chunk_position
                let content = chunk.content().to_string();
                debug!("Received chunk: {}", content);
                let is_final = content.is_empty();

                Ok(ProtoChatStreamResponse {
                    chunk: Some(ProtoAiMessageChunk {
                        content,
                        id: None,
                        name: None,
                        tool_calls: vec![],
                        invalid_tool_calls: vec![],
                        tool_call_chunks: vec![],
                        usage_metadata: None,
                        additional_kwargs: None,
                        response_metadata: None,
                        chunk_position: None,
                    }),
                    is_final,
                })
            }
            Err(e) => Err(Status::internal(e.to_string())),
        });

        Ok(Response::new(
            Box::pin(output_stream) as Self::ChatStreamStream
        ))
    }
}
