//! Server-side chat-WebSocket-backed [`RemoteToolBus`] implementation.
//!
//! Each chat turn owns one [`ChatRemoteBus`]: it allocates correlation ids,
//! emits [`ChatServerMessage::ToolRequest`] frames on the per-turn outbound
//! channel, parks the calling task on a oneshot via
//! [`request_correlator::RequestCorrelator`], and races the response against
//! the descriptor-supplied timeout and the chat-level cancellation token.
//!
//! Inbound [`thread_core::ChatClientMessage::ToolResponse`] frames land via
//! [`ChatRemoteBus::resolve`] — the chat handler's reader task forwards
//! them. Late responses for already-removed call ids are dropped silently:
//! the bus has already woken the caller with `Timeout` or `Cancelled` and
//! updated its state accordingly.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use request_correlator::{RequestCorrelator, WaitError};
use serde_json::Value;
use thread_core::{ChatServerMessage, ToolErrorWire, WireToolDescriptor};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Server-side bus used by the agent loop to dispatch a tool whose
/// `ToolSource` doesn't run in-process.
///
/// Returns [`ToolErrorWire`] directly — the wire shape is sufficient for
/// the loop's tool-message rendering and for the chat-handler tests, and
/// removing the in-process `eurora_tools::ToolError` shadow drops a whole
/// crate boundary that used to exist purely for source-preservation.
#[async_trait]
pub trait RemoteToolBus: Send + Sync {
    async fn call(
        &self,
        descriptor: &WireToolDescriptor,
        arguments: Value,
    ) -> Result<Value, ToolErrorWire>;
}

/// Per-turn remote-tool dispatcher backed by the chat WebSocket.
///
/// Construction returns an `Arc` because the bus is shared between (a) the
/// agent loop, which calls [`RemoteToolBus::call`], and (b) the chat
/// handler's inbound reader task, which calls [`Self::resolve`] when a
/// `ToolResponse` frame arrives.
pub struct ChatRemoteBus {
    outbound: mpsc::Sender<ChatServerMessage>,
    correlator: RequestCorrelator<u32, Value, ToolErrorWire>,
    next_call_id: AtomicU32,
    chat_cancel: CancellationToken,
}

impl ChatRemoteBus {
    /// Build a bus tied to a specific chat turn's outbound channel and
    /// cancellation token.
    pub fn new(
        outbound: mpsc::Sender<ChatServerMessage>,
        chat_cancel: CancellationToken,
    ) -> Arc<Self> {
        Arc::new(Self {
            outbound,
            correlator: RequestCorrelator::new(),
            next_call_id: AtomicU32::new(0),
            chat_cancel,
        })
    }

    /// Fulfil a pending call with the result delivered over the wire.
    ///
    /// Called by the chat handler's inbound reader task when it receives a
    /// `ChatClientMessage::ToolResponse`. A no-op when no call is pending
    /// under `call_id` (e.g. the call already timed out or was cancelled).
    pub fn resolve(&self, call_id: u32, result: Result<Value, ToolErrorWire>) {
        self.correlator.resolve(call_id, result);
    }

    /// Drop every pending call, waking each waiter with
    /// [`ToolErrorWire::Transport`]. Call when the turn ends so no
    /// agent-loop task is left awaiting a oneshot that will never
    /// resolve.
    pub fn shutdown(&self) {
        self.correlator.shutdown_with(|_| ToolErrorWire::Transport {
            message: "turn ended before tool response arrived".into(),
        });
    }
}

