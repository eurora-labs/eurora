//! Local IPC bridge between the Eurora desktop app and external clients.
//!
//! Exposes a single WebSocket transport at `[::1]:APP_BRIDGE_PORT` that every
//! kind of client (browser native-messaging host, Office.js add-in, future
//! first-party integrations) connects to. Messages are JSON-encoded
//! [`Frame`]s as defined in [`euro_bridge_protocol`].
//!
//! The [`AppBridgeService`] singleton owns the client registry, the
//! broadcast channels that fan inbound frames out to subscribers, and the
//! request/response correlation map that desktop code uses via
//! [`AppBridgeService::send_request`].

mod outbound;
mod process_name;
mod registry;
mod router;
mod service;
mod websocket;

pub use euro_bridge_protocol::{
    BridgeError, CancelFrame, ClientKind, ErrorFrame, EventFrame, Frame, FrameKind, RegisterFrame,
    RequestFrame, ResponseFrame,
};
pub use registry::{ClientRegistry, RegisteredClient, RegistrationEvent};
pub use service::{APP_BRIDGE_PORT, AppBridgeService};
pub use websocket::{is_ws_server_running, serve_ws, start_ws_server, stop_ws_server};

/// Start the WebSocket bridge. Idempotent.
pub async fn start_app_bridge() {
    let service = AppBridgeService::get_or_init().await;
    start_ws_server(service).await;
}

/// Stop the WebSocket bridge. Idempotent. The shared
/// [`AppBridgeService`] singleton is intentionally not torn down — its
/// channels are reused if the bridge is restarted.
pub async fn stop_app_bridge() {
    stop_ws_server().await;
}
