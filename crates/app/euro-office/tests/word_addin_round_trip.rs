//! End-to-end test for the desktop⇄Word-add-in bridge contract.
//!
//! Exercises the full path that `WordStrategy` relies on in production:
//! the add-in registers as a non-PID client with
//! `app_kind = Some("microsoft-word")`, the desktop locates it via
//! [`BridgeService::find_clients_by_kind`], issues `GET_ASSETS`, and
//! deserializes the response payload directly as [`WordDocumentAsset`]
//! — no `NativeMessage` envelope.
//!
//! Each test binds the bridge to an ephemeral loopback port via
//! `BridgeService::bind_on(([127, 0, 0, 1], 0).into())` over plaintext
//! WS, so the suite never collides with a locally-running desktop.

mod common;

use std::net::SocketAddr;
use std::time::Duration;

use euro_bridge::{
    BridgeService, Frame, FrameKind, Payload, RegisterFrame, ResponseFrame, bridge_url_for,
};
use euro_office::{ACTION_GET_ASSETS, MICROSOFT_WORD_KIND, WordDocumentAsset, fetch_word_asset};
use futures_util::{SinkExt, StreamExt};
use tokio::time::timeout;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::protocol::Message as TMessage;

type ClientWs = WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

const WAIT: Duration = Duration::from_secs(2);

/// Connect a tungstenite client over plaintext WS, register as the
/// Word add-in with `app_pid`, and wait for the bridge to surface the
/// registration.
async fn connect_word_addin(service: &BridgeService, addr: SocketAddr, app_pid: u32) -> ClientWs {
    let mut registrations = service.subscribe_to_registrations();

    let url = bridge_url_for(addr);
    let (mut ws, _) = tokio_tungstenite::connect_async(url)
        .await
        .expect("connect to bridge over plaintext ws");

    let register = serde_json::to_string(&Frame::from(RegisterFrame {
        host_pid: 0,
        app_pid,
        app_kind: Some(MICROSOFT_WORD_KIND.into()),
    }))
    .unwrap();
    ws.send(TMessage::text(register)).await.unwrap();

    let event = timeout(WAIT, registrations.recv())
        .await
        .expect("registration not received in time")
        .expect("registration channel closed");
    assert_eq!(event.app_pid, app_pid);
    assert_eq!(event.app_kind.as_deref(), Some(MICROSOFT_WORD_KIND));

    ws
}

/// Read the next text frame off the WebSocket and decode it as a
/// `Frame`. Panics on timeout, transport error, or non-text payload —
/// every test relies on this happening promptly.
async fn next_frame(ws: &mut ClientWs) -> Frame {
    let message = timeout(WAIT, ws.next())
        .await
        .expect("frame not received in time")
        .expect("websocket closed unexpectedly")
        .expect("websocket transport error");
    let TMessage::Text(text) = message else {
        panic!("expected text frame, got {message:?}");
    };
    serde_json::from_str(text.as_str()).expect("decode frame")
}

#[tokio::test]
async fn fetch_word_asset_round_trips_through_a_real_websocket() {
    let (service, addr, serve_handle) = common::start_ephemeral_bridge().await;
    let app_pid = 0xC0FFEE;
    let mut ws = connect_word_addin(&service, addr, app_pid).await;

    let asset = WordDocumentAsset {
        document_name: "Quarterly Report.docx".into(),
        text: "All systems nominal.".into(),
    };

    // Spawn the mock add-in: receive one Request, reply with the
    // asset payload, then yield the socket back to the main task.
    let mock_asset = asset.clone();
    let mock = tokio::spawn(async move {
        let frame = next_frame(&mut ws).await;
        let FrameKind::Request(req) = frame.kind else {
            panic!("expected Request, got {:?}", frame.kind);
        };
        assert_eq!(req.action, ACTION_GET_ASSETS);
        assert!(
            req.payload.is_none(),
            "GET_ASSETS request should carry no payload, got {:?}",
            req.payload
        );

        let payload = Payload::from_value(&mock_asset).unwrap();
        let reply = serde_json::to_string(&Frame::from(ResponseFrame {
            id: req.id,
            action: req.action,
            payload: Some(payload),
        }))
        .unwrap();
        ws.send(TMessage::text(reply)).await.unwrap();
        ws
    });

    let fetched = timeout(WAIT, fetch_word_asset(&service))
        .await
        .expect("fetch_word_asset hung")
        .expect("expected Some(asset) when add-in is connected and responds");
    assert_eq!(fetched, asset);

    let mut ws = mock.await.expect("mock task panicked");
    ws.close(None).await.unwrap();
    service.stop_server().await;
    serve_handle
        .await
        .expect("serve task")
        .expect("clean shutdown");
}

