//! Eurora native messaging host. The binary speaks Chrome's native
//! messaging protocol on stdin/stdout (length-prefixed JSON) and
//! bridges to the desktop app over a WebSocket. Chrome launches one
//! copy per browser instance.

use std::process;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use backon::{ConstantBuilder, Retryable};
use euro_browser::utils::{read_framed, write_framed};
use euro_browser::{Frame, FrameKind, RegisterFrame, bridge_url, parent_pid};
use futures_util::{SinkExt, StreamExt};
use tokio::io;
use tokio::sync::{Mutex, broadcast, mpsc};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

/// Backoff between WebSocket reconnect attempts.
const RECONNECT_INTERVAL: Duration = Duration::from_secs(2);
/// Bound on the queue feeding stdout from the WebSocket.
const FROM_SERVER_QUEUE: usize = 1024;
/// Bound on the broadcast channel feeding the WebSocket from stdin.
/// Broadcast lets us redeliver frames after a reconnect without losing
/// them between consumer instances.
const TO_SERVER_QUEUE: usize = 1024;

/// Outcome of a single bridge WebSocket session. Determines whether the
/// outer reconnect loop sleeps and tries again, or breaks and lets the
/// process exit.
enum SessionOutcome {
    /// The session ended for a transient reason — desktop unreachable,
    /// network blip, server restart. Back off and try again.
    Reconnect,
    /// The desktop asked us to terminate (because it has just installed
    /// an updated messenger binary). Stop reconnecting; let the process
    /// exit so the browser respawns us from the new binary.
    Shutdown,
}

