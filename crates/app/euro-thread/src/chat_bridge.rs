//! Per-turn chat state machine.
//!
//! [`ChatBridge`] owns one chat WebSocket for the duration of a single
//! turn. At turn start it asks the active [`ToolBackend`] for the
//! LLM-visible tool surface and emits a `CapabilityUpdate` frame; the
//! opening frame (`Send` or `Regenerate`) follows. From there the bridge
//! multiplexes three responsibilities over the WS:
//!
//! - forward user-visible chat frames (`Chunk`, `ConfirmedHumanMessage`,
//!   `Final`, `Error`) to a caller-owned [`ChatEventSink`],
//! - dispatch incoming `ToolRequest` frames through the [`ToolBackend`]
//!   and emit the matching `ToolResponse` on completion, and
//! - propagate cancellation: UI-level cancel triggers
//!   `ChatClientMessage::Cancel` and cancels every in-flight dispatch,
//!   server-issued `ToolCancel` targets a single call.
//!
//! The backend is a `Send + Sync` trait object so `ChatBridge` has no
//! dependency on activity strategies or the bridge service. The
//! production wiring lives in `euro-activity`; tests stub the trait
//! directly.

use std::sync::Arc;

use agent_chain_core::messages::ContentBlock;
use dashmap::DashMap;
use euro_transport_policy::CHAT_DISPATCH_DRAIN;
use serde_json::Value;
use thread_core::{
    CapabilityUpdatePayload, ChatClientMessage, ChatSendRequest, ChatServerMessage,
    RegenerateRequest, ToolBackend, ToolBackendCall, ToolErrorWire, WireToolDescriptor,
};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::chat_socket::{ChatOutbound, ChatSocket};
use crate::error::{Error, Result};

/// Opening frame for a chat turn — `Send` for a fresh human turn,
/// `Regenerate` to re-roll an existing AI message under the same parent.
#[derive(Debug, Clone)]
pub enum TurnOpening {
    Send(ChatSendRequest),
    Regenerate(RegenerateRequest),
}

impl From<TurnOpening> for ChatClientMessage {
    fn from(opening: TurnOpening) -> Self {
        match opening {
            TurnOpening::Send(req) => ChatClientMessage::Send(req),
            TurnOpening::Regenerate(req) => ChatClientMessage::Regenerate(req),
        }
    }
}

/// Failure surface for a [`ChatEventSink`].
///
/// Wraps whatever string the underlying sink (Tauri channel, mpsc, …)
/// produced when it rejected the event. The bridge surfaces sink errors
/// through [`Error::Sink`].
#[derive(Debug, thiserror::Error)]
#[error("chat event sink closed: {0}")]
pub struct ChatSinkError(pub String);

/// Caller-owned sink for user-visible chat frames.
///
/// Implemented blanket-style for any
/// `Fn(ChatServerMessage) -> Result<(), ChatSinkError>` closure —
/// production code wraps the Tauri `Channel<ChatServerMessage>` in a
/// closure; tests typically forward into an
/// [`tokio::sync::mpsc::UnboundedSender`].
pub trait ChatEventSink: Send + Sync {
    fn send(&self, event: ChatServerMessage) -> std::result::Result<(), ChatSinkError>;
}

impl<F> ChatEventSink for F
where
    F: Fn(ChatServerMessage) -> std::result::Result<(), ChatSinkError> + Send + Sync,
{
    fn send(&self, event: ChatServerMessage) -> std::result::Result<(), ChatSinkError> {
        self(event)
    }
}

/// Per-turn dispatcher and chat-frame router.
///
/// The bridge holds a shared reference to whatever [`ToolBackend`] the
/// app wired in. One bridge can drive many turns sequentially — there's
/// no per-turn state on `self`; all turn state lives on the stack of
/// [`Self::run_turn`].
pub struct ChatBridge {
    backend: Arc<dyn ToolBackend>,
}

