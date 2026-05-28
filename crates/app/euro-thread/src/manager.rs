//! Desktop-side client for the Eurora thread HTTP / WebSocket service.
//!
//! Mirrors `euro-activity::ActivityStorage` in shape: a single
//! [`EndpointManager`] gives the live base URL, an [`AuthManager`] supplies
//! bearer tokens, and a shared [`reqwest::Client`] handles the JSON request
//! / response round-trips. Streaming chat is delegated to
//! [`crate::chat_bridge::ChatBridge`] via the [`ChatSocket`] returned from
//! [`ThreadManager::open_chat_socket`].

use std::sync::Arc;

use euro_auth::AuthManager;
use euro_endpoint::EndpointManager;
use reqwest::header;
use secrecy::ExposeSecret;
use serde::de::DeserializeOwned;
use serde::ser::Serialize;
use thread_core::{
    CreateThreadRequest, CreateThreadResponse, DeleteThreadResponse, GenerateThreadTitleRequest,
    GenerateThreadTitleResponse, GetMessagesQuery, GetMessagesResponse, GetThreadResponse,
    ListThreadsQuery, ListThreadsResponse, MessageNode, SearchMessagesQuery,
    SearchMessagesResponse, SearchThreadsQuery, SearchThreadsResponse, SwitchBranchRequest, Thread,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::chat_socket::ChatSocket;
use crate::error::{Error, Result};

/// HTTP / WebSocket client for the thread service.
///
/// Cheap to clone: holds an `Arc<EndpointManager>`, an `AuthManager` (itself
/// `Arc` internally), and a `reqwest::Client` (which uses internal
/// reference counting).
#[derive(Clone)]
pub struct ThreadManager {
    endpoint_manager: Arc<EndpointManager>,
    auth_manager: AuthManager,
    http: reqwest::Client,
}

impl ThreadManager {
    pub fn new(endpoint_manager: Arc<EndpointManager>, auth_manager: AuthManager) -> Self {
        let http = endpoint_manager.client();
        Self {
            endpoint_manager,
            auth_manager,
            http,
        }
    }

    fn url(&self, path: &str) -> reqwest::Url {
        self.endpoint_manager.url(path)
    }

    fn ws_url(&self, path: &str) -> Result<reqwest::Url> {
        let mut url = self.endpoint_manager.url(path);
        let new_scheme = match url.scheme() {
            "https" => "wss",
            "http" => "ws",
            other => {
                return Err(Error::InvalidUrl(format!(
                    "endpoint URL has unsupported scheme for WebSocket: {other}"
                )));
            }
        };
        url.set_scheme(new_scheme)
            .map_err(|()| Error::InvalidUrl(format!("Failed to switch scheme to {new_scheme}")))?;
        Ok(url)
    }

    async fn bearer(&self) -> Result<String> {
        let token = self
            .auth_manager
            .get_or_refresh_access_token()
            .await
            .map_err(|e| Error::Auth(e.to_string()))?;
        Ok(format!("Bearer {}", token.expose_secret()))
    }

    async fn get_json<R: DeserializeOwned>(&self, path: &str) -> Result<R> {
        let bearer = self.bearer().await?;
        let response = self
            .http
            .get(self.url(path))
            .header(header::AUTHORIZATION, bearer)
            .send()
            .await?;
        decode(response).await
    }

    async fn get_json_query<Q: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        query: &Q,
    ) -> Result<R> {
        let bearer = self.bearer().await?;
        let response = self
            .http
            .get(self.url(path))
            .header(header::AUTHORIZATION, bearer)
            .query(query)
            .send()
            .await?;
        decode(response).await
    }

    async fn post_json<B: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<R> {
        let bearer = self.bearer().await?;
        let response = self
            .http
            .post(self.url(path))
            .header(header::AUTHORIZATION, bearer)
            .json(body)
            .send()
            .await?;
        decode(response).await
    }

    async fn delete<R: DeserializeOwned>(&self, path: &str) -> Result<R> {
        let bearer = self.bearer().await?;
        let response = self
            .http
            .delete(self.url(path))
            .header(header::AUTHORIZATION, bearer)
            .send()
            .await?;
        decode(response).await
    }

    pub async fn create(&self, title: Option<String>) -> Result<Thread> {
        let body = CreateThreadRequest { title };
        let response: CreateThreadResponse = self.post_json("/threads", &body).await?;
        Ok(response.thread)
    }

    pub async fn list_threads(&self, limit: u32, offset: u32) -> Result<Vec<Thread>> {
        let query = ListThreadsQuery {
            limit: Some(limit),
            offset: Some(offset),
        };
        let response: ListThreadsResponse = self.get_json_query("/threads", &query).await?;
        Ok(response.threads)
    }

    pub async fn list_threads_for_activity(
        &self,
        activity_id: Uuid,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Thread>> {
        let query = ListThreadsQuery {
            limit: Some(limit),
            offset: Some(offset),
        };
        let response: ListThreadsResponse = self
            .get_json_query(&format!("/threads/by-activity/{activity_id}"), &query)
            .await?;
        Ok(response.threads)
    }

    pub async fn get_thread(&self, thread_id: Uuid) -> Result<Thread> {
        let response: GetThreadResponse = self.get_json(&format!("/threads/{thread_id}")).await?;
        Ok(response.thread)
    }

    pub async fn delete_thread(&self, thread_id: Uuid) -> Result<()> {
        let _: DeleteThreadResponse = self.delete(&format!("/threads/{thread_id}")).await?;
        Ok(())
    }

    pub async fn get_messages(
        &self,
        thread_id: Uuid,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<MessageNode>> {
        let query = GetMessagesQuery {
            limit: Some(limit),
            offset: Some(offset),
        };
        let response: GetMessagesResponse = self
            .get_json_query(&format!("/threads/{thread_id}/messages"), &query)
            .await?;
        Ok(response.messages)
    }

    pub async fn switch_branch(
        &self,
        thread_id: Uuid,
        message_id: Uuid,
        direction: i32,
    ) -> Result<Vec<MessageNode>> {
        let body = SwitchBranchRequest {
            message_id,
            direction,
        };
        let response: GetMessagesResponse = self
            .post_json(
                &format!("/threads/{thread_id}/messages/switch-branch"),
                &body,
            )
            .await?;
        Ok(response.messages)
    }

    pub async fn generate_thread_title(&self, thread_id: Uuid) -> Result<Thread> {
        let body = GenerateThreadTitleRequest::default();
        let response: GenerateThreadTitleResponse = self
            .post_json(&format!("/threads/{thread_id}/title"), &body)
            .await?;
        Ok(response.thread)
    }

    pub async fn search_threads(
        &self,
        query: String,
        limit: u32,
        offset: u32,
    ) -> Result<SearchThreadsResponse> {
        let query = SearchThreadsQuery {
            q: query,
            limit: Some(limit),
            offset: Some(offset),
        };
        self.get_json_query("/threads/search", &query).await
    }

    pub async fn search_messages(
        &self,
        query: String,
        limit: u32,
        offset: u32,
    ) -> Result<SearchMessagesResponse> {
        let query = SearchMessagesQuery {
            q: query,
            limit: Some(limit),
            offset: Some(offset),
        };
        self.get_json_query("/threads/messages/search", &query)
            .await
    }

    /// Open a bidirectional chat WebSocket for `thread_id`.
    ///
    /// The handshake authenticates with a fresh bearer; the returned
    /// [`ChatSocket`] owns the multiplexing driver task and exposes
    /// typed `recv` / `try_send` halves. The caller — typically
    /// [`crate::chat_bridge::ChatBridge`] — is responsible for sending
    /// the per-turn `CapabilityUpdate` and opening frame and driving the
    /// inbound loop.
    pub async fn open_chat_socket(
        &self,
        thread_id: Uuid,
        cancel: CancellationToken,
    ) -> Result<ChatSocket> {
        let url = self.ws_url(&format!("/threads/{thread_id}/chat"))?;
        let bearer = self.bearer().await?;
        ChatSocket::connect(url, bearer, cancel).await
    }
}

async fn decode<R: DeserializeOwned>(response: reqwest::Response) -> Result<R> {
    let status = response.status();
    if status.is_success() {
        let body = response.text().await?;
        return serde_json::from_str(&body).map_err(Error::Decode);
    }
    let body = response.text().await.unwrap_or_default();
    Err(Error::from_response(status, &body))
}
