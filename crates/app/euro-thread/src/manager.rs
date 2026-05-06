//! Desktop-side client for the Eurora thread HTTP / WebSocket service.
//!
//! Mirrors `euro-activity::ActivityStorage` in shape: a single
//! [`EndpointManager`] gives the live base URL, an [`AuthManager`] supplies
//! bearer tokens, and a shared [`reqwest::Client`] handles the JSON request
//! / response round-trips. Streaming chat goes over a WebSocket established
//! via `tokio-tungstenite`.

use std::sync::Arc;

use euro_auth::AuthManager;
use euro_endpoint::EndpointManager;
use euro_secret::ExposeSecret;
use futures::{SinkExt, StreamExt};
use reqwest::header;
use serde::de::DeserializeOwned;
use serde::ser::Serialize;
use thread_core::{
    ChatClientMessage, ChatSendRequest, ChatServerMessage, CreateThreadRequest,
    CreateThreadResponse, DeleteThreadResponse, GenerateThreadTitleRequest,
    GenerateThreadTitleResponse, GetMessagesQuery, GetMessagesResponse, GetThreadResponse,
    ListThreadsQuery, ListThreadsResponse, MessageNode, SearchMessagesQuery,
    SearchMessagesResponse, SearchThreadsQuery, SearchThreadsResponse, SwitchBranchRequest, Thread,
};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{Connector, MaybeTlsStream, WebSocketStream};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

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
        all_variants: bool,
    ) -> Result<Vec<MessageNode>> {
        let query = GetMessagesQuery {
            limit: Some(limit),
            offset: Some(offset),
            all_variants,
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

    /// Open a chat WebSocket and stream the turn back to the caller.
    ///
    /// Sends a single [`ChatClientMessage::Send`] frame, then yields each
    /// inbound [`ChatServerMessage`] on the returned receiver. The receiver
    /// closes when the server emits a `Final` or `Error` frame, or when
    /// `cancel` fires (which prompts a `Cancel` frame and a clean close).
    pub async fn chat_stream(
        &self,
        thread_id: Uuid,
        request: ChatSendRequest,
        cancel: CancellationToken,
    ) -> Result<mpsc::UnboundedReceiver<Result<ChatServerMessage>>> {
        let url = self.ws_url(&format!("/threads/{thread_id}/chat"))?;
        let bearer = self.bearer().await?;

        let mut req = url
            .as_str()
            .into_client_request()
            .map_err(|e| Error::InvalidUrl(e.to_string()))?;
        req.headers_mut().insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&bearer)
                .map_err(|e| Error::ChatProtocol(format!("Invalid bearer header: {e}")))?,
        );

        let (mut stream, _response) = tokio_tungstenite::connect_async_tls_with_config(
            req,
            None,
            false,
            None as Option<Connector>,
        )
        .await?;

        // Send the opening Send frame.
        let send_frame =
            serde_json::to_string(&ChatClientMessage::Send(request)).map_err(Error::Encode)?;
        stream.send(Message::Text(send_frame.into())).await?;

        let (tx, rx) = mpsc::unbounded_channel::<Result<ChatServerMessage>>();
        tokio::spawn(drive_chat(stream, tx, cancel));
        Ok(rx)
    }
}

async fn drive_chat(
    mut stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    tx: mpsc::UnboundedSender<Result<ChatServerMessage>>,
    cancel: CancellationToken,
) {
    loop {
        tokio::select! {
            biased;
            () = cancel.cancelled() => {
                let cancel_frame = serde_json::to_string(&ChatClientMessage::Cancel)
                    .expect("cancel frame serializes");
                let _ = stream.send(Message::Text(cancel_frame.into())).await;
                let _ = stream.close(None).await;
                let _ = tx.send(Err(Error::Cancelled));
                return;
            }
            msg = stream.next() => {
                let Some(msg) = msg else { return };
                let msg = match msg {
                    Ok(m) => m,
                    Err(e) => {
                        let _ = tx.send(Err(Error::WebSocket(e)));
                        return;
                    }
                };
                match msg {
                    Message::Text(text) => match serde_json::from_str::<ChatServerMessage>(&text) {
                        Ok(event) => {
                            let is_terminal = matches!(
                                &event,
                                ChatServerMessage::Final { .. } | ChatServerMessage::Error { .. }
                            );
                            if tx.send(Ok(event)).is_err() {
                                let _ = stream.close(None).await;
                                return;
                            }
                            if is_terminal {
                                let _ = stream.close(None).await;
                                return;
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(Err(Error::Decode(e)));
                            return;
                        }
                    },
                    Message::Close(_) => return,
                    Message::Ping(payload) => {
                        // Respond to pings to keep the connection alive; some proxies require this.
                        let _ = stream.send(Message::Pong(payload)).await;
                    }
                    Message::Binary(_) | Message::Pong(_) | Message::Frame(_) => {
                        // Ignore — the wire protocol is text-only.
                    }
                }
            }
        }
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
