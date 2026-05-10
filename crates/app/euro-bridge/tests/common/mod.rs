//! Shared test utilities for the bridge integration tests. Each
//! integration test in this crate stands up an ephemeral plaintext
//! bridge on a freshly-allocated loopback port.
//!
//! Lives under `tests/common/` per the standard cargo idiom for
//! integration-test helpers (suppresses the
//! `unused_crate_dependencies` warning that a lone `mod.rs` would
//! otherwise trip if linked from a single test file).

#![allow(dead_code)]

use std::net::SocketAddr;

use euro_bridge::{BoundServer, BridgeService};

/// Bind a fresh [`BridgeService`] to an ephemeral loopback port and
/// spawn its accept loop. Returns the live service, the bound address,
/// and the join handle for the accept loop so tests can `await` it on
/// shutdown.
///
/// Tests that need to drive shutdown explicitly should call
/// `service.stop_server().await` and then await the returned handle.
pub async fn start_ephemeral_bridge() -> (
    BridgeService,
    SocketAddr,
    tokio::task::JoinHandle<Result<(), euro_bridge::BridgeError>>,
) {
    let service = BridgeService::new();
    let bound = service
        .bind_on(([127, 0, 0, 1], 0).into())
        .await
        .expect("bind ephemeral bridge");
    let addr = bound.local_addr();
    let serve_handle = spawn_serve(bound);
    (service, addr, serve_handle)
}

/// Spawn the accept loop for an already-bound listener, returning the
/// task handle. Wrapped because [`BoundServer::serve`] consumes the
/// listener and tests want to keep the join handle around for clean
/// shutdown.
pub fn spawn_serve(
    bound: BoundServer,
) -> tokio::task::JoinHandle<Result<(), euro_bridge::BridgeError>> {
    tokio::spawn(async move { bound.serve().await })
}
