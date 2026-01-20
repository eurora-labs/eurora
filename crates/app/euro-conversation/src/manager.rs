use crate::{
    Conversation,
    error::{Error, Result},
    types::ConversationEvent,
};
use agent_chain::{BaseMessage, HumanMessage};
use euro_auth::{AuthedChannel, get_authed_channel};
use proto_gen::conversation::{
    AddHumanMessageRequest, ChatStreamRequest, CreateConversationRequest, GetConversationRequest,
    GetMessagesRequest, ListConversationsRequest,
    proto_conversation_service_client::ProtoConversationServiceClient,
};
use std::pin::Pin;
use tokio::sync::broadcast;
use tokio_stream::{Stream, StreamExt};

pub struct ConversationManager {
    current_conversation: Conversation,
    conversation_client: ProtoConversationServiceClient<AuthedChannel>,
    conversation_event_tx: broadcast::Sender<ConversationEvent>,
}

impl ConversationManager {
    pub async fn new() -> Self {
        let channel = get_authed_channel().await;
        let conversation_client = ProtoConversationServiceClient::new(channel.clone());
        let (conversation_event_tx, _) = broadcast::channel(100);

        Self {
            current_conversation: Conversation::default(),
            conversation_client,
            conversation_event_tx,
        }
    }

    pub async fn switch_conversation(&mut self, conversation_id: String) -> Result<&Conversation> {
        let mut client = self.conversation_client.clone();
        let conversation = client
            .get_conversation(GetConversationRequest { conversation_id })
            .await?
            .into_inner()
            .conversation
            .ok_or(Error::ConversationNotFound)?;

        self.current_conversation = conversation.into();

        // self.conversation_event_tx
        //     .send(ConversationEvent::NewConversation {
        //         id: self.current_conversation.id(),
        //         title: self.current_conversation.title().to_string(),
        //     })?;

        Ok(&self.current_conversation)
    }

    pub async fn clear_conversation(&mut self) -> Result<&Conversation> {
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

    pub async fn ensure_remote_conversation(&mut self) -> Result<()> {
        if self.current_conversation.id().is_none() {
            let request = CreateConversationRequest::default();
            self.save_current_conversation(request).await?;
        }
        Ok(())
    }

    pub async fn list_conversations(
        &self,
        request: ListConversationsRequest,
    ) -> Result<Vec<Conversation>> {
        let mut client = self.conversation_client.clone();
        let response = client.list_conversations(request).await?.into_inner();

        Ok(response
            .conversations
            .into_iter()
            .map(Conversation::from)
            .collect())
    }

    pub async fn get_current_messages(&self, limit: u32, offset: u32) -> Result<Vec<BaseMessage>> {
        let mut client = self.conversation_client.clone();
        let id = self
            .current_conversation
            .id()
            .ok_or(Error::InvalidConversationId)?;

        let response = client
            .get_messages(GetMessagesRequest {
                conversation_id: id.to_string(),
                limit,
                offset,
            })
            .await?
            .into_inner();

        Ok(response
            .messages
            .into_iter()
            .map(BaseMessage::from)
            .collect())
    }

    pub async fn get_conversation(&self, conversation_id: String) -> Result<Conversation> {
        let mut client = self.conversation_client.clone();
        let response = client
            .get_conversation(GetConversationRequest { conversation_id })
            .await?
            .into_inner();

        if let Some(conversation) = response.conversation {
            Ok(conversation.into())
        } else {
            Err(Error::ConversationNotFound)
        }
    }
}

impl ConversationManager {
    pub async fn add_human_message(&mut self, message: &HumanMessage) -> Result<()> {
        let mut client = self.conversation_client.clone();
        client
            .add_human_message(AddHumanMessageRequest {
                conversation_id: self.current_conversation.id().unwrap().to_string(),
                content: message.content().to_string(),
            })
            .await?;
        Ok(())
    }
}

impl ConversationManager {
    pub async fn chat_stream(
        &mut self,
        content: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        let mut client = self.conversation_client.clone();
        let stream = client
            .chat_stream(ChatStreamRequest {
                conversation_id: self.current_conversation.id().unwrap().to_string(),
                content,
            })
            .await?
            .into_inner();

        let mapped_stream =
            stream.map(|result| result.map(|response| response.chunk).map_err(Error::from));

        Ok(Box::pin(mapped_stream))
    }
}
