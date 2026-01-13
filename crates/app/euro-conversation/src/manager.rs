use euro_auth::{AuthedChannel, get_authed_channel};
use proto_gen::conversation::{
    CreateConversationRequest, CreateConversationResponse,
    proto_conversation_service_client::ProtoConversationServiceClient,
};
use tonic::Status;

pub struct ConversationManager {
    client: ProtoConversationServiceClient<AuthedChannel>,
}

impl ConversationManager {
    pub async fn new() -> Self {
        let channel = get_authed_channel().await;
        let client = ProtoConversationServiceClient::new(channel);
        Self { client }
    }

    pub async fn create_conversation(
        &self,
        request: CreateConversationRequest,
    ) -> Result<CreateConversationResponse, Status> {
        let mut client = self.client.clone();
        let response = client.create_conversation(request).await?.into_inner();
        Ok(response)
    }
}
