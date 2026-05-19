//! Bidirectional chat-WebSocket handle.
//!
//! The chat protocol is bidirectional from day one: the client opens the
//! socket, sends `CapabilityUpdate` + `Send`/`Regenerate`, then exchanges
//! `ToolRequest` / `ToolResponse` frames alongside the streaming chat
//! events. This module owns the framing details — the user-facing
//! [`ChatSocket`] presents a typed read half ([`ChatSocket::recv`]) and a
//! cloneable write handle ([`ChatSocket::outbound_sender`]) on top of a
//! single background driver task that multiplexes both directions over
//! one `WebSocketStream`.
//!
//! [`ChatBridge`](crate::chat_bridge::ChatBridge) is the only production
//! consumer; tests build sockets via [`ChatSocket::test_pair`].

use futures::{SinkExt, StreamExt};
use reqwest::header;
use thread_core::{ChatClientMessage, ChatServerMessage};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{Connector, MaybeTlsStream, WebSocketStream};
use tokio_util::sync::CancellationToken;

use crate::error::{Error, Result};

/// Owned handle to a chat WebSocket.
///
/// Construct with [`ChatSocket::connect`] for production WS traffic or
/// [`ChatSocket::test_pair`] for unit tests that drive the protocol
/// directly. The background driver task exits naturally on:
///
/// - cancellation of the [`CancellationToken`] handed to `connect`,
/// - a terminal server frame (`Final` / `Error`), after delivering it,
/// - a transport-level error, after delivering it as `Err(_)`, or
/// - the consumer side dropping `ChatSocket` *and* every cloned
///   [`ChatOutbound`], which collapses both channels.
///
/// Dropping `ChatSocket` is non-blocking: it aborts the driver as a
/// safety net so a panic or early return on the consumer side can't
/// leak a parked task. In the steady-state lifecycle the driver has
/// already exited cleanly before the abort fires.
pub struct ChatSocket {
    inbound: mpsc::UnboundedReceiver<Result<ChatServerMessage>>,
    outbound: ChatOutbound,
    driver: Option<JoinHandle<()>>,
}

/// Cloneable write handle for the outbound side of a chat socket.
///
/// Spawned dispatch tasks hold a `ChatOutbound` so they can emit
/// `ToolResponse` frames without borrowing the bridge. Sends are
/// non-blocking (the underlying channel is unbounded) and only fail
/// once the driver task has shut down — i.e. the socket is closed.
#[derive(Clone)]
pub struct ChatOutbound {
    tx: mpsc::UnboundedSender<ChatClientMessage>,
}

impl ChatOutbound {
    /// Enqueue a frame for the driver task to write to the WS. Returns
    /// [`Error::ChatProtocol`] once the driver has exited and the
    /// receiver side of the channel has been dropped.
    pub fn send(&self, frame: ChatClientMessage) -> Result<()> {
        self.tx
            .send(frame)
            .map_err(|_| Error::ChatProtocol("chat socket closed".into()))
    }
}

impl ChatSocket {
    /// Open a chat WebSocket against `url`, authenticating with `bearer`.
    pub async fn connect(
        url: reqwest::Url,
        bearer: String,
        cancel: CancellationToken,
    ) -> Result<Self> {
        let mut req = url
            .as_str()
            .into_client_request()
            .map_err(|e| Error::InvalidUrl(e.to_string()))?;
        req.headers_mut().insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&bearer)
                .map_err(|e| Error::ChatProtocol(format!("Invalid bearer header: {e}")))?,
        );

        let (stream, _response) = tokio_tungstenite::connect_async_tls_with_config(
            req,
            None,
            false,
            None as Option<Connector>,
        )
        .await?;

        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();
        let driver = tokio::spawn(drive_socket(stream, inbound_tx, outbound_rx, cancel));

        Ok(Self {
            inbound: inbound_rx,
            outbound: ChatOutbound { tx: outbound_tx },
            driver: Some(driver),
        })
    }

    /// Receive the next server frame. Yields `None` once the driver has
    /// shut down and the inbound channel has drained.
    pub async fn recv(&mut self) -> Option<Result<ChatServerMessage>> {
        self.inbound.recv().await
    }

    /// Send a frame to the server. Convenience wrapper around the
    /// underlying outbound channel; equivalent to
    /// `self.outbound_sender().send(frame)`.
    pub fn try_send(&self, frame: ChatClientMessage) -> Result<()> {
        self.outbound.send(frame)
    }

    /// Clone the cloneable outbound handle. Hand these out to spawned
    /// dispatcher tasks that need to emit `ToolResponse` frames without
    /// keeping the socket borrowed.
    pub fn outbound_sender(&self) -> ChatOutbound {
        self.outbound.clone()
    }

    /// Test-only constructor. Returns the socket plus a harness that
    /// emulates the server side: push inbound frames via
    /// [`ChatSocketHarness::server_to_client`], observe outbound frames
    /// on [`ChatSocketHarness::client_to_server`].
    #[cfg(test)]
    pub(crate) fn test_pair() -> (Self, ChatSocketHarness) {
        let (server_to_client, inbound_rx) = mpsc::unbounded_channel();
        let (outbound_tx, client_to_server) = mpsc::unbounded_channel();
        (
            Self {
                inbound: inbound_rx,
                outbound: ChatOutbound { tx: outbound_tx },
                driver: None,
            },
            ChatSocketHarness {
                server_to_client,
                client_to_server,
            },
        )
    }
}

