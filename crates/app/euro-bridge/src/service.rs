//! [`AppBridgeService`] is the singleton that the rest of the desktop
//! interacts with. It owns the client registry, the broadcast channels that
//! fan inbound frames out to subscribers, and the [`OutboundDispatcher`]
//! that desktop code uses to make requests of connected clients. The
//! WebSocket transport in [`super::websocket`] feeds frames into this
//! service.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use euro_bridge_protocol::{BridgeError, ClientKind, EventFrame, Frame, ResponseFrame};
use tokio::sync::{OnceCell, broadcast};

use crate::outbound::{DEFAULT_REQUEST_TIMEOUT, OutboundDispatcher};
use crate::registry::{ClientRegistry, RegisteredClient, RegistrationEvent};
use crate::router::spawn_router;

/// Loopback WebSocket port for every connected client (browser
/// native-messaging hosts, Office.js add-ins, future first-party
/// integrations).
pub const APP_BRIDGE_PORT: u16 = 1431;

/// Channel capacity for the broadcast that the transport publishes inbound
/// frames onto.
const FRAMES_CHANNEL_CAPACITY: usize = 256;
/// Channel capacity for the rebroadcast of `EventFrame`s.
const EVENTS_CHANNEL_CAPACITY: usize = 256;
/// Channel capacity for registration / disconnect lifecycle events.
const LIFECYCLE_CHANNEL_CAPACITY: usize = 64;

static GLOBAL_SERVICE: OnceCell<AppBridgeService> = OnceCell::const_new();
static ROUTER_STARTED: AtomicBool = AtomicBool::new(false);

#[derive(Clone)]
pub struct AppBridgeService {
    pub registry: ClientRegistry,
    /// Inbound frames from the transport: `(app_pid, frame)`.
    pub frames_tx: broadcast::Sender<(u32, Frame)>,
    events_tx: broadcast::Sender<(u32, EventFrame)>,
    registrations_tx: broadcast::Sender<RegistrationEvent>,
    disconnects_tx: broadcast::Sender<RegistrationEvent>,
    outbound: OutboundDispatcher,
}

impl AppBridgeService {
    pub fn new() -> Self {
        let (frames_tx, _) = broadcast::channel(FRAMES_CHANNEL_CAPACITY);
        let (events_tx, _) = broadcast::channel(EVENTS_CHANNEL_CAPACITY);
        let (registrations_tx, _) = broadcast::channel(LIFECYCLE_CHANNEL_CAPACITY);
        let (disconnects_tx, _) = broadcast::channel(LIFECYCLE_CHANNEL_CAPACITY);

        let registry = ClientRegistry::new();
        let outbound = OutboundDispatcher::new(registry.clone());

        Self {
            registry,
            frames_tx,
            events_tx,
            registrations_tx,
            disconnects_tx,
            outbound,
        }
    }

