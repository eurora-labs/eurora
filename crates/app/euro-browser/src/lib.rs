mod process_name;
pub mod server;

pub use euro_bridge_protocol::{
    BRIDGE_BIND_IP, BRIDGE_HOST, BRIDGE_PATH, BRIDGE_PORT, BRIDGE_SCHEME, BridgeError, CancelFrame,
    ErrorFrame, EventFrame, Frame, FrameKind, RegisterFrame, RequestFrame, ResponseFrame,
    bridge_ca_path, bridge_data_dir, bridge_url, bridge_url_for, eurora_data_root,
};
pub use server::{
    BridgeService, RegisteredClient, RegistrationEvent, TlsMaterial,
    install_default_crypto_provider,
};

/// Initialize and start the browser bridge WebSocket server. Idempotent —
/// safe to call multiple times. Requires
/// [`BridgeService::configure_tls`] to have been called first; returns
/// [`BridgeError::TlsNotConfigured`] otherwise. Returns the resolved
/// local address on success.
pub async fn start_bridge_server() -> Result<std::net::SocketAddr, BridgeError> {
    BridgeService::get_or_init().await.start_server().await
}

/// Signal the running bridge server to shut down and wait for it to
/// terminate. No-op if the server isn't running (or was never started).
pub async fn stop_bridge_server() {
    if let Some(service) = BridgeService::get() {
        service.stop_server().await;
    }
}
