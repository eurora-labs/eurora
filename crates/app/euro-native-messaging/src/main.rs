use std::{env, sync::Arc, time::Duration};

use anyhow::Result;
use backon::{ConstantBuilder, Retryable};
use euro_bridge_protocol::{ClientKind, Frame, FrameKind, RegisterFrame};
use euro_native_messaging::{
    BRIDGE_URL, BridgeClient, BridgeWriter, parent_pid,
    utils::{generate_typescript_definitions, read_framed, write_framed},
};
use tokio::io;
use tokio::sync::{Mutex, broadcast, mpsc};

/// Time to wait between failed connection attempts and after a clean
/// disconnect before reconnecting.
const RETRY_INTERVAL: Duration = Duration::from_secs(2);

/// Capacity of the channel from the server-connection task to the Chrome
/// writer task. Sized for short hiccups; backpressure from a slow Chrome
/// reader is preferable to dropping frames.
const FROM_SERVER_CAPACITY: usize = 1024;

/// Capacity of the broadcast that fans Chrome-originated frames out to the
/// server-connection task. Lagging subscribers skip ahead rather than
/// stalling, which is fine for our use case (a stale snapshot is no worse
/// than no snapshot).
const TO_SERVER_CAPACITY: usize = 1024;

#[tokio::main]
async fn main() -> Result<()> {
    parent_pid::capture_parent_pid();

    #[cfg(debug_assertions)]
    init_debug_logging();

    if matches!(env::args().nth(1).as_deref(), Some("--generate_specta")) {
        return generate_typescript_definitions();
    }

    let app_pid = parent_pid::get_parent_pid();
    let host_pid = std::process::id();

    tracing::info!("Starting native messaging client: host_pid={host_pid}, app_pid={app_pid}");

    let (from_server_tx, from_server_rx) = mpsc::channel::<Frame>(FROM_SERVER_CAPACITY);
    let (to_server_tx, _) = broadcast::channel::<Frame>(TO_SERVER_CAPACITY);

    // Cached frames that get replayed to the server on every (re)connect so
    // the desktop immediately sees the latest known browser state.
    let cached_assets: Arc<Mutex<Option<Frame>>> = Arc::new(Mutex::new(None));
    let cached_snapshot: Arc<Mutex<Option<Frame>>> = Arc::new(Mutex::new(None));

    let chrome_writer = tokio::spawn(chrome_writer_task(from_server_rx));
    let chrome_reader = tokio::spawn(chrome_reader_task(
        to_server_tx.clone(),
        Arc::clone(&cached_assets),
        Arc::clone(&cached_snapshot),
    ));
    let _server = tokio::spawn(server_connection_task(
        host_pid,
        app_pid,
        from_server_tx,
        to_server_tx,
        cached_assets,
        cached_snapshot,
    ));

    // Either side terminating means the bridge is no longer useful; the
    // server task is implicitly torn down when the runtime shuts down.
    tokio::select! {
        _ = chrome_writer => tracing::info!("Chrome writer task stopped"),
        _ = chrome_reader => tracing::info!("Chrome reader task stopped"),
    }

    Ok(())
}

async fn chrome_writer_task(mut from_server_rx: mpsc::Receiver<Frame>) {
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
}

async fn chrome_reader_task(
    to_server_tx: broadcast::Sender<Frame>,
    cached_assets: Arc<Mutex<Option<Frame>>>,
    cached_snapshot: Arc<Mutex<Option<Frame>>>,
) {
    let mut stdin = io::stdin();
    tracing::info!("Chrome reader task started");
    loop {
        match read_framed(&mut stdin).await {
            Ok(Some(frame)) => {
                tracing::debug!("Read frame from Chrome: {}", frame_summary(&frame));
                cache_if_replayable(&frame, &cached_assets, &cached_snapshot).await;
                let _ = to_server_tx.send(frame);
            }
            Ok(None) => {
                tracing::info!("EOF from Chrome, connection closed");
                break;
            }
            Err(err) => {
                tracing::error!("Native host read error: {err:?}");
                break;
            }
        }
    }
    tracing::info!("Chrome reader task stopped");
}

async fn cache_if_replayable(
    frame: &Frame,
    cached_assets: &Mutex<Option<Frame>>,
    cached_snapshot: &Mutex<Option<Frame>>,
) {
    let FrameKind::Event(event) = &frame.kind else {
        return;
    };
    match event.action.as_str() {
        "ASSETS" => *cached_assets.lock().await = Some(frame.clone()),
        "SNAPSHOT" => *cached_snapshot.lock().await = Some(frame.clone()),
        _ => {}
    }
}