    /// Lazily initialise the global service and ensure the router task is
    /// running.
    pub async fn get_or_init() -> &'static AppBridgeService {
        let service = GLOBAL_SERVICE
            .get_or_init(|| async { AppBridgeService::new() })
            .await;
        service.start_router();
        service
    }

    /// Start the inbound frame router. Idempotent.
    pub fn start_router(&self) {
        if ROUTER_STARTED.swap(true, Ordering::SeqCst) {
            return;
        }
        spawn_router(
            self.frames_tx.subscribe(),
            self.outbound.pending(),
            self.events_tx.clone(),
        );
    }

    /// Subscribe to every inbound frame from every connected client.
    pub fn subscribe_to_frames(&self) -> broadcast::Receiver<(u32, Frame)> {
        self.frames_tx.subscribe()
    }

    /// Subscribe to inbound `EventFrame`s.
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<(u32, EventFrame)> {
        self.events_tx.subscribe()
    }

    /// Subscribe to registration events. Symmetric with
    /// [`Self::subscribe_to_disconnects`].
    pub fn subscribe_to_registrations(&self) -> broadcast::Receiver<RegistrationEvent> {
        self.registrations_tx.subscribe()
    }

    pub fn subscribe_to_disconnects(&self) -> broadcast::Receiver<RegistrationEvent> {
        self.disconnects_tx.subscribe()
    }

    /// Insert a freshly-registered client and announce it on the
    /// registrations channel. Used by the transport right after it receives
    /// a valid `RegisterFrame`.
    pub(crate) async fn register_client(&self, client: RegisteredClient) {
        let event = RegistrationEvent {
            app_pid: client.app_pid,
            process_name: client.process_name.clone(),
            client_kind: client.client_kind,
        };

        self.registry.insert(client).await;
        let _ = self.registrations_tx.send(event);
    }

    /// Remove a client (only if `host_pid` matches the stored entry, to
    /// avoid clobbering a fresh registration that beat the disconnect
    /// cleanup) and announce the disconnect.
    pub(crate) async fn unregister_client(&self, app_pid: u32, host_pid: u32) {
        let Some(removed) = self
            .registry
            .remove_if_host_matches(app_pid, host_pid)
            .await
        else {
            tracing::warn!(
                "Failed to unregister client: app_pid={app_pid} host_pid={host_pid} not found or mismatched"
            );
            return;
        };
        let _ = self.disconnects_tx.send(RegistrationEvent {
            app_pid: removed.app_pid,
            process_name: removed.process_name,
            client_kind: removed.client_kind,
        });
    }

    pub async fn is_registered(&self, app_pid: u32) -> bool {
        self.registry.contains(app_pid).await
    }

    pub async fn registered_pids(&self) -> Vec<u32> {
        self.registry.pids().await
    }

    pub async fn connection_count(&self) -> usize {
        self.registry.len().await
    }

    pub async fn find_pid_by_process_name(
        &self,
        process_name: &str,
        kind: Option<ClientKind>,
    ) -> Option<u32> {
        self.registry
            .find_pid_by_process_name(process_name, kind)
            .await
    }

    /// Send an outbound `RequestFrame` and await the matching response.
    /// Uses [`DEFAULT_REQUEST_TIMEOUT`].
    pub async fn send_request(
        &self,
        app_pid: u32,
        action: &str,
        payload: Option<String>,
    ) -> Result<ResponseFrame, BridgeError> {
        self.outbound
            .send_request(app_pid, action, payload, DEFAULT_REQUEST_TIMEOUT)
            .await
    }

    /// Variant of [`Self::send_request`] with a custom timeout.
    pub async fn send_request_with_timeout(
        &self,
        app_pid: u32,
        action: &str,
        payload: Option<String>,
        timeout: Duration,
    ) -> Result<ResponseFrame, BridgeError> {
        self.outbound
            .send_request(app_pid, action, payload, timeout)
            .await
    }

    /// Convenience wrapper used by activity strategies.
    pub async fn get_metadata(&self, app_pid: u32) -> Result<ResponseFrame, BridgeError> {
        self.send_request(app_pid, "GET_METADATA", None).await
    }
}

