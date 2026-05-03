use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use axum::Router;
use axum::extract::ConnectInfo;
use axum::extract::State;
use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket, WebSocketUpgrade, close_code};
use axum::response::IntoResponse;
use axum::routing::get;
use dashmap::DashMap;
use euro_bridge_protocol::{
    BRIDGE_HOST, BRIDGE_PATH, BRIDGE_PORT, BridgeError, CancelFrame, EventFrame, Frame, FrameKind,
    RegisterFrame, RequestFrame, ResponseFrame,
};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::{Mutex, OnceCell, broadcast, mpsc, oneshot, watch};
use tokio::task::JoinHandle;

use crate::process_name::get_process_name;

const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const FIRST_FRAME_TIMEOUT: Duration = Duration::from_secs(5);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const OUTBOUND_QUEUE_SIZE: usize = 32;

static GLOBAL_SERVICE: OnceCell<BridgeService> = OnceCell::const_new();

/// A WebSocket-connected bridge client (typically a browser
/// native-messaging host).
#[derive(Debug)]
pub struct RegisteredClient {
    /// Outbound queue for frames the desktop wants to send to this
    /// client. The connection's writer task drains this onto the
    /// websocket sink.
    pub tx: mpsc::Sender<Frame>,
    pub host_pid: u32,
    pub app_pid: u32,
    pub app_name: String,
}

/// Lightweight summary of a registered client, broadcast on the
/// registrations / disconnects channels so subscribers can react to
/// connect/disconnect transitions without reading the registry.
#[derive(Debug, Clone)]
pub struct RegistrationEvent {
    pub app_pid: u32,
    pub app_name: String,
}

struct ServerHandle {
    shutdown_tx: watch::Sender<bool>,
    task: JoinHandle<()>,
}

/// In-process bridge service. A single instance is shared via
/// [`BridgeService::get_or_init`]; clones are cheap because all state
/// lives behind `Arc`s and tokio channels.
#[derive(Clone)]
pub struct BridgeService {
    registry: Arc<DashMap<u32, RegisteredClient>>,
    frames_from_clients_tx: broadcast::Sender<(u32, Frame)>,
    events_tx: broadcast::Sender<(u32, EventFrame)>,
    registrations_tx: broadcast::Sender<RegistrationEvent>,
    disconnects_tx: broadcast::Sender<RegistrationEvent>,
    pending_requests: Arc<DashMap<u32, oneshot::Sender<Frame>>>,
    request_id_counter: Arc<AtomicU32>,
    server: Arc<Mutex<Option<ServerHandle>>>,
}

impl Default for BridgeService {
    fn default() -> Self {
        Self::new()
    }
}

impl BridgeService {
    /// Build a new service and spawn its frame-dispatch task. The
    /// dispatch task runs for the lifetime of the service (it exits
    /// when the inbound broadcast sender is dropped, i.e. when the
    /// last clone of this service is gone).
    pub fn new() -> Self {
        let (frames_from_clients_tx, _) = broadcast::channel(100);
        let (events_tx, _) = broadcast::channel(100);
        let (registrations_tx, _) = broadcast::channel(32);
        let (disconnects_tx, _) = broadcast::channel(32);

        let service = Self {
            registry: Arc::new(DashMap::new()),
            frames_from_clients_tx,
            events_tx,
            registrations_tx,
            disconnects_tx,
            pending_requests: Arc::new(DashMap::new()),
            request_id_counter: Arc::new(AtomicU32::new(1)),
            server: Arc::new(Mutex::new(None)),
        };
        service.spawn_frame_handler();
        service
    }