#[tokio::test]
async fn fetch_word_asset_returns_none_when_no_addin_is_connected() {
    let (service, _addr, serve_handle) = common::start_ephemeral_bridge().await;
    assert!(fetch_word_asset(&service).await.is_none());
    service.stop_server().await;
    serve_handle
        .await
        .expect("serve task")
        .expect("clean shutdown");
}

/// A connected add-in that returns garbage in `ResponseFrame.payload`
/// must not crash the strategy — `fetch_word_asset` swallows the error
/// and returns `None` so the next collection tick can try again.
#[tokio::test]
async fn fetch_word_asset_returns_none_when_payload_is_malformed() {
    let (service, addr, serve_handle) = common::start_ephemeral_bridge().await;
    let app_pid = 0xBADF00D;
    let mut ws = connect_word_addin(&service, addr, app_pid).await;

    let mock = tokio::spawn(async move {
        let frame = next_frame(&mut ws).await;
        let FrameKind::Request(req) = frame.kind else {
            panic!("expected Request, got {:?}", frame.kind);
        };
        // Structurally valid JSON, but not a `WordDocumentAsset` — the
        // strategy must swallow the decode error and yield `None`.
        let reply = serde_json::to_string(&Frame::from(ResponseFrame {
            id: req.id,
            action: req.action,
            payload: Some(Payload::from_value(&"not the right shape").unwrap()),
        }))
        .unwrap();
        ws.send(TMessage::text(reply)).await.unwrap();
        ws
    });

    let fetched = timeout(WAIT, fetch_word_asset(&service))
        .await
        .expect("fetch_word_asset hung");
    assert!(fetched.is_none(), "malformed payload must yield None");

    let mut ws = mock.await.expect("mock task panicked");
    ws.close(None).await.unwrap();
    service.stop_server().await;
    serve_handle
        .await
        .expect("serve task")
        .expect("clean shutdown");
}

/// `fetch_word_asset` picks the first registered Word client. With two
/// add-ins connected, only one gets the request — the other sees no
/// traffic. This pins the "first-client policy" documented on the
/// public API and keeps the test honest about the multi-document
/// limitation.
#[tokio::test]
async fn fetch_word_asset_picks_first_registered_client() {
    let (service, addr, serve_handle) = common::start_ephemeral_bridge().await;

    let mut ws_a = connect_word_addin(&service, addr, 1001).await;
    let mut ws_b = connect_word_addin(&service, addr, 1002).await;

    let asset = WordDocumentAsset {
        document_name: "A.docx".into(),
        text: "from client A".into(),
    };

    // Both clients race to answer; whichever the bridge addresses
    // wins. We pin the assertion on "exactly one of them sees the
    // request" rather than on identity, because `find_clients_by_kind`
    // doesn't promise an ordering.
    let asset_a = asset.clone();
    let asset_b = asset.clone();
    let task_a = tokio::spawn(async move { try_answer_one_request(&mut ws_a, asset_a).await });
    let task_b = tokio::spawn(async move { try_answer_one_request(&mut ws_b, asset_b).await });

    let fetched = timeout(WAIT, fetch_word_asset(&service))
        .await
        .expect("fetch_word_asset hung")
        .expect("one of the two add-ins must answer");
    assert_eq!(fetched, asset);

    let answered_a = timeout(Duration::from_millis(500), task_a)
        .await
        .ok()
        .and_then(|r| r.ok())
        .unwrap_or(false);
    let answered_b = timeout(Duration::from_millis(500), task_b)
        .await
        .ok()
        .and_then(|r| r.ok())
        .unwrap_or(false);
    assert!(
        answered_a ^ answered_b,
        "exactly one client should see the request (a={answered_a}, b={answered_b})",
    );

    service.stop_server().await;
    serve_handle
        .await
        .expect("serve task")
        .expect("clean shutdown");
}

/// Wait briefly for one inbound request and reply with `asset`.
/// Returns `true` if a request was answered, `false` if no request
/// arrived before the deadline. Either outcome is valid — the caller
/// asserts on the exclusive-or of two such tasks.
async fn try_answer_one_request(ws: &mut ClientWs, asset: WordDocumentAsset) -> bool {
    let Ok(maybe_message) = timeout(Duration::from_millis(300), ws.next()).await else {
        return false;
    };
    let Some(Ok(TMessage::Text(text))) = maybe_message else {
        return false;
    };
    let frame: Frame = match serde_json::from_str(text.as_str()) {
        Ok(frame) => frame,
        Err(_) => return false,
    };
    let FrameKind::Request(req) = frame.kind else {
        return false;
    };
    let payload = Payload::from_value(&asset).unwrap();
    let reply = serde_json::to_string(&Frame::from(ResponseFrame {
        id: req.id,
        action: req.action,
        payload: Some(payload),
    }))
    .unwrap();
    ws.send(TMessage::text(reply)).await.is_ok()
}
