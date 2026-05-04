use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use std::time::Duration;

use axum::Router;
use axum::extract::ConnectInfo;
use axum::extract::State;
use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket, WebSocketUpgrade, close_code};
use axum::response::IntoResponse;
use axum::routing::get;
use axum_server::Handle as AxumHandle;
use axum_server::tls_rustls::RustlsConfig;
use dashmap::DashMap;
use euro_bridge_protocol::{
    BRIDGE_BIND_IP, BRIDGE_PATH, BRIDGE_PORT, BridgeError, CancelFrame, ErrorFrame, EventFrame,
    Frame, FrameKind, RegisterFrame, RequestFrame, ResponseFrame,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::process_name::get_process_name;

const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const FIRST_FRAME_TIMEOUT: Duration = Duration::from_secs(5);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const OUTBOUND_QUEUE_SIZE: usize = 32;
/// Time we let in-flight connections drain on shutdown before forcing
/// the listener closed.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

static GLOBAL_SERVICE: OnceLock<BridgeService> = OnceLock::new();

/// Idempotently install the rustls process-wide crypto provider. Safe
/// to call multiple times and from multiple threads — only the first
/// call has effect. If a provider has already been installed (by us or
/// by another crate in the process), this logs at debug level and
/// leaves the existing provider in place. Must be called before any
/// rustls/axum-server work runs in the process; convention is "once at
/// the top of `main`".
pub fn install_default_crypto_provider() {
    if rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .is_err()
    {
        tracing::debug!(
            "rustls default crypto provider was already installed; leaving existing provider in place"
        );
    }
}

/// Server-side TLS material. The bridge requires this to be configured
/// before [`BridgeService::bind`] is called — there is no plaintext
/// fallback.
#[derive(Clone, Debug)]
pub struct TlsMaterial {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

/// A WebSocket-connected bridge client (typically a browser
/// native-messaging host or an Office add-in runtime).
#[derive(Debug)]
pub struct RegisteredClient {
    /// Outbound queue for frames the desktop wants to send to this
    /// client. The connection's writer task drains this onto the
    /// websocket sink.
    pub tx: mpsc::Sender<Frame>,
    pub host_pid: u32,
    pub app_pid: u32,
    pub app_name: String,
    /// Logical kind sent by the client during registration. `None` for
    /// PID-based clients (browsers); `Some` for sandboxed integrations
    /// like the Word add-in (`Some("microsoft-word")`).
    pub app_kind: Option<String>,
}

/// Lightweight summary of a registered client, broadcast on the
/// registrations / disconnects channels so subscribers can react to
/// connect/disconnect transitions without reading the registry.
#[derive(Debug, Clone)]
pub struct RegistrationEvent {
    pub app_pid: u32,
    pub app_name: String,
    pub app_kind: Option<String>,
}

/// Bookkeeping for a bridge listener that the service owns. Held
/// inside the service's `server` mutex; keyed off the listener's
/// lifetime — a slot is populated as soon as [`BridgeService::bind_on`]
/// hands out a [`BoundServer`] and stays populated until that server
/// is either served and stopped, or dropped without serving.
struct ServerHandle {
    state: ServerState,
    local_addr: SocketAddr,
}

enum ServerState {
    /// `bind_on` returned a [`BoundServer`] but no one has called
    /// [`BoundServer::serve`] on it yet (or the BoundServer was
    /// dropped without serving — Drop clears the slot before that
    /// becomes observable).
    Bound,
    /// A serve loop is running. `axum_handle` is what
    /// [`BridgeService::stop_server`] uses to trigger graceful
    /// shutdown; `done` is the one-shot signal flipped by `serve`
    /// when its accept loop exits, awaited by `stop_server` to drain
    /// in-flight connections. `done` becomes `None` once a stopper
    /// has taken it.
    Serving {
        axum_handle: AxumHandle,
        done: Option<oneshot::Receiver<()>>,
    },
}

/// A bridge listener whose kernel socket is in `LISTEN` state but whose
/// accept loop has not started yet. Returned by
/// [`BridgeService::bind`] / [`BridgeService::bind_on`]; consumed by
/// [`BoundServer::serve`].
///
/// The split between bind and serve makes the "port is open" guarantee
/// observable in the type system: a caller that holds a `BoundServer`
/// has already passed the bind, regardless of whether anyone is polling
/// the accept loop yet. Dropping a `BoundServer` without serving
/// releases the socket and clears the service's bookkeeping — useful
/// in tests but a programming bug in production paths, hence `#[must_use]`.
#[must_use = "BoundServer drops the listening socket if not served"]
pub struct BoundServer {
    service: BridgeService,
    /// `None` only between `serve()` consuming the listener and the
    /// struct being dropped — Drop uses this to detect "served vs
    /// abandoned" so it knows whether to clear the slot.
    listener: Option<std::net::TcpListener>,
    tls: Option<RustlsConfig>,
    local_addr: SocketAddr,
}

impl std::fmt::Debug for BoundServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoundServer")
            .field("local_addr", &self.local_addr)
            .finish_non_exhaustive()
    }
}

