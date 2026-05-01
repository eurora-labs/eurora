//! Inbound frame router. Drains the broadcast channel that the WebSocket
//! transport publishes onto and dispatches each `Frame`:
//!
//! * `Response` / `Error` → resolve the matching pending outbound request
//! * `Cancel` → discard the matching pending outbound request
//! * `Event` → re-broadcast on the events channel
//! * `Request` → currently logged and dropped; no client speaks
//!   desktop-bound requests today
//! * `Register` → handled by the transport during connection setup; we just
//!   debug-log spurious occurrences here

use std::sync::Arc;

use euro_bridge_protocol::{EventFrame, Frame, FrameKind};
use tokio::sync::broadcast;

use crate::outbound::PendingRequestMap;

pub(crate) fn spawn_router(
    mut frames_rx: broadcast::Receiver<(u32, Frame)>,
    pending: Arc<PendingRequestMap>,
    events_tx: broadcast::Sender<(u32, EventFrame)>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tracing::debug!("Frame router task started");
        loop {
            let (app_pid, frame) = match frames_rx.recv().await {
                Ok(value) => value,
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("Frame router lagged by {n} frames, resuming");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            };

            match frame.kind {
                FrameKind::Request(request) => {
                    tracing::warn!(
                        "Received unsupported request frame from app_pid {app_pid}: id={}, action={}",
                        request.id,
                        request.action
                    );
                }
                FrameKind::Response(response) => match pending.remove(response.id) {
                    Some(pending_request) => {
                        if pending_request.sender.send(Frame::from(response)).is_err() {
                            tracing::warn!("Dropped response: receiver gone");
                        }
                    }
                    None => {
                        tracing::debug!(
                            "Received response with no pending request: id={}, action={}",
                            response.id,
                            response.action
                        );
                    }
                },
                FrameKind::Event(event) => {
                    tracing::debug!(
                        "Received event frame from app_pid {app_pid}: action={}",
                        event.action
                    );
                    if events_tx.send((app_pid, event)).is_err() {
                        tracing::trace!("No event subscribers");
                    }
                }
                FrameKind::Error(error) => {
                    tracing::error!(
                        "Received error frame from app_pid {app_pid}: id={}, message={}",
                        error.id,
                        error.message
                    );
                    if let Some(pending_request) = pending.remove(error.id)
                        && pending_request.sender.send(Frame::from(error)).is_err()
                    {
                        tracing::warn!("Dropped error: receiver gone");
                    }
                }
                FrameKind::Cancel(cancel) => {
                    tracing::debug!(
                        "Received cancel frame from app_pid {app_pid}: id={}",
                        cancel.id
                    );
                    if pending.remove(cancel.id).is_some() {
                        tracing::debug!("Cancelled pending request: id={}", cancel.id);
                    }
                }
                FrameKind::Register(_) => {
                    tracing::debug!(
                        "Spurious register frame from app_pid {app_pid} (already registered)"
                    );
                }
            }
        }
        tracing::debug!("Frame router task ended");
    })
}
