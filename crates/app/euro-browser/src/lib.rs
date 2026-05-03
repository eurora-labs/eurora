mod process_name;
pub mod server;

pub use euro_bridge_protocol::{
    BRIDGE_HOST, BRIDGE_PATH, BRIDGE_PORT, BridgeError, CancelFrame, ErrorFrame, EventFrame, Frame,
    FrameKind, RegisterFrame, RequestFrame, ResponseFrame, bridge_url,
};
pub use server::{BridgeService, RegisteredClient, RegistrationEvent};

/// Initialize and start the browser bridge WebSocket server. Idempotent —
/// safe to call multiple times. Returns the bind error if the listener
/// can't be opened.
pub async fn start_bridge_server() -> Result<(), std::io::Error> {
    BridgeService::get_or_init().await.start_server().await
}

/// Signal the running bridge server to shut down and wait for it to
/// terminate. No-op if the server isn't running (or was never started).
pub async fn stop_bridge_server() {
    if let Some(service) = BridgeService::get() {
        service.stop_server().await;
    }
}
