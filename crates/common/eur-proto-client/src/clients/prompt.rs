use crate::get_secure_channel;
use anyhow::{Result, anyhow};
use eur_proto::proto_prompt_service::proto_prompt_service_client::ProtoPromptServiceClient;
pub use eur_proto::proto_prompt_service::{
    ProtoChatMessage, SendPromptRequest, SendPromptResponse,
    proto_prompt_service_server::ProtoPromptService,
};
use eur_secret::secret;
use tonic::{
    Request, Status, Streaming, metadata::MetadataValue, service::interceptor::InterceptedService,
    transport::Channel,
};
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
    pub async fn try_init_client(
        &self,
    ) -> Result<
        ProtoPromptServiceClient<
            InterceptedService<Channel, impl Fn(Request<()>) -> Result<Request<()>, Status>>,
        >,
    > {
        let channel = get_secure_channel(self.base_url.clone())
            .await?
            .ok_or_else(|| anyhow!("Failed to initialize prompt channel"))?;

        let access_token = secret::retrieve("AUTH_ACCESS_TOKEN", secret::Namespace::BuildKind)?;
        let token: MetadataValue<_> = format!("Bearer {}", access_token.unwrap().0).parse()?;
        let client =
            ProtoPromptServiceClient::with_interceptor(channel, move |mut req: Request<()>| {
                req.metadata_mut().insert("authorization", token.clone());
                Ok(req)
            });

        info!("Connected to prompt service at {}", self.base_url);
        Ok(client)
    }

    pub async fn send_prompt(
        &self,
        request: SendPromptRequest,
    ) -> Result<Streaming<SendPromptResponse>> {
        let mut client = self.try_init_client().await?;

        // Add timeout for the initial gRPC call
        let timeout_duration = std::time::Duration::from_secs(30);
        let stream = tokio::time::timeout(timeout_duration, client.send_prompt(request))
            .await
            .map_err(|_| anyhow!("gRPC call timed out after 30 seconds"))?
            .map_err(|e| anyhow!("gRPC call failed: {}", e))?
            .into_inner();

        Ok(stream)
    }
}
