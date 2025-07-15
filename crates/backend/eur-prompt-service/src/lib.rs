use anyhow::{Result, anyhow};
use eur_auth::{Claims, JwtConfig, validate_access_token};
use ferrous_llm::{
    ChatRequest, Message, MessageContent, Role, StreamingProvider,
    openai::{OpenAIConfig, OpenAIProvider},
};
// use eur_proto::proto_prompt_service::{
//     ProtoChatMessage, SendPromptRequest, SendPromptResponse,
//     proto_prompt_service_server::ProtoPromptService,
// };
use std::pin::Pin;
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status};
use tracing::info;

mod chat {
    tonic::include_proto!("eurora.chat");
}

use chat::proto_chat_service_server::ProtoChatService;

use crate::chat::{ProtoChatRequest, ProtoChatResponse};

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

#[derive(Debug)]
pub struct PromptService {
    provider: OpenAIProvider,
    jwt_config: JwtConfig,
}

impl PromptService {
    pub fn new(jwt_config: Option<JwtConfig>) -> Self {
        let config = OpenAIConfig::new(
            std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            "gpt-4o-2024-11-20",
        );
        Self {
            provider: OpenAIProvider::new(config).expect("Failed to create OpenAI provider"),
            jwt_config: jwt_config.unwrap_or_default(),
        }
    }
}

type SendPromptResult<T> = Result<Response<T>, Status>;
type SendPromptResponseStream =
    Pin<Box<dyn Stream<Item = Result<SendPromptResponse, Status>> + Send>>;

#[tonic::async_trait]
impl ProtoChatService for PromptService {
    type ChatStreamStream = SendPromptResponseStream;

    async fn chat(&self, request: Request<ProtoChatRequest>) -> Result<ProtoChatResponse> {
        authenticate_request(&request, &self.jwt_config)
            .map_err(|e| Status::unauthenticated(e.to_string()))?;
        info!("Received send_prompt request");
        let request_inner = request.into_inner();

        let messages = to_llm_message(request_inner.messages);

        let stream = self
            .provider
            .chat(ChatRequest {
                messages,
                parameters: Default::default(),
                metadata: Default::default(),
            })
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(Box::pin(stream) as Self::ChatStreamStream))
    }

    async fn chat_stream(
        &self,
        request: Request<ProtoChatRequest>,
    ) -> SendPromptResult<Self::ChatStreamStream> {
        authenticate_request(&request, &self.jwt_config)
            .map_err(|e| Status::unauthenticated(e.to_string()))?;
        info!("Received send_prompt request");
        let request_inner = request.into_inner();

        let messages = to_llm_message(request_inner.messages);

        let stream = self
            .provider
            .chat_stream(ChatRequest {
                messages,
                parameters: Default::default(),
                metadata: Default::default(),
            })
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Direct stream mapping - much simpler than channel bridging
        let output_stream = stream.map(|result| {
            result
                .map(|message| SendPromptResponse { response: message })
                .map_err(|e| Status::internal(e.to_string()))
        });

        Ok(Response::new(
            Box::pin(output_stream) as Self::SendPromptStream
        ))
    }
}

fn to_llm_message(messages: Vec<ProtoChatMessage>) -> Vec<Message> {
    messages
        .into_iter()
        .map(|proto_message| Message {
            role: match proto_message.role.as_str() {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                "system" => Role::System,
                _ => Role::User,
            },
            content: MessageContent::Text(proto_message.content),
        })
        .collect()
}
