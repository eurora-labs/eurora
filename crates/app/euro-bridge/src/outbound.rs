//! Desktop-initiated request/response correlation, with `CancelFrame`
//! propagation in both directions:
//!
//! * If the desktop's `send_request` times out or its caller drops, we send
//!   a `CancelFrame` to the client so it can abort its in-flight work.
//! * If the client sends a `CancelFrame` first, the
//!   [`super::router`] removes the entry, which in turn suppresses the
//!   dispatch-side cancel.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use dashmap::DashMap;
use euro_bridge_protocol::{
    BridgeError, CancelFrame, Frame, FrameKind, RequestFrame, ResponseFrame,
};
use tokio::sync::oneshot;

use crate::registry::ClientRegistry;

pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// One in-flight outbound request awaiting a reply.
pub(crate) struct PendingRequest {
    pub sender: oneshot::Sender<Frame>,
}

impl std::fmt::Debug for PendingRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingRequest")
            .field("sender", &"oneshot::Sender<Frame>")
            .finish()
    }
}

#[derive(Debug, Default)]
pub(crate) struct PendingRequestMap {
    inner: DashMap<u32, PendingRequest>,
}

impl PendingRequestMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, id: u32, request: PendingRequest) {
        self.inner.insert(id, request);
    }

    pub fn remove(&self, id: u32) -> Option<PendingRequest> {
        self.inner.remove(&id).map(|(_, request)| request)
    }
}

/// RAII guard ensuring a pending request is cleaned up on every exit path.
/// If the entry is still in the map at drop time we treat that as "no reply
/// was delivered" and emit a `CancelFrame` so the client can abort its work;
/// on the happy path callers invoke [`Self::complete`] to suppress that.
struct PendingGuard {
    request_id: u32,
    pending: Arc<PendingRequestMap>,
    registry: ClientRegistry,
    app_pid: u32,
    completed: bool,
}

impl PendingGuard {
    fn new(
        request_id: u32,
        pending: Arc<PendingRequestMap>,
        registry: ClientRegistry,
        app_pid: u32,
    ) -> Self {
        Self {
            request_id,
            pending,
            registry,
            app_pid,
            completed: false,
        }
    }

    fn complete(mut self) {
        self.completed = true;
    }
}

impl Drop for PendingGuard {
    fn drop(&mut self) {
        if self.completed {
            return;
        }
        // If the entry is gone the router already delivered something, so
        // there's nothing to cancel.
        if self.pending.remove(self.request_id).is_none() {
            return;
        }

        let registry = self.registry.clone();
        let app_pid = self.app_pid;
        let id = self.request_id;
        tokio::spawn(async move {
            let frame = Frame::from(CancelFrame { id });
            if let Err(err) = registry.send_to(app_pid, frame).await {
                tracing::debug!("Failed to send CancelFrame to app_pid={app_pid} id={id}: {err}");
            }
        });
    }
}

/// Outbound request infrastructure used by
/// [`super::service::AppBridgeService`].
#[derive(Debug, Clone)]
pub struct OutboundDispatcher {
    pending: Arc<PendingRequestMap>,
    request_id_counter: Arc<AtomicU32>,
    registry: ClientRegistry,
}

impl OutboundDispatcher {
    pub fn new(registry: ClientRegistry) -> Self {
        Self {
            pending: Arc::new(PendingRequestMap::new()),
            request_id_counter: Arc::new(AtomicU32::new(1)),
            registry,
        }
    }

    pub(crate) fn pending(&self) -> Arc<PendingRequestMap> {
        Arc::clone(&self.pending)
    }

    /// Send a [`RequestFrame`] to the client identified by `app_pid` and wait
    /// up to `timeout` for the matching response. On timeout, the
    /// [`PendingGuard`] sends a `CancelFrame` so the client can abort its
    /// in-flight work.
    pub async fn send_request(
        &self,
        app_pid: u32,
        action: &str,
        payload: Option<String>,
        timeout: Duration,
    ) -> Result<ResponseFrame, BridgeError> {
        let request_id = self.request_id_counter.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.pending
            .insert(request_id, PendingRequest { sender: tx });

        let guard = PendingGuard::new(
            request_id,
            Arc::clone(&self.pending),
            self.registry.clone(),
            app_pid,
        );

        let frame = Frame::from(RequestFrame {
            id: request_id,
            action: action.to_string(),
            payload,
        });

        tracing::debug!(
            "Sending request frame: id={request_id}, action={action}, app_pid={app_pid}"
        );

        if let Err(err) = self.registry.send_to(app_pid, frame).await {
            // The frame never reached the client; nothing to cancel.
            guard.complete();
            return Err(err);
        }

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(frame)) => {
                // The router removed the entry to deliver this frame, so
                // the guard has nothing to do.
                guard.complete();
                match frame.kind {
                    FrameKind::Response(response) => {
                        tracing::debug!("Received response for request {request_id}");
                        Ok(response)
                    }
                    FrameKind::Error(err) => Err(BridgeError::Client {
                        message: err.message,
                        details: err.details,
                    }),
                    FrameKind::Request(_) => Err(BridgeError::UnexpectedFrame("Request")),
                    FrameKind::Event(_) => Err(BridgeError::UnexpectedFrame("Event")),
                    FrameKind::Cancel(_) => Err(BridgeError::UnexpectedFrame("Cancel")),
                    FrameKind::Register(_) => Err(BridgeError::UnexpectedFrame("Register")),
                }
            }
            Ok(Err(_)) => {
                // The oneshot was dropped without sending. The router
                // already discarded the entry; no cancel to emit.
                guard.complete();
                tracing::error!("Response channel closed for request {request_id}");
                Err(BridgeError::ChannelClosed)
            }
            Err(_) => {
                tracing::warn!("Timeout waiting for response to request {request_id}");
                // Let `guard` drop without `complete()`: this fires the
                // CancelFrame towards the client.
                Err(BridgeError::Timeout)
            }
        }
    }
}
