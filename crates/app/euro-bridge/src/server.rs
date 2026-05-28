use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::ConnectInfo;
use axum::extract::State;
use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket, WebSocketUpgrade, close_code};
use axum::response::IntoResponse;
use axum::routing::get;
use axum_server::Handle as AxumHandle;
use dashmap::DashMap;
use euro_bridge_protocol::{
    BRIDGE_BIND_IP, BRIDGE_PATH, BRIDGE_PORT, BridgeError, CancelFrame, ErrorFrame, EventFrame,
    Frame, FrameKind, Payload, RegisterFrame, RequestFrame, ResponseFrame, ShutdownFrame,
    bridge_url_for,
};
use euro_process::lookup_process_name;
use euro_transport_policy::{
    BRIDGE_HEARTBEAT_INTERVAL, BRIDGE_REGISTER_TIMEOUT, BRIDGE_REQUEST_TIMEOUT,
    BRIDGE_SHUTDOWN_GRACE,
};
use futures_util::{SinkExt, StreamExt};
use request_correlator::{RequestCorrelator, WaitError};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, oneshot};

/// `EventFrame.action` clients send to publish a fresh
/// [`BundledExtensionState`] to the desktop. Payload is JSON-encoded
/// [`ExtensionStatePayload`].
pub const EXTENSION_STATE_EVENT: &str = "EXTENSION_STATE_CHANGED";

const OUTBOUND_QUEUE_SIZE: usize = 32;

static GLOBAL_SERVICE: OnceLock<BridgeService> = OnceLock::new();

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

/// State of an extension that ships bundled with its host application —
/// today, the Safari Web Extension wrapped by the macOS launcher, but the
/// shape generalizes to any future bundled integration.
///
/// Browsers that distribute via a public store (Chrome Web Store, AMO,
/// Edge Add-ons) don't use this — for those, "extension is connected to
/// the bridge" is itself a sufficient signal that the extension is both
/// installed and enabled. Bundled extensions need an out-of-band probe
/// because the host app may run with the extension turned off in the
/// browser's settings, in which case the extension never connects but is
/// nevertheless installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BundledExtensionState {
    /// The extension is installed and enabled in the host browser. The
    /// content-side connection may still race ahead of or behind the
    /// browser actually launching it.
    Enabled,
    /// The extension is installed but disabled in the host browser's
    /// settings. The user must enable it manually — there is no API to
    /// flip this state programmatically.
    Disabled,
    /// The host browser has no record of the extension. Typically the
    /// containing app has never been launched on this machine, so Safari
    /// hasn't indexed the bundled `.appex`.
    NotDiscovered,
    /// Probe failed or the publisher hasn't reported yet. Treated as a
    /// transient unknown — the UI should fall back to a generic affordance.
    Unknown,
}

/// Wire payload of the [`EXTENSION_STATE_EVENT`] event. Clients serialize
/// this as JSON into `EventFrame.payload`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionStatePayload {
    /// Logical client identifier (matches `RegisterFrame.app_kind`). The
    /// desktop indexes bundled-extension state by this key, so a single
    /// publisher can report state for the kind it represents (e.g. a
    /// macOS launcher reporting `"safari"`).
    pub app_kind: String,
    pub state: BundledExtensionState,
}

/// Broadcast on the extension-state channel whenever a client publishes a
/// fresh [`BundledExtensionState`]. Subscribers receive the *latest known*
/// state for that `app_kind`; duplicate values are not deduplicated here
/// because the publisher is expected to send only on transitions.
#[derive(Debug, Clone)]
pub struct ExtensionStateUpdate {
    pub app_kind: String,
    pub state: BundledExtensionState,
}

/// Bookkeeping for a bridge listener that the service owns. Held
/// inside the service's `server` mutex; keyed off the listener's
/// lifetime — a slot is populated as soon as [`BridgeService::bind_on`]
/// hands out a [`BoundServer`] and stays populated until that server
/// is either served and stopped, or dropped without serving.
struct ServerHandle {
    state: ServerState,
    local_addr: SocketAddr,
    /// Sibling-family loopback listener address — see
    /// [`BoundServer::secondary_local_addr`]. `None` if the sibling bind
    /// was skipped or soft-failed.
    secondary_local_addr: Option<SocketAddr>,
    /// Created in [`BridgeService::bind_on`] so [`BridgeService::stop_server`]
    /// can always signal shutdown — even when a spawned `serve` task hasn't
    /// been polled yet. axum-server stores the shutdown flag on an
    /// `AtomicBool`, so a pre-notified handle causes the accept loop to
    /// exit on its first iteration when serve eventually runs.
    axum_handle: AxumHandle,
    /// Fires when the slot is fully cleared — either by `serve` completing
    /// or by an unserved [`BoundServer`] being dropped. `None` once a
    /// `stop_server` caller has taken it.
    done: Option<oneshot::Receiver<()>>,
}

