mod process_name;
pub mod server;

pub use euro_bridge_protocol::{
    BRIDGE_HOST, BRIDGE_PATH, BRIDGE_PORT, BridgeError, CancelFrame, ErrorFrame, EventFrame, Frame,
    FrameKind, RegisterFrame, RequestFrame, ResponseFrame, bridge_url,
};
pub use server::{BridgeService, RegisteredClient, RegistrationEvent};

/// Initialize and start the browser bridge WebSocket server. Idempotent —
/// safe to call multiple times.
pub async fn start_bridge_server() {
    let service = BridgeService::get_or_init().await;
    service.start_frame_handler();
    service.start_server().await;
}

/// Signal the running bridge server to shut down gracefully. No-op if
/// the server isn't running.
pub async fn stop_bridge_server() {
    BridgeService::stop_server().await;
}