fn frame_summary(frame: &Frame) -> String {
    match &frame.kind {
        FrameKind::Request(r) => format!("Request(id={}, action={})", r.id, r.action),
        FrameKind::Response(r) => format!("Response(id={}, action={})", r.id, r.action),
        FrameKind::Event(e) => format!("Event(action={})", e.action),
        FrameKind::Error(e) => format!("Error(id={}, code={})", e.id, e.code),
        FrameKind::Cancel(c) => format!("Cancel(id={})", c.id),
        FrameKind::Register(r) => match &r.app_kind {
            Some(kind) => format!(
                "Register(host={}, app={}, kind={kind})",
                r.host_pid, r.app_pid
            ),
            None => format!("Register(host={}, app={})", r.host_pid, r.app_pid),
        },
        FrameKind::Shutdown(s) => match &s.reason {
            Some(reason) => format!("Shutdown(reason={reason})"),
            None => "Shutdown".to_string(),
        },
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    parent_pid::capture_parent_pid();

    let app_pid = parent_pid::get_parent_pid();
    let host_pid = process::id();

    tracing::info!("Starting native messaging host: host_pid={host_pid}, app_pid={app_pid}");

    let (from_server_tx, mut from_server_rx) = mpsc::channel::<Frame>(FROM_SERVER_QUEUE);
    let (to_server_tx, _) = broadcast::channel::<Frame>(TO_SERVER_QUEUE);

    let cached_assets: Arc<Mutex<Option<Frame>>> = Arc::new(Mutex::new(None));
    let cached_snapshot: Arc<Mutex<Option<Frame>>> = Arc::new(Mutex::new(None));

    let chrome_writer_handle = tokio::spawn(async move {
        let mut stdout = io::stdout();
        tracing::info!("Chrome writer task started");
        while let Some(frame) = from_server_rx.recv().await {
            tracing::debug!("Writing frame to Chrome: {}", frame_summary(&frame));
            if let Err(err) = write_framed(&mut stdout, &frame).await {
                tracing::error!("Native host write error: {err:?}");
                break;
            }
        }
        tracing::info!("Chrome writer task stopped");
    });

    let chrome_reader_handle = {
        let to_server_tx = to_server_tx.clone();
        let cached_assets = Arc::clone(&cached_assets);
        let cached_snapshot = Arc::clone(&cached_snapshot);
        tokio::spawn(async move {
            let mut stdin = io::stdin();
            tracing::info!("Chrome reader task started");
            loop {
                match read_framed(&mut stdin).await {
                    Ok(Some(frame)) => {
                        tracing::debug!("Read frame from Chrome: {}", frame_summary(&frame));
                        if let FrameKind::Event(ref e) = frame.kind {
                            match e.action.as_str() {
                                "ASSETS" => {
                                    *cached_assets.lock().await = Some(frame.clone());
                                }
                                "SNAPSHOT" => {
                                    *cached_snapshot.lock().await = Some(frame.clone());
                                }
                                _ => {}
                            }
                        }
                        let _ = to_server_tx.send(frame);
                    }
                    Ok(None) => {
                        tracing::info!("EOF from Chrome, connection closed");
                        break;
                    }
                    Err(e) => {
                        tracing::error!("Native host read error: {e:?}");
                        break;
                    }
                }
            }
            tracing::info!("Chrome reader task stopped");
        })
    };

    let bridge_handle = {
        let to_server_tx = to_server_tx.clone();
        let cached_assets = Arc::clone(&cached_assets);
        let cached_snapshot = Arc::clone(&cached_snapshot);
        tokio::spawn(async move {
            let url = bridge_url();
            loop {
                match run_bridge_session(
                    &url,
                    host_pid,
                    app_pid,
                    &from_server_tx,
                    &to_server_tx,
                    &cached_assets,
                    &cached_snapshot,
                )
                .await
                {
                    SessionOutcome::Reconnect => {
                        tracing::info!(
                            "Bridge connection lost; reconnecting in {}s",
                            RECONNECT_INTERVAL.as_secs()
                        );
                        tokio::time::sleep(RECONNECT_INTERVAL).await;
                    }
                    SessionOutcome::Shutdown => {
                        tracing::info!("Bridge requested shutdown; stopping reconnect loop");
                        break;
                    }
                }
            }
        })
    };

    tokio::select! {
        _ = chrome_writer_handle => tracing::info!("Chrome writer task ended"),
        _ = chrome_reader_handle => tracing::info!("Chrome reader task ended"),
        _ = bridge_handle => tracing::info!("Bridge task ended; exiting messenger"),
    }

    Ok(())
}

/// One round-trip of: connect -> register -> replay cached events ->
/// pump frames in both directions until either side closes. The returned
/// [`SessionOutcome`] tells the outer loop whether to reconnect or to
/// stop the loop entirely (the latter when the desktop sends a
/// [`FrameKind::Shutdown`]).
async fn run_bridge_session(
    url: &str,
    host_pid: u32,
    app_pid: u32,
    from_server_tx: &mpsc::Sender<Frame>,
    to_server_tx: &broadcast::Sender<Frame>,
    cached_assets: &Arc<Mutex<Option<Frame>>>,
    cached_snapshot: &Arc<Mutex<Option<Frame>>>,
) -> SessionOutcome {
    let socket =
        (|| async {
            let request = url.into_client_request().map_err(
                |err: tokio_tungstenite::tungstenite::Error| format!("invalid bridge URL: {err}"),
            )?;
            tokio_tungstenite::connect_async(request)
                .await
                .map(|(ws, _)| ws)
                .map_err(|err| err.to_string())
        })
        .retry(
            ConstantBuilder::default()
                .with_delay(RECONNECT_INTERVAL)
                .without_max_times(),
        )
        .sleep(tokio::time::sleep)
        .notify(|err, dur| {
            tracing::warn!("Failed to connect to bridge: {err}. Retrying in {dur:?}…");
        })
        .await
        .expect("infinite retry never returns Err");

    tracing::info!("Bridge WebSocket connected at {url}; registering");

    let (mut sink, mut stream) = socket.split();

    let register = Frame::from(RegisterFrame {
        host_pid,
        app_pid,
        app_kind: None,
    });
    if let Err(err) = send_frame(&mut sink, &register).await {
        tracing::error!("Failed to send Register frame: {err}");
        return SessionOutcome::Reconnect;
    }

    if let Some(assets) = cached_assets.lock().await.clone() {
        tracing::info!("Replaying cached ASSETS frame");
        if let Err(err) = send_frame(&mut sink, &assets).await {
            tracing::error!("Failed to replay cached ASSETS frame: {err}");
            return SessionOutcome::Reconnect;
        }
    }
    if let Some(snapshot) = cached_snapshot.lock().await.clone() {
        tracing::info!("Replaying cached SNAPSHOT frame");
        if let Err(err) = send_frame(&mut sink, &snapshot).await {
            tracing::error!("Failed to replay cached SNAPSHOT frame: {err}");
            return SessionOutcome::Reconnect;
        }
    }

    let mut to_server_rx = to_server_tx.subscribe();
    let mut outcome = SessionOutcome::Reconnect;

    loop {
        tokio::select! {
            outbound = to_server_rx.recv() => {
                match outbound {
                    Ok(frame) => {
                        tracing::debug!("Forwarding to bridge: {}", frame_summary(&frame));
                        if let Err(err) = send_frame(&mut sink, &frame).await {
                            tracing::warn!("Bridge send error: {err}");
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Outbound channel lagged by {n} frames");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::info!("Outbound channel closed");
                        break;
                    }
                }
            }
            inbound = stream.next() => {
                let Some(message) = inbound else {
                    tracing::info!("Bridge stream ended");
                    break;
                };
                let message = match message {
                    Ok(m) => m,
                    Err(err) => {
                        tracing::warn!("Bridge stream error: {err}");
                        break;
                    }
                };
                match message {
                    Message::Text(text) => {
                        match serde_json::from_str::<Frame>(text.as_str()) {
                            Ok(frame) => {
                                tracing::debug!("Received from bridge: {}", frame_summary(&frame));
                                if let FrameKind::Shutdown(ref s) = frame.kind {
                                    tracing::info!(
                                        reason = ?s.reason,
                                        "Received Shutdown from bridge; closing session",
                                    );
                                    outcome = SessionOutcome::Shutdown;
                                    break;
                                }
                                if from_server_tx.send(frame).await.is_err() {
                                    tracing::warn!("Chrome writer queue closed");
                                    break;
                                }
                            }
                            Err(err) => {
                                tracing::warn!("Bad JSON frame from bridge: {err}");
                            }
                        }
                    }
                    Message::Binary(_) => {
                        tracing::warn!("Ignoring unexpected binary frame from bridge");
                    }
                    Message::Ping(payload) => {
                        if let Err(err) = sink.send(Message::Pong(payload)).await {
                            tracing::warn!("Failed to send Pong: {err}");
                            break;
                        }
                    }
                    Message::Pong(_) => {}
                    Message::Frame(_) => {}
                    Message::Close(close) => {
                        tracing::info!("Bridge closed connection: {close:?}");
                        break;
                    }
                }
            }
        }
    }

    if matches!(outcome, SessionOutcome::Shutdown) {
        // Best-effort clean WebSocket close so the desktop logs a normal
        // disconnect instead of a TCP RST when the messenger process exits.
        let close = Message::Close(Some(CloseFrame {
            code: CloseCode::Normal,
            reason: "messenger exiting after Shutdown frame".into(),
        }));
        if let Err(err) = sink.send(close).await {
            tracing::debug!("Failed to send WebSocket Close on shutdown: {err}");
        }
    }

    outcome
}

async fn send_frame<S>(sink: &mut S, frame: &Frame) -> anyhow::Result<()>
where
    S: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    let json = serde_json::to_string(frame)?;
    sink.send(Message::Text(json.into())).await?;
    Ok(())
}