    pub async fn get_or_init() -> &'static BridgeService {
        GLOBAL_SERVICE
            .get_or_init(|| async { BridgeService::new() })
            .await
    }

    /// Returns the global service if [`get_or_init`] has been called at
    /// least once, otherwise `None`. Useful for shutdown paths that
    /// shouldn't accidentally bring the service into existence.
    pub fn get() -> Option<&'static BridgeService> {
        GLOBAL_SERVICE.get()
    }

    fn spawn_frame_handler(&self) {
        let pending_requests = Arc::clone(&self.pending_requests);
        let events_tx = self.events_tx.clone();
        let mut frames_rx = self.frames_from_clients_tx.subscribe();

        tokio::spawn(async move {
            tracing::debug!("Frame handler task started");
            loop {
                let (app_pid, frame) = match frames_rx.recv().await {
                    Ok(val) => val,
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Frame handler lagged by {n} frames, resuming");
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                };

                match frame.kind {
                    FrameKind::Request(req) => {
                        tracing::warn!(
                            "Received unsupported request from client: app_pid={app_pid}, action={}",
                            req.action
                        );
                    }
                    FrameKind::Response(resp) => {
                        let id = resp.id;
                        if let Some((_, sender)) = pending_requests.remove(&id) {
                            if sender.send(Frame::from(resp)).is_err() {
                                tracing::debug!(
                                    "Pending request {id} was dropped before response arrived"
                                );
                            }
                        } else {
                            tracing::debug!(
                                "Received response for unknown request id={id}, action={}",
                                resp.action
                            );
                        }
                    }
                    FrameKind::Event(evt) => {
                        if events_tx.send((app_pid, evt)).is_err() {
                            tracing::trace!(
                                "No event subscribers for event from app_pid={app_pid}"
                            );
                        }
                    }
                    FrameKind::Error(err) => {
                        let id = err.id;
                        tracing::warn!(
                            "Received error frame from app_pid={app_pid}: id={id}, message={}",
                            err.message
                        );
                        if let Some((_, sender)) = pending_requests.remove(&id)
                            && sender.send(Frame::from(err)).is_err()
                        {
                            tracing::debug!(
                                "Pending request {id} was dropped before error arrived"
                            );
                        }
                    }
                    FrameKind::Cancel(cancel) => {
                        if pending_requests.remove(&cancel.id).is_some() {
                            tracing::debug!("Cancelled pending request id={}", cancel.id);
                        }
                    }
                    FrameKind::Register(_) => {
                        tracing::warn!(
                            "Received Register frame outside the handshake from app_pid={app_pid}"
                        );
                    }
                }
            }
            tracing::debug!("Frame handler task ended");
        });
    }

    /// Bind and serve the bridge on `{BRIDGE_HOST}:{BRIDGE_PORT}`. The
    /// listener is bound before this returns; the accept loop runs in
    /// the background. Calling again while the server is already
    /// running is a no-op. If a previous server task ended (e.g. the
    /// process panicked), it is reaped and a fresh server is started.
    pub async fn start_server(&self) -> Result<(), std::io::Error> {
        let mut guard = self.server.lock().await;

        if let Some(handle) = guard.as_ref()
            && handle.task.is_finished()
        {
            tracing::warn!("Reaping bridge server task that exited unexpectedly; restarting");
            *guard = None;
        }

        if guard.is_some() {
            tracing::debug!("Bridge WebSocket server already running");
            return Ok(());
        }

        let bind_addr: SocketAddr = (
            BRIDGE_HOST
                .parse::<std::net::IpAddr>()
                .expect("valid loopback ip"),
            BRIDGE_PORT,
        )
            .into();
        let listener = TcpListener::bind(bind_addr).await?;
        tracing::info!("Bridge WebSocket server listening on {bind_addr}{BRIDGE_PATH}");

        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        let app = Router::new()
            .route(BRIDGE_PATH, get(ws_upgrade))
            .with_state(self.clone());

        let task = tokio::spawn(async move {
            let serve = axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.wait_for(|v| *v).await;
                tracing::info!("Bridge WebSocket server received shutdown signal");
            });

            if let Err(err) = serve.await {
                tracing::error!("Bridge WebSocket server error: {err}");
            }

            tracing::info!("Bridge WebSocket server stopped");
        });

        *guard = Some(ServerHandle { shutdown_tx, task });
        Ok(())
    }

    /// Signal the running server to shut down, then wait for the
    /// accept loop and any in-flight connections to fully terminate.
    /// No-op if the server isn't running. The lock is held across the
    /// wait so a concurrent [`start_server`] doesn't race the listener
    /// for the port.
    pub async fn stop_server(&self) {
        let mut guard = self.server.lock().await;
        let Some(handle) = guard.take() else {
            tracing::debug!("Bridge WebSocket server is not running");
            return;
        };

        tracing::info!("Sending shutdown signal to bridge WebSocket server");
        let _ = handle.shutdown_tx.send(true);

        if let Err(err) = handle.task.await {
            tracing::warn!("Bridge WebSocket server task ended unexpectedly: {err}");
        }
    }

    /// Receive an [`EventFrame`] every time a client pushes one. The
    /// `u32` is the `app_pid` it came from.
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<(u32, EventFrame)> {
        self.events_tx.subscribe()
    }

    /// Receive a [`RegistrationEvent`] every time a client registers.
    pub fn subscribe_to_registrations(&self) -> broadcast::Receiver<RegistrationEvent> {
        self.registrations_tx.subscribe()
    }

    /// Receive a [`RegistrationEvent`] every time a client disconnects.
    pub fn subscribe_to_disconnects(&self) -> broadcast::Receiver<RegistrationEvent> {
        self.disconnects_tx.subscribe()
    }

    pub fn connection_count(&self) -> usize {
        self.registry.len()
    }

    pub fn find_pid_by_app_name(&self, app_name: &str) -> Option<u32> {
        self.registry
            .iter()
            .find(|entry| entry.value().app_name == app_name)
            .map(|entry| entry.value().app_pid)
    }

    /// Send a request to `app_pid` and await a correlated response.
    /// Times out after [`DEFAULT_REQUEST_TIMEOUT`]; on timeout a
    /// `Cancel` frame is sent so the client can drop any work it
    /// started.
    pub async fn send_request(
        &self,
        app_pid: u32,
        action: &str,
        payload: Option<String>,
    ) -> Result<ResponseFrame, BridgeError> {
        let request_id = self.request_id_counter.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.pending_requests.insert(request_id, tx);

        let request = Frame::from(RequestFrame {
            id: request_id,
            action: action.to_string(),
            payload,
        });

        tracing::debug!("Sending request to app_pid={app_pid}: id={request_id}, action={action}");

        if let Err(err) = self.send_to_client(app_pid, request).await {
            self.pending_requests.remove(&request_id);
            return Err(err);
        }

        match tokio::time::timeout(DEFAULT_REQUEST_TIMEOUT, rx).await {
            Ok(Ok(frame)) => match frame.kind {
                FrameKind::Response(resp) => Ok(resp),
                FrameKind::Error(err) => Err(BridgeError::Client {
                    message: err.message,
                    details: err.details,
                }),
                other => Err(BridgeError::UnexpectedFrame(frame_kind_label(&other))),
            },
            Ok(Err(_)) => {
                self.pending_requests.remove(&request_id);
                Err(BridgeError::ChannelClosed)
            }
            Err(_) => {
                self.pending_requests.remove(&request_id);
                let cancel = Frame::from(CancelFrame { id: request_id });
                if let Err(err) = self.send_to_client(app_pid, cancel).await {
                    tracing::debug!(
                        "Failed to send Cancel for timed-out request {request_id}: {err}"
                    );
                }
                Err(BridgeError::Timeout)
            }
        }
    }

    pub async fn get_metadata(&self, app_pid: u32) -> Result<ResponseFrame, BridgeError> {
        self.send_request(app_pid, "GET_METADATA", None).await
    }

    async fn send_to_client(&self, app_pid: u32, frame: Frame) -> Result<(), BridgeError> {
        let tx = self
            .registry
            .get(&app_pid)
            .map(|entry| entry.value().tx.clone())
            .ok_or(BridgeError::NotFound { app_pid })?;
        tx.send(frame)
            .await
            .map_err(|err| BridgeError::Send(err.to_string()))
    }
}