impl ChatBridge {
    pub fn new(backend: Arc<dyn ToolBackend>) -> Self {
        Self { backend }
    }

    /// Drive one chat turn end-to-end.
    ///
    /// Consumes `socket` so the WS closes when the turn ends. The
    /// caller-owned `cancel` token represents UI-level cancellation;
    /// firing it sends a `ChatClientMessage::Cancel` to the server,
    /// cancels every in-flight dispatch, and keeps draining the
    /// socket until the server emits a terminal frame or the WS
    /// closes.
    pub async fn run_turn<S: ChatEventSink>(
        &self,
        mut socket: ChatSocket,
        opening: TurnOpening,
        cancel: CancellationToken,
        sink: &S,
    ) -> Result<()> {
        // `list_tools` and `collect_system_blocks` are independent backend
        // probes — for the browser strategy each round-trips through the
        // native-messenger bridge — so we issue them concurrently to keep
        // the prelude overhead bounded by the slower of the two.
        let (tools, system_blocks): (Vec<WireToolDescriptor>, Vec<ContentBlock>) = tokio::join!(
            self.backend.list_tools(),
            self.backend.collect_system_blocks(),
        );

        socket.try_send(ChatClientMessage::CapabilityUpdate(
            CapabilityUpdatePayload {
                tools,
                contexts: Vec::new(),
                system_blocks,
            },
        ))?;
        socket.try_send(opening.into())?;

        // `turn_cancel` is intentionally a *standalone* token rather
        // than a child of the caller's `cancel`. Child propagation is
        // synchronous: if `turn_cancel` were a child, a UI cancel would
        // wake every in-flight dispatch *before* the bridge's main loop
        // got a chance to write `ChatClientMessage::Cancel` to the
        // socket. Decoupling the two lets the bridge enforce the
        // ordering "Cancel frame first, in-flight dispatches cancelled
        // second" inside the cancel arm below.
        let turn = TurnState {
            turn_cancel: CancellationToken::new(),
            inflight: Arc::new(DashMap::new()),
        };

        let mut dispatches: JoinSet<()> = JoinSet::new();
        let mut cancel_sent = false;

        let outcome: Result<()> = loop {
            tokio::select! {
                biased;
                () = cancel.cancelled(), if !cancel_sent => {
                    // One Cancel frame, then keep draining the socket
                    // until the server emits a terminal frame. The
                    // guard disables this arm after a single fire so
                    // we don't send Cancel twice on the same turn.
                    let _ = socket.try_send(ChatClientMessage::Cancel);
                    turn.turn_cancel.cancel();
                    cancel_sent = true;
                }
                inbound = socket.recv() => {
                    let Some(msg) = inbound else {
                        break Err(Error::ChatProtocol(
                            "chat socket closed before terminal frame".into(),
                        ));
                    };
                    let event = match msg {
                        Ok(event) => event,
                        Err(err) => break Err(err),
                    };
                    match event {
                        ChatServerMessage::ToolRequest {
                            call_id,
                            descriptor,
                            arguments,
                        } => {
                            self.spawn_dispatch(
                                &turn,
                                &mut dispatches,
                                call_id,
                                descriptor,
                                arguments,
                                socket.outbound_sender(),
                            );
                        }
                        ChatServerMessage::ToolCancel { call_id } => {
                            if let Some((_, token)) = turn.inflight.remove(&call_id) {
                                token.cancel();
                            }
                        }
                        ev @ (ChatServerMessage::Chunk { .. }
                            | ChatServerMessage::ConfirmedHumanMessage { .. }) => {
                            sink.send(ev).map_err(Error::Sink)?;
                        }
                        ev @ ChatServerMessage::Final { .. } => {
                            sink.send(ev).map_err(Error::Sink)?;
                            break Ok(());
                        }
                        ev @ ChatServerMessage::Error { .. } => {
                            sink.send(ev).map_err(Error::Sink)?;
                            break Ok(());
                        }
                    }
                }
            }
        };

        // Cancel everything still in flight and drain with a grace
        // period. Stragglers are aborted via `JoinSet::shutdown` so a
        // stuck adapter cannot pin the turn beyond `CHAT_DISPATCH_DRAIN`.
        turn.turn_cancel.cancel();
        let drain = async { while dispatches.join_next().await.is_some() {} };
        if tokio::time::timeout(CHAT_DISPATCH_DRAIN, drain)
            .await
            .is_err()
        {
            dispatches.shutdown().await;
        }

        outcome
    }

