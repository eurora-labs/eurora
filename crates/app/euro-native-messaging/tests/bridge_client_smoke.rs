//! End-to-end smoke test: spin up the real `euro-bridge` WebSocket server,
//! point a `BridgeClient` at it, register, exchange a request/response, and
//! verify the round-trip over the JSON wire.

use std::time::Duration;

use euro_bridge::AppBridgeService;
use euro_bridge_protocol::{ClientKind, FrameKind, RegisterFrame, ResponseFrame};
use euro_native_messaging::BridgeClient;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

#[tokio::test]
async fn bridge_client_register_and_request_roundtrip() {
    // Fresh, isolated bridge service.
    let service: &'static AppBridgeService = Box::leak(Box::new(AppBridgeService::new()));
    service.start_router();

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        let shutdown = async move {
            let _ = shutdown_rx.await;
        };
        euro_bridge::serve_ws(listener, service, shutdown)
            .await
            .expect("serve_ws");
    });

    // Client connects and registers.
    let url = format!("ws://{addr}/");
    let mut client = BridgeClient::connect(&url).await.expect("connect");
    client
        .register(RegisterFrame {
            host_pid: 9001,
            app_pid: 9002,
            client_kind: ClientKind::Browser,
        })
        .await
        .expect("register");

    let (mut reader, mut writer) = client.split();

    // Wait for the registry entry to land.
    for _ in 0..50 {
        if service.is_registered(9002).await {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(service.is_registered(9002).await);

    // Desktop initiates a request.
    let request_task = tokio::spawn(async move {
        service
            .send_request_with_timeout(
                9002,
                "GET_METADATA",
                Some("ping".into()),
                Duration::from_secs(2),
            )
            .await
    });

    // Client receives the request frame and replies.
    let request_frame = tokio::time::timeout(Duration::from_secs(2), reader.next_frame())
        .await
        .expect("timeout waiting for request")
        .expect("read error")
        .expect("stream ended");
    let request = match request_frame.kind {
        FrameKind::Request(r) => r,
        other => panic!("expected request, got {other:?}"),
    };
    assert_eq!(request.action, "GET_METADATA");
    assert_eq!(request.payload.as_deref(), Some("ping"));

    writer
        .send_frame(
            &ResponseFrame {
                id: request.id,
                action: request.action.clone(),
                payload: Some("pong".into()),
            }
            .into(),
        )
        .await
        .expect("send response");

    let response = request_task
        .await
        .expect("task did not panic")
        .expect("send_request");
    assert_eq!(response.payload.as_deref(), Some("pong"));

    // Tear down.
    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(2), server).await;
}
