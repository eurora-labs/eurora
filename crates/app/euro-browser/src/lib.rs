mod process_name;
pub mod server;

pub use euro_bridge_protocol::{
    BRIDGE_BIND_IP, BRIDGE_HOST, BRIDGE_PATH, BRIDGE_PORT, BRIDGE_SCHEME, BridgeError, CancelFrame,
    ErrorFrame, EventFrame, Frame, FrameKind, RegisterFrame, RequestFrame, ResponseFrame,
    bridge_ca_path, bridge_data_dir, bridge_url, bridge_url_for, eurora_data_root,
};
pub use server::{
    BoundServer, BridgeService, RegisteredClient, RegistrationEvent, TlsMaterial,
    install_default_crypto_provider,
};

/// Bind the bridge WebSocket listener on its well-known port and return
/// the [`BoundServer`] handle whose [`serve`](BoundServer::serve) future
/// drives the accept loop. The kernel socket is in `LISTEN` state by
/// the time this returns, so spawning `serve()` afterwards is sufficient
/// to expose the listener — clients can no longer race the bind.
///
/// Requires [`BridgeService::configure_tls`] to have been called first;
/// returns [`BridgeError::TlsNotConfigured`] otherwise. Returns
/// [`BridgeError::AlreadyRunning`] if a previous serve loop is still
/// registered.
pub async fn bind_bridge_server() -> Result<BoundServer, BridgeError> {
    BridgeService::get_or_init().bind().await
}

/// Signal the running bridge server to shut down and wait for it to
/// terminate. No-op if the server isn't running (or was never started).
pub async fn stop_bridge_server() {
    if let Some(service) = BridgeService::get() {
        service.stop_server().await;
    }
}
