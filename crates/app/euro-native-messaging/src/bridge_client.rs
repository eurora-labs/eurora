//! Thin WebSocket client that speaks the
//! [`euro_bridge_protocol`] JSON wire format. Handles connection,
//! registration, and frame-level send/receive; reconnect logic and
//! cache replay live in `main.rs`.
//!
//! Once connected and registered, callers split the client into
//! independent reader and writer halves so the inbound and outbound
//! pumps can run on separate tasks without contending for the socket.

use anyhow::{Context, Result};
use euro_bridge_protocol::{Frame, RegisterFrame};
use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// A connected WebSocket client. Hold this only long enough to
/// [`Self::register`] and then [`Self::split`] into the reader/writer
/// halves.
pub struct BridgeClient {
    socket: Socket,
}

impl BridgeClient {
    /// Open a WebSocket to `url`, e.g. `ws://[::1]:1431/`.
    pub async fn connect(url: &str) -> Result<Self> {
        let (socket, _response) = connect_async(url)
            .await
            .with_context(|| format!("connecting to {url}"))?;
        Ok(Self { socket })
    }

    /// Send the mandatory [`RegisterFrame`] that the bridge requires as the
    /// first message on every connection.
    pub async fn register(&mut self, register: RegisterFrame) -> Result<()> {
        send_frame(&mut self.socket, &Frame::from(register))
            .await
            .context("sending RegisterFrame")
    }

    /// Split the socket into independent reader and writer halves so each
    /// can be driven by its own task.
    pub fn split(self) -> (BridgeReader, BridgeWriter) {
        let (sink, stream) = self.socket.split();
        (BridgeReader { inner: stream }, BridgeWriter { inner: sink })
    }
}

/// Read half of a [`BridgeClient`].
pub struct BridgeReader {
    inner: SplitStream<Socket>,
}

impl BridgeReader {
    /// Pull the next [`Frame`] off the stream. Returns `Ok(None)` on a
    /// clean close. Pings/pongs and raw protocol frames are silently
    /// skipped.
    pub async fn next_frame(&mut self) -> Result<Option<Frame>> {
        loop {
            let message = match self.inner.next().await {
                Some(Ok(message)) => message,
                Some(Err(err)) => {
                    return Err(err).context("reading WebSocket message");
                }
                None => return Ok(None),
            };
            match message {
                Message::Text(text) => {
                    let frame = serde_json::from_str(text.as_str())
                        .with_context(|| format!("decoding frame from text: {text}"))?;
                    return Ok(Some(frame));
                }
                Message::Binary(bytes) => {
                    let frame =
                        serde_json::from_slice(&bytes).context("decoding frame from binary")?;
                    return Ok(Some(frame));
                }
                Message::Close(_) => return Ok(None),
                Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => continue,
            }
        }
    }
}

/// Write half of a [`BridgeClient`].
pub struct BridgeWriter {
    inner: SplitSink<Socket, Message>,
}

impl BridgeWriter {
    /// Encode `frame` as JSON and send it as a WebSocket text message.
    pub async fn send_frame(&mut self, frame: &Frame) -> Result<()> {
        send_frame_sink(&mut self.inner, frame).await
    }
}

async fn send_frame(socket: &mut Socket, frame: &Frame) -> Result<()> {
    let json = serde_json::to_string(frame).context("serializing Frame")?;
    socket
        .send(Message::Text(json.into()))
        .await
        .context("writing WebSocket message")
}

async fn send_frame_sink(sink: &mut SplitSink<Socket, Message>, frame: &Frame) -> Result<()> {
    let json = serde_json::to_string(frame).context("serializing Frame")?;
    sink.send(Message::Text(json.into()))
        .await
        .context("writing WebSocket message")
}