#[async_trait]
impl RemoteToolBus for ChatRemoteBus {
    async fn call(
        &self,
        descriptor: &WireToolDescriptor,
        arguments: Value,
    ) -> Result<Value, ToolErrorWire> {
        let call_id = self.next_call_id.fetch_add(1, Ordering::Relaxed);
        let guard = self.correlator.register(call_id);

        let request = ChatServerMessage::ToolRequest {
            call_id,
            descriptor: descriptor.clone(),
            arguments,
        };
        if self.outbound.send(request).await.is_err() {
            return Err(ToolErrorWire::Transport {
                message: "chat outbound channel closed before ToolRequest sent".into(),
            });
        }

        let timeout = Duration::from_millis(descriptor.timeout_ms.into());
        match guard.wait_cancellable(timeout, &self.chat_cancel).await {
            Ok(result) => result,
            Err(WaitError::Cancelled) => {
                self.send_cancel(call_id).await;
                Err(ToolErrorWire::Cancelled)
            }
            Err(WaitError::Timeout) => {
                self.send_cancel(call_id).await;
                Err(ToolErrorWire::Timeout)
            }
            Err(WaitError::SenderDropped) => Err(ToolErrorWire::Transport {
                message: "tool response channel dropped before result arrived".into(),
            }),
        }
    }
}