impl BoundServer {
    /// Address the listener is bound to. Stable across the bind/serve
    /// transition.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Run the accept loop until the owning service is asked to stop
    /// (via [`BridgeService::stop_server`]) or the loop exits with an
    /// error.
    ///
    /// This future drives the accept loop directly; the typical startup
    /// pattern is `tokio::spawn(bound.serve())`. Awaiting it inline
    /// blocks until the loop ends.
    pub async fn serve(mut self) -> Result<(), BridgeError> {
        // Both fields are populated by `bind_on` and only taken here.
        // `serve` consumes `self`, so a second call is statically
        // impossible.
        let listener = self.listener.take().unwrap();
        let tls = self.tls.take().unwrap();
        let service = self.service.clone();
        let local_addr = self.local_addr;
        // Drop early so the Drop impl runs before we hand off control —
        // it sees `listener` is `None` and leaves the slot alone.
        drop(self);

        let axum_handle = AxumHandle::new();
        let (done_tx, done_rx) = oneshot::channel::<()>();

        {
            let mut guard = service.server.lock().expect("server slot poisoned");
            let slot = guard
                .as_mut()
                .expect("BoundServer outlives its service slot");
            match &slot.state {
                ServerState::Bound => {
                    slot.state = ServerState::Serving {
                        axum_handle: axum_handle.clone(),
                        done: Some(done_rx),
                    };
                }
                ServerState::Serving { .. } => {
                    return Err(BridgeError::AlreadyRunning { local_addr });
                }
            }
        }

        let app = Router::new()
            .route(BRIDGE_PATH, get(ws_upgrade))
            .with_state(service.clone());

        let result = axum_server::from_tcp_rustls(listener, tls)
            .handle(axum_handle)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .await;

        // Clear the slot before signalling done so a follow-up `bind`
        // observes a clean state immediately.
        {
            let mut guard = service.server.lock().expect("server slot poisoned");
            *guard = None;
        }
        let _ = done_tx.send(());

        match result {
            Ok(()) => {
                tracing::info!(%local_addr, "Bridge WebSocket server stopped");
                Ok(())
            }
            Err(source) => {
                tracing::error!(%local_addr, error = %source, "Bridge WebSocket server error");
                Err(BridgeError::Serve { source })
            }
        }
    }
}

impl Drop for BoundServer {
    fn drop(&mut self) {
        // If `serve` ran, it took the listener and TLS config and is
        // responsible for clearing the slot. If we still hold them, the
        // BoundServer is being abandoned — release the slot so the
        // service is rebindable.
        if self.listener.is_none() {
            return;
        }
        match self.service.server.lock() {
            Ok(mut guard) => {
                if matches!(guard.as_ref(), Some(handle) if matches!(handle.state, ServerState::Bound))
                {
                    *guard = None;
                }
            }
            Err(_) => tracing::warn!(
                local_addr = %self.local_addr,
                "Bridge server slot poisoned while dropping unserved BoundServer",
            ),
        }
    }
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
    pending_requests: Arc<DashMap<u32, oneshot::Sender<Result<ResponseFrame, ErrorFrame>>>>,
    request_id_counter: Arc<AtomicU32>,
    server: Arc<StdMutex<Option<ServerHandle>>>,
    tls: Arc<OnceLock<TlsMaterial>>,
}

impl Default for BridgeService {
    fn default() -> Self {
        Self::new()
    }
}