impl Default for AppBridgeService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use euro_bridge_protocol::{
        BridgeError, CancelFrame, ClientKind, Frame, FrameKind, ResponseFrame,
    };
    use tokio::sync::mpsc;

    use super::*;
    use crate::registry::RegisteredClient;

    async fn register_test_client(
        service: &AppBridgeService,
        app_pid: u32,
    ) -> mpsc::Receiver<Frame> {
        let (tx, rx) = mpsc::channel(8);
        service
            .register_client(RegisteredClient {
                tx,
                host_pid: 1,
                app_pid,
                process_name: "test".to_string(),
                client_kind: ClientKind::Browser,
            })
            .await;
        rx
    }

    fn router_for(service: &AppBridgeService) {
        // Tests construct fresh services; route their inbound traffic
        // independently of the global router.
        crate::router::spawn_router(
            service.frames_tx.subscribe(),
            service.outbound.pending(),
            service.events_tx.clone(),
        );
    }

    #[tokio::test]
    async fn send_request_resolves_with_response() {
        let service = AppBridgeService::new();
        router_for(&service);
        let mut rx = register_test_client(&service, 100).await;

        let svc = service.clone();
        let task = tokio::spawn(async move {
            let frame = rx.recv().await.expect("request");
            let request = match frame.kind {
                FrameKind::Request(r) => r,
                other => panic!("expected request, got {other:?}"),
            };
            let response = ResponseFrame {
                id: request.id,
                action: request.action,
                payload: Some("ok".into()),
            };
            let _ = svc.frames_tx.send((100, Frame::from(response)));
            rx
        });

        let response = service
            .send_request_with_timeout(100, "TEST", None, Duration::from_secs(2))
            .await
            .expect("response");
        assert_eq!(response.payload.as_deref(), Some("ok"));

        let _ = task.await;
    }

    #[tokio::test]
    async fn send_request_times_out_and_emits_cancel() {
        let service = AppBridgeService::new();
        router_for(&service);
        let mut rx = register_test_client(&service, 101).await;

        let svc = service.clone();
        let request_task = tokio::spawn(async move {
            svc.send_request_with_timeout(101, "TEST", None, Duration::from_millis(50))
                .await
        });

        let request_frame = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("request frame must arrive")
            .expect("frame must be Some");
        let request_id = match request_frame.kind {
            FrameKind::Request(r) => r.id,
            other => panic!("expected request, got {other:?}"),
        };

        let result = request_task.await.expect("task did not panic");
        assert!(matches!(result, Err(BridgeError::Timeout)));

        let cancel_frame = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("cancel frame must arrive")
            .expect("frame must be Some");
        match cancel_frame.kind {
            FrameKind::Cancel(CancelFrame { id }) => {
                assert_eq!(id, request_id, "cancel id must match request id");
            }
            other => panic!("expected cancel frame, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn client_cancel_resolves_request_without_echo() {
        let service = AppBridgeService::new();
        router_for(&service);
        let mut rx = register_test_client(&service, 102).await;

        let svc = service.clone();
        let request_task = tokio::spawn(async move {
            svc.send_request_with_timeout(102, "TEST", None, Duration::from_secs(5))
                .await
        });

        let request_frame = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("request frame must arrive")
            .expect("frame must be Some");
        let request_id = match request_frame.kind {
            FrameKind::Request(r) => r.id,
            other => panic!("expected request, got {other:?}"),
        };

        // Client decides to cancel.
        let _ = service
            .frames_tx
            .send((102, Frame::from(CancelFrame { id: request_id })));

        // The desktop-side request should resolve (with channel-closed,
        // since the entry was dropped).
        let result = tokio::time::timeout(Duration::from_secs(1), request_task)
            .await
            .expect("must resolve quickly")
            .expect("task did not panic");
        assert!(matches!(result, Err(BridgeError::ChannelClosed)));

        // No echo cancel should reach the client.
        let pending = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(
            pending.is_err(),
            "no further frames should be delivered to the client"
        );
    }

    #[tokio::test]
    async fn send_request_to_unknown_pid_returns_not_found() {
        let service = AppBridgeService::new();
        router_for(&service);

        let result = service
            .send_request_with_timeout(999, "TEST", None, Duration::from_millis(50))
            .await;
        assert!(matches!(
            result,
            Err(BridgeError::NotFound { app_pid: 999 })
        ));
    }

    #[tokio::test]
    async fn dropped_caller_emits_cancel_to_client() {
        let service = AppBridgeService::new();
        router_for(&service);
        let mut rx = register_test_client(&service, 103).await;

        let svc = service.clone();
        let request_task = tokio::spawn(async move {
            svc.send_request_with_timeout(103, "TEST", None, Duration::from_secs(30))
                .await
        });

        let request_frame = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("request frame must arrive")
            .expect("frame must be Some");
        let request_id = match request_frame.kind {
            FrameKind::Request(r) => r.id,
            other => panic!("expected request, got {other:?}"),
        };

        // Caller drops mid-flight.
        request_task.abort();
        let _ = request_task.await;

        let cancel_frame = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("cancel frame must arrive")
            .expect("frame must be Some");
        match cancel_frame.kind {
            FrameKind::Cancel(CancelFrame { id }) => assert_eq!(id, request_id),
            other => panic!("expected cancel frame, got {other:?}"),
        }
    }
}
