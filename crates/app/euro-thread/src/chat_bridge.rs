//! Per-turn chat state machine.
//!
//! [`ChatBridge`] owns one chat WebSocket for the duration of a single
//! turn. At turn start it snapshots the client [`ContextRegistry`],
//! filters the [`Catalog`] of dispatchers down to tools whose required
//! contexts are all live, and emits a `CapabilityUpdate` frame describing
//! the LLM-visible capability surface for the turn. The opening frame
//! (`Send` or `Regenerate`) follows; from there the bridge multiplexes
//! three responsibilities over the WS:
//!
//! - forward user-visible chat frames (`Chunk`, `ConfirmedHumanMessage`,
//!   `Final`, `Error`) to a caller-owned [`ChatEventSink`],
//! - dispatch incoming `ToolRequest` frames through the catalog and emit
//!   the matching `ToolResponse` on completion, and
//! - propagate cancellation: UI-level cancel triggers
//!   `ChatClientMessage::Cancel` and cancels every in-flight dispatch,
//!   server-issued `ToolCancel` targets a single call.
//!
//! All in-flight dispatches share a [`tokio::task::JoinSet`] rooted on
//! `run_turn`'s stack — a grace period after the terminal frame lets
//! straggler responses ride out the socket, then the set is aborted so
//! a stuck adapter cannot pin the turn beyond
//! [`DISPATCH_DRAIN`].

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use eurora_tools::{
    ActiveContext, Catalog, ContextRegistry, IncomingCall, Origin, ToolDescriptor, ToolError,
};
use serde_json::Value;
use thread_core::{
    CapabilityUpdatePayload, ChatClientMessage, ChatSendRequest, ChatServerMessage,
    RegenerateRequest, ToolErrorWire, WireActiveContext, WireToolDescriptor,
};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::chat_socket::{ChatOutbound, ChatSocket};
use crate::error::{Error, Result};

/// Bridge-level shutdown grace period.
///
/// After `Final` / `Error` arrives, in-flight dispatch tasks have this
/// long to deliver their final `ToolResponse` before being aborted. The
/// window is conservative — well below the longest per-tool timeout but
/// long enough to absorb a single ms-scale serialise + send through the
/// outbound channel.
const DISPATCH_DRAIN: Duration = Duration::from_secs(1);

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
/// The bridge holds shared references to the client [`ContextRegistry`]
/// and the [`Catalog`] of registered dispatchers, both of which are
/// app-wide singletons managed by Tauri. One bridge can drive many
/// turns sequentially — there's no per-turn state on `self`; all turn
/// state lives on the stack of [`Self::run_turn`].
pub struct ChatBridge {
    contexts: Arc<ContextRegistry>,
    catalog: Arc<Catalog>,
}