/// Pair of bridge listeners owned by a [`BoundServer`]: the primary at
/// the caller-requested address plus an optional sibling-family loopback
/// listener bound to the same port. The sibling listener is what lets
/// the bridge be reachable through both `127.0.0.1` and `[::1]`
/// regardless of which family a client's resolver returns first.
struct Listeners {
    primary: std::net::TcpListener,
    /// `None` when the primary is not loopback (no canonical sibling
    /// exists) or when the sibling bind soft-failed (IPv6 disabled,
    /// port held on the sibling family, sandbox rules).
    secondary: Option<std::net::TcpListener>,
}

enum ServerState {
    /// `bind_on` returned a [`BoundServer`] but no one has called
    /// [`BoundServer::serve`] on it yet (or the BoundServer was
    /// dropped without serving — Drop clears the slot before that
    /// becomes observable).
    Bound,
    /// A serve loop is running.
    Serving,
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
#[must_use = "BoundServer drops the listening socket(s) if not served"]
pub struct BoundServer {
    service: BridgeService,
    /// `None` only between `serve()` consuming the listeners and the
    /// struct being dropped — Drop uses this to detect "served vs
    /// abandoned" so it knows whether to clear the slot.
    listeners: Option<Listeners>,
    /// Address of the primary listener. Stable across the bind/serve
    /// transition.
    local_addr: SocketAddr,
    /// Address of the sibling-family loopback listener, or `None` when
    /// the sibling bind was skipped or soft-failed. Stable across the
    /// bind/serve transition.
    secondary_local_addr: Option<SocketAddr>,
    /// Fires the slot's `done` receiver. Taken by `serve` and fired after
    /// the accept loop exits; otherwise fired by Drop when this struct is
    /// abandoned without serving so any in-flight `stop_server` waiter is
    /// unblocked.
    done_tx: Option<oneshot::Sender<()>>,
}

impl std::fmt::Debug for BoundServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoundServer")
            .field("local_addr", &self.local_addr)
            .field("secondary_local_addr", &self.secondary_local_addr)
            .finish_non_exhaustive()
    }
}

impl BoundServer {
    /// Address of the primary listener. Stable across the bind/serve
    /// transition.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Address of the sibling-family loopback listener bound to the same
    /// port as [`local_addr`](Self::local_addr), or `None` if the
    /// sibling bind was skipped (non-loopback primary) or soft-failed
    /// (IPv6 disabled, port held on the sibling family, sandbox rules).
    /// Stable across the bind/serve transition.
    pub fn secondary_local_addr(&self) -> Option<SocketAddr> {
        self.secondary_local_addr
    }

    /// Run the accept loop until the owning service is asked to stop
    /// (via [`BridgeService::stop_server`]) or one of the loops exits
    /// with an error.
    ///
    /// This future drives the accept loop(s) directly; the typical
    /// startup pattern is `tokio::spawn(bound.serve())`. Awaiting it
    /// inline blocks until both loops have ended. When a sibling
    /// listener was bound, both axum servers share the same
    /// `AxumHandle`, so [`BridgeService::stop_server`] fans its
    /// graceful-shutdown signal out to both.
    pub async fn serve(mut self) -> Result<(), BridgeError> {
        // Both fields are populated by `bind_on` and only taken here.
        // `serve` consumes `self`, so a second call is statically
        // impossible.
        let listeners = self.listeners.take().unwrap();
        let done_tx = self.done_tx.take().unwrap();
        let service = self.service.clone();
        let local_addr = self.local_addr;
        let secondary_local_addr = self.secondary_local_addr;
        // Drop early so the Drop impl runs before we hand off control —
        // it sees `listeners` is `None` and leaves the slot alone.
        drop(self);

        let axum_handle = {
            let mut guard = service.server.lock().expect("server slot poisoned");
            let slot = guard
                .as_mut()
                .expect("BoundServer outlives its service slot");
            match slot.state {
                ServerState::Bound => {
                    slot.state = ServerState::Serving;
                    slot.axum_handle.clone()
                }
                ServerState::Serving => {
                    return Err(BridgeError::AlreadyRunning { local_addr });
                }
            }
        };

        let app = Router::new()
            .route(BRIDGE_PATH, get(ws_upgrade))
            .with_state(service.clone());

        let serve_primary = axum_server::from_tcp(listeners.primary)
            .handle(axum_handle.clone())
            .serve(
                app.clone()
                    .into_make_service_with_connect_info::<SocketAddr>(),
            );

        let result = match listeners.secondary {
            None => serve_primary.await,
            Some(secondary) => {
                let serve_secondary = axum_server::from_tcp(secondary)
                    .handle(axum_handle)
                    .serve(app.into_make_service_with_connect_info::<SocketAddr>());
                tokio::try_join!(serve_primary, serve_secondary).map(|_| ())
            }
        };

        // Clear the slot before signalling done so a follow-up `bind`
        // observes a clean state immediately.
        {
            let mut guard = service.server.lock().expect("server slot poisoned");
            *guard = None;
        }
        let _ = done_tx.send(());

        match result {
            Ok(()) => {
                tracing::info!(
                    %local_addr,
                    secondary = ?secondary_local_addr,
                    "Bridge WebSocket server stopped",
                );
                Ok(())
            }
            Err(source) => {
                tracing::error!(
                    %local_addr,
                    secondary = ?secondary_local_addr,
                    error = %source,
                    "Bridge WebSocket server error",
                );
                Err(BridgeError::Serve { source })
            }
        }
    }
}

