use crate::{
    Conversation,
    error::{Error, Result},
    types::ConversationEvent,
};
// use agent_chain_core::BaseMessage;
use euro_auth::{AuthedChannel, get_authed_channel};
use proto_gen::conversation::{
    CreateConversationRequest, ListConversationsRequest, ListConversationsResponse,
    proto_conversation_service_client::ProtoConversationServiceClient,
};
use tokio::sync::broadcast;

pub struct ConversationManager {
    current_conversation: Conversation,
    conversation_client: ProtoConversationServiceClient<AuthedChannel>,
    conversation_event_tx: broadcast::Sender<ConversationEvent>,
    // chat_client: ProtoChatServiceClient<AuthedChannel>,
}

impl ConversationManager {
    pub async fn new() -> Self {
        let channel = get_authed_channel().await;
        let conversation_client = ProtoConversationServiceClient::new(channel.clone());
        let (conversation_event_tx, _) = broadcast::channel(100);
        // let chat_client = ProtoChatServiceClient::new(channel);

        Self {
            current_conversation: Conversation::default(),
            conversation_client,
            conversation_event_tx,
            // chat_client,
        }
    }

    pub async fn create_new_conversation(&mut self) -> Result<&Conversation> {
        self.current_conversation = Conversation::default();

        self.conversation_event_tx
            .send(ConversationEvent::NewConversation {
                id: self.current_conversation.id(),
                title: self.current_conversation.title().to_string(),
            })?;

        Ok(&self.current_conversation)
    }

    pub async fn get_current_conversation(&self) -> &Conversation {
        &self.current_conversation
    }

    pub async fn save_current_conversation(
        &mut self,
        request: CreateConversationRequest,
    ) -> Result<Conversation> {
        let mut client = self.conversation_client.clone();
        let response = client.create_conversation(request).await?.into_inner();
        if let Some(conversation) = response.conversation {
            // Assign the id if it's not already set
            if self.current_conversation.id().is_none() {
                self.current_conversation
                    .set_id(uuid::Uuid::parse_str(&conversation.id).unwrap())?;
            }
            Ok(Conversation::default())
        } else {
            Err(Error::CreateConversation(
                "Server did not return the saved conversation".to_string(),
            ))
        }
    }

    pub async fn list_conversations(
        &self,
        request: ListConversationsRequest,
    ) -> Result<ListConversationsResponse> {
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
