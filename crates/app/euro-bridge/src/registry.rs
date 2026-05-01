//! Registered-client bookkeeping. Each connected client is represented by a
//! [`RegisteredClient`] in the registry, keyed by `app_pid`. Lifecycle
//! transitions are broadcast on the [`super::service::AppBridgeService`]
//! registration / disconnect channels so the rest of the desktop can react
//! without polling.

use std::collections::HashMap;
use std::sync::Arc;

use euro_bridge_protocol::{BridgeError, ClientKind, Frame};
use tokio::sync::{RwLock, mpsc};

/// A single connected client. The `tx` is the only handle the rest of the
/// service needs to push frames to that client; everything else is metadata.
#[derive(Debug)]
pub struct RegisteredClient {
    pub tx: mpsc::Sender<Frame>,
    pub host_pid: u32,
    pub app_pid: u32,
    pub process_name: String,
    pub client_kind: ClientKind,
}

/// Lightweight summary broadcast on register / disconnect. Subscribers can
/// observe lifecycle without taking a read lock on the registry.
#[derive(Debug, Clone)]
pub struct RegistrationEvent {
    pub app_pid: u32,
    pub process_name: String,
    pub client_kind: ClientKind,
}

#[derive(Debug, Clone, Default)]
pub struct ClientRegistry {
    inner: Arc<RwLock<HashMap<u32, RegisteredClient>>>,
}

impl ClientRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn insert(&self, client: RegisteredClient) {
        let mut guard = self.inner.write().await;
        guard.insert(client.app_pid, client);
    }

    /// Remove the registration for `app_pid` only if the stored entry's
    /// `host_pid` still matches, guarding against the case where a stale
    /// disconnect cleanup races a fresh re-registration from the same PID.
    pub async fn remove_if_host_matches(
        &self,
        app_pid: u32,
        host_pid: u32,
    ) -> Option<RegisteredClient> {
        let mut guard = self.inner.write().await;
        match guard.get(&app_pid) {
            Some(existing) if existing.host_pid == host_pid => guard.remove(&app_pid),
            _ => None,
        }
    }

    pub async fn contains(&self, app_pid: u32) -> bool {
        self.inner.read().await.contains_key(&app_pid)
    }

    pub async fn pids(&self) -> Vec<u32> {
        self.inner.read().await.keys().copied().collect()
    }

    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.inner.read().await.is_empty()
    }

    /// Send a frame to the client with the given `app_pid`.
    pub async fn send_to(&self, app_pid: u32, frame: Frame) -> Result<(), BridgeError> {
        let guard = self.inner.read().await;
        let Some(client) = guard.get(&app_pid) else {
            return Err(BridgeError::NotFound { app_pid });
        };
        client
            .tx
            .send(frame)
            .await
            .map_err(|e| BridgeError::Send(e.to_string()))
    }

    /// Find the first registered client matching `process_name` (and
    /// optionally a [`ClientKind`]) and return its `app_pid`. Used by the
    /// desktop UI to bridge OS-level focus state ("the user is in
    /// chrome.exe") to bridge-level state ("a Chrome extension messenger
    /// is registered for that PID").
    pub async fn find_pid_by_process_name(
        &self,
        process_name: &str,
        kind: Option<ClientKind>,
    ) -> Option<u32> {
        let guard = self.inner.read().await;
        guard
            .values()
            .find(|client| {
                client.process_name == process_name && kind.is_none_or(|k| client.client_kind == k)
            })
            .map(|client| client.app_pid)
    }
}
