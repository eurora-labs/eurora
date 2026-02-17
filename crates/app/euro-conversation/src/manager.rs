use crate::{
    Conversation,
    error::{Error, Result},
    types::ConversationEvent,
};
use agent_chain::{BaseMessage, HumanMessage, SystemMessage};
use euro_auth::{AuthManager, AuthedChannel, build_authed_channel};
use proto_gen::agent_chain::{ProtoHumanMessage, ProtoSystemMessage};
use proto_gen::conversation::{
    AddHiddenHumanMessageRequest, AddHumanMessageRequest, AddSystemMessageRequest,
    ChatStreamRequest, CreateConversationRequest, GenerateConversationTitleRequest,
    GetConversationRequest, GetMessagesRequest, ListConversationsRequest,
    proto_conversation_service_client::ProtoConversationServiceClient,
};
use std::pin::Pin;
use tokio::sync::{broadcast, watch};
use tokio_stream::{Stream, StreamExt};
use tonic::transport::Channel;

pub struct ConversationManager {
    channel_rx: watch::Receiver<Channel>,
    auth_manager: AuthManager,
    current_conversation: Conversation,
    conversation_event_tx: broadcast::Sender<ConversationEvent>,
}

impl ConversationManager {
    pub fn new(channel_rx: watch::Receiver<Channel>) -> Self {
        let auth_manager = AuthManager::new(channel_rx.clone());
        let (conversation_event_tx, _) = broadcast::channel(100);

        Self {
            channel_rx,
            auth_manager,
            current_conversation: Conversation::default(),
            conversation_event_tx,
        }
    }

    fn client(&self) -> ProtoConversationServiceClient<AuthedChannel> {
        let channel = self.channel_rx.borrow().clone();
        let authed = build_authed_channel(channel, self.auth_manager.clone());
        ProtoConversationServiceClient::new(authed)
    }

    pub async fn create_empty_conversation(&mut self) -> Result<&Conversation> {
        self.current_conversation = Conversation::default();
        Ok(&self.current_conversation)
    }

    pub async fn switch_conversation(&mut self, conversation_id: String) -> Result<&Conversation> {
        let mut client = self.client();
        let conversation = client
            .get_conversation(GetConversationRequest { conversation_id })
            .await?
            .into_inner()
            .conversation
            .ok_or(Error::ConversationNotFound)?;

        self.current_conversation = conversation.into();

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
        let mut client = self.client();
        let response = client.create_conversation(request).await?.into_inner();
        if let Some(conversation) = response.conversation {
            if self.current_conversation.id().is_none() {
                self.current_conversation
                    .set_id(uuid::Uuid::parse_str(&conversation.id).unwrap())?;
            }
            Ok(conversation.into())
        } else {
            Err(Error::CreateConversation(
                "Server did not return the saved conversation".to_string(),
            ))
        }
    }

    pub async fn ensure_remote_conversation(&mut self) -> Result<Conversation> {
        if self.current_conversation.id().is_none() {
            let request = CreateConversationRequest::default();
            let conversation = self.save_current_conversation(request).await?;
            return Ok(conversation);
        }

        Ok(self.current_conversation.clone())
    }

    pub async fn list_conversations(
        &self,
        request: ListConversationsRequest,
    ) -> Result<Vec<Conversation>> {
        let mut client = self.client();
        let response = client.list_conversations(request).await?.into_inner();

        Ok(response
            .conversations
            .into_iter()
            .map(Conversation::from)
            .collect())
    }

    pub async fn get_current_messages(&self, limit: u32, offset: u32) -> Result<Vec<BaseMessage>> {
        let mut client = self.client();
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
        let mut client = self.client();
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

    pub async fn get_messages(
        &self,
        conversation_id: String,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<BaseMessage>> {
        let mut client = self.client();
        let response = client
            .get_messages(GetMessagesRequest {
                conversation_id,
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

    pub async fn generate_conversation_title(
        &self,
        conversation_id: String,
        content: String,
    ) -> Result<Conversation> {
        let mut client = self.client();
        let response = client
            .generate_conversation_title(GenerateConversationTitleRequest {
                conversation_id,
                content,
            })
            .await?
            .into_inner();

        match response.conversation {
            Some(conversation) => Ok(conversation.into()),
            None => Err(Error::UpdateConversation(
                "Conversation title could not be generated".to_string(),
            )),
        }
    }
}

impl ConversationManager {
    pub async fn add_human_message(&mut self, message: &HumanMessage) -> Result<()> {
        let mut client = self.client();
        let proto_message: ProtoHumanMessage = message.clone().into();
        let conversation_id = self
            .current_conversation
            .id()
            .ok_or(Error::InvalidConversationId)?;

        client
            .add_human_message(AddHumanMessageRequest {
                conversation_id: conversation_id.to_string(),
                message: Some(proto_message),
            })
            .await?;
        Ok(())
    }

    pub async fn add_hidden_human_message(&mut self, message: &HumanMessage) -> Result<()> {
        let mut client = self.client();
        let proto_message: ProtoHumanMessage = message.clone().into();
        client
            .add_hidden_human_message(AddHiddenHumanMessageRequest {
                conversation_id: self.current_conversation.id().unwrap().to_string(),
                message: Some(proto_message),
            })
            .await?;
        Ok(())
    }

    pub async fn add_system_message(&mut self, message: &SystemMessage) -> Result<()> {
        let mut client = self.client();
        let proto_message: ProtoSystemMessage = message.clone().into();
        client
            .add_system_message(AddSystemMessageRequest {
                conversation_id: self.current_conversation.id().unwrap().to_string(),
                message: Some(proto_message),
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
        let mut client = self.client();
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
