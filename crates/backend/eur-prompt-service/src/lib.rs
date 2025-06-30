//! The Eurora Prompt service that provides gRPC endpoints for image transcription with JWT authentication.

use anyhow::{Result, anyhow};
use eur_auth::{Claims, JwtConfig, validate_access_token};
use eur_prompt_kit::{LLMMessage, PromptKitService, Role};

use eur_proto::proto_prompt_service::{
    ProtoChatMessage, SendPromptRequest, SendPromptResponse,
    proto_prompt_service_server::ProtoPromptService,
};
use std::{error::Error, io::ErrorKind, net::ToSocketAddrs, pin::Pin, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt, wrappers::ReceiverStream};
use tonic::{Request, Response, Status, Streaming, transport::Server};
use tracing::{info, warn};

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
        Self {
            prompt_service: PromptKitService::default(),
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
        let request_inner = request.into_inner();
        let messages = to_llm_message(request_inner.messages);

        let mut stream = self
            .prompt_service
            .chat_stream(messages)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let (tx, rx) = mpsc::channel(128);

        tokio::spawn(async move {
            while let Some(item) = stream.next().await {
                match item {
                    Ok(message) => {
                        tx.send(Ok(SendPromptResponse { response: message }))
                            .await
                            .unwrap();
                    }
                    Err(e) => {
                        warn!("LLM error: {}", e);
                        break;
                    }
                }
            }
        })
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

        let output_stream = ReceiverStream::new(rx);

        Ok(Response::new(
            Box::pin(output_stream) as Self::SendPromptStream
        ))
    }
}

fn to_llm_message(messages: Vec<ProtoChatMessage>) -> Vec<LLMMessage> {
    messages.into_iter().map(|message| message.into()).collect()
}