fn frame_kind_label(kind: &FrameKind) -> &'static str {
    match kind {
        FrameKind::Request(_) => "Request",
        FrameKind::Response(_) => "Response",
        FrameKind::Event(_) => "Event",
        FrameKind::Error(_) => "Error",
        FrameKind::Cancel(_) => "Cancel",
        FrameKind::Register(_) => "Register",
    }
}

async fn ws_upgrade(
    State(service): State<BridgeService>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    upgrade: WebSocketUpgrade,
) -> impl IntoResponse {
    if !peer.ip().is_loopback() {
        tracing::warn!("Rejecting bridge connection from non-loopback peer: {peer}");
        return (
            axum::http::StatusCode::FORBIDDEN,
            "bridge only accepts loopback connections",
        )
            .into_response();
    }

    upgrade
        .max_message_size(16 * 1024 * 1024)
        .max_frame_size(16 * 1024 * 1024)
        .on_upgrade(move |socket| handle_socket(service, socket, peer))
}

async fn handle_socket(service: BridgeService, socket: WebSocket, peer: SocketAddr) {
    let (mut sink, mut stream) = socket.split();

    let register = match read_register_frame(&mut stream).await {
        Ok(frame) => frame,
        Err(err) => {
            tracing::warn!("Bridge handshake failed from {peer}: {err}");
            let _ = sink
                .send(Message::Close(Some(CloseFrame {
                    code: close_code::PROTOCOL,
                    reason: Utf8Bytes::from_static("expected Register frame"),
                })))
                .await;
            return;
        }
    };

    let app_pid = register.app_pid;
    let host_pid = register.host_pid;
    let app_name = get_process_name(app_pid).unwrap_or_else(|| format!("unknown_{app_pid}"));

    let (outbound_tx, outbound_rx) = mpsc::channel::<Frame>(OUTBOUND_QUEUE_SIZE);

    if let Some(prev) = service.registry.insert(
        app_pid,
        RegisteredClient {
            tx: outbound_tx.clone(),
            host_pid,
            app_pid,
            app_name: app_name.clone(),
        },
    ) {
        tracing::warn!(
            "Replacing existing registration for app_pid={app_pid} (previous host_pid={})",
            prev.host_pid
        );
    }

    tracing::info!(
        "Bridge client registered: app_pid={app_pid} host_pid={host_pid} app_name={app_name:?} (peer={peer})"
    );

    let _ = service.registrations_tx.send(RegistrationEvent {
        app_pid,
        app_name: app_name.clone(),
    });

    let writer = tokio::spawn(writer_task(sink, outbound_rx));
    reader_loop(&service, &mut stream, app_pid).await;

    drop(outbound_tx);

    if let Some((_, removed)) = service
        .registry
        .remove_if(&app_pid, |_, client| client.host_pid == host_pid)
    {
        let _ = service.disconnects_tx.send(RegistrationEvent {
            app_pid,
            app_name: removed.app_name,
        });
        tracing::info!(
            "Bridge client unregistered: app_pid={app_pid} host_pid={host_pid} (remaining={})",
            service.registry.len()
        );
    } else {
        tracing::warn!(
            "Did not unregister app_pid={app_pid}: registration was replaced by host_pid={host_pid} or already removed"
        );
    }

    if let Err(err) = writer.await {
        tracing::debug!("Writer task for app_pid={app_pid} ended: {err}");
    }
}