    fn spawn_dispatch(
        &self,
        turn: &TurnState,
        set: &mut JoinSet<()>,
        call_id: u32,
        descriptor: WireToolDescriptor,
        arguments: Value,
        outbound: ChatOutbound,
    ) {
        let cancel = turn.turn_cancel.child_token();
        turn.inflight.insert(call_id, cancel.clone());
        let inflight = turn.inflight.clone();
        let backend = Arc::clone(&self.backend);
        let name = descriptor.definition.name;

        set.spawn(async move {
            let call = ToolBackendCall {
                call_id,
                name,
                arguments,
                cancel: cancel.clone(),
            };
            let result: std::result::Result<Value, ToolErrorWire> = backend.dispatch(call).await;
            inflight.remove(&call_id);
            let _ = outbound.send(ChatClientMessage::ToolResponse { call_id, result });
        });
    }
}

struct TurnState {
    /// Standalone token cancelled when the turn ends (or on UI cancel)
    /// so every in-flight dispatch wakes up. Independent from the
    /// caller's UI cancel so the bridge can write the `Cancel` frame
    /// before signalling dispatches — see `run_turn`'s commentary.
    turn_cancel: CancellationToken,
    /// Per-call cancellation tokens so `ToolCancel { call_id }` can
    /// abort exactly one dispatch without disturbing siblings.
    inflight: Arc<DashMap<u32, CancellationToken>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_socket::ChatSocketHarness;
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Mutex as StdMutex;
    use std::time::Duration;
    use thread_core::ToolSource;
    use tokio::sync::mpsc;
    use tokio::time::timeout;

    const TIMESTAMP_TOOL: &str = "browser_youtube_get_current_timestamp";

    fn sample_descriptor(name: &str) -> WireToolDescriptor {
        WireToolDescriptor {
            definition: agent_chain_core::tools::ToolDefinition {
                name: name.to_string(),
                description: "test tool".to_string(),
                parameters: json!({"type": "object"}),
            },
            output_schema: json!({"type": "object"}),
            timeout_ms: 2_000,
            source: ToolSource::Bridge {
                app_kind: "browser".into(),
            },
            required_contexts: Vec::new(),
            requires_user_approval: false,
        }
    }

    /// Stub `ToolBackend` whose canned outcome is returned to every
    /// dispatch. Records every call for assertions; can be configured to
    /// block on the call-level cancel token instead of returning
    /// immediately for the cancellation tests.
    struct StubBackend {
        canned: Value,
        tools: Vec<WireToolDescriptor>,
        system_blocks: Vec<ContentBlock>,
        await_cancel: bool,
        seen: Arc<StdMutex<Vec<ToolBackendCall>>>,
    }

    impl StubBackend {
        fn new(canned: Value, tools: Vec<WireToolDescriptor>) -> Arc<Self> {
            Arc::new(Self {
                canned,
                tools,
                system_blocks: Vec::new(),
                await_cancel: false,
                seen: Arc::new(StdMutex::new(Vec::new())),
            })
        }

        fn with_system_blocks(
            canned: Value,
            tools: Vec<WireToolDescriptor>,
            system_blocks: Vec<ContentBlock>,
        ) -> Arc<Self> {
            Arc::new(Self {
                canned,
                tools,
                system_blocks,
                await_cancel: false,
                seen: Arc::new(StdMutex::new(Vec::new())),
            })
        }

