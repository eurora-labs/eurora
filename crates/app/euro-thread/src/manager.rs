use crate::error::{Error, Result};
use agent_chain::messages::{ContentBlock, ContentBlocks};
use agent_chain_core::proto::{BaseMessageWithSibling, ChatStreamResponse, ProtoContentBlock};
use euro_auth::{AuthManager, AuthedChannel, build_authed_channel};
use proto_gen::thread::{
    ChatStreamRequest, CreateThreadRequest, DeleteThreadRequest, GenerateThreadTitleRequest,
    GetMessageTreeRequest, GetMessageTreeResponse, GetMessagesRequest, GetMessagesResponse,
    GetThreadRequest, ListThreadsRequest, ProtoThread, SavePreliminaryContentBlocksRequest,
    SearchMessagesRequest, SearchMessagesResponse, SearchThreadsRequest, SearchThreadsResponse,
    SwitchBranchRequest, proto_thread_service_client::ProtoThreadServiceClient,
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
            .max_decoding_message_size(1024 * 1024 * 1024)
            .max_encoding_message_size(1024 * 1024 * 1024)
    }

    pub async fn create(&self, request: CreateThreadRequest) -> Result<ProtoThread> {
        let mut client = self.client();
        let response = client.create_thread(request).await?.into_inner();
        if let Some(thread) = response.thread {
            Ok(thread)
        } else {
            Err(Error::CreateThread(
                "Server did not return the saved thread".to_string(),
            ))
        }
    }

    pub async fn list_threads(&self, request: ListThreadsRequest) -> Result<Vec<ProtoThread>> {
        let mut client = self.client();
        let response = client.list_threads(request).await?.into_inner();

        Ok(response.threads.into_iter().collect())
    }

    pub async fn get_current_messages(
        &self,
        request: GetMessagesRequest,
    ) -> Result<Vec<BaseMessageWithSibling>> {
        let mut client = self.client();

        let response = client.get_messages(request).await?.into_inner();
        Ok(response.messages)

        // Ok(response
        //     .messages
        //     .into_iter()
        //     .map(AnyMessage::from)
        //     .collect())
    }

    pub async fn delete_thread(&self, thread_id: String) -> Result<()> {
        let mut client = self.client();
        client
            .delete_thread(DeleteThreadRequest { thread_id })
            .await?;
        Ok(())
    }

    pub async fn get_thread(&self, thread_id: String) -> Result<ProtoThread> {
        let mut client = self.client();
        let response = client
            .get_thread(GetThreadRequest { thread_id })
            .await?
            .into_inner();

        if let Some(thread) = response.thread {
            Ok(thread)
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
    ) -> Result<ProtoThread> {
        let mut client = self.client();
        let response = client
            .generate_thread_title(GenerateThreadTitleRequest { thread_id, content })
            .await?
            .into_inner();

        match response.thread {
            Some(thread) => Ok(thread),
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
    pub async fn save_preliminary_content_blocks(
        &mut self,
        thread_id: String,
        blocks: ContentBlocks,
    ) -> Result<ContentBlocks> {
        let mut client = self.client();
        let proto_blocks: Vec<ProtoContentBlock> =
            blocks.into_inner().into_iter().map(|b| b.into()).collect();

        let response = client
            .save_preliminary_content_blocks(SavePreliminaryContentBlocksRequest {
                thread_id,
                content_blocks: proto_blocks,
            })
            .await?
            .into_inner();

        let content_blocks: Vec<ContentBlock> = response
            .content_blocks
            .into_iter()
            .map(ContentBlock::from)
            .collect();

        Ok(content_blocks.into())
    }
}

impl ThreadManager {
    pub async fn chat_stream(
        &mut self,
        thread_id: String,
        content_blocks: ContentBlocks,
        parent_message_id: Option<String>,
        asset_chips_json: Option<String>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatStreamResponse>> + Send>>> {
        let mut client = self.client();
        let proto_blocks: Vec<ProtoContentBlock> = content_blocks
            .into_inner()
            .into_iter()
            .map(|b| b.into())
            .collect();
        let stream = client
            .chat_stream(ChatStreamRequest {
                thread_id,
                content_blocks: proto_blocks,
                parent_message_id,
                asset_chips_json,
            })
            .await?
            .into_inner();

        let mapped_stream = stream.map(|result| result.map_err(Error::from));

        Ok(Box::pin(mapped_stream))
    }
}