async fn read_register_frame<S>(stream: &mut S) -> Result<RegisterFrame, String>
where
    S: futures_util::Stream<Item = Result<Message, axum::Error>> + Unpin,
{
    let next = tokio::time::timeout(FIRST_FRAME_TIMEOUT, stream.next())
        .await
        .map_err(|_| "timed out waiting for Register frame".to_string())?;

    let message = next
        .ok_or_else(|| "client disconnected before sending Register frame".to_string())?
        .map_err(|err| format!("websocket error during handshake: {err}"))?;

    let payload: Utf8Bytes = match message {
        Message::Text(t) => t,
        Message::Close(_) => return Err("client closed connection during handshake".into()),
        other => {
            return Err(format!(
                "expected Text Register frame, got {}",
                message_label(&other)
            ));
        }
    };

    let frame: Frame = serde_json::from_str(payload.as_str())
        .map_err(|err| format!("invalid Register JSON: {err}"))?;

    match frame.kind {
        FrameKind::Register(register) => Ok(register),
        other => Err(format!(
            "first frame must be Register, got {}",
            frame_kind_label(&other)
        )),
    }
}

async fn writer_task(
    mut sink: futures_util::stream::SplitSink<WebSocket, Message>,
    mut outbound_rx: mpsc::Receiver<Frame>,
) {
    let mut heartbeat = tokio::time::interval(HEARTBEAT_INTERVAL);
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // Skip the immediate first tick.
    heartbeat.tick().await;

    loop {
        tokio::select! {
            biased;
            frame = outbound_rx.recv() => {
                let Some(frame) = frame else { break };
                let json = match serde_json::to_string(&frame) {
                    Ok(json) => json,
                    Err(err) => {
                        tracing::error!("Failed to serialize outbound frame: {err}");
                        continue;
                    }
                };
                if let Err(err) = sink.send(Message::Text(json.into())).await {
                    tracing::debug!("Failed to write outbound frame: {err}");
                    break;
                }
            }
            _ = heartbeat.tick() => {
                if let Err(err) = sink.send(Message::Ping(Default::default())).await {
                    tracing::debug!("Heartbeat ping failed: {err}");
                    break;
                }
            }
        }
    }

    let _ = sink
        .send(Message::Close(Some(CloseFrame {
            code: close_code::NORMAL,
            reason: Utf8Bytes::from_static("server closing connection"),
        })))
        .await;
}