        fn await_cancel(tools: Vec<WireToolDescriptor>) -> Arc<Self> {
            Arc::new(Self {
                canned: json!({}),
                tools,
                system_blocks: Vec::new(),
                await_cancel: true,
                seen: Arc::new(StdMutex::new(Vec::new())),
            })
        }

        fn seen(&self) -> Vec<ObservedCall> {
            self.seen
                .lock()
                .unwrap()
                .iter()
                .map(|c| ObservedCall {
                    call_id: c.call_id,
                    name: c.name.clone(),
                    arguments: c.arguments.clone(),
                })
                .collect()
        }
    }

    #[derive(Debug, Clone)]
    struct ObservedCall {
        call_id: u32,
        name: String,
        arguments: Value,
    }

    #[async_trait]
    impl ToolBackend for StubBackend {
        async fn list_tools(&self) -> Vec<WireToolDescriptor> {
            self.tools.clone()
        }

        async fn collect_system_blocks(&self) -> Vec<ContentBlock> {
            self.system_blocks.clone()
        }

        async fn dispatch(
            &self,
            call: ToolBackendCall,
        ) -> std::result::Result<Value, ToolErrorWire> {
            let cancel = call.cancel.clone();
            let canned = self.canned.clone();
            self.seen.lock().unwrap().push(call);
            if self.await_cancel {
                cancel.cancelled().await;
                return Err(ToolErrorWire::Cancelled);
            }
            Ok(canned)
        }
    }

    fn collecting_sink() -> (impl ChatEventSink, Arc<StdMutex<Vec<ChatServerMessage>>>) {
        let captured = Arc::new(StdMutex::new(Vec::<ChatServerMessage>::new()));
        let sink_buf = captured.clone();
        let sink = move |event: ChatServerMessage| {
            sink_buf.lock().unwrap().push(event);
            Ok(())
        };
        (sink, captured)
    }

    async fn next_outbound(harness: &mut ChatSocketHarness) -> ChatClientMessage {
        timeout(Duration::from_secs(2), harness.client_to_server.recv())
            .await
            .expect("outbound frame timed out")
            .expect("outbound channel closed")
    }

    async fn drain_until<F>(
        harness: &mut ChatSocketHarness,
        mut predicate: F,
    ) -> Vec<ChatClientMessage>
    where
        F: FnMut(&ChatClientMessage) -> bool,
    {
        let mut buffer = Vec::new();
        loop {
            let frame = next_outbound(harness).await;
            let stop = predicate(&frame);
            buffer.push(frame);
            if stop {
                return buffer;
            }
        }
    }

    fn send_opening() -> TurnOpening {
        TurnOpening::Send(ChatSendRequest {
            content_blocks: Vec::new(),
            parent_message_id: None,
            asset_chips_json: None,
            activity_id: None,
        })
    }

