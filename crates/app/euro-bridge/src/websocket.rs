//! WebSocket transport. Each connection carries the same `Frame` envelope
//! defined in [`euro_bridge_protocol`], serialized as JSON inside text
//! WebSocket messages — the wire is JSON-native because every consumer
//! (browser native-messaging hosts, Office.js add-ins, the macOS Safari
//! launcher) already speaks JSON natively.

use std::future::Future;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::Router;
use axum::extract::{
    State, WebSocketUpgrade,
    ws::{Message, WebSocket},
};
use axum::response::IntoResponse;
use axum::routing::any;
use euro_bridge_protocol::{Frame, FrameKind};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::{OnceCell, mpsc, watch};

use crate::process_name::get_process_name;
use crate::registry::RegisteredClient;
use crate::service::{APP_BRIDGE_PORT, AppBridgeService};

const PER_CLIENT_TX_CAPACITY: usize = 32;

static WS_SERVER_STARTED: AtomicBool = AtomicBool::new(false);
static WS_SHUTDOWN_TX: OnceCell<watch::Sender<bool>> = OnceCell::const_new();

/// Start the WebSocket server bound to `[::1]:APP_BRIDGE_PORT`. Idempotent.
pub async fn start_ws_server(service: &'static AppBridgeService) {
    if WS_SERVER_STARTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        tracing::debug!("App bridge WebSocket server already running");
        return;
    }

    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let _ = WS_SHUTDOWN_TX.set(shutdown_tx);

    tokio::spawn(async move {
        let bind_addr: SocketAddr = match format!("[::1]:{APP_BRIDGE_PORT}").parse() {
            Ok(addr) => addr,
            Err(err) => {
                tracing::error!("Invalid WebSocket bind address: {err}");
                WS_SERVER_STARTED.store(false, Ordering::SeqCst);
                return;
            }
        };

        let listener = match TcpListener::bind(bind_addr).await {
            Ok(listener) => listener,
            Err(err) => {
                tracing::error!("Failed to bind WebSocket listener at {bind_addr}: {err}");
                WS_SERVER_STARTED.store(false, Ordering::SeqCst);
                return;
            }
        };

        tracing::info!("Starting app bridge WebSocket server at {bind_addr}");

        let shutdown = async move {
            loop {
                if shutdown_rx.changed().await.is_err() {
                    break;
                }
                if *shutdown_rx.borrow() {
                    tracing::info!("Received shutdown signal for app bridge WebSocket server");
                    break;
                }
            }
        };

        if let Err(err) = serve_ws(listener, service, shutdown).await {
            tracing::error!("App bridge WebSocket server error: {err}");
        }

        WS_SERVER_STARTED.store(false, Ordering::SeqCst);
        tracing::info!("App bridge WebSocket server ended");
    });
}

/// Signal the WebSocket server to shut down. Idempotent.
pub async fn stop_ws_server() {
    if !WS_SERVER_STARTED.load(Ordering::SeqCst) {
        tracing::debug!("App bridge WebSocket server is not running");
        return;
    }
    if let Some(tx) = WS_SHUTDOWN_TX.get() {
        tracing::info!("Sending shutdown signal to app bridge WebSocket server");
        let _ = tx.send(true);
    }
}

pub fn is_ws_server_running() -> bool {
    WS_SERVER_STARTED.load(Ordering::SeqCst)
}

/// Run the WebSocket server until `shutdown` resolves. Useful for
/// integration tests that want a freshly-bound listener on an ephemeral
/// port. The public [`start_ws_server`] is a thin wrapper around this.
pub async fn serve_ws(
    listener: TcpListener,
    service: &'static AppBridgeService,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let app = Router::new()
        .route("/", any(ws_handler))
        .with_state(WsState { service });

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
}

#[derive(Clone)]
struct WsState {
    service: &'static AppBridgeService,
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<WsState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state.service))
}

