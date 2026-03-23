use crate::{
    Thread,
    error::{Error, Result},
};
use agent_chain::{AnyMessage, HumanMessage, SystemMessage, messages::AIMessageChunk};
use euro_auth::{AuthManager, AuthedChannel, build_authed_channel};
use proto_gen::agent_chain::{ProtoHumanMessage, ProtoSystemMessage};
use proto_gen::thread::{
    AddHiddenHumanMessageRequest, AddHumanMessageRequest, AddSystemMessageRequest,
    ChatStreamRequest, CreateThreadRequest, DeleteThreadRequest, GenerateThreadTitleRequest,
    GetMessageTreeRequest, GetMessageTreeResponse, GetMessagesRequest, GetMessagesResponse,
    GetThreadRequest, ListThreadsRequest, SearchMessagesRequest, SearchMessagesResponse,
    SearchThreadsRequest, SearchThreadsResponse, SwitchBranchRequest,
    proto_thread_service_client::ProtoThreadServiceClient,
};
use std::pin::Pin;
use tokio::sync::watch;
use tokio_stream::{Stream, StreamExt};
use tonic::transport::Channel;

pub struct ThreadManager {
    channel_rx: watch::Receiver<Channel>,
    auth_manager: AuthManager,
}

impl ThreadManager {
    pub fn new(channel_rx: watch::Receiver<Channel>) -> Self {
        let auth_manager = AuthManager::new(channel_rx.clone());

        Self {
            channel_rx,
            auth_manager,
        }
    }

    fn client(&self) -> ProtoThreadServiceClient<AuthedChannel> {
        let channel = self.channel_rx.borrow().clone();
        let authed = build_authed_channel(channel, self.auth_manager.clone());
        ProtoThreadServiceClient::new(authed)
    }

    pub async fn create(&self, request: CreateThreadRequest) -> Result<Thread> {
        let mut client = self.client();
        let response = client.create_thread(request).await?.into_inner();
        if let Some(thread) = response.thread {
            Ok(thread.into())
        } else {
            Err(Error::CreateThread(
                "Server did not return the saved thread".to_string(),
            ))
        }
    }

    pub async fn list_threads(&self, request: ListThreadsRequest) -> Result<Vec<Thread>> {
        let mut client = self.client();
        let response = client.list_threads(request).await?.into_inner();

        Ok(response.threads.into_iter().map(Thread::from).collect())
    }

    pub async fn get_current_messages(
        &self,
        request: GetMessagesRequest,
    ) -> Result<Vec<AnyMessage>> {
        let mut client = self.client();

        let response = client.get_messages(request).await?.into_inner();

        Ok(response
            .messages
            .into_iter()
            .map(AnyMessage::from)
            .collect())
    }

    pub async fn delete_thread(&self, thread_id: String) -> Result<()> {
        let mut client = self.client();
        client
            .delete_thread(DeleteThreadRequest { thread_id })
            .await?;
        Ok(())
    }

    pub async fn get_thread(&self, thread_id: String) -> Result<Thread> {
        let mut client = self.client();
        let response = client
            .get_thread(GetThreadRequest { thread_id })
            .await?
            .into_inner();

        if let Some(thread) = response.thread {
            Ok(thread.into())
        } else {
            Err(Error::ThreadNotFound)
        }
    }

    pub async fn get_messages(
        &self,
        thread_id: String,
        limit: u32,
        offset: u32,
    ) -> Result<GetMessagesResponse> {
        let mut client = self.client();
        let response = client
            .get_messages(GetMessagesRequest {
                thread_id,
                limit,
                offset,
            })
            .await?
            .into_inner();

        Ok(response)
    }

    pub async fn switch_branch(
        &self,
        thread_id: String,
        message_id: String,
        direction: i32,
    ) -> Result<GetMessagesResponse> {
        let mut client = self.client();
        let response = client
            .switch_branch(SwitchBranchRequest {
                thread_id,
                message_id,
                direction,
            })
            .await?
            .into_inner();

        Ok(response)
    }

    pub async fn get_message_tree(
        &self,
        thread_id: String,
        start_level: u32,
        end_level: u32,
        parent_node_ids: Vec<String>,
    ) -> Result<GetMessageTreeResponse> {
        let mut client = self.client();
        let response = client
            .get_message_tree(GetMessageTreeRequest {
                thread_id,
                start_level,
                end_level,
                parent_node_ids,
            })
            .await?
            .into_inner();

        Ok(response)
    }

    pub async fn generate_thread_title(
        &self,
        thread_id: String,
        content: String,
    ) -> Result<Thread> {
        let mut client = self.client();
        let response = client
            .generate_thread_title(GenerateThreadTitleRequest { thread_id, content })
            .await?
            .into_inner();

        match response.thread {
            Some(thread) => Ok(thread.into()),
            None => Err(Error::UpdateThread(
                "Thread title could not be generated".to_string(),
            )),
        }
    }

    pub async fn search_threads(
        &self,
        query: String,
        limit: u32,
        offset: u32,
    ) -> Result<SearchThreadsResponse> {
        let mut client = self.client();
        let response = client
            .search_threads(SearchThreadsRequest {
                query,
                limit,
                offset,
            })
            .await?
            .into_inner();

        Ok(response)
    }

    pub async fn search_messages(
        &self,
        query: String,
        limit: u32,
        offset: u32,
    ) -> Result<SearchMessagesResponse> {
        let mut client = self.client();
        let response = client
            .search_messages(SearchMessagesRequest {
                query,
                limit,
                offset,
            })
            .await?
            .into_inner();

        Ok(response)
    }
}

impl ThreadManager {
    pub async fn add_human_message(
        &mut self,
        thread_id: String,
        message: &HumanMessage,
    ) -> Result<()> {
        let mut client = self.client();
        let proto_message: ProtoHumanMessage = message.clone().into();

        client
            .add_human_message(AddHumanMessageRequest {
                thread_id,
                message: Some(proto_message),
            })
            .await?;
        Ok(())
    }

    pub async fn add_hidden_human_message(
        &mut self,
        thread_id: String,
        message: &HumanMessage,
    ) -> Result<()> {
        let mut client = self.client();
        let proto_message: ProtoHumanMessage = message.clone().into();
        client
            .add_hidden_human_message(AddHiddenHumanMessageRequest {
                thread_id,
                message: Some(proto_message),
            })
            .await?;
        Ok(())
    }

    pub async fn add_system_message(
        &mut self,
        thread_id: String,
        message: &SystemMessage,
    ) -> Result<()> {
        let mut client = self.client();
        let proto_message: ProtoSystemMessage = message.clone().into();
        client
            .add_system_message(AddSystemMessageRequest {
                thread_id,
                message: Some(proto_message),
            })
            .await?;
        Ok(())
    }
}

impl ThreadManager {
    pub async fn chat_stream(
        &mut self,
        thread_id: String,
        content: String,
        parent_message_id: Option<String>,
        asset_chips_json: Option<String>,
        image_asset_ids_json: Option<String>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<AIMessageChunk>> + Send>>> {
        let mut client = self.client();
        let stream = client
            .chat_stream(ChatStreamRequest {
                thread_id,
                content,
                parent_message_id,
                asset_chips_json,
                image_asset_ids_json,
            })
            .await?
            .into_inner();

        let mapped_stream = stream.map(|result| match result {
            Ok(chunk) => Ok(AIMessageChunk::from(chunk)),
            Err(e) => Err(Error::from(e)),
        });

        Ok(Box::pin(mapped_stream))
    }
}