impl Drop for BoundServer {
    fn drop(&mut self) {
        // If `serve` ran, it took the listeners and is responsible for
        // clearing the slot. If we still hold them, the BoundServer is
        // being abandoned — release the slot so the service is
        // rebindable.
        if self.listeners.is_none() {
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
                secondary = ?self.secondary_local_addr,
                "Bridge server slot poisoned while dropping unserved BoundServer",
            ),
        }
        // Wake any `stop_server` caller waiting on `done`. The receiver
        // may have already been dropped along with the slot above, in
        // which case `send` returns `Err` and we ignore it.
        if let Some(done_tx) = self.done_tx.take() {
            let _ = done_tx.send(());
        }
    }
}

/// In-process bridge service. A single instance is shared via
/// [`BridgeService::get_or_init`]; clones are cheap because all state
/// lives behind `Arc`s and tokio channels.
#[derive(Clone)]
pub struct BridgeService {
    registry: Arc<DashMap<u32, RegisteredClient>>,
    /// Latest [`BundledExtensionState`] per `app_kind`. Populated when a
    /// client publishes [`EXTENSION_STATE_EVENT`]; cleared on the
    /// publisher's disconnect so the desktop doesn't keep serving stale
    /// state across launcher restarts.
    extension_states: Arc<DashMap<String, BundledExtensionState>>,
    frames_from_clients_tx: broadcast::Sender<(u32, Frame)>,
    events_tx: broadcast::Sender<(u32, EventFrame)>,
    registrations_tx: broadcast::Sender<RegistrationEvent>,
    disconnects_tx: broadcast::Sender<RegistrationEvent>,
    extension_states_tx: broadcast::Sender<ExtensionStateUpdate>,
    pending_requests: RequestCorrelator<u32, ResponseFrame, ErrorFrame>,
    request_id_counter: Arc<AtomicU32>,
    server: Arc<StdMutex<Option<ServerHandle>>>,
    /// Browser-purge deadline. While `Instant::now() < purge_until`,
    /// every newly-registering browser client (`app_kind == None`) is
    /// sent a [`ShutdownFrame`] right after the handshake. Stored as
    /// nanoseconds since [`Self::anchor`] in an atomic so the per-
    /// connection check is lock-free; `0` means "no window open".
    purge_until_nanos: Arc<AtomicU64>,
    /// Monotonic anchor used to convert [`Instant`]s to/from the
    /// nanoseconds stored in [`Self::purge_until_nanos`].
    anchor: Instant,
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
        let (extension_states_tx, _) = broadcast::channel(32);

