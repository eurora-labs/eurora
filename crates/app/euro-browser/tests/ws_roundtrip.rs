//! End-to-end test of the bridge WebSocket server with a real
//! `tokio-tungstenite` client over rustls. Uses the production server
//! entrypoint so the upgrade path, registration, dispatch, and
//! shutdown are exercised together.
//!
//! Each test binds to an ephemeral port via `BridgeService::bind_on`,
//! spawns the accept loop, and generates a fresh CA + `localhost` leaf
//! into a tempdir, so the suite can run alongside (or in parallel
//! with) anything that holds the well-known bridge port.

mod common;

use std::net::SocketAddr;
use std::time::Duration;

use euro_browser::{
    BridgeError, BridgeService, EventFrame, Frame, FrameKind, RegisterFrame, ResponseFrame,
    bridge_url_for, install_default_crypto_provider,
};
use futures_util::{SinkExt, StreamExt};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::Message as TMessage;

/// Bind the bridge to an ephemeral loopback port for the duration of a
/// single test and spawn its accept loop. Returns the live service, the
/// bound socket address, and the test trust chain (kept alive on the
/// stack so cert files survive).
async fn start_ephemeral_bridge() -> (BridgeService, SocketAddr, common::TestChain) {
    install_default_crypto_provider();
    let chain = common::mint_localhost_chain();
    let service = BridgeService::new();
    service.configure_tls(chain.material.clone());
    let bound = service
        .bind_on(([127, 0, 0, 1], 0).into())
        .await
        .expect("bind ephemeral bridge");
    let addr = bound.local_addr();
    tokio::spawn(async move {
        if let Err(err) = bound.serve().await {
            eprintln!("ephemeral bridge serve loop ended: {err}");
        }
    });
    (service, addr, chain)
}

/// Connect a tungstenite client to the ephemeral bridge using the test
/// CA. Returns the open WebSocket stream.
async fn connect_test_client(
    addr: SocketAddr,
    chain: &common::TestChain,
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    let url = bridge_url_for(addr);
    let request = url.into_client_request().expect("build request");
    let connector = common::client_connector(&chain.ca_pem);
    let (ws, _resp) =
        tokio_tungstenite::connect_async_tls_with_config(request, None, false, Some(connector))
            .await
            .expect("connect over rustls");
    ws
}

#[tokio::test]
async fn round_trip_request_response() {
    let (service, addr, chain) = start_ephemeral_bridge().await;

    let mut ws = connect_test_client(addr, &chain).await;

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
    let (service, addr, chain) = start_ephemeral_bridge().await;

    let mut ws = connect_test_client(addr, &chain).await;

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
    install_default_crypto_provider();
    let chain = common::mint_localhost_chain();
    let service = BridgeService::new();
    service.configure_tls(chain.material.clone());

    let bound = service
        .bind_on(([127, 0, 0, 1], 0).into())
        .await
        .expect("first bind");
    let first_addr = bound.local_addr();
    let serve_handle = tokio::spawn({
        let bound = bound;
        async move { bound.serve().await }
    });
    assert_eq!(service.local_addr(), Some(first_addr));

    service.stop_server().await;
    serve_handle
        .await
        .expect("serve task")
        .expect("clean shutdown");
    assert_eq!(service.local_addr(), None);

    // After a clean stop the listener must be free, and a second bind
    // must succeed without leaking the previous shutdown signal. We
    // deliberately request a fresh ephemeral port — the OS may or may
    // not hand back the same one, and the test should not depend on it.
    let bound = service
        .bind_on(([127, 0, 0, 1], 0).into())
        .await
        .expect("second bind");
    let second_addr = bound.local_addr();
    let serve_handle = tokio::spawn(async move { bound.serve().await });

    let mut ws = connect_test_client(second_addr, &chain).await;
    ws.close(None).await.unwrap();

    service.stop_server().await;
    serve_handle
        .await
        .expect("serve task")
        .expect("clean shutdown");
}

/// Calling `bind_on` while a serve loop is already registered must
/// surface the conflict explicitly so callers can decide whether to
/// reuse the running listener or treat it as an error. Silent no-op
/// behaviour was the symptom that motivated the bind/serve split.
#[tokio::test]
async fn bind_on_rejects_double_bind() {
    install_default_crypto_provider();
    let chain = common::mint_localhost_chain();
    let service = BridgeService::new();
    service.configure_tls(chain.material);

    let bound = service
        .bind_on(([127, 0, 0, 1], 0).into())
        .await
        .expect("first bind");
    let addr = bound.local_addr();
    let serve_handle = tokio::spawn(async move { bound.serve().await });

    let err = service
        .bind_on(([127, 0, 0, 1], 0).into())
        .await
        .expect_err("second bind must be rejected");
    match err {
        BridgeError::AlreadyRunning { local_addr } => assert_eq!(local_addr, addr),
        other => panic!("expected AlreadyRunning, got {other:?}"),
    }

    service.stop_server().await;
    serve_handle
        .await
        .expect("serve task")
        .expect("clean shutdown");
}

/// Regression: the kernel socket must already be in `LISTEN` state by
/// the time `bind_on` returns. A `TcpStream::connect` against the bound
/// address must succeed even before the `serve()` future has been
/// polled — this is the property the bind/serve split exists to
/// guarantee, and it's what eliminates the slow-first-connect race the
/// add-in used to hit.
#[tokio::test]
async fn bind_on_returns_with_port_in_listen_state() {
    install_default_crypto_provider();
    let chain = common::mint_localhost_chain();
    let service = BridgeService::new();
    service.configure_tls(chain.material);

    let bound = service
        .bind_on(([127, 0, 0, 1], 0).into())
        .await
        .expect("bind");
    let addr = bound.local_addr();

    // No serve() yet. The TCP connection itself must still complete —
    // we don't try to do a TLS handshake because nothing is accepting
    // application data, just observe that the SYN gets ACK'd.
    let stream = tokio::task::spawn_blocking(move || std::net::TcpStream::connect(addr))
        .await
        .expect("join");
    stream.expect("port is in LISTEN before serve() is polled");

    drop(bound);
}