impl BridgeService {
    /// Build a new service and spawn its frame-dispatch task. Must be
    /// called from within a tokio runtime: the dispatch task runs for
    /// the lifetime of the service (it exits when the inbound broadcast
    /// sender is dropped, i.e. when the last clone of this service is
    /// gone).
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
            server: Arc::new(StdMutex::new(None)),
            tls: Arc::new(OnceLock::new()),
        };
        service.spawn_frame_handler();
        service
    }

    /// Get the process-wide bridge service, constructing it on first
    /// call. Must be called from within a tokio runtime context (the
    /// initializer spawns the frame-dispatch task).
    pub fn get_or_init() -> &'static BridgeService {
        GLOBAL_SERVICE.get_or_init(BridgeService::new)
    }

    /// Returns the global service if [`get_or_init`](Self::get_or_init)
    /// has been called at least once, otherwise `None`. Useful for
    /// shutdown paths that shouldn't accidentally bring the service
    /// into existence.
    pub fn get() -> Option<&'static BridgeService> {
        GLOBAL_SERVICE.get()
    }

    /// Configure the TLS material the bridge listener will use. Must
    /// be called before [`bind`](BridgeService::bind) /
    /// [`bind_on`](BridgeService::bind_on). First writer wins —
    /// subsequent calls with a different material are logged and
    /// dropped, since changing certs while the server runs would race
    /// the listener.
    pub fn configure_tls(&self, material: TlsMaterial) {
        match self.tls.set(material.clone()) {
            Ok(()) => tracing::debug!(
                cert_path = %material.cert_path.display(),
                key_path = %material.key_path.display(),
                "Bridge TLS material configured",
            ),
            Err(_) => {
                let existing = self
                    .tls
                    .get()
                    .expect("set returned Err so OnceLock is populated");
                if existing.cert_path != material.cert_path
                    || existing.key_path != material.key_path
                {
                    tracing::warn!(
                        existing_cert = %existing.cert_path.display(),
                        existing_key = %existing.key_path.display(),
                        attempted_cert = %material.cert_path.display(),
                        attempted_key = %material.key_path.display(),
                        "Bridge TLS material already configured; ignoring later attempt to reconfigure with different material",
                    );
                }
            }
        }
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
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!(skipped, "Frame handler lagged, resuming");
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                };

                match frame.kind {
                    FrameKind::Request(req) => {
                        tracing::warn!(
                            app_pid,
                            action = %req.action,
                            "Received unsupported request from client",
                        );
                    }
                    FrameKind::Response(resp) => {
                        let id = resp.id;
                        if let Some((_, sender)) = pending_requests.remove(&id) {
                            if sender.send(Ok(resp)).is_err() {
                                tracing::debug!(
                                    request_id = id,
                                    "Pending request was dropped before response arrived",
                                );
                            }
                        } else {
                            tracing::debug!(
                                request_id = id,
                                action = %resp.action,
                                "Received response for unknown request id",
                            );
                        }
                    }
                    FrameKind::Event(evt) => {
                        if events_tx.send((app_pid, evt)).is_err() {
                            tracing::trace!(app_pid, "No event subscribers for inbound event");
                        }
                    }
                    FrameKind::Error(err) => {
                        let id = err.id;
                        tracing::warn!(
                            app_pid,
                            request_id = id,
                            message = %err.message,
                            "Received error frame from client",
                        );
                        if let Some((_, sender)) = pending_requests.remove(&id)
                            && sender.send(Err(err)).is_err()
                        {
                            tracing::debug!(
                                request_id = id,
                                "Pending request was dropped before error arrived",
                            );
                        }
                    }
                    FrameKind::Cancel(cancel) => {
                        if pending_requests.remove(&cancel.id).is_some() {
                            tracing::debug!(request_id = cancel.id, "Cancelled pending request");
                        }
                    }
                    FrameKind::Register(_) => {
                        tracing::warn!(app_pid, "Received Register frame outside the handshake",);
                    }
                }
            }
            tracing::debug!("Frame handler task ended");
        });
    }

    /// Bind the bridge listener on the well-known
    /// `{BRIDGE_BIND_IP}:{BRIDGE_PORT}`. Equivalent to [`bind_on`] with
    /// that address.
    ///
    /// [`bind_on`]: BridgeService::bind_on
    pub async fn bind(&self) -> Result<BoundServer, BridgeError> {
        self.bind_on((BRIDGE_BIND_IP, BRIDGE_PORT).into()).await
    }

    /// Bind the bridge listener on `bind_addr` and return a
    /// [`BoundServer`] whose [`serve`](BoundServer::serve) method runs
    /// the accept loop. Use port `0` to let the OS pick an ephemeral
    /// port; the bound socket address is available via
    /// [`BoundServer::local_addr`] before serving begins.
    ///
    /// The kernel socket is in `LISTEN` state by the time this returns,
    /// so clients dialing the address can never race the bind.
    /// Requires [`configure_tls`](BridgeService::configure_tls) to have
    /// been called first; returns [`BridgeError::TlsNotConfigured`]
    /// otherwise.
    ///
    /// If a previous accept loop is still registered on the service,
    /// returns [`BridgeError::AlreadyRunning`] — callers that want
    /// "ensure running" semantics should check
    /// [`local_addr`](BridgeService::local_addr) first.
    pub async fn bind_on(&self, bind_addr: SocketAddr) -> Result<BoundServer, BridgeError> {
        let material = self.tls.get().ok_or(BridgeError::TlsNotConfigured)?.clone();

        // axum-server's `from_tcp_rustls` adopts an already-bound
        // `std::net::TcpListener`, which is exactly the shape we want:
        // the `local_addr` is observable here, and the kernel socket is
        // accepting connections before the caller ever sees the
        // `BoundServer`.
        //
        // Reserve the slot synchronously around the TCP bind so two
        // concurrent callers can't both succeed. The slot stays
        // populated through the async TLS load below; on failure we
        // roll it back.
        let listener = {
            let mut guard = self.server.lock().expect("server slot poisoned");
            if let Some(handle) = guard.as_ref() {
                return Err(BridgeError::AlreadyRunning {
                    local_addr: handle.local_addr,
                });
            }
            let listener =
                std::net::TcpListener::bind(bind_addr).map_err(|source| BridgeError::Bind {
                    addr: bind_addr,
                    source,
                })?;
            listener
                .set_nonblocking(true)
                .map_err(|source| BridgeError::Bind {
                    addr: bind_addr,
                    source,
                })?;
            let local_addr = listener.local_addr().map_err(|source| BridgeError::Bind {
                addr: bind_addr,
                source,
            })?;
            *guard = Some(ServerHandle {
                state: ServerState::Bound,
                local_addr,
            });
            listener
        };
        let local_addr = listener
            .local_addr()
            .expect("local_addr was readable above");

        let tls = match RustlsConfig::from_pem_file(&material.cert_path, &material.key_path).await {
            Ok(tls) => tls,
            Err(source) => {
                // Roll back the slot reservation so the service is
                // bindable again.
                let mut guard = self.server.lock().expect("server slot poisoned");
                *guard = None;
                return Err(BridgeError::TlsLoad {
                    cert_path: material.cert_path.clone(),
                    key_path: material.key_path.clone(),
                    source,
                });
            }
        };

        tracing::info!(
            %local_addr,
            cert_path = %material.cert_path.display(),
            key_path = %material.key_path.display(),
            url = %format_args!("wss://{local_addr}{BRIDGE_PATH}"),
            "Bridge transport bound (TLS / wss)",
        );

        Ok(BoundServer {
            service: self.clone(),
            listener: Some(listener),
            tls: Some(tls),
            local_addr,
        })
    }

    /// Address the listener is bound to, or `None` if no listener is
    /// currently registered. Becomes `Some` as soon as
    /// [`bind_on`](BridgeService::bind_on) returns a [`BoundServer`]
    /// (whether or not its `serve()` has been polled yet) and stays
    /// `Some` until the listener is dropped or the serve loop ends.
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.server
            .lock()
            .expect("server slot poisoned")
            .as_ref()
            .map(|handle| handle.local_addr)
    }

    /// Signal the running server to shut down, then wait for the
    /// accept loop and any in-flight connections to fully terminate.
    /// No-op if no serve loop is running (including the case where a
    /// [`BoundServer`] has been issued but never served).
    ///
    /// A concurrent [`bind`](BridgeService::bind) during shutdown sees
    /// the slot still populated and returns
    /// [`BridgeError::AlreadyRunning`] until the loop fully exits and
    /// `serve` clears the slot.
    pub async fn stop_server(&self) {
        let (axum_handle, done) = {
            let mut guard = self.server.lock().expect("server slot poisoned");
            let Some(slot) = guard.as_mut() else {
                tracing::debug!("Bridge WebSocket server is not running");
                return;
            };
            match &mut slot.state {
                ServerState::Bound => {
                    tracing::debug!(
                        "Bridge listener is bound but no serve loop is running; nothing to stop"
                    );
                    return;
                }
                ServerState::Serving { axum_handle, done } => (axum_handle.clone(), done.take()),
            }
        };

        tracing::info!("Sending shutdown signal to bridge WebSocket server");
        axum_handle.graceful_shutdown(Some(SHUTDOWN_GRACE));

        if let Some(done) = done {
            // The sender is dropped (or fired) by `serve` exactly once
            // when its accept loop ends; either outcome means the loop
            // has stopped, so we don't distinguish them.
            let _ = done.await;
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

    /// Return the `app_pid`s of every currently-registered client whose
    /// `app_kind` equals `kind`. Used by integrations whose clients
    /// don't correspond to an OS process (e.g. the Word add-in
    /// strategy locating its in-Word runtime).
    pub fn find_clients_by_kind(&self, kind: &str) -> Vec<u32> {
        self.registry
            .iter()
            .filter(|entry| entry.value().app_kind.as_deref() == Some(kind))
            .map(|entry| entry.value().app_pid)
            .collect()
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
        let request_id = self.request_id_counter.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.pending_requests.insert(request_id, tx);

        let request = Frame::from(RequestFrame {
            id: request_id,
            action: action.to_string(),
            payload,
        });

        tracing::debug!(app_pid, request_id, action, "Sending bridge request");

        if let Err(err) = self.send_to_client(app_pid, request).await {
            self.pending_requests.remove(&request_id);
            return Err(err);
        }

        match tokio::time::timeout(DEFAULT_REQUEST_TIMEOUT, rx).await {
            Ok(Ok(Ok(resp))) => Ok(resp),
            Ok(Ok(Err(err))) => Err(BridgeError::Client {
                message: err.message,
                details: err.details,
            }),
            Ok(Err(_)) => {
                self.pending_requests.remove(&request_id);
                Err(BridgeError::ChannelClosed)
            }
            Err(_) => {
                self.pending_requests.remove(&request_id);
                let cancel = Frame::from(CancelFrame { id: request_id });
                if let Err(err) = self.send_to_client(app_pid, cancel).await {
                    tracing::debug!(
                        request_id,
                        error = %err,
                        "Failed to send Cancel for timed-out request",
                    );
                }
                Err(BridgeError::Timeout)
            }
        }
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

async fn ws_upgrade(
    State(service): State<BridgeService>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    upgrade: WebSocketUpgrade,
) -> impl IntoResponse {
    if !peer.ip().is_loopback() {
        tracing::warn!(%peer, "Rejecting bridge connection from non-loopback peer");
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
            tracing::warn!(%peer, error = %err, "Bridge handshake failed");
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
    let app_kind = register.app_kind;
    let app_name = match &app_kind {
        Some(kind) => kind.clone(),
        None => get_process_name(app_pid).unwrap_or_else(|| format!("unknown_{app_pid}")),
    };

    let (outbound_tx, outbound_rx) = mpsc::channel::<Frame>(OUTBOUND_QUEUE_SIZE);

    if let Some(prev) = service.registry.insert(
        app_pid,
        RegisteredClient {
            tx: outbound_tx.clone(),
            host_pid,
            app_pid,
            app_name: app_name.clone(),
            app_kind: app_kind.clone(),
        },
    ) {
        tracing::warn!(
            app_pid,
            previous_host_pid = prev.host_pid,
            "Replacing existing registration for app_pid",
        );
    }

    tracing::info!(
        app_pid,
        host_pid,
        app_name = %app_name,
        ?app_kind,
        %peer,
        "Bridge client registered",
    );

    let _ = service.registrations_tx.send(RegistrationEvent {
        app_pid,
        app_name: app_name.clone(),
        app_kind: app_kind.clone(),
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
            app_kind: removed.app_kind,
        });
        tracing::info!(
            app_pid,
            host_pid,
            remaining = service.registry.len(),
            "Bridge client unregistered",
        );
    } else {
        tracing::warn!(
            app_pid,
            host_pid,
            "Did not unregister: registration was replaced or already removed",
        );
    }

    if let Err(err) = writer.await {
        tracing::debug!(app_pid, error = %err, "Writer task ended");
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
            other.variant_name()
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
                        tracing::error!(error = %err, "Failed to serialize outbound frame");
                        continue;
                    }
                };
                if let Err(err) = sink.send(Message::Text(json.into())).await {
                    tracing::debug!(error = %err, "Failed to write outbound frame");
                    break;
                }
            }
            _ = heartbeat.tick() => {
                if let Err(err) = sink.send(Message::Ping(Default::default())).await {
                    tracing::debug!(error = %err, "Heartbeat ping failed");
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
                tracing::debug!(app_pid, error = %err, "Websocket error from client");
                break;
            }
        };

        match message {
            Message::Text(text) => match serde_json::from_str::<Frame>(text.as_str()) {
                Ok(frame) => {
                    if let Err(err) = service.frames_from_clients_tx.send((app_pid, frame)) {
                        tracing::trace!(
                            app_pid,
                            error = %err,
                            "No subscribers for inbound frame",
                        );
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        app_pid,
                        error = %err,
                        "Failed to parse inbound frame",
                    );
                }
            },
            Message::Binary(_) => {
                tracing::warn!(app_pid, "Ignoring unexpected binary frame");
            }
            Message::Ping(_) | Message::Pong(_) => {
                // axum auto-responds to Ping with Pong; nothing to do.
            }
            Message::Close(frame) => {
                tracing::debug!(app_pid, ?frame, "Client closed connection");
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
    use tokio::time::timeout;

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
                app_kind: None,
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

    #[tokio::test]
    async fn send_request_surfaces_client_error_frame() {
        let service = BridgeService::new();

        let (outbound_tx, mut outbound_rx) = mpsc::channel::<Frame>(8);
        service.registry.insert(
            9,
            RegisteredClient {
                tx: outbound_tx,
                host_pid: 1,
                app_pid: 9,
                app_name: "test".into(),
                app_kind: None,
            },
        );

        let svc = service.clone();
        let request_handle = tokio::spawn(async move { svc.send_request(9, "PING", None).await });

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
                9,
                Frame::from(ErrorFrame {
                    id: req.id,
                    code: 42,
                    message: "boom".into(),
                    details: Some("trace".into()),
                }),
            ))
            .expect("broadcast send");

        let result = timeout(Duration::from_secs(1), request_handle)
            .await
            .expect("request future")
            .expect("join");
        match result {
            Err(BridgeError::Client { message, details }) => {
                assert_eq!(message, "boom");
                assert_eq!(details.as_deref(), Some("trace"));
            }
            other => panic!("expected Client error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn find_clients_by_kind_returns_only_matching_clients() {
        let service = BridgeService::new();
        let (tx, _rx) = mpsc::channel::<Frame>(8);

        service.registry.insert(
            10,
            RegisteredClient {
                tx: tx.clone(),
                host_pid: 1,
                app_pid: 10,
                app_name: "microsoft-word".into(),
                app_kind: Some("microsoft-word".into()),
            },
        );
        service.registry.insert(
            11,
            RegisteredClient {
                tx: tx.clone(),
                host_pid: 1,
                app_pid: 11,
                app_name: "Chrome".into(),
                app_kind: None,
            },
        );
        service.registry.insert(
            12,
            RegisteredClient {
                tx,
                host_pid: 2,
                app_pid: 12,
                app_name: "microsoft-word".into(),
                app_kind: Some("microsoft-word".into()),
            },
        );

        let mut found = service.find_clients_by_kind("microsoft-word");
        found.sort_unstable();
        assert_eq!(found, vec![10, 12]);
        assert!(service.find_clients_by_kind("safari").is_empty());
    }

    #[tokio::test]
    async fn bind_without_tls_fails_loudly() {
        let service = BridgeService::new();
        let err = service
            .bind_on(([127, 0, 0, 1], 0).into())
            .await
            .expect_err("bind must fail without TLS material");
        assert!(matches!(err, BridgeError::TlsNotConfigured));
    }
}