async fn reader_loop<S>(service: &BridgeService, stream: &mut S, app_pid: u32)
where
    S: futures_util::Stream<Item = Result<Message, axum::Error>> + Unpin,
{
    while let Some(message) = stream.next().await {
        let message = match message {
            Ok(m) => m,
            Err(err) => {
                tracing::debug!("Websocket error from app_pid={app_pid}: {err}");
                break;
            }
        };

        match message {
            Message::Text(text) => match serde_json::from_str::<Frame>(text.as_str()) {
                Ok(frame) => {
                    if let Err(err) = service.frames_from_clients_tx.send((app_pid, frame)) {
                        tracing::trace!(
                            "No subscribers for inbound frame from app_pid={app_pid}: {err}"
                        );
                    }
                }
                Err(err) => {
                    tracing::warn!("Failed to parse inbound frame from app_pid={app_pid}: {err}");
                }
            },
            Message::Binary(_) => {
                tracing::warn!("Ignoring unexpected binary frame from app_pid={app_pid}");
            }
            Message::Ping(_) | Message::Pong(_) => {
                // axum auto-responds to Ping with Pong; nothing to do.
            }
            Message::Close(frame) => {
                tracing::debug!("Client app_pid={app_pid} closed connection: {frame:?}");
                break;
            }
        }
    }
}

fn message_label(message: &Message) -> &'static str {
    match message {
        Message::Text(_) => "Text",
        Message::Binary(_) => "Binary",
        Message::Ping(_) => "Ping",
        Message::Pong(_) => "Pong",
        Message::Close(_) => "Close",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use euro_bridge_protocol::EventFrame;
    use tokio::time::timeout;

    #[test]
    fn frame_kind_label_covers_all_variants() {
        assert_eq!(
            frame_kind_label(&FrameKind::Cancel(CancelFrame { id: 1 })),
            "Cancel"
        );
        assert_eq!(
            frame_kind_label(&FrameKind::Event(EventFrame {
                action: "X".into(),
                payload: None,
            })),
            "Event"
        );
    }

    #[tokio::test]
    async fn send_request_returns_not_found_when_client_missing() {
        let service = BridgeService::new();
        let result = service.send_request(42, "GET_METADATA", None).await;
        assert!(matches!(result, Err(BridgeError::NotFound { app_pid: 42 })));
    }

    #[tokio::test]
    async fn send_request_resolves_when_response_arrives() {
        let service = BridgeService::new();

        let (outbound_tx, mut outbound_rx) = mpsc::channel::<Frame>(8);
        service.registry.insert(
            7,
            RegisteredClient {
                tx: outbound_tx,
                host_pid: 1,
                app_pid: 7,
                app_name: "test".into(),
            },
        );

        let svc = service.clone();
        let request_handle = tokio::spawn(async move { svc.send_request(7, "PING", None).await });

        let outbound = timeout(Duration::from_secs(1), outbound_rx.recv())
            .await
            .expect("outbound frame")
            .expect("frame present");
        let FrameKind::Request(req) = outbound.kind else {
            panic!("expected Request frame");
        };

        service
            .frames_from_clients_tx
            .send((
                7,
                Frame::from(ResponseFrame {
                    id: req.id,
                    action: req.action,
                    payload: Some("pong".into()),
                }),
            ))
            .expect("broadcast send");

        let response = timeout(Duration::from_secs(1), request_handle)
            .await
            .expect("request future")
            .expect("join")
            .expect("response");
        assert_eq!(response.payload.as_deref(), Some("pong"));
    }
}