impl ChatRemoteBus {
    async fn send_cancel(&self, call_id: u32) {
        if self
            .outbound
            .send(ChatServerMessage::ToolCancel { call_id })
            .await
            .is_err()
        {
            tracing::debug!(
                call_id,
                "chat outbound channel closed before ToolCancel could be emitted"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;
    use tokio::time::Duration as TokioDuration;

    fn sample_descriptor(timeout_ms: u32) -> WireToolDescriptor {
        crate::test_support::bridge_descriptor("browser::test::echo", timeout_ms)
    }

    async fn expect_tool_request(rx: &mut mpsc::Receiver<ChatServerMessage>) -> u32 {
        match rx.recv().await {
            Some(ChatServerMessage::ToolRequest { call_id, .. }) => call_id,
            other => panic!("expected ToolRequest, got {other:?}"),
        }
    }

    async fn expect_tool_cancel(rx: &mut mpsc::Receiver<ChatServerMessage>) -> u32 {
        match rx.recv().await {
            Some(ChatServerMessage::ToolCancel { call_id }) => call_id,
            other => panic!("expected ToolCancel, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn happy_path_returns_resolved_value() {
        let (tx, mut rx) = mpsc::channel(8);
        let cancel = CancellationToken::new();
        let bus = ChatRemoteBus::new(tx, cancel);

        let bus_clone = bus.clone();
        let descriptor = sample_descriptor(5_000);
        let handle =
            tokio::spawn(
                async move { bus_clone.call(&descriptor, json!({"hello": "world"})).await },
            );

        let call_id = expect_tool_request(&mut rx).await;
        bus.resolve(call_id, Ok(json!({"echo": "world"})));

        let result = handle.await.expect("task didn't panic").expect("call ok");
        assert_eq!(result, json!({"echo": "world"}));
    }

    #[tokio::test(start_paused = true)]
    async fn timeout_emits_cancel_and_returns_timeout() {
        let (tx, mut rx) = mpsc::channel(8);
        let cancel = CancellationToken::new();
        let bus = ChatRemoteBus::new(tx, cancel);

        let bus_clone = bus.clone();
        let descriptor = sample_descriptor(50);
        let handle = tokio::spawn(async move { bus_clone.call(&descriptor, json!({})).await });

        let call_id = expect_tool_request(&mut rx).await;
        tokio::time::advance(TokioDuration::from_millis(100)).await;
        let cancel_id = expect_tool_cancel(&mut rx).await;
        assert_eq!(cancel_id, call_id);

        let err = handle.await.expect("task didn't panic").unwrap_err();
        assert!(matches!(err, ToolErrorWire::Timeout), "got {err:?}");
    }

    #[tokio::test]
    async fn cancel_emits_cancel_frame_and_returns_cancelled() {
        let (tx, mut rx) = mpsc::channel(8);
        let cancel = CancellationToken::new();
        let bus = ChatRemoteBus::new(tx, cancel.clone());

        let bus_clone = bus.clone();
        let descriptor = sample_descriptor(60_000);
        let handle = tokio::spawn(async move { bus_clone.call(&descriptor, json!({})).await });

        let call_id = expect_tool_request(&mut rx).await;
        cancel.cancel();
        let cancel_id = expect_tool_cancel(&mut rx).await;
        assert_eq!(cancel_id, call_id);

        let err = handle.await.expect("task didn't panic").unwrap_err();
        assert!(matches!(err, ToolErrorWire::Cancelled), "got {err:?}");
    }

    #[tokio::test(start_paused = true)]
    async fn late_resolve_after_timeout_is_dropped() {
        let (tx, mut rx) = mpsc::channel(8);
        let cancel = CancellationToken::new();
        let bus = ChatRemoteBus::new(tx, cancel);

        let bus_clone = bus.clone();
        let descriptor = sample_descriptor(20);
        let handle = tokio::spawn(async move { bus_clone.call(&descriptor, json!({})).await });

        let call_id = expect_tool_request(&mut rx).await;
        tokio::time::advance(TokioDuration::from_millis(100)).await;
        expect_tool_cancel(&mut rx).await;

        bus.resolve(call_id, Ok(json!({"late": true})));
        let err = handle.await.expect("task didn't panic").unwrap_err();
        assert!(matches!(err, ToolErrorWire::Timeout));
        assert!(rx.try_recv().is_err(), "no further outbound frames");
    }

    #[tokio::test]
    async fn transport_failure_when_outbound_closed_returns_transport_error() {
        let (tx, rx) = mpsc::channel(8);
        drop(rx);
        let cancel = CancellationToken::new();
        let bus = ChatRemoteBus::new(tx, cancel);

        let descriptor = sample_descriptor(5_000);
        let err = bus.call(&descriptor, json!({})).await.unwrap_err();
        match err {
            ToolErrorWire::Transport { message: msg } => {
                assert!(msg.contains("chat outbound channel closed"), "{msg}");
            }
            other => panic!("expected Transport, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn shutdown_wakes_pending_callers_with_transport_error() {
        let (tx, mut rx) = mpsc::channel(8);
        let cancel = CancellationToken::new();
        let bus = ChatRemoteBus::new(tx, cancel);

        let bus_clone = bus.clone();
        let descriptor = sample_descriptor(60_000);
        let handle = tokio::spawn(async move { bus_clone.call(&descriptor, json!({})).await });

        expect_tool_request(&mut rx).await;
        bus.shutdown();
        let err = handle.await.expect("task didn't panic").unwrap_err();
        match err {
            ToolErrorWire::Transport { message: msg } => assert!(msg.contains("turn ended"), "{msg}"),
            other => panic!("expected Transport, got {other:?}"),
        }
    }

    /// `call_id` allocation is monotone within a turn. The next-id counter
    /// is `u32`; tests at the boundary aren't needed because the wraparound
    /// case is exercised by the underlying `AtomicU32` semantics.
    #[tokio::test]
    async fn call_ids_are_distinct_per_turn() {
        let (tx, mut rx) = mpsc::channel(8);
        let cancel = CancellationToken::new();
        let bus = ChatRemoteBus::new(tx, cancel);

        let bus_a = bus.clone();
        let bus_b = bus.clone();
        let descriptor = sample_descriptor(60_000);
        let d1 = descriptor.clone();
        let d2 = descriptor.clone();
        let h1 = tokio::spawn(async move { bus_a.call(&d1, json!({})).await });
        let h2 = tokio::spawn(async move { bus_b.call(&d2, json!({})).await });

        let id_a = expect_tool_request(&mut rx).await;
        let id_b = expect_tool_request(&mut rx).await;
        assert_ne!(id_a, id_b);

        bus.resolve(id_a, Ok(json!({"id": "a"})));
        bus.resolve(id_b, Ok(json!({"id": "b"})));
        let _ = h1.await;
        let _ = h2.await;
    }
}