async fn handle_socket(socket: WebSocket, service: &'static AppBridgeService) {
    let (mut sink, mut stream) = socket.split();

    let first_frame = match recv_next_frame(&mut stream).await {
        Ok(Some(frame)) => frame,
        Ok(None) => {
            tracing::info!("WebSocket closed before RegisterFrame");
            return;
        }
        Err(err) => {
            tracing::warn!("Failed to decode first WebSocket frame: {err}");
            return;
        }
    };

    let register = match first_frame.kind {
        FrameKind::Register(register) => register,
        _ => {
            tracing::warn!("First WebSocket frame was not a RegisterFrame");
            return;
        }
    };

    let host_pid = register.host_pid;
    let app_pid = register.app_pid;
    let client_kind = register.client_kind;

    let process_name = get_process_name(app_pid).unwrap_or_else(|| format!("unknown_{app_pid}"));

    let (tx_to_client, mut rx_to_client) = mpsc::channel::<Frame>(PER_CLIENT_TX_CAPACITY);

    service
        .register_client(RegisteredClient {
            tx: tx_to_client,
            host_pid,
            app_pid,
            process_name: process_name.clone(),
            client_kind,
        })
        .await;

    tracing::info!(
        "WebSocket client registered: kind={client_kind:?} app_pid={app_pid} host_pid={host_pid} process_name={process_name:?}"
    );

    // Push outbound frames from the desktop to this client.
    let writer_handle = tokio::spawn(async move {
        while let Some(frame) = rx_to_client.recv().await {
            let payload = match serde_json::to_string(&frame) {
                Ok(json) => json,
                Err(err) => {
                    tracing::error!("Failed to serialise outbound frame: {err}");
                    continue;
                }
            };
            if let Err(err) = sink.send(Message::Text(payload.into())).await {
                tracing::debug!("WebSocket sink error (app_pid={app_pid}): {err}");
                return;
            }
        }
        let _ = sink.send(Message::Close(None)).await;
    });

    // Forward inbound frames from this client into the router.
    let frames_tx = service.frames_tx.clone();
    loop {
        match recv_next_frame(&mut stream).await {
            Ok(Some(frame)) => {
                if frames_tx.send((app_pid, frame)).is_err() {
                    tracing::trace!("No frame subscribers (app_pid={app_pid})");
                }
            }
            Ok(None) => {
                tracing::info!("WebSocket client disconnected (app_pid={app_pid})");
                break;
            }
            Err(err) => {
                tracing::warn!("Failed to decode frame from WebSocket (app_pid={app_pid}): {err}");
                break;
            }
        }
    }

    writer_handle.abort();
    service.unregister_client(app_pid, host_pid).await;
}

/// Pull the next JSON-encoded `Frame` off the stream. Skips pings/pongs
/// (we tolerate them for diagnostic clients) and accepts JSON in either
/// `Text` or `Binary` frames so clients that can only send binary still
/// work. Returns `Ok(None)` on clean close.
async fn recv_next_frame<S>(stream: &mut S) -> Result<Option<Frame>, FrameDecodeError>
where
    S: futures_util::Stream<Item = Result<Message, axum::Error>> + Unpin,
{
    while let Some(item) = stream.next().await {
        let message = item.map_err(FrameDecodeError::Transport)?;
        match message {
            Message::Text(text) => {
                let frame = serde_json::from_str::<Frame>(text.as_str())
                    .map_err(FrameDecodeError::Decode)?;
                return Ok(Some(frame));
            }
            Message::Binary(bytes) => {
                let frame =
                    serde_json::from_slice::<Frame>(&bytes).map_err(FrameDecodeError::Decode)?;
                return Ok(Some(frame));
            }
            Message::Close(_) => return Ok(None),
            Message::Ping(_) | Message::Pong(_) => continue,
        }
    }
    Ok(None)
}

#[derive(Debug, thiserror::Error)]
enum FrameDecodeError {
    #[error("transport error: {0}")]
    Transport(#[from] axum::Error),
    #[error("frame decode error: {0}")]
    Decode(#[from] serde_json::Error),
}