async fn server_connection_task(
    host_pid: u32,
    app_pid: u32,
    from_server_tx: mpsc::Sender<Frame>,
    to_server_tx: broadcast::Sender<Frame>,
    cached_assets: Arc<Mutex<Option<Frame>>>,
    cached_snapshot: Arc<Mutex<Option<Frame>>>,
) {
    loop {
        if let Err(err) = run_one_connection(
            host_pid,
            app_pid,
            &from_server_tx,
            &to_server_tx,
            &cached_assets,
            &cached_snapshot,
        )
        .await
        {
            tracing::warn!("Server connection ended: {err:#}");
        }

        tracing::info!("Reconnecting in {RETRY_INTERVAL:?}");
        tokio::time::sleep(RETRY_INTERVAL).await;
    }
}

async fn run_one_connection(
    host_pid: u32,
    app_pid: u32,
    from_server_tx: &mpsc::Sender<Frame>,
    to_server_tx: &broadcast::Sender<Frame>,
    cached_assets: &Mutex<Option<Frame>>,
    cached_snapshot: &Mutex<Option<Frame>>,
) -> Result<()> {
    let mut client = connect_with_retry().await;

    tracing::info!("Sending registration: host_pid={host_pid}, app_pid={app_pid}");
    client
        .register(RegisterFrame {
            host_pid,
            app_pid,
            client_kind: ClientKind::Browser,
        })
        .await?;

    let (mut reader, mut writer) = client.split();

    replay_cached_frame(&mut writer, cached_assets, "ASSETS").await?;
    replay_cached_frame(&mut writer, cached_snapshot, "SNAPSHOT").await?;

    let to_server_rx = to_server_tx.subscribe();
    let writer_task = tokio::spawn(forward_to_server(to_server_rx, writer));

    let reader_outcome = forward_from_server(&mut reader, from_server_tx).await;

    writer_task.abort();
    let _ = writer_task.await;

    reader_outcome
}

async fn replay_cached_frame(
    writer: &mut BridgeWriter,
    cache: &Mutex<Option<Frame>>,
    label: &'static str,
) -> Result<()> {
    let cached = cache.lock().await.clone();
    if let Some(frame) = cached {
        tracing::info!("Replaying cached {label} frame");
        writer.send_frame(&frame).await?;
    }
    Ok(())
}

async fn forward_to_server(mut to_server_rx: broadcast::Receiver<Frame>, mut writer: BridgeWriter) {
    loop {
        match to_server_rx.recv().await {
            Ok(frame) => {
                tracing::debug!("Forwarding frame to server: {}", frame_summary(&frame));
                if let Err(err) = writer.send_frame(&frame).await {
                    tracing::error!("Failed to forward frame to server: {err:#}");
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("Server connection lagged by {n} frames");
            }
            Err(broadcast::error::RecvError::Closed) => {
                tracing::info!("Chrome reader channel closed; ending forward task");
                break;
            }
        }
    }
}

async fn forward_from_server(
    reader: &mut euro_native_messaging::BridgeReader,
    from_server_tx: &mpsc::Sender<Frame>,
) -> Result<()> {
    loop {
        match reader.next_frame().await? {
            Some(frame) => {
                tracing::debug!("Received frame from server: {}", frame_summary(&frame));
                if from_server_tx.send(frame).await.is_err() {
                    tracing::error!("Chrome writer channel closed; ending receive task");
                    return Ok(());
                }
            }
            None => {
                tracing::info!("Server stream ended");
                return Ok(());
            }
        }
    }
}

async fn connect_with_retry() -> BridgeClient {
    (|| async {
        tracing::info!("Attempting to connect to app bridge at {BRIDGE_URL}");
        BridgeClient::connect(BRIDGE_URL).await
    })
    .retry(
        ConstantBuilder::default()
            .with_delay(RETRY_INTERVAL)
            .without_max_times(),
    )
    .sleep(tokio::time::sleep)
    .notify(|err, dur| {
        tracing::warn!("Failed to connect to app bridge: {err:#}. Retrying in {dur:?}...");
    })
    .await
    .expect("infinite retry should never return Err")
}

fn frame_summary(frame: &Frame) -> String {
    match &frame.kind {
        FrameKind::Request(r) => format!("Request(id={}, action={})", r.id, r.action),
        FrameKind::Response(r) => format!("Response(id={}, action={})", r.id, r.action),
        FrameKind::Event(e) => format!("Event(action={})", e.action),
        FrameKind::Error(e) => format!("Error(id={}, code={})", e.id, e.code),
        FrameKind::Cancel(c) => format!("Cancel(id={})", c.id),
        FrameKind::Register(r) => format!("Register(host={}, app={})", r.host_pid, r.app_pid),
    }
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
