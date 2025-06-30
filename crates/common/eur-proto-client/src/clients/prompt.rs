use crate::get_secure_channel;
use anyhow::{Result, anyhow};
use eur_proto::proto_prompt_service::proto_prompt_service_client::ProtoPromptServiceClient;
pub use eur_proto::proto_prompt_service::{
    ProtoChatMessage, SendPromptRequest, SendPromptResponse,
    proto_prompt_service_server::ProtoPromptService,
};
use tonic::{Streaming, transport::Channel};
use tracing::info;

#[derive(Clone)]
pub struct PromptClient {
    base_url: String,
}

impl PromptClient {
    pub async fn new(base_url: Option<String>) -> Result<Self> {
        let base_url = base_url.unwrap_or(
            std::env::var("API_BASE_URL").unwrap_or("http://localhost:50051".to_string()),
        );
        Ok(Self { base_url })
    }
}

impl PromptClient {
    pub async fn try_init_client(&self) -> Result<Option<ProtoPromptServiceClient<Channel>>> {
        let channel = get_secure_channel(self.base_url.clone())
            .await?
            .ok_or_else(|| anyhow!("Failed to initialize prompt channel"))?;

        let client = ProtoPromptServiceClient::new(channel);

        info!("Connected to prompt service at {}", self.base_url);
        Ok(Some(client))
    }

    pub async fn send_prompt(
        &self,
        request: SendPromptRequest,
    ) -> Result<Streaming<SendPromptResponse>> {
        let mut client = self
            .try_init_client()
            .await?
            .ok_or_else(|| anyhow!("Failed to initialize prompt client"))?;

        let stream = client.send_prompt(request).await?.into_inner();
        Ok(stream)
    }
}