impl ChatBridge {
    pub fn new(contexts: Arc<ContextRegistry>, catalog: Arc<Catalog>) -> Self {
        Self { contexts, catalog }
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
        let snapshot = self.contexts.snapshot();
        let descriptors = descriptors_for(&self.catalog, &snapshot);

        let targets: HashMap<String, Origin> = snapshot
            .iter()
            .map(|c| (c.key.clone(), c.origin.clone()))
            .collect();
        let capabilities: HashSet<String> = descriptors.iter().map(|d| d.name.to_owned()).collect();

        let wire_tools: Vec<WireToolDescriptor> =
            descriptors.iter().map(ToolDescriptor::to_wire).collect();
        let wire_contexts: Vec<WireActiveContext> = snapshot
            .into_iter()
            .map(|c| WireActiveContext {
                key: c.key,
                activated_at: c.activated_at,
                data: c.data,
            })
            .collect();

        socket.try_send(ChatClientMessage::CapabilityUpdate(
            CapabilityUpdatePayload {
                tools: wire_tools,
                contexts: wire_contexts,
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
            targets,
            capabilities,
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
        // stuck adapter cannot pin the turn beyond `DISPATCH_DRAIN`.
        turn.turn_cancel.cancel();
        let drain = async { while dispatches.join_next().await.is_some() {} };
        if tokio::time::timeout(DISPATCH_DRAIN, drain).await.is_err() {
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
        let wire_name = descriptor.definition.name.clone();

        // Defense in depth (1): only dispatch what we advertised.
        // Catches schema drift, server bugs, and stale capability
        // assumptions.
        if !turn.capabilities.contains(&wire_name) {
            let _ = outbound.send(ChatClientMessage::ToolResponse {
                call_id,
                result: Err(ToolErrorWire::ContextUnavailable {
                    tool: wire_name,
                    reason: "tool not advertised in this turn".into(),
                }),
            });
            return;
        }

        // Resolve origin from the frozen turn snapshot. v1 tools all
        // declare at least one `required_context`; a descriptor with
        // no required contexts has no routing target on the client
        // side and is rejected here.
        let origin = descriptor
            .required_contexts
            .iter()
            .find_map(|ctx| turn.targets.get(ctx).cloned());

        let dispatcher = self.catalog.dispatcher_for(&wire_name);
        let cancel = turn.turn_cancel.child_token();
        turn.inflight.insert(call_id, cancel.clone());
        let inflight = turn.inflight.clone();

        set.spawn(async move {
            let result: std::result::Result<Value, ToolError> = match (origin, dispatcher) {
                (Some(origin), Some(dispatcher)) => {
                    match dispatcher.descriptor_name_for(&wire_name) {
                        Some(static_name) => {
                            dispatcher
                                .dispatch(IncomingCall {
                                    call_id,
                                    descriptor_name: static_name,
                                    arguments,
                                    origin,
                                    cancel: cancel.clone(),
                                })
                                .await
                        }
                        None => Err(ToolError::Remote {
                            code: 404,
                            message: format!("dispatcher does not recognise `{wire_name}`"),
                            details: None,
                        }),
                    }
                }
                (None, _) => Err(ToolError::ContextUnavailable {
                    tool: Cow::Owned(wire_name),
                    reason: Cow::Borrowed("no live context at turn start"),
                }),
                (_, None) => Err(ToolError::Remote {
                    code: 404,
                    message: format!("no dispatcher for `{wire_name}`"),
                    details: None,
                }),
            };

            inflight.remove(&call_id);
            let _ = outbound.send(ChatClientMessage::ToolResponse {
                call_id,
                result: result.map_err(ToolErrorWire::from),
            });
        });
    }
}

struct TurnState {
    /// Frozen per-context routing targets — `context_key → Origin`.
    targets: HashMap<String, Origin>,
    /// Wire names advertised in `CapabilityUpdate` this turn. Used to
    /// reject `ToolRequest` frames the server invents or sends past
    /// the capability surface.
    capabilities: HashSet<String>,
    /// Child of the caller's cancel token; fires on UI cancel and at
    /// the end of `run_turn` to wake every dispatch task.
    turn_cancel: CancellationToken,
    /// Per-call cancellation tokens so `ToolCancel { call_id }` can
    /// abort exactly one dispatch without disturbing siblings.
    inflight: Arc<DashMap<u32, CancellationToken>>,
}

/// Filter the catalog down to the descriptors whose every required
/// context is currently active.
fn descriptors_for(catalog: &Catalog, snapshot: &[ActiveContext]) -> Vec<ToolDescriptor> {
    let active: HashSet<&str> = snapshot.iter().map(|c| c.key.as_str()).collect();
    catalog
        .all_descriptors()
        .into_iter()
        .filter(|d| d.required_contexts.iter().all(|c| active.contains(*c)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_socket::ChatSocketHarness;
    use chrono::Utc;
    use eurora_tools::{BrowserOrigin, Dispatcher, Empty, schema_of};
    use futures::future::BoxFuture;
    use serde_json::json;
    use std::sync::Mutex as StdMutex;
    use std::time::Duration;
    use thread_core::ToolSource;
    use tokio::sync::mpsc;
    use tokio::time::timeout;

    const TIMESTAMP_TOOL: &str = "browser::youtube::get_current_timestamp";

    /// `ToolSource::Bridge { app_kind: String }` can't be constructed in a
    /// `const`, but the bridge logic ignores `source` (it's a server-side
    /// concern). `ClientLocal` keeps the descriptor table `const`-friendly.
    const YOUTUBE_DESCRIPTORS: &[ToolDescriptor] = &[ToolDescriptor {
        name: TIMESTAMP_TOOL,
        description: "Return the user's current playback position.",
        input_schema: schema_of::<Empty>,
        output_schema: schema_of::<Empty>,
        timeout: Duration::from_millis(2_000),
        source: ToolSource::ClientLocal,
        required_contexts: &["youtube::watch_page"],
        requires_user_approval: false,
    }];

    fn youtube_descriptor() -> &'static [ToolDescriptor] {
        YOUTUBE_DESCRIPTORS
    }

    /// Captures every dispatch and returns a canned value. The
    /// per-call cancellation token can race the canned response —
    /// `await_cancel` toggles that for the "block on cancel" tests.
    struct StubDispatcher {
        canned: Value,
        await_cancel: bool,
        descriptors: &'static [ToolDescriptor],
        seen: Arc<StdMutex<Vec<DispatchObservation>>>,
    }

    #[derive(Debug, Clone)]
    struct DispatchObservation {
        descriptor_name: &'static str,
        call_id: u32,
        origin_variant: &'static str,
    }

    impl StubDispatcher {
        fn new(canned: Value) -> Arc<Self> {
            Arc::new(Self {
                canned,
                await_cancel: false,
                descriptors: youtube_descriptor(),
                seen: Arc::new(StdMutex::new(Vec::new())),
            })
        }

        fn await_cancel(canned: Value) -> Arc<Self> {
            Arc::new(Self {
                canned,
                await_cancel: true,
                descriptors: youtube_descriptor(),
                seen: Arc::new(StdMutex::new(Vec::new())),
            })
        }

        fn seen(&self) -> Vec<DispatchObservation> {
            self.seen.lock().unwrap().clone()
        }
    }

    impl Dispatcher for StubDispatcher {
        fn descriptors(&self) -> &'static [ToolDescriptor] {
            self.descriptors
        }

        fn dispatch(
            &self,
            call: IncomingCall,
        ) -> BoxFuture<'_, std::result::Result<Value, ToolError>> {
            let canned = self.canned.clone();
            let await_cancel = self.await_cancel;
            let seen = self.seen.clone();
            Box::pin(async move {
                seen.lock().unwrap().push(DispatchObservation {
                    descriptor_name: call.descriptor_name,
                    call_id: call.call_id,
                    origin_variant: call.origin.variant_name(),
                });
                let _ = call.arguments;
                if await_cancel {
                    call.cancel.cancelled().await;
                    return Err(ToolError::Cancelled);
                }
                Ok(canned)
            })
        }
    }

    fn registry_with_youtube_context() -> Arc<ContextRegistry> {
        let registry = ContextRegistry::new();
        registry.activate(ActiveContext {
            key: "youtube::watch_page".into(),
            activated_at: Utc::now(),
            data: json!({"video_id": "abc123"}),
            origin: Origin::Browser(BrowserOrigin {
                process_id: 4242,
                tab_id: 19,
                window_id: Some("win-0".into()),
                page_url: "https://www.youtube.com/watch?v=abc123".into(),
            }),
        });
        Arc::new(registry)
    }

    fn empty_registry() -> Arc<ContextRegistry> {
        Arc::new(ContextRegistry::new())
    }

    fn catalog_with(dispatcher: Arc<dyn Dispatcher>) -> Arc<Catalog> {
        let catalog = Catalog::new();
        catalog.register(dispatcher);
        Arc::new(catalog)
    }

    /// Build a `(sink, captured)` pair where `sink` is the
    /// `ChatEventSink` closure handed to `run_turn` and `captured`
    /// gives the test read access to the frames it received.
    fn collecting_sink() -> (impl ChatEventSink, Arc<StdMutex<Vec<ChatServerMessage>>>) {
        let captured = Arc::new(StdMutex::new(Vec::<ChatServerMessage>::new()));
        let sink_buf = captured.clone();
        let sink = move |event: ChatServerMessage| {
            sink_buf.lock().unwrap().push(event);
            Ok(())
        };
        (sink, captured)
    }

    /// Synthesize a `WireToolDescriptor` for `TIMESTAMP_TOOL` so tests
    /// can craft `ToolRequest` frames without parsing the actual
    /// `CapabilityUpdate` the bridge emits.
    fn timestamp_wire_descriptor() -> WireToolDescriptor {
        youtube_descriptor()[0].to_wire()
    }

    /// Receive on `client_to_server` with a small per-message
    /// timeout — keeps tests responsive on assertion failures.
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

    #[tokio::test(flavor = "current_thread")]
    async fn capability_update_precedes_opening_frame() {
        let stub = StubDispatcher::new(json!({"ok": true}));
        let bridge = ChatBridge::new(registry_with_youtube_context(), catalog_with(stub.clone()));

        let (socket, mut harness) = ChatSocket::test_pair();
        let (sink, _captured) = collecting_sink();
        let cancel = CancellationToken::new();

        let opening = TurnOpening::Send(ChatSendRequest {
            content_blocks: Vec::new(),
            parent_message_id: None,
            asset_chips_json: None,
            activity_id: None,
        });

        let run =
            tokio::spawn(async move { bridge.run_turn(socket, opening, cancel, &sink).await });

        match next_outbound(&mut harness).await {
            ChatClientMessage::CapabilityUpdate(payload) => {
                assert_eq!(payload.tools.len(), 1);
                assert_eq!(payload.tools[0].definition.name, TIMESTAMP_TOOL);
                assert_eq!(payload.contexts.len(), 1);
                assert_eq!(payload.contexts[0].key, "youtube::watch_page");
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
    async fn empty_registry_advertises_no_tools() {
        let stub = StubDispatcher::new(json!({}));
        let bridge = ChatBridge::new(empty_registry(), catalog_with(stub.clone()));

        let (socket, mut harness) = ChatSocket::test_pair();
        let (sink, _captured) = collecting_sink();
        let cancel = CancellationToken::new();

        let run = tokio::spawn(async move {
            bridge
                .run_turn(
                    socket,
                    TurnOpening::Send(ChatSendRequest {
                        content_blocks: Vec::new(),
                        parent_message_id: None,
                        asset_chips_json: None,
                        activity_id: None,
                    }),
                    cancel,
                    &sink,
                )
                .await
        });

        match next_outbound(&mut harness).await {
            ChatClientMessage::CapabilityUpdate(payload) => {
                assert!(payload.tools.is_empty());
                assert!(payload.contexts.is_empty());
            }
            other => panic!("expected CapabilityUpdate, got {other:?}"),
        }
        // Drain the Send frame so the harness's recv doesn't lag.
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
    async fn tool_request_dispatches_through_catalog() {
        let stub = StubDispatcher::new(json!({
            "video_id": "abc123",
            "timestamp_seconds": 42.0,
            "duration_seconds": 100.0,
            "playing": true,
        }));
        let bridge = ChatBridge::new(registry_with_youtube_context(), catalog_with(stub.clone()));

        let (socket, mut harness) = ChatSocket::test_pair();
        let (sink, _captured) = collecting_sink();
        let cancel = CancellationToken::new();

        let run = tokio::spawn(async move {
            bridge
                .run_turn(
                    socket,
                    TurnOpening::Send(ChatSendRequest {
                        content_blocks: Vec::new(),
                        parent_message_id: None,
                        asset_chips_json: None,
                        activity_id: None,
                    }),
                    cancel,
                    &sink,
                )
                .await
        });

        // CapabilityUpdate + Send.
        let _ = next_outbound(&mut harness).await;
        let _ = next_outbound(&mut harness).await;

        harness
            .server_to_client
            .send(Ok(ChatServerMessage::ToolRequest {
                call_id: 7,
                descriptor: timestamp_wire_descriptor(),
                arguments: json!({}),
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
            Ok(value) => assert_eq!(value["video_id"], json!("abc123")),
            Err(err) => panic!("expected Ok ToolResponse, got {err:?}"),
        }

        let observations = stub.seen();
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].descriptor_name, TIMESTAMP_TOOL);
        assert_eq!(observations[0].call_id, 7);
        assert_eq!(observations[0].origin_variant, "Browser");

        harness
            .server_to_client
            .send(Ok(ChatServerMessage::Final {
                messages: Vec::new(),
            }))
            .unwrap();
        run.await.unwrap().expect("turn completes");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn tool_request_for_unadvertised_tool_returns_context_unavailable() {
        // Empty registry → no tools advertised, but the catalog still
        // has a youtube dispatcher. The bridge should refuse the
        // request without ever invoking the dispatcher.
        let stub = StubDispatcher::new(json!({}));
        let bridge = ChatBridge::new(empty_registry(), catalog_with(stub.clone()));

        let (socket, mut harness) = ChatSocket::test_pair();
        let (sink, _captured) = collecting_sink();
        let cancel = CancellationToken::new();

        let run = tokio::spawn(async move {
            bridge
                .run_turn(
                    socket,
                    TurnOpening::Send(ChatSendRequest {
                        content_blocks: Vec::new(),
                        parent_message_id: None,
                        asset_chips_json: None,
                        activity_id: None,
                    }),
                    cancel,
                    &sink,
                )
                .await
        });

        let _ = next_outbound(&mut harness).await;
        let _ = next_outbound(&mut harness).await;

        harness
            .server_to_client
            .send(Ok(ChatServerMessage::ToolRequest {
                call_id: 9,
                descriptor: timestamp_wire_descriptor(),
                arguments: json!({}),
            }))
            .unwrap();

        match next_outbound(&mut harness).await {
            ChatClientMessage::ToolResponse {
                call_id,
                result: Err(ToolErrorWire::ContextUnavailable { tool, .. }),
            } => {
                assert_eq!(call_id, 9);
                assert_eq!(tool, TIMESTAMP_TOOL);
            }
            other => panic!("expected ContextUnavailable, got {other:?}"),
        }
        assert!(
            stub.seen().is_empty(),
            "dispatcher must not have been invoked"
        );

        harness
            .server_to_client
            .send(Ok(ChatServerMessage::Final {
                messages: Vec::new(),
            }))
            .unwrap();
        run.await.unwrap().expect("turn completes");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn tool_request_with_no_resolvable_origin_returns_context_unavailable() {
        // Custom descriptor with NO required contexts so the bridge
        // exercises the "no resolvable origin" branch — the dispatcher
        // is advertised but the snapshot supplies no route.
        const NO_CTX_TOOL: &str = "browser::test::no_context";
        const DESCRIPTORS: &[ToolDescriptor] = &[ToolDescriptor {
            name: NO_CTX_TOOL,
            description: "No context required.",
            input_schema: schema_of::<Empty>,
            output_schema: schema_of::<Empty>,
            timeout: Duration::from_millis(100),
            source: ToolSource::ClientLocal,
            required_contexts: &[],
            requires_user_approval: false,
        }];

        struct ContextlessDispatcher;
        impl Dispatcher for ContextlessDispatcher {
            fn descriptors(&self) -> &'static [ToolDescriptor] {
                DESCRIPTORS
            }
            fn dispatch(
                &self,
                _call: IncomingCall,
            ) -> BoxFuture<'_, std::result::Result<Value, ToolError>> {
                Box::pin(async { Ok(json!({})) })
            }
        }

        let bridge = ChatBridge::new(
            empty_registry(),
            catalog_with(Arc::new(ContextlessDispatcher)),
        );

        let (socket, mut harness) = ChatSocket::test_pair();
        let (sink, _captured) = collecting_sink();
        let cancel = CancellationToken::new();

        let run = tokio::spawn(async move {
            bridge
                .run_turn(
                    socket,
                    TurnOpening::Send(ChatSendRequest {
                        content_blocks: Vec::new(),
                        parent_message_id: None,
                        asset_chips_json: None,
                        activity_id: None,
                    }),
                    cancel,
                    &sink,
                )
                .await
        });

        // CapabilityUpdate is the first frame — confirm the tool is advertised.
        match next_outbound(&mut harness).await {
            ChatClientMessage::CapabilityUpdate(payload) => {
                assert_eq!(payload.tools.len(), 1);
            }
            other => panic!("expected CapabilityUpdate, got {other:?}"),
        }
        let _ = next_outbound(&mut harness).await;

        harness
            .server_to_client
            .send(Ok(ChatServerMessage::ToolRequest {
                call_id: 11,
                descriptor: DESCRIPTORS[0].to_wire(),
                arguments: json!({}),
            }))
            .unwrap();

        match next_outbound(&mut harness).await {
            ChatClientMessage::ToolResponse {
                call_id,
                result: Err(ToolErrorWire::ContextUnavailable { tool, .. }),
            } => {
                assert_eq!(call_id, 11);
                assert_eq!(tool, NO_CTX_TOOL);
            }
            other => panic!("expected ContextUnavailable, got {other:?}"),
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
    async fn server_tool_cancel_cancels_the_matching_call() {
        let stub = StubDispatcher::await_cancel(json!({}));
        let bridge = ChatBridge::new(registry_with_youtube_context(), catalog_with(stub.clone()));

        let (socket, mut harness) = ChatSocket::test_pair();
        let (sink, _captured) = collecting_sink();
        let cancel = CancellationToken::new();

        let run = tokio::spawn(async move {
            bridge
                .run_turn(
                    socket,
                    TurnOpening::Send(ChatSendRequest {
                        content_blocks: Vec::new(),
                        parent_message_id: None,
                        asset_chips_json: None,
                        activity_id: None,
                    }),
                    cancel,
                    &sink,
                )
                .await
        });

        let _ = next_outbound(&mut harness).await;
        let _ = next_outbound(&mut harness).await;

        harness
            .server_to_client
            .send(Ok(ChatServerMessage::ToolRequest {
                call_id: 13,
                descriptor: timestamp_wire_descriptor(),
                arguments: json!({}),
            }))
            .unwrap();

        // Give the dispatch task a moment to reach `.cancelled().await`.
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
        let stub = StubDispatcher::await_cancel(json!({}));
        let bridge = ChatBridge::new(registry_with_youtube_context(), catalog_with(stub.clone()));

        let (socket, mut harness) = ChatSocket::test_pair();
        let (sink, _captured) = collecting_sink();
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let run = tokio::spawn(async move {
            bridge
                .run_turn(
                    socket,
                    TurnOpening::Send(ChatSendRequest {
                        content_blocks: Vec::new(),
                        parent_message_id: None,
                        asset_chips_json: None,
                        activity_id: None,
                    }),
                    cancel_clone,
                    &sink,
                )
                .await
        });

        let _ = next_outbound(&mut harness).await;
        let _ = next_outbound(&mut harness).await;

        harness
            .server_to_client
            .send(Ok(ChatServerMessage::ToolRequest {
                call_id: 17,
                descriptor: timestamp_wire_descriptor(),
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

        // Server eventually emits Final once it sees the Cancel frame.
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
        let stub = StubDispatcher::new(json!({}));
        let bridge = ChatBridge::new(empty_registry(), catalog_with(stub.clone()));

        let (socket, mut harness) = ChatSocket::test_pair();
        let (sink_tx, mut sink_rx) = mpsc::unbounded_channel::<ChatServerMessage>();
        let sink = move |event: ChatServerMessage| {
            sink_tx
                .send(event)
                .map_err(|e| ChatSinkError(e.to_string()))
        };
        let cancel = CancellationToken::new();

        let run = tokio::spawn(async move {
            bridge
                .run_turn(
                    socket,
                    TurnOpening::Send(ChatSendRequest {
                        content_blocks: Vec::new(),
                        parent_message_id: None,
                        asset_chips_json: None,
                        activity_id: None,
                    }),
                    cancel,
                    &sink,
                )
                .await
        });

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
        let stub = StubDispatcher::new(json!({}));
        let bridge = ChatBridge::new(empty_registry(), catalog_with(stub.clone()));

        let (socket, mut harness) = ChatSocket::test_pair();
        let (sink, _captured) = collecting_sink();
        let cancel = CancellationToken::new();

        let run = tokio::spawn(async move {
            bridge
                .run_turn(
                    socket,
                    TurnOpening::Send(ChatSendRequest {
                        content_blocks: Vec::new(),
                        parent_message_id: None,
                        asset_chips_json: None,
                        activity_id: None,
                    }),
                    cancel,
                    &sink,
                )
                .await
        });

        // Drain CapabilityUpdate + Send so the bridge has progressed
        // into its inbound loop before we sever the socket; otherwise
        // the bridge would error on the very first outbound `try_send`
        // and the test wouldn't exercise the "inbound closed
        // mid-stream" path it's meant to.
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
        let stub = StubDispatcher::new(json!({}));
        let bridge = ChatBridge::new(registry_with_youtube_context(), catalog_with(stub.clone()));

        let (socket, mut harness) = ChatSocket::test_pair();
        let (sink, captured) = collecting_sink();
        let cancel = CancellationToken::new();

        let run = tokio::spawn(async move {
            bridge
                .run_turn(
                    socket,
                    TurnOpening::Send(ChatSendRequest {
                        content_blocks: Vec::new(),
                        parent_message_id: None,
                        asset_chips_json: None,
                        activity_id: None,
                    }),
                    cancel,
                    &sink,
                )
                .await
        });

        let _ = next_outbound(&mut harness).await;
        let _ = next_outbound(&mut harness).await;

        harness
            .server_to_client
            .send(Ok(ChatServerMessage::ToolRequest {
                call_id: 21,
                descriptor: timestamp_wire_descriptor(),
                arguments: json!({}),
            }))
            .unwrap();
        // Wait for the matching ToolResponse to be flushed so the
        // dispatch path is exercised end-to-end.
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
