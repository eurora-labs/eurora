//! End-to-end smoke test: a real WebSocket client (`tokio-tungstenite`)
//! registers as `Office`, the desktop sends a `RequestFrame`, the test
//! client replies with a `ResponseFrame`, and we verify the round-trip over
//! the JSON wire format.

use std::time::Duration;

use euro_bridge::{AppBridgeService, ClientKind, Frame, FrameKind, RegisterFrame, ResponseFrame};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_tungstenite::tungstenite::Message as WsMessage;

#[tokio::test]
async fn websocket_register_and_request_roundtrip() {
    // Fresh, isolated service for this test (the global one is reused
    // across the rest of the process).
    let service: &'static AppBridgeService = Box::leak(Box::new(AppBridgeService::new()));
    service.start_router();

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let server_handle = tokio::spawn(async move {
        let shutdown = async move {
            let _ = shutdown_rx.await;
        };
        euro_bridge::serve_ws(listener, service, shutdown)
            .await
            .expect("serve_ws");
    });

    let url = format!("ws://{addr}/");
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(url)
        .await
        .expect("connect");

    // Send the RegisterFrame.
    let register = Frame::from(RegisterFrame {
        host_pid: 4242,
        app_pid: 4243,
        client_kind: ClientKind::Office,
    });
    ws_stream
        .send(WsMessage::Text(
            serde_json::to_string(&register)
                .expect("encode register")
                .into(),
        ))
        .await
        .expect("send register");

    // Wait for the registry entry to land.
    for _ in 0..50 {
        if service.is_registered(4243).await {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        service.is_registered(4243).await,
        "client should be registered after RegisterFrame"
    );

    // Desktop initiates a request.
    let request_task = tokio::spawn({
        async move {
            service
                .send_request_with_timeout(4243, "GET_DOCUMENT", None, Duration::from_secs(2))
                .await
        }
    });

    // Pull the request off the WebSocket and reply with a ResponseFrame.
    let request_msg = tokio::time::timeout(Duration::from_secs(2), ws_stream.next())
        .await
        .expect("timeout waiting for request")
        .expect("stream ended")
        .expect("ws error");
    let text = match request_msg {
        WsMessage::Text(text) => text,
        other => panic!("expected text, got {other:?}"),
    };
    let frame: Frame = serde_json::from_str(text.as_str()).expect("decode");
    let request = match frame.kind {
        FrameKind::Request(r) => r,
        other => panic!("expected request, got {other:?}"),
    };
    assert_eq!(request.action, "GET_DOCUMENT");

    let response = Frame::from(ResponseFrame {
        id: request.id,
        action: request.action.clone(),
        payload: Some("doc-payload".into()),
    });
    ws_stream
        .send(WsMessage::Text(
            serde_json::to_string(&response)
                .expect("encode response")
                .into(),
        ))
        .await
        .expect("send response");

    // The desktop-side request should resolve with our payload.
    let outcome = request_task
        .await
        .expect("task did not panic")
        .expect("send_request ok");
    assert_eq!(outcome.payload.as_deref(), Some("doc-payload"));

    // Tear down.
    let _ = shutdown_tx.send(());
    let _ = ws_stream.close(None).await;
    let _ = tokio::time::timeout(Duration::from_secs(2), server_handle).await;
}
