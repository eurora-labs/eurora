//! Eurora native messaging host. The binary speaks Chrome's native
//! messaging protocol on stdin/stdout (length-prefixed JSON) and
//! bridges to the desktop app over a WebSocket. Chrome launches one
//! copy per browser instance.

use std::sync::Arc;
use std::time::Duration;
use std::{env, process};

use anyhow::Result;
use backon::{ConstantBuilder, Retryable};
use euro_native_messaging::utils::{generate_typescript_definitions, read_framed, write_framed};
use euro_native_messaging::{Frame, FrameKind, RegisterFrame, bridge_url, parent_pid};
use futures_util::{SinkExt, StreamExt};
use tokio::io;
use tokio::sync::{Mutex, broadcast, mpsc};
use tokio_tungstenite::tungstenite::Message;

/// Backoff between WebSocket reconnect attempts.
const RECONNECT_INTERVAL: Duration = Duration::from_secs(2);
/// Bound on the queue feeding stdout from the WebSocket.
const FROM_SERVER_QUEUE: usize = 1024;
/// Bound on the broadcast channel feeding the WebSocket from stdin.
/// Broadcast lets us redeliver frames after a reconnect without losing
/// them between consumer instances.
const TO_SERVER_QUEUE: usize = 1024;

fn frame_summary(frame: &Frame) -> String {
    match &frame.kind {
        FrameKind::Request(r) => format!("Request(id={}, action={})", r.id, r.action),
        FrameKind::Response(r) => format!("Response(id={}, action={})", r.id, r.action),
        FrameKind::Event(e) => format!("Event(action={})", e.action),
        FrameKind::Error(e) => format!("Error(id={}, code={})", e.id, e.code),
        FrameKind::Cancel(c) => format!("Cancel(id={})", c.id),
        FrameKind::Register(r) => {
            format!("Register(host={}, app={})", r.host_pid, r.app_pid)
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    parent_pid::capture_parent_pid();

    #[cfg(debug_assertions)]
    init_debug_logging();

    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--generate_specta") {
        return generate_typescript_definitions();
    }

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

    let _bridge_handle = {
        let to_server_tx = to_server_tx.clone();
        let cached_assets = Arc::clone(&cached_assets);
        let cached_snapshot = Arc::clone(&cached_snapshot);
        tokio::spawn(async move {
            let url = bridge_url();
            loop {
                run_bridge_session(
                    &url,
                    host_pid,
                    app_pid,
                    &from_server_tx,
                    &to_server_tx,
                    &cached_assets,
                    &cached_snapshot,
                )
                .await;
                tracing::info!(
                    "Bridge connection lost; reconnecting in {}s",
                    RECONNECT_INTERVAL.as_secs()
                );
                tokio::time::sleep(RECONNECT_INTERVAL).await;
            }
        })
    };

    tokio::select! {
        _ = chrome_writer_handle => tracing::info!("Chrome writer task ended"),
        _ = chrome_reader_handle => tracing::info!("Chrome reader task ended"),
    }

    Ok(())
}

/// One round-trip of: connect -> register -> replay cached events ->
/// pump frames in both directions until either side closes. Returns
/// when the WebSocket session ends; the caller backs off and reconnects.
async fn run_bridge_session(
    url: &str,
    host_pid: u32,
    app_pid: u32,
    from_server_tx: &mpsc::Sender<Frame>,
    to_server_tx: &broadcast::Sender<Frame>,
    cached_assets: &Arc<Mutex<Option<Frame>>>,
    cached_snapshot: &Arc<Mutex<Option<Frame>>>,
) {
    let socket = (|| async {
        tracing::info!("Connecting to bridge at {url}");
        tokio_tungstenite::connect_async(url)
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

    tracing::info!("Bridge WebSocket connected; registering");

    let (mut sink, mut stream) = socket.split();

    let register = Frame::from(RegisterFrame { host_pid, app_pid });
    if let Err(err) = send_frame(&mut sink, &register).await {
        tracing::error!("Failed to send Register frame: {err}");
        return;
    }

    if let Some(assets) = cached_assets.lock().await.clone() {
        tracing::info!("Replaying cached ASSETS frame");
        if let Err(err) = send_frame(&mut sink, &assets).await {
            tracing::error!("Failed to replay cached ASSETS frame: {err}");
            return;
        }
    }
    if let Some(snapshot) = cached_snapshot.lock().await.clone() {
        tracing::info!("Replaying cached SNAPSHOT frame");
        if let Err(err) = send_frame(&mut sink, &snapshot).await {
            tracing::error!("Failed to replay cached SNAPSHOT frame: {err}");
            return;
        }
    }

    let mut to_server_rx = to_server_tx.subscribe();

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
}

async fn send_frame<S>(sink: &mut S, frame: &Frame) -> anyhow::Result<()>
where
    S: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    let json = serde_json::to_string(frame)?;
    sink.send(Message::Text(json.into())).await?;
    Ok(())
}

#[cfg(debug_assertions)]
fn init_debug_logging() {
    use std::fs;
    use tracing_subscriber::prelude::*;

    let log_path = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("euro-native-messaging.log")))
        .unwrap_or_else(|| "/tmp/euro-native-messaging.log".into());
    let log_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .expect("failed to open log file");
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::sync::Mutex::new(log_file))
                .with_ansi(false),
        )
        .init();
}
