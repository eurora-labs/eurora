use anyhow::{Result, anyhow};
use eur_auth::{Claims, JwtConfig, validate_access_token};
use eur_prompt_kit::{EurLLMService, LLMMessage, PromptKitService, RemoteConfig};

use eur_proto::proto_prompt_service::{
    ProtoChatMessage, SendPromptRequest, SendPromptResponse,
    proto_prompt_service_server::ProtoPromptService,
};
use std::pin::Pin;
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

#[derive(Debug, Default)]
pub struct PromptService {
    prompt_service: PromptKitService,
    jwt_config: JwtConfig,
}

impl PromptService {
    pub fn new(jwt_config: Option<JwtConfig>) -> Self {
        let mut prompt_service = PromptKitService::default();
        prompt_service
            .switch_to_remote(RemoteConfig {
                provider: EurLLMService::OpenAI,
                api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
                model: "gpt-4o-2024-11-20".to_string(),
            })
            .unwrap();
        Self {
            prompt_service,
            jwt_config: jwt_config.unwrap_or_default(),
        }
    }
}

type SendPromptResult<T> = Result<Response<T>, Status>;
type SendPromptResponseStream =
    Pin<Box<dyn Stream<Item = Result<SendPromptResponse, Status>> + Send>>;

#[tonic::async_trait]
impl ProtoPromptService for PromptService {
    type SendPromptStream = SendPromptResponseStream;

    async fn send_prompt(
        &self,
        request: Request<SendPromptRequest>,
    ) -> SendPromptResult<Self::SendPromptStream> {
        authenticate_request(&request, &self.jwt_config)
            .map_err(|e| Status::unauthenticated(e.to_string()))?;
        info!("Received send_prompt request");
        let request_inner = request.into_inner();

        let messages = to_llm_message(request_inner.messages);

        let stream = self
            .prompt_service
            .chat_stream(messages)
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

fn to_llm_message(messages: Vec<ProtoChatMessage>) -> Vec<LLMMessage> {
    messages.into_iter().map(|message| message.into()).collect()
}
