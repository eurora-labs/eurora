use crate::Conversation;
// use agent_chain_core::BaseMessage;
use euro_auth::{AuthedChannel, get_authed_channel};
use proto_gen::conversation::{
    CreateConversationRequest, CreateConversationResponse, ListConversationsRequest,
    ListConversationsResponse, proto_conversation_service_client::ProtoConversationServiceClient,
};
use tonic::Status;

pub struct ConversationManager {
    // conversation: Option<Conversation>,
    conversation_client: ProtoConversationServiceClient<AuthedChannel>,
    // chat_client: ProtoChatServiceClient<AuthedChannel>,
}

impl ConversationManager {
    pub async fn new() -> Self {
        let channel = get_authed_channel().await;
        let conversation_client = ProtoConversationServiceClient::new(channel.clone());
        // let chat_client = ProtoChatServiceClient::new(channel);

        Self {
            // conversation: None,
            conversation_client,
            // chat_client,
        }
    }

    pub async fn create_empty_conversation(&self) -> Result<Conversation, Status> {
        let conversation = Conversation::default();
        Ok(conversation)
    }

    pub async fn create_conversation(
        &self,
        request: CreateConversationRequest,
    ) -> Result<CreateConversationResponse, Status> {
        let mut client = self.conversation_client.clone();
        let response = client.create_conversation(request).await?.into_inner();
        Ok(response)
    }

    pub async fn list_conversations(
        &self,
        request: ListConversationsRequest,
    ) -> Result<ListConversationsResponse, Status> {
        let mut client = self.conversation_client.clone();
        let response = client.list_conversations(request).await?.into_inner();
        Ok(response)
    }

    // pub async fn list_messages(&self, limit: u32, offset: u32) -> Result<Vec<BaseMessage>, Status> {
    //     let mut client = self.conversation_client.clone();
    //     let response = client.list_messages().await?.into_inner();
    //     Ok(response)
    // }
}