        let service = Self {
            registry: Arc::new(DashMap::new()),
            extension_states: Arc::new(DashMap::new()),
            frames_from_clients_tx,
            events_tx,
            registrations_tx,
            disconnects_tx,
            extension_states_tx,
            pending_requests: RequestCorrelator::new(),
            request_id_counter: Arc::new(AtomicU32::new(1)),
            server: Arc::new(StdMutex::new(None)),
            purge_until_nanos: Arc::new(AtomicU64::new(0)),
            anchor: Instant::now(),
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

    fn spawn_frame_handler(&self) {
        let pending_requests = self.pending_requests.clone();
        let events_tx = self.events_tx.clone();
        let extension_states = Arc::clone(&self.extension_states);
        let extension_states_tx = self.extension_states_tx.clone();
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
                        let action = resp.action.clone();
                        pending_requests.resolve(id, Ok(resp));
                        tracing::trace!(
                            request_id = id,
                            action = %action,
                            "Resolved pending request with response",
                        );
                    }
                    FrameKind::Event(evt) => {
                        if evt.action == EXTENSION_STATE_EVENT {
                            handle_extension_state_event(
                                app_pid,
                                &evt,
                                &extension_states,
                                &extension_states_tx,
                            );
                            // Fall through: also broadcast on the generic event
                            // channel so subscribers that want raw frames still
                            // see it.
                        }
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
                        pending_requests.resolve(id, Err(err));
                    }
                    FrameKind::Cancel(cancel) => {
                        // Drop the pending entry without synthesizing a
                        // structured error: the waiter wakes with
                        // `WaitError::SenderDropped`, which `send_request`
                        // maps to `BridgeError::ChannelClosed` — preserving
                        // the pre-correlator behaviour.
                        pending_requests.drop_silently(cancel.id);
                        tracing::debug!(request_id = cancel.id, "Cancelled pending request");
                    }
                    FrameKind::Register(_) => {
                        tracing::warn!(app_pid, "Received Register frame outside the handshake",);
                    }
                    FrameKind::Shutdown(_) => {
                        tracing::warn!(
                            app_pid,
                            "Received Shutdown frame from client; only the desktop may send this",
                        );
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
    /// When `bind_addr` is a loopback address, [`bind_on`](Self::bind_on)
    /// also opens a best-effort sibling-family loopback listener on the
    /// same port: `[::1]` if the primary is IPv4, `127.0.0.1` if the
    /// primary is IPv6. This makes the bridge reachable through both
    /// families regardless of which one a client's resolver returns
    /// first — the failure mode we hit on Windows machines where
    /// `localhost` resolves to `::1` ahead of `127.0.0.1`. Sibling-bind
    /// failures (IPv6 disabled, sibling port held, sandbox rules) log a
    /// warning and proceed with the primary listener only; callers can
    /// observe whether the sibling came up via
    /// [`BoundServer::secondary_local_addr`].
    ///
    /// The kernel socket(s) are in `LISTEN` state by the time this
    /// returns, so clients dialing either address can never race the
    /// bind.
    ///
    /// If a previous accept loop is still registered on the service,
    /// returns [`BridgeError::AlreadyRunning`] — callers that want
    /// "ensure running" semantics should check
    /// [`local_addr`](BridgeService::local_addr) first.
    pub async fn bind_on(&self, bind_addr: SocketAddr) -> Result<BoundServer, BridgeError> {
        // axum-server's `from_tcp` adopts an already-bound
        // `std::net::TcpListener`, which is exactly the shape we want:
        // the `local_addr` is observable here, and the kernel socket is
        // accepting connections before the caller ever sees the
        // `BoundServer`.
        //
        // Reserve the slot synchronously around the TCP bind so two
        // concurrent callers can't both succeed.
        let mut guard = self.server.lock().expect("server slot poisoned");
        if let Some(handle) = guard.as_ref() {
            return Err(BridgeError::AlreadyRunning {
                local_addr: handle.local_addr,
            });
        }

        let primary = bind_loopback_listener(bind_addr).map_err(|source| BridgeError::Bind {
            addr: bind_addr,
            source,
        })?;
        let local_addr = primary.local_addr().map_err(|source| BridgeError::Bind {
            addr: bind_addr,
            source,
        })?;

        // Sibling-family listener — best-effort. A failure here is
        // expected on hosts where IPv6 is disabled, the sibling port is
        // taken by another process, or sandbox rules prevent the bind;
        // any of those degrades us to single-family operation without
        // failing startup.
        let secondary =
            sibling_loopback_addr(local_addr).and_then(|addr| match bind_loopback_listener(addr) {
                Ok(listener) => Some(listener),
                Err(err) => {
                    tracing::warn!(
                        sibling = %addr,
                        error = %err,
                        "Could not bind sibling-family loopback listener; \
                         bridge will only accept on primary",
                    );
                    None
                }
            });
        let secondary_local_addr = secondary
            .as_ref()
            .and_then(|listener| listener.local_addr().ok());

        let axum_handle = AxumHandle::new();
        let (done_tx, done_rx) = oneshot::channel::<()>();
        *guard = Some(ServerHandle {
            state: ServerState::Bound,
            local_addr,
            secondary_local_addr,
            axum_handle,
            done: Some(done_rx),
        });
        drop(guard);

        tracing::info!(
            primary = %local_addr,
            secondary = ?secondary_local_addr,
            url = %bridge_url_for(local_addr),
            "Bridge transport bound (plaintext / ws, dual-stack loopback)",
        );

        Ok(BoundServer {
            service: self.clone(),
            listeners: Some(Listeners { primary, secondary }),
            local_addr,
            secondary_local_addr,
            done_tx: Some(done_tx),
        })
    }

    /// Address of the primary listener, or `None` if no listener is
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

    /// Address of the sibling-family loopback listener, or `None` when
    /// no listener is registered, the primary is not loopback, or the
    /// sibling bind soft-failed. See [`BoundServer::secondary_local_addr`].
    pub fn secondary_local_addr(&self) -> Option<SocketAddr> {
        self.server
            .lock()
            .expect("server slot poisoned")
            .as_ref()
            .and_then(|handle| handle.secondary_local_addr)
    }

    /// Signal the running server to shut down, then wait for the
    /// accept loop and any in-flight connections to fully terminate.
    /// No-op if no listener is currently registered.
    ///
    /// Calling this while the slot is in [`ServerState::Bound`] (i.e. a
    /// [`BoundServer`] was issued but its `serve()` future hasn't been
    /// polled yet) pre-notifies the axum handle: when `serve` eventually
    /// runs, its accept loop sees the shutdown flag on the first
    /// iteration and exits immediately. This is what makes the typical
    /// `bind_on` → `tokio::spawn(serve)` → `stop_server` sequence
    /// cancellation-safe under a current-thread runtime, where the
    /// spawned `serve` may not have had a chance to run yet.
    ///
    /// A concurrent [`bind`](BridgeService::bind) during shutdown sees
    /// the slot still populated and returns
    /// [`BridgeError::AlreadyRunning`] until the slot is cleared (by
    /// `serve` ending or the unserved [`BoundServer`] being dropped).
    pub async fn stop_server(&self) {
        let (axum_handle, done) = {
            let mut guard = self.server.lock().expect("server slot poisoned");
            let Some(slot) = guard.as_mut() else {
                tracing::debug!("Bridge WebSocket server is not running");
                return;
            };
            (slot.axum_handle.clone(), slot.done.take())
        };

        tracing::info!("Sending shutdown signal to bridge WebSocket server");
        axum_handle.graceful_shutdown(Some(BRIDGE_SHUTDOWN_GRACE));

        if let Some(done) = done {
            // Fired by `serve` when its accept loop ends, or by
            // `BoundServer::drop` when an unserved listener is
            // abandoned — either way the slot is gone by the time we
            // wake up.
            let _ = done.await;
        }
    }

    /// Open a window during which every newly-registering browser
    /// messenger (`app_kind == None`) is sent a [`ShutdownFrame`]
    /// immediately after the handshake.
    ///
    /// Used by the desktop right after it has replaced the messenger
    /// binary on disk: the previous-session messengers that are sitting
    /// in their reconnect-backoff loop reconnect to the new bridge, land
    /// inside this window, and are cleared out so the browser respawns
    /// them from the new binary. After the window closes, normal
    /// operation resumes — newly-spawned messengers register and stay
    /// connected.
    ///
    /// The Word add-in (`app_kind == Some("microsoft-word")`) is never
    /// shut down by this mechanism; its sandboxed runtime can't be
    /// respawned the same way as a native-messaging host.
    ///
    /// Calling this while a window is already open extends the deadline
    /// only if the new deadline is later than the existing one.
    pub fn open_browser_purge_window(&self, duration: Duration) {
        let new_deadline = self.instant_to_nanos(Instant::now() + duration);
        // `fetch_max` keeps the later deadline if a window is already
        // open. Two concurrent callers always converge on the latest
        // requested deadline without ever shrinking it.
        self.purge_until_nanos
            .fetch_max(new_deadline, Ordering::Relaxed);
        tracing::info!(
            duration_ms = duration.as_millis(),
            "Opened browser-messenger purge window",
        );
    }

    /// Returns `true` while a purge window is active. Lock-free.
    fn is_in_purge_window(&self) -> bool {
        let deadline = self.purge_until_nanos.load(Ordering::Relaxed);
        if deadline == 0 {
            return false;
        }
        self.instant_to_nanos(Instant::now()) < deadline
    }

    /// Convert an [`Instant`] to nanoseconds since [`Self::anchor`].
    /// Saturates at `u64::MAX`; clamps below the anchor to `1` so the
    /// "no window" sentinel of `0` is never collided with.
    fn instant_to_nanos(&self, when: Instant) -> u64 {
        when.checked_duration_since(self.anchor)
            .map(|d| d.as_nanos().min(u64::MAX as u128) as u64)
            .map(|n| n.max(1))
            .unwrap_or(1)
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

    /// Receive a [`ExtensionStateUpdate`] every time a client publishes a
    /// fresh [`BundledExtensionState`] via [`EXTENSION_STATE_EVENT`].
    pub fn subscribe_to_extension_states(&self) -> broadcast::Receiver<ExtensionStateUpdate> {
        self.extension_states_tx.subscribe()
    }

    /// Latest [`BundledExtensionState`] published for `app_kind`, or
    /// [`BundledExtensionState::Unknown`] if no client has reported one yet
    /// (or the publisher has disconnected since).
    pub fn bundled_extension_state(&self, app_kind: &str) -> BundledExtensionState {
        self.extension_states
            .get(app_kind)
            .map_or(BundledExtensionState::Unknown, |entry| *entry.value())
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

    /// Return whether any client is currently connected for the given
    /// `app_name`. Cheaper than [`find_pid_by_app_name`] for callers that
    /// only need a presence test.
    pub fn is_connected_by_app_name(&self, app_name: &str) -> bool {
        self.registry
            .iter()
            .any(|entry| entry.value().app_name == app_name)
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
    /// Times out after [`BRIDGE_REQUEST_TIMEOUT`]; on timeout a
    /// `Cancel` frame is sent so the client can drop any work it
    /// started.
    pub async fn send_request(
        &self,
        app_pid: u32,
        action: &str,
        payload: Option<Payload>,
    ) -> Result<ResponseFrame, BridgeError> {
        let request_id = self.request_id_counter.fetch_add(1, Ordering::Relaxed);
        // Drop-guard removes the pending entry on every exit path,
        // including the early return on send failure and panics.
        let guard = self.pending_requests.register(request_id);

        let request = Frame::from(RequestFrame {
            id: request_id,
            action: action.to_string(),
            payload,
        });

        tracing::debug!(app_pid, request_id, action, "Sending bridge request");

        self.send_to_client(app_pid, request).await?;

        match guard.wait(BRIDGE_REQUEST_TIMEOUT).await {
            Ok(Ok(resp)) => Ok(resp),
            Ok(Err(err)) => Err(BridgeError::Client {
                code: err.code,
                message: err.message,
                details: err.details,
            }),
            Err(WaitError::SenderDropped) => Err(BridgeError::ChannelClosed),
            Err(WaitError::Cancelled) => Err(BridgeError::ChannelClosed),
            Err(WaitError::Timeout) => {
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
        None => lookup_process_name(app_pid).unwrap_or_else(|| format!("unknown_{app_pid}")),
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

    // Browser messengers reconnecting from the previous desktop session
    // land here right after the new bridge binds. While the purge window
    // is open, ask them to exit so the browser respawns them from the
    // freshly-installed binary. The Word add-in is exempt — see
    // `open_browser_purge_window`.
    if app_kind.is_none() && service.is_in_purge_window() {
        let shutdown = Frame::from(ShutdownFrame {
            reason: Some("desktop installed an updated messenger binary".into()),
        });
        if let Err(err) = outbound_tx.send(shutdown).await {
            tracing::debug!(
                app_pid,
                host_pid,
                error = %err,
                "Failed to enqueue Shutdown for stale browser messenger",
            );
        } else {
            tracing::info!(
                app_pid,
                host_pid,
                "Sent Shutdown to stale browser messenger",
            );
        }
    }

    let writer = tokio::spawn(writer_task(sink, outbound_rx));
    reader_loop(&service, &mut stream, app_pid).await;

    drop(outbound_tx);

    if let Some((_, removed)) = service
        .registry
        .remove_if(&app_pid, |_, client| client.host_pid == host_pid)
    {
        // Drop any bundled-extension state this client owned. Without this
        // the desktop would keep serving the last value forever — fine while
        // the publisher is alive, but misleading after the launcher exits or
        // crashes.
        if let Some(kind) = removed.app_kind.as_deref()
            && service.extension_states.remove(kind).is_some()
        {
            let _ = service.extension_states_tx.send(ExtensionStateUpdate {
                app_kind: kind.to_owned(),
                state: BundledExtensionState::Unknown,
            });
        }
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
    let next = tokio::time::timeout(BRIDGE_REGISTER_TIMEOUT, stream.next())
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
    let mut heartbeat = tokio::time::interval(BRIDGE_HEARTBEAT_INTERVAL);
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

/// Decode an [`EXTENSION_STATE_EVENT`] payload, store it in
/// `extension_states`, and broadcast the update. Malformed payloads are
/// dropped with a warning — clients are expected to send valid JSON
/// matching [`ExtensionStatePayload`], and the wrong shape is a publisher
/// bug rather than something the desktop should paper over.
fn handle_extension_state_event(
    app_pid: u32,
    evt: &EventFrame,
    extension_states: &DashMap<String, BundledExtensionState>,
    extension_states_tx: &broadcast::Sender<ExtensionStateUpdate>,
) {
    let Some(payload_value) = evt.payload.as_ref() else {
        tracing::warn!(app_pid, "EXTENSION_STATE_CHANGED event missing payload");
        return;
    };
    let payload: ExtensionStatePayload = match payload_value.deserialize() {
        Ok(payload) => payload,
        Err(err) => {
            tracing::warn!(
                app_pid,
                error = %err,
                "EXTENSION_STATE_CHANGED payload was not valid JSON",
            );
            return;
        }
    };

    // Only emit on transitions to keep subscribers from churning on
    // duplicate ticks. Publishers (e.g. the macOS launcher's 1-Hz Safari
    // poll) intentionally re-send the current state on each tick to be
    // robust to dropped frames; deduplicating here is the right place to
    // absorb that.
    let prev = extension_states.insert(payload.app_kind.clone(), payload.state);
    if prev == Some(payload.state) {
        return;
    }

    tracing::debug!(
        app_pid,
        app_kind = %payload.app_kind,
        state = ?payload.state,
        "Bundled extension state updated",
    );
    let _ = extension_states_tx.send(ExtensionStateUpdate {
        app_kind: payload.app_kind,
        state: payload.state,
    });
}

/// Bind a non-blocking TCP listener at `addr`. Wraps the std bind and
/// the immediately-following `set_nonblocking` so callers handle a
/// single `io::Result` instead of two. `axum-server::from_tcp` requires
/// the listener already be non-blocking.
fn bind_loopback_listener(addr: SocketAddr) -> std::io::Result<std::net::TcpListener> {
    let listener = std::net::TcpListener::bind(addr)?;
    listener.set_nonblocking(true)?;
    Ok(listener)
}

/// Canonical sibling-family loopback address for dual-stack bind: the
/// IPv6 loopback for an IPv4 primary and vice-versa, on the primary's
/// port. Returns `None` for non-loopback primaries — those bindings are
/// explicit per-address requests where opening a sibling would surprise
/// the caller.
fn sibling_loopback_addr(primary: SocketAddr) -> Option<SocketAddr> {
    match primary.ip() {
        IpAddr::V4(ip) if ip.is_loopback() => Some(SocketAddr::new(
            IpAddr::V6(Ipv6Addr::LOCALHOST),
            primary.port(),
        )),
        IpAddr::V6(ip) if ip.is_loopback() => Some(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            primary.port(),
        )),
        _ => None,
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
                    payload: Some(Payload::from_value(&"pong").unwrap()),
                }),
            ))
            .expect("broadcast send");

        let response = timeout(Duration::from_secs(1), request_handle)
            .await
            .expect("request future")
            .expect("join")
            .expect("response");
        let payload: String = response
            .payload
            .as_ref()
            .expect("payload present")
            .deserialize()
            .expect("decode payload");
        assert_eq!(payload, "pong");
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
                    details: Some(Payload::from_value(&"trace").unwrap()),
                }),
            ))
            .expect("broadcast send");

        let result = timeout(Duration::from_secs(1), request_handle)
            .await
            .expect("request future")
            .expect("join");
        match result {
            Err(BridgeError::Client {
                code,
                message,
                details,
            }) => {
                assert_eq!(code, 42);
                assert_eq!(message, "boom");
                let details: String = details
                    .as_ref()
                    .expect("details present")
                    .deserialize()
                    .expect("decode details");
                assert_eq!(details, "trace");
            }
            other => panic!("expected Client error, got {other:?}"),
        }
    }

    mod purge_window {
        use super::*;
        use euro_bridge_protocol::bridge_url_for;
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::tungstenite::Message as WsMessage;
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;

        /// Bind on an ephemeral port and start serving. Returns the
        /// service plus the URL clients should dial. The serve task is
        /// detached; the test relies on `tokio` runtime teardown to
        /// reap it once the test exits.
        async fn spawn_serving_bridge() -> (BridgeService, String) {
            let service = BridgeService::new();
            let bound = service
                .bind_on(([127, 0, 0, 1], 0).into())
                .await
                .expect("bind");
            let url = bridge_url_for(bound.local_addr());
            tokio::spawn(async move {
                let _ = bound.serve().await;
            });
            (service, url)
        }

        async fn dial(
            url: &str,
        ) -> tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        > {
            let request = url.into_client_request().expect("client request");
            let (socket, _) = tokio_tungstenite::connect_async(request)
                .await
                .expect("connect");
            socket
        }

        async fn send_register<S>(socket: &mut S, host_pid: u32, app_pid: u32, kind: Option<&str>)
        where
            S: SinkExt<WsMessage, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
        {
            let frame = Frame::from(RegisterFrame {
                host_pid,
                app_pid,
                app_kind: kind.map(str::to_string),
            });
            let json = serde_json::to_string(&frame).expect("serialize");
            socket
                .send(WsMessage::Text(json.into()))
                .await
                .expect("send Register");
        }

        async fn next_text_frame<S>(socket: &mut S) -> Option<Frame>
        where
            S: StreamExt<Item = Result<WsMessage, tokio_tungstenite::tungstenite::Error>> + Unpin,
        {
            let result = timeout(Duration::from_millis(500), socket.next()).await;
            match result {
                Ok(Some(Ok(WsMessage::Text(text)))) => {
                    Some(serde_json::from_str(text.as_str()).expect("frame json"))
                }
                _ => None,
            }
        }

        #[tokio::test]
        async fn shutdowns_browser_client_within_window() {
            let (service, url) = spawn_serving_bridge().await;
            service.open_browser_purge_window(Duration::from_secs(1));

            let mut socket = dial(&url).await;
            send_register(&mut socket, 1234, 5678, None).await;

            let frame = next_text_frame(&mut socket)
                .await
                .expect("expected Shutdown frame on outbound");
            assert!(
                matches!(frame.kind, FrameKind::Shutdown(_)),
                "expected Shutdown, got {:?}",
                frame.kind
            );
        }

        #[tokio::test]
        async fn does_not_shutdown_office_addin_within_window() {
            let (service, url) = spawn_serving_bridge().await;
            service.open_browser_purge_window(Duration::from_secs(1));

            let mut socket = dial(&url).await;
            send_register(&mut socket, 1234, 5678, Some("microsoft-word")).await;

            let frame = next_text_frame(&mut socket).await;
            assert!(
                frame.is_none(),
                "did not expect any frame for office add-in, got {frame:?}",
            );
        }

        #[tokio::test]
        async fn does_not_shutdown_browser_after_window_closes() {
            let (service, url) = spawn_serving_bridge().await;
            service.open_browser_purge_window(Duration::from_millis(50));
            tokio::time::sleep(Duration::from_millis(150)).await;

            let mut socket = dial(&url).await;
            send_register(&mut socket, 1234, 5678, None).await;

            let frame = next_text_frame(&mut socket).await;
            assert!(
                frame.is_none(),
                "expected no Shutdown after window closed, got {frame:?}",
            );
        }

        #[tokio::test]
        async fn no_window_open_means_no_shutdown() {
            let (_service, url) = spawn_serving_bridge().await;

            let mut socket = dial(&url).await;
            send_register(&mut socket, 1234, 5678, None).await;

            let frame = next_text_frame(&mut socket).await;
            assert!(
                frame.is_none(),
                "expected no Shutdown without an open window, got {frame:?}",
            );
        }

        #[tokio::test]
        async fn open_window_extends_but_never_shrinks_deadline() {
            let service = BridgeService::new();
            service.open_browser_purge_window(Duration::from_secs(60));
            let long_deadline = service.purge_until_nanos.load(Ordering::Relaxed);
            assert!(long_deadline > 0);

            // A shorter window must not retract the longer deadline.
            service.open_browser_purge_window(Duration::from_millis(10));
            assert_eq!(
                service.purge_until_nanos.load(Ordering::Relaxed),
                long_deadline,
                "shorter window should not retract a longer one",
            );
            assert!(
                service.is_in_purge_window(),
                "longer window should still be active",
            );
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
    async fn extension_state_event_updates_state_and_broadcasts() {
        let service = BridgeService::new();
        let mut updates = service.subscribe_to_extension_states();

        assert_eq!(
            service.bundled_extension_state("safari"),
            BundledExtensionState::Unknown,
        );

        let payload = Payload::from_value(&ExtensionStatePayload {
            app_kind: "safari".into(),
            state: BundledExtensionState::Disabled,
        })
        .unwrap();
        service
            .frames_from_clients_tx
            .send((
                42,
                Frame::from(EventFrame {
                    action: EXTENSION_STATE_EVENT.into(),
                    payload: Some(payload),
                }),
            ))
            .expect("broadcast send");

        let update = timeout(Duration::from_secs(1), updates.recv())
            .await
            .expect("update arrives")
            .expect("recv");
        assert_eq!(update.app_kind, "safari");
        assert_eq!(update.state, BundledExtensionState::Disabled);
        assert_eq!(
            service.bundled_extension_state("safari"),
            BundledExtensionState::Disabled,
        );
    }

    #[tokio::test]
    async fn duplicate_extension_state_is_deduplicated() {
        let service = BridgeService::new();
        let mut updates = service.subscribe_to_extension_states();

        let payload = Payload::from_value(&ExtensionStatePayload {
            app_kind: "safari".into(),
            state: BundledExtensionState::Enabled,
        })
        .unwrap();

        for _ in 0..3 {
            service
                .frames_from_clients_tx
                .send((
                    1,
                    Frame::from(EventFrame {
                        action: EXTENSION_STATE_EVENT.into(),
                        payload: Some(payload.clone()),
                    }),
                ))
                .expect("broadcast send");
        }

        let first = timeout(Duration::from_secs(1), updates.recv())
            .await
            .expect("first update")
            .expect("recv");
        assert_eq!(first.state, BundledExtensionState::Enabled);

        let second = timeout(Duration::from_millis(200), updates.recv()).await;
        assert!(
            second.is_err(),
            "expected no further updates for repeated state, got {second:?}",
        );
    }

    #[tokio::test]
    async fn malformed_extension_state_payload_is_ignored() {
        let service = BridgeService::new();
        service
            .frames_from_clients_tx
            .send((
                1,
                Frame::from(EventFrame {
                    action: EXTENSION_STATE_EVENT.into(),
                    // Valid JSON (a bare string), but not the
                    // `ExtensionStatePayload` shape — the handler
                    // should refuse to decode it.
                    payload: Some(Payload::from_value(&"not the right shape").unwrap()),
                }),
            ))
            .expect("broadcast send");

        // Give the handler a beat to (not) process the bad frame.
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(
            service.bundled_extension_state("safari"),
            BundledExtensionState::Unknown,
        );
    }

    #[test]
    fn bundled_extension_state_serializes_snake_case() {
        for (state, expected) in [
            (BundledExtensionState::Enabled, "\"enabled\""),
            (BundledExtensionState::Disabled, "\"disabled\""),
            (BundledExtensionState::NotDiscovered, "\"not_discovered\""),
            (BundledExtensionState::Unknown, "\"unknown\""),
        ] {
            assert_eq!(serde_json::to_string(&state).unwrap(), expected);
        }
    }
}
