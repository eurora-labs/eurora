//! Test utilities for the Word add-in round-trip integration test.
//! Mirrors the shape of `euro-bridge`'s `tests/common/mod.rs` — the
//! two are not literally shared because doing so would require a
//! cross-crate test-feature, and the helper is small.

#![allow(dead_code)]

use std::net::SocketAddr;

use euro_bridge::{BoundServer, BridgeService};

/// Bind a fresh [`BridgeService`] to an ephemeral loopback port and
/// spawn its accept loop. Returns the live service, the bound address,
/// and the join handle for the accept loop so tests can `await` it on
/// shutdown.
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

pub fn spawn_serve(
    bound: BoundServer,
) -> tokio::task::JoinHandle<Result<(), euro_bridge::BridgeError>> {
    tokio::spawn(async move { bound.serve().await })
}
