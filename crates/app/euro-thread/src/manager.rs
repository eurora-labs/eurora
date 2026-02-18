use crate::{
    Thread,
    error::{Error, Result},
    types::ThreadEvent,
};
use agent_chain::{BaseMessage, HumanMessage, SystemMessage};
use euro_auth::{AuthManager, AuthedChannel, build_authed_channel};
use proto_gen::agent_chain::{ProtoHumanMessage, ProtoSystemMessage};
use proto_gen::thread::{
    AddHiddenHumanMessageRequest, AddHumanMessageRequest, AddSystemMessageRequest,
    ChatStreamRequest, CreateThreadRequest, GenerateThreadTitleRequest, GetMessagesRequest,
    GetThreadRequest, ListThreadsRequest, proto_thread_service_client::ProtoThreadServiceClient,
};
use std::pin::Pin;
use tokio::sync::{broadcast, watch};
use tokio_stream::{Stream, StreamExt};
use tonic::transport::Channel;

pub struct ThreadManager {
    channel_rx: watch::Receiver<Channel>,
    auth_manager: AuthManager,
    current_thread: Thread,
    thread_event_tx: broadcast::Sender<ThreadEvent>,
}

impl ThreadManager {
    pub fn new(channel_rx: watch::Receiver<Channel>) -> Self {
        let auth_manager = AuthManager::new(channel_rx.clone());
        let (thread_event_tx, _) = broadcast::channel(100);

        Self {
            channel_rx,
            auth_manager,
            current_thread: Thread::default(),
            thread_event_tx,
        }
    }

    fn client(&self) -> ProtoThreadServiceClient<AuthedChannel> {
        let channel = self.channel_rx.borrow().clone();
        let authed = build_authed_channel(channel, self.auth_manager.clone());
        ProtoThreadServiceClient::new(authed)
    }

    pub async fn create_empty_thread(&mut self) -> Result<&Thread> {
        self.current_thread = Thread::default();
        Ok(&self.current_thread)
    }

    pub async fn switch_thread(&mut self, thread_id: String) -> Result<&Thread> {
        let mut client = self.client();
        let thread = client
            .get_thread(GetThreadRequest { thread_id })
            .await?
            .into_inner()
            .thread
            .ok_or(Error::ThreadNotFound)?;

        self.current_thread = thread.into();

        Ok(&self.current_thread)
    }

    pub async fn clear_thread(&mut self) -> Result<&Thread> {
        self.current_thread = Thread::default();

        self.thread_event_tx.send(ThreadEvent::NewThread {
            id: self.current_thread.id(),
            title: self.current_thread.title().to_string(),
        })?;

        Ok(&self.current_thread)
    }

    pub async fn get_current_thread(&self) -> &Thread {
        &self.current_thread
    }

    pub async fn save_current_thread(&mut self, request: CreateThreadRequest) -> Result<Thread> {
        let mut client = self.client();
        let response = client.create_thread(request).await?.into_inner();
        if let Some(thread) = response.thread {
            if self.current_thread.id().is_none() {
                self.current_thread
                    .set_id(uuid::Uuid::parse_str(&thread.id).unwrap())?;
            }
            Ok(thread.into())
        } else {
            Err(Error::CreateThread(
                "Server did not return the saved thread".to_string(),
            ))
        }
    }

    pub async fn ensure_remote_thread(&mut self) -> Result<Thread> {
        if self.current_thread.id().is_none() {
            let request = CreateThreadRequest::default();
            let thread = self.save_current_thread(request).await?;
            return Ok(thread);
        }

        Ok(self.current_thread.clone())
    }

    pub async fn list_threads(&self, request: ListThreadsRequest) -> Result<Vec<Thread>> {
        let mut client = self.client();
        let response = client.list_threads(request).await?.into_inner();

        Ok(response.threads.into_iter().map(Thread::from).collect())
    }

    pub async fn get_current_messages(&self, limit: u32, offset: u32) -> Result<Vec<BaseMessage>> {
        let mut client = self.client();
        let id = self.current_thread.id().ok_or(Error::InvalidThreadId)?;

        let response = client
            .get_messages(GetMessagesRequest {
                thread_id: id.to_string(),
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
    ) -> Result<Vec<BaseMessage>> {
        let mut client = self.client();
        let response = client
            .get_messages(GetMessagesRequest {
                thread_id,
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
}

impl ThreadManager {
    pub async fn add_human_message(&mut self, message: &HumanMessage) -> Result<()> {
        let mut client = self.client();
        let proto_message: ProtoHumanMessage = message.clone().into();
        let thread_id = self.current_thread.id().ok_or(Error::InvalidThreadId)?;

        client
            .add_human_message(AddHumanMessageRequest {
                thread_id: thread_id.to_string(),
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
                thread_id: self.current_thread.id().unwrap().to_string(),
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
                thread_id: self.current_thread.id().unwrap().to_string(),
                message: Some(proto_message),
            })
            .await?;
        Ok(())
    }
}

impl ThreadManager {
    pub async fn chat_stream(
        &mut self,
        content: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        let mut client = self.client();
        let stream = client
            .chat_stream(ChatStreamRequest {
                thread_id: self.current_thread.id().unwrap().to_string(),
                content,
            })
            .await?
            .into_inner();

        let mapped_stream =
            stream.map(|result| result.map(|response| response.chunk).map_err(Error::from));

        Ok(Box::pin(mapped_stream))
    }
}
