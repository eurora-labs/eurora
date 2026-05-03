//! End-to-end test of the bridge WebSocket server with a real
//! `tokio-tungstenite` client. Uses the production server entrypoint so
//! the upgrade path, registration, dispatch, and shutdown are
//! exercised together.

use std::time::Duration;

use euro_browser::{
    BridgeError, BridgeService, EventFrame, Frame, FrameKind, RegisterFrame, ResponseFrame,
};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::protocol::Message as TMessage;

#[tokio::test]
async fn round_trip_request_response() {
    // The bridge always binds the well-known port; skip if something
    // else is already on it (e.g. a desktop app running locally).
    if TcpListener::bind(("127.0.0.1", euro_browser::BRIDGE_PORT))
        .await
        .is_err()
    {
        eprintln!("skipping: bridge port already bound");
        return;
    }

    let service = BridgeService::new();
    service.start_server().await.expect("bind bridge");

    let url = euro_browser::bridge_url();
    let (mut ws, _resp) = tokio_tungstenite::connect_async(&url).await.unwrap();

    let host_pid = 9_999_999;
    let app_pid = std::process::id();

    let register =
        serde_json::to_string(&Frame::from(RegisterFrame { host_pid, app_pid })).unwrap();
    ws.send(TMessage::text(register)).await.unwrap();

    let mut registrations = service.subscribe_to_registrations();
    let event = timeout(Duration::from_secs(2), registrations.recv())
        .await
        .expect("registration not received in time")
        .expect("registration channel closed");
    assert_eq!(event.app_pid, app_pid);
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

#[tokio::test]
async fn server_can_be_stopped_and_restarted() {
    if TcpListener::bind(("127.0.0.1", euro_browser::BRIDGE_PORT))
        .await
        .is_err()
    {
        eprintln!("skipping: bridge port already bound");
        return;
    }

    let service = BridgeService::new();

    service.start_server().await.expect("first bind");
    service.stop_server().await;

    // After a clean stop the listener must be free, and a second
    // start_server must succeed without leaking the previous shutdown
    // signal.
    service.start_server().await.expect("second bind");

    let url = euro_browser::bridge_url();
    let (mut ws, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("connect after restart");
    ws.close(None).await.unwrap();

    service.stop_server().await;
}
