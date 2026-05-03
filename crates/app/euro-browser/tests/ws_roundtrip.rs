//! End-to-end test of the bridge WebSocket server with a real
//! `tokio-tungstenite` client. Uses the production server entrypoint so
//! the upgrade path, registration, dispatch, and shutdown are
//! exercised together.
//!
//! Each test binds to an ephemeral port via `start_server_on` so the
//! suite can run alongside (or in parallel with) anything that holds
//! the well-known bridge port.

use std::net::SocketAddr;
use std::time::Duration;

use euro_browser::{
    BridgeError, BridgeService, EventFrame, Frame, FrameKind, RegisterFrame, ResponseFrame,
    bridge_url_for,
};
use futures_util::{SinkExt, StreamExt};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::protocol::Message as TMessage;

/// Bind the bridge to an ephemeral loopback port for the duration of a
/// single test.
async fn start_ephemeral_bridge() -> (BridgeService, SocketAddr) {
    let service = BridgeService::new();
    let addr = service
        .start_server_on(([127, 0, 0, 1], 0).into())
        .await
        .expect("bind ephemeral bridge");
    (service, addr)
}

#[tokio::test]
async fn round_trip_request_response() {
    let (service, addr) = start_ephemeral_bridge().await;

    let url = bridge_url_for(addr);
    let (mut ws, _resp) = tokio_tungstenite::connect_async(&url).await.unwrap();

    let host_pid = 9_999_999;
    let app_pid = std::process::id();

    let register = serde_json::to_string(&Frame::from(RegisterFrame {
        host_pid,
        app_pid,
        app_kind: None,
    }))
    .unwrap();
    ws.send(TMessage::text(register)).await.unwrap();

    let mut registrations = service.subscribe_to_registrations();
    let event = timeout(Duration::from_secs(2), registrations.recv())
        .await
        .expect("registration not received in time")
        .expect("registration channel closed");
    assert_eq!(event.app_pid, app_pid);
    assert!(event.app_kind.is_none());
    assert_eq!(service.connection_count(), 1);

    let svc = service.clone();
    let request_handle = tokio::spawn(async move {
        svc.send_request(app_pid, "PING", Some("hi".into()))
            .await
            .unwrap()
    });

    let next = timeout(Duration::from_secs(2), ws.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let TMessage::Text(text) = next else {
        panic!("expected text frame, got {next:?}");
    };
    let frame: Frame = serde_json::from_str(text.as_str()).unwrap();
    let FrameKind::Request(req) = frame.kind else {
        panic!("expected Request frame");
    };
    assert_eq!(req.action, "PING");
    assert_eq!(req.payload.as_deref(), Some("hi"));

    let reply = serde_json::to_string(&Frame::from(ResponseFrame {
        id: req.id,
        action: req.action.clone(),
        payload: Some("pong".into()),
    }))
    .unwrap();
    ws.send(TMessage::text(reply)).await.unwrap();

    let response = timeout(Duration::from_secs(2), request_handle)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(response.payload.as_deref(), Some("pong"));

    let mut events = service.subscribe_to_events();
    let event_payload = serde_json::to_string(&Frame::from(EventFrame {
        action: "TAB_ACTIVATED".into(),
        payload: Some("{}".into()),
    }))
    .unwrap();
    ws.send(TMessage::text(event_payload)).await.unwrap();

    let (event_pid, event_frame) = timeout(Duration::from_secs(2), events.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(event_pid, app_pid);
    assert_eq!(event_frame.action, "TAB_ACTIVATED");

    let mut disconnects = service.subscribe_to_disconnects();
    ws.close(None).await.unwrap();
    drop(ws);

    let disc = timeout(Duration::from_secs(2), disconnects.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(disc.app_pid, app_pid);

    service.stop_server().await;
}

#[tokio::test]
async fn send_request_to_unregistered_app_returns_not_found() {
    let service = BridgeService::new();
    let result = service.send_request(0, "GET_METADATA", None).await;
    assert!(matches!(result, Err(BridgeError::NotFound { app_pid: 0 })));
}

/// Sandboxed clients (Word add-in, future Office integrations) register
/// with a logical `app_kind` instead of a real OS PID. The desktop must
/// surface the kind on the registration event, persist it on the
/// registry, and expose it via `find_clients_by_kind`.
#[tokio::test]
async fn register_with_app_kind_is_discoverable() {
    let (service, addr) = start_ephemeral_bridge().await;

    let url = bridge_url_for(addr);
    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

    let host_pid = 0;
    let app_pid = 0xC0FFEE;
    let kind = "microsoft-word";

    let register = serde_json::to_string(&Frame::from(RegisterFrame {
        host_pid,
        app_pid,
        app_kind: Some(kind.into()),
    }))
    .unwrap();

    let mut registrations = service.subscribe_to_registrations();
    ws.send(TMessage::text(register)).await.unwrap();

    let event = timeout(Duration::from_secs(2), registrations.recv())
        .await
        .expect("registration not received in time")
        .expect("registration channel closed");
    assert_eq!(event.app_pid, app_pid);
    assert_eq!(event.app_kind.as_deref(), Some(kind));

    let pids = service.find_clients_by_kind(kind);
    assert_eq!(pids, vec![app_pid]);

    let mut disconnects = service.subscribe_to_disconnects();
    ws.close(None).await.unwrap();
    drop(ws);

    let disc = timeout(Duration::from_secs(2), disconnects.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(disc.app_pid, app_pid);
    assert_eq!(disc.app_kind.as_deref(), Some(kind));
    assert!(service.find_clients_by_kind(kind).is_empty());

    service.stop_server().await;
}

#[tokio::test]
async fn server_can_be_stopped_and_restarted() {
    let service = BridgeService::new();

    let first_addr = service
        .start_server_on(([127, 0, 0, 1], 0).into())
        .await
        .expect("first bind");
    assert_eq!(service.local_addr().await, Some(first_addr));
    service.stop_server().await;
    assert_eq!(service.local_addr().await, None);

    // After a clean stop the listener must be free, and a second
    // start_server must succeed without leaking the previous shutdown
    // signal. We deliberately request a fresh ephemeral port — the OS
    // may or may not hand back the same one, and the test should not
    // depend on it.
    let second_addr = service
        .start_server_on(([127, 0, 0, 1], 0).into())
        .await
        .expect("second bind");

    let url = bridge_url_for(second_addr);
    let (mut ws, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("connect after restart");
    ws.close(None).await.unwrap();

    service.stop_server().await;
}

/// Re-binding while already running must be a no-op that surfaces the
/// existing local address, not a spurious bind error.
#[tokio::test]
async fn start_server_on_is_idempotent() {
    let service = BridgeService::new();
    let first = service
        .start_server_on(([127, 0, 0, 1], 0).into())
        .await
        .expect("first bind");

    // Asking for a different port while running must still return the
    // currently-bound address rather than rebinding.
    let second = service
        .start_server_on(([127, 0, 0, 1], 0).into())
        .await
        .expect("second call returns running addr");
    assert_eq!(first, second);

    service.stop_server().await;
}