impl Drop for ChatSocket {
    fn drop(&mut self) {
        if let Some(handle) = self.driver.take() {
            handle.abort();
        }
    }
}

/// Test harness emulating the server side of a chat WebSocket.
///
/// Exposed only under `cfg(test)`; production code uses
/// [`ChatSocket::connect`].
#[cfg(test)]
pub(crate) struct ChatSocketHarness {
    /// Push `Result<ChatServerMessage>` values to deliver them to the
    /// bridge as if they arrived from the server. Sending `Err(_)`
    /// simulates a transport error.
    pub server_to_client: mpsc::UnboundedSender<Result<ChatServerMessage>>,
    /// Observe outbound traffic — what the bridge wrote to the WS.
    pub client_to_server: mpsc::UnboundedReceiver<ChatClientMessage>,
}

async fn drive_socket(
    mut stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    inbound: mpsc::UnboundedSender<Result<ChatServerMessage>>,
    mut outbound: mpsc::UnboundedReceiver<ChatClientMessage>,
    cancel: CancellationToken,
) {
    loop {
        tokio::select! {
            biased;
            () = cancel.cancelled() => {
                let _ = stream.close(None).await;
                let _ = inbound.send(Err(Error::Cancelled));
                return;
            }
            frame = outbound.recv() => {
                let Some(frame) = frame else {
                    // The sender side has been dropped — graceful shutdown.
                    let _ = stream.close(None).await;
                    return;
                };
                let encoded = match serde_json::to_string(&frame) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = inbound.send(Err(Error::Encode(e)));
                        let _ = stream.close(None).await;
                        return;
                    }
                };
                if let Err(e) = stream.send(Message::Text(encoded.into())).await {
                    let _ = inbound.send(Err(Error::WebSocket(e)));
                    return;
                }
            }
            msg = stream.next() => {
                let Some(msg) = msg else { return };
                let msg = match msg {
                    Ok(m) => m,
                    Err(e) => {
                        let _ = inbound.send(Err(Error::WebSocket(e)));
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
                            if inbound.send(Ok(event)).is_err() {
                                let _ = stream.close(None).await;
                                return;
                            }
                            if is_terminal {
                                let _ = stream.close(None).await;
                                return;
                            }
                        }
                        Err(e) => {
                            let _ = inbound.send(Err(Error::Decode(e)));
                            return;
                        }
                    },
                    Message::Close(_) => return,
                    Message::Ping(payload) => {
                        // Some proxies need an explicit pong to keep the
                        // socket alive; doing it eagerly tightens the
                        // keepalive loop versus relying on tungstenite's
                        // implicit auto-pong on next read.
                        let _ = stream.send(Message::Pong(payload)).await;
                    }
                    Message::Binary(_) | Message::Pong(_) | Message::Frame(_) => {
                        // Chat protocol is text-only; ignore the rest.
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use thread_core::{ChatSendRequest, ChatServerMessage};

    fn sample_send_frame() -> ChatClientMessage {
        ChatClientMessage::Send(ChatSendRequest {
            content_blocks: Vec::new(),
            parent_message_id: None,
            asset_chips_json: None,
            activity_id: None,
        })
    }

    #[tokio::test(flavor = "current_thread")]
    async fn recv_passes_through_inbound_in_order() {
        let (mut socket, harness) = ChatSocket::test_pair();
        harness
            .server_to_client
            .send(Ok(ChatServerMessage::Final {
                messages: Vec::new(),
            }))
            .unwrap();
        harness
            .server_to_client
            .send(Ok(ChatServerMessage::Final {
                messages: Vec::new(),
            }))
            .unwrap();

        assert!(matches!(
            socket.recv().await,
            Some(Ok(ChatServerMessage::Final { .. }))
        ));
        assert!(matches!(
            socket.recv().await,
            Some(Ok(ChatServerMessage::Final { .. }))
        ));

        drop(harness);
        assert!(socket.recv().await.is_none());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn try_send_appears_on_outbound_in_order() {
        let (socket, mut harness) = ChatSocket::test_pair();
        socket.try_send(ChatClientMessage::Cancel).unwrap();
        socket.try_send(sample_send_frame()).unwrap();

        assert!(matches!(
            harness.client_to_server.recv().await,
            Some(ChatClientMessage::Cancel)
        ));
        assert!(matches!(
            harness.client_to_server.recv().await,
            Some(ChatClientMessage::Send(_))
        ));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn outbound_sender_clones_share_one_channel() {
        let (socket, mut harness) = ChatSocket::test_pair();
        let sender = socket.outbound_sender();
        let sender_clone = sender.clone();

        sender.send(ChatClientMessage::Cancel).unwrap();
        sender_clone.send(ChatClientMessage::Cancel).unwrap();
        socket.try_send(ChatClientMessage::Cancel).unwrap();

        for _ in 0..3 {
            assert!(matches!(
                harness.client_to_server.recv().await,
                Some(ChatClientMessage::Cancel)
            ));
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn try_send_after_harness_drop_errors() {
        let (socket, harness) = ChatSocket::test_pair();
        drop(harness);
        let err = socket
            .try_send(ChatClientMessage::Cancel)
            .expect_err("send after drop must fail");
        match err {
            Error::ChatProtocol(msg) => assert!(msg.contains("closed")),
            other => panic!("expected ChatProtocol, got {other:?}"),
        }
    }
}