    #[tokio::test(flavor = "current_thread")]
    async fn capability_update_precedes_opening_frame() {
        let backend =
            StubBackend::new(json!({"ok": true}), vec![sample_descriptor(TIMESTAMP_TOOL)]);
        let bridge = ChatBridge::new(backend.clone());

        let (socket, mut harness) = ChatSocket::test_pair();
        let (sink, _captured) = collecting_sink();
        let cancel = CancellationToken::new();

        let run =
            tokio::spawn(
                async move { bridge.run_turn(socket, send_opening(), cancel, &sink).await },
            );

        match next_outbound(&mut harness).await {
            ChatClientMessage::CapabilityUpdate(payload) => {
                assert_eq!(payload.tools.len(), 1);
                assert_eq!(payload.tools[0].definition.name, TIMESTAMP_TOOL);
                assert!(payload.contexts.is_empty());
                assert!(payload.system_blocks.is_empty());
            }
            other => panic!("expected CapabilityUpdate, got {other:?}"),
        }
        assert!(matches!(
            next_outbound(&mut harness).await,
            ChatClientMessage::Send(_)
        ));

        harness
            .server_to_client
            .send(Ok(ChatServerMessage::Final {
                messages: Vec::new(),
            }))
            .unwrap();
        run.await.unwrap().expect("turn completes");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn empty_backend_advertises_no_tools() {
        let backend = StubBackend::new(json!({}), Vec::new());
        let bridge = ChatBridge::new(backend);

        let (socket, mut harness) = ChatSocket::test_pair();
        let (sink, _captured) = collecting_sink();
        let cancel = CancellationToken::new();

        let run =
            tokio::spawn(
                async move { bridge.run_turn(socket, send_opening(), cancel, &sink).await },
            );

        match next_outbound(&mut harness).await {
            ChatClientMessage::CapabilityUpdate(payload) => {
                assert!(payload.tools.is_empty());
                assert!(payload.contexts.is_empty());
                assert!(payload.system_blocks.is_empty());
            }
            other => panic!("expected CapabilityUpdate, got {other:?}"),
        }
        assert!(matches!(
            next_outbound(&mut harness).await,
            ChatClientMessage::Send(_)
        ));

        harness
            .server_to_client
            .send(Ok(ChatServerMessage::Final {
                messages: Vec::new(),
            }))
            .unwrap();
        run.await.unwrap().expect("turn completes");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn capability_update_carries_system_blocks_from_backend() {
        let prelude = vec![ContentBlock::Text(
            agent_chain_core::messages::TextContentBlock::builder()
                .text("The user is watching `Tokio async patterns`.")
                .build(),
        )];
        let backend = StubBackend::with_system_blocks(
            json!({"ok": true}),
            vec![sample_descriptor(TIMESTAMP_TOOL)],
            prelude.clone(),
        );
        let bridge = ChatBridge::new(backend);

        let (socket, mut harness) = ChatSocket::test_pair();
        let (sink, _captured) = collecting_sink();
        let cancel = CancellationToken::new();

        let run =
            tokio::spawn(
                async move { bridge.run_turn(socket, send_opening(), cancel, &sink).await },
            );

        match next_outbound(&mut harness).await {
            ChatClientMessage::CapabilityUpdate(payload) => {
                assert_eq!(payload.tools.len(), 1);
                assert_eq!(payload.system_blocks, prelude);
            }
            other => panic!("expected CapabilityUpdate, got {other:?}"),
        }
        assert!(matches!(
            next_outbound(&mut harness).await,
            ChatClientMessage::Send(_)
        ));

        harness
            .server_to_client
            .send(Ok(ChatServerMessage::Final {
                messages: Vec::new(),
            }))
            .unwrap();
        run.await.unwrap().expect("turn completes");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn tool_request_dispatches_through_backend() {
        let backend = StubBackend::new(
            json!({"timestamp_seconds": 42.0}),
            vec![sample_descriptor(TIMESTAMP_TOOL)],
        );
        let bridge = ChatBridge::new(backend.clone());

        let (socket, mut harness) = ChatSocket::test_pair();
        let (sink, _captured) = collecting_sink();
        let cancel = CancellationToken::new();

        let run =
            tokio::spawn(
                async move { bridge.run_turn(socket, send_opening(), cancel, &sink).await },
            );

        // CapabilityUpdate + Send.
        let _ = next_outbound(&mut harness).await;
        let _ = next_outbound(&mut harness).await;

        harness
            .server_to_client
            .send(Ok(ChatServerMessage::ToolRequest {
                call_id: 7,
                descriptor: sample_descriptor(TIMESTAMP_TOOL),
                arguments: json!({"k": "v"}),
            }))
            .unwrap();

        let frames = drain_until(&mut harness, |f| {
            matches!(f, ChatClientMessage::ToolResponse { .. })
        })
        .await;
        let response = frames
            .into_iter()
            .find_map(|f| match f {
                ChatClientMessage::ToolResponse { call_id, result } => Some((call_id, result)),
                _ => None,
            })
            .expect("tool response present");
        assert_eq!(response.0, 7);
        match response.1 {
            Ok(value) => assert_eq!(value["timestamp_seconds"], json!(42.0)),
            Err(err) => panic!("expected Ok ToolResponse, got {err:?}"),
        }

        let observations = backend.seen();
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].name, TIMESTAMP_TOOL);
        assert_eq!(observations[0].call_id, 7);
        assert_eq!(observations[0].arguments, json!({"k": "v"}));

        harness
            .server_to_client
            .send(Ok(ChatServerMessage::Final {
                messages: Vec::new(),
            }))
            .unwrap();
        run.await.unwrap().expect("turn completes");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn server_tool_cancel_cancels_the_matching_call() {
        let backend = StubBackend::await_cancel(vec![sample_descriptor(TIMESTAMP_TOOL)]);
        let bridge = ChatBridge::new(backend);

        let (socket, mut harness) = ChatSocket::test_pair();
        let (sink, _captured) = collecting_sink();
        let cancel = CancellationToken::new();

        let run =
            tokio::spawn(
                async move { bridge.run_turn(socket, send_opening(), cancel, &sink).await },
            );

        let _ = next_outbound(&mut harness).await;
        let _ = next_outbound(&mut harness).await;

        harness
            .server_to_client
            .send(Ok(ChatServerMessage::ToolRequest {
                call_id: 13,
                descriptor: sample_descriptor(TIMESTAMP_TOOL),
                arguments: json!({}),
            }))
            .unwrap();
        tokio::task::yield_now().await;

        harness
            .server_to_client
            .send(Ok(ChatServerMessage::ToolCancel { call_id: 13 }))
            .unwrap();

        match next_outbound(&mut harness).await {
            ChatClientMessage::ToolResponse {
                call_id,
                result: Err(ToolErrorWire::Cancelled),
            } => assert_eq!(call_id, 13),
            other => panic!("expected Cancelled ToolResponse, got {other:?}"),
        }

        harness
            .server_to_client
            .send(Ok(ChatServerMessage::Final {
                messages: Vec::new(),
            }))
            .unwrap();
        run.await.unwrap().expect("turn completes");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn ui_cancel_emits_cancel_frame_and_cancels_dispatches() {
        let backend = StubBackend::await_cancel(vec![sample_descriptor(TIMESTAMP_TOOL)]);
        let bridge = ChatBridge::new(backend);

        let (socket, mut harness) = ChatSocket::test_pair();
        let (sink, _captured) = collecting_sink();
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let run = tokio::spawn(async move {
            bridge
                .run_turn(socket, send_opening(), cancel_clone, &sink)
                .await
        });

        let _ = next_outbound(&mut harness).await;
        let _ = next_outbound(&mut harness).await;

        harness
            .server_to_client
            .send(Ok(ChatServerMessage::ToolRequest {
                call_id: 17,
                descriptor: sample_descriptor(TIMESTAMP_TOOL),
                arguments: json!({}),
            }))
            .unwrap();
        tokio::task::yield_now().await;

        cancel.cancel();

        let frames = drain_until(&mut harness, |f| {
            matches!(f, ChatClientMessage::ToolResponse { .. })
        })
        .await;
        let cancel_seen = frames
            .iter()
            .any(|f| matches!(f, ChatClientMessage::Cancel));
        assert!(cancel_seen, "Cancel frame must be emitted on UI cancel");
        let response = frames.iter().find_map(|f| match f {
            ChatClientMessage::ToolResponse { call_id, result } => Some((*call_id, result.clone())),
            _ => None,
        });
        match response {
            Some((call_id, Err(ToolErrorWire::Cancelled))) => assert_eq!(call_id, 17),
            other => panic!("expected Cancelled ToolResponse, got {other:?}"),
        }

        harness
            .server_to_client
            .send(Ok(ChatServerMessage::Final {
                messages: Vec::new(),
            }))
            .unwrap();
        run.await.unwrap().expect("turn completes after cancel");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn error_frame_terminates_turn_through_sink() {
        let backend = StubBackend::new(json!({}), Vec::new());
        let bridge = ChatBridge::new(backend);

        let (socket, mut harness) = ChatSocket::test_pair();
        let (sink_tx, mut sink_rx) = mpsc::unbounded_channel::<ChatServerMessage>();
        let sink = move |event: ChatServerMessage| {
            sink_tx
                .send(event)
                .map_err(|e| ChatSinkError(e.to_string()))
        };
        let cancel = CancellationToken::new();

        let run =
            tokio::spawn(
                async move { bridge.run_turn(socket, send_opening(), cancel, &sink).await },
            );

        let _ = next_outbound(&mut harness).await;
        let _ = next_outbound(&mut harness).await;

        harness
            .server_to_client
            .send(Ok(ChatServerMessage::Error {
                kind: "boom".into(),
                message: "the server is sad".into(),
            }))
            .unwrap();

        run.await.unwrap().expect("turn returns Ok on Error frame");
        let event = sink_rx.recv().await.expect("error frame forwarded");
        match event {
            ChatServerMessage::Error { kind, message } => {
                assert_eq!(kind, "boom");
                assert_eq!(message, "the server is sad");
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn unexpected_socket_close_is_an_error() {
        let backend = StubBackend::new(json!({}), Vec::new());
        let bridge = ChatBridge::new(backend);

        let (socket, mut harness) = ChatSocket::test_pair();
        let (sink, _captured) = collecting_sink();
        let cancel = CancellationToken::new();

        let run =
            tokio::spawn(
                async move { bridge.run_turn(socket, send_opening(), cancel, &sink).await },
            );

        let _ = next_outbound(&mut harness).await;
        let _ = next_outbound(&mut harness).await;
        drop(harness);

        let err = run
            .await
            .unwrap()
            .expect_err("socket close without Final is an error");
        match err {
            Error::ChatProtocol(msg) => assert!(
                msg.contains("closed before terminal frame"),
                "unexpected ChatProtocol message: {msg}"
            ),
            other => panic!("expected ChatProtocol, got {other:?}"),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn tool_request_is_not_forwarded_to_ui_sink() {
        let backend = StubBackend::new(json!({}), vec![sample_descriptor(TIMESTAMP_TOOL)]);
        let bridge = ChatBridge::new(backend);

        let (socket, mut harness) = ChatSocket::test_pair();
        let (sink, captured) = collecting_sink();
        let cancel = CancellationToken::new();

        let run =
            tokio::spawn(
                async move { bridge.run_turn(socket, send_opening(), cancel, &sink).await },
            );

        let _ = next_outbound(&mut harness).await;
        let _ = next_outbound(&mut harness).await;

        harness
            .server_to_client
            .send(Ok(ChatServerMessage::ToolRequest {
                call_id: 21,
                descriptor: sample_descriptor(TIMESTAMP_TOOL),
                arguments: json!({}),
            }))
            .unwrap();
        let _ = drain_until(&mut harness, |f| {
            matches!(f, ChatClientMessage::ToolResponse { .. })
        })
        .await;

        harness
            .server_to_client
            .send(Ok(ChatServerMessage::Final {
                messages: Vec::new(),
            }))
            .unwrap();
        run.await.unwrap().expect("turn completes");

        let frames = captured.lock().unwrap();
        assert_eq!(
            frames.len(),
            1,
            "sink should have received only the Final frame"
        );
        assert!(matches!(frames[0], ChatServerMessage::Final { .. }));
    }
}
