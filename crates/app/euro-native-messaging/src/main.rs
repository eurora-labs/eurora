use anyhow::Result;
use backon::{ConstantBuilder, Retryable};
use euro_native_messaging::PORT;
use euro_native_messaging::{
    parent_pid,
    server::{BrowserBridgeClient, Frame, FrameKind, RegisterFrame},
    utils::{generate_typescript_definitions, read_framed, write_framed},
};
use std::{env, sync::Arc, time::Duration};
use tokio::io::{self};
use tokio::sync::{Mutex, broadcast, mpsc};
use tonic::transport::Channel;

const RETRY_INTERVAL_SECS: u64 = 2;

fn frame_summary(frame: &Frame) -> String {
    match &frame.kind {
        Some(FrameKind::Request(r)) => format!("Request(id={}, action={})", r.id, r.action),
        Some(FrameKind::Response(r)) => format!("Response(id={}, action={})", r.id, r.action),
        Some(FrameKind::Event(e)) => format!("Event(action={})", e.action),
        Some(FrameKind::Error(e)) => format!("Error(id={}, code={})", e.id, e.code),
        Some(FrameKind::Cancel(c)) => format!("Cancel(id={})", c.id),
        Some(FrameKind::Register(r)) => {
            format!("Register(host={}, browser={})", r.host_pid, r.browser_pid)
        }
        None => "None".into(),
    }
}

async fn connect_with_retry(server_addr: &str) -> BrowserBridgeClient<Channel> {
    let addr = server_addr.to_string();
    (|| {
        let addr = addr.clone();
        async move {
            tracing::info!("Attempting to connect to euro-activity server at {}", addr);
            BrowserBridgeClient::connect(addr)
                .await
                .map_err(|e| e.to_string())
        }
    })
    .retry(
        ConstantBuilder::default()
            .with_delay(Duration::from_secs(RETRY_INTERVAL_SECS))
            .without_max_times(),
    )
    .sleep(tokio::time::sleep)
    .notify(|err, dur| {
        tracing::warn!("Failed to connect to euro-activity server: {err}. Retrying in {dur:?}...");
    })
    .await
    .expect("infinite retry should never return Err")
}

#[tokio::main]
async fn main() -> Result<()> {
    parent_pid::capture_parent_pid();

    #[cfg(debug_assertions)]
    {
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

    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && args[1] == "--generate_specta" {
        return generate_typescript_definitions();
    }

    let browser_pid = parent_pid::get_parent_pid();
    let host_pid = std::process::id();

    tracing::info!(
        "Starting native messaging client: host_pid={}, browser_pid={}",
        host_pid,
        browser_pid
    );

    let server_addr = format!("http://[::1]:{}", PORT);

    let (from_server_tx, mut from_server_rx) = mpsc::channel::<Frame>(1024);
    let (to_server_tx, _) = broadcast::channel::<Frame>(1024);

    let cached_assets: Arc<Mutex<Option<Frame>>> = Arc::new(Mutex::new(None));
    let cached_snapshot: Arc<Mutex<Option<Frame>>> = Arc::new(Mutex::new(None));

    let chrome_writer_handle = tokio::spawn(async move {
        let mut stdout = io::stdout();
        tracing::info!("Chrome writer task started");
        while let Some(frame) = from_server_rx.recv().await {
            tracing::debug!("Writing frame to Chrome: {}", frame_summary(&frame));
            if let Err(err) = write_framed(&mut stdout, &frame).await {
                tracing::error!("Native host write error: {:?}", err);
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
                        if let Some(FrameKind::Event(ref e)) = frame.kind {
                            match e.action.as_str() {
                                "ASSETS" => *cached_assets.lock().await = Some(frame.clone()),
                                "SNAPSHOT" => *cached_snapshot.lock().await = Some(frame.clone()),
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
                        tracing::error!("Native host read error: {:?}", e);
                        break;
                    }
                }
            }
            tracing::info!("Chrome reader task stopped");
        })
    };

    let _server_connection_handle = {
        let to_server_tx = to_server_tx.clone();
        let cached_assets = Arc::clone(&cached_assets);
        let cached_snapshot = Arc::clone(&cached_snapshot);
        tokio::spawn(async move {
            loop {
                let mut client = connect_with_retry(&server_addr).await;
                let mut to_server_rx = to_server_tx.subscribe();

                let register_frame = Frame {
                    kind: Some(FrameKind::Register(RegisterFrame {
                        host_pid,
                        browser_pid,
                    })),
                };

                let replay_assets = cached_assets.lock().await.clone();
                let replay_snapshot = cached_snapshot.lock().await.clone();

                let outbound_stream = async_stream::stream! {
                    tracing::info!("Sending registration frame: host_pid={}, browser_pid={}", host_pid, browser_pid);
                    yield register_frame;

                    if let Some(assets) = replay_assets {
                        tracing::info!("Replaying cached ASSETS frame");
                        yield assets;
                    }
                    if let Some(snapshot) = replay_snapshot {
                        tracing::info!("Replaying cached SNAPSHOT frame");
                        yield snapshot;
                    }

                    loop {
                        match to_server_rx.recv().await {
                            Ok(frame) => {
                                tracing::debug!("Forwarding frame to server: {}", frame_summary(&frame));
                                yield frame;
                            }
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                tracing::warn!("Server connection lagged by {} frames", n);
                                continue;
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                tracing::info!("Chrome reader channel closed");
                                break;
                            }
                        }
                    }
                };

                let response = match client.open(outbound_stream).await {
                    Ok(response) => {
                        tracing::info!("Bidirectional stream opened successfully");
                        response
                    }
                    Err(e) => {
                        tracing::error!("Failed to open bidirectional stream: {}", e);
                        tracing::info!(
                            "Waiting {} seconds before reconnecting...",
                            RETRY_INTERVAL_SECS
                        );
                        tokio::time::sleep(Duration::from_secs(RETRY_INTERVAL_SECS)).await;
                        continue;
                    }
                };

                let mut inbound_stream = response.into_inner();

                loop {
                    match inbound_stream.message().await {
                        Ok(Some(frame)) => {
                            tracing::debug!(
                                "Received frame from server: {}",
                                frame_summary(&frame)
                            );
                            if let Err(e) = from_server_tx.send(frame).await {
                                tracing::error!("Failed to forward frame from server: {}", e);
                                break;
                            }
                        }
                        Ok(None) => {
                            tracing::info!("Server stream ended");
                            break;
                        }
                        Err(e) => {
                            tracing::error!("Error receiving from server: {}", e);
                            break;
                        }
                    }
                }

                tracing::warn!("Server connection lost, reconnecting...");
                tracing::info!(
                    "Waiting {} seconds before reconnecting...",
                    RETRY_INTERVAL_SECS
                );
                tokio::time::sleep(Duration::from_secs(RETRY_INTERVAL_SECS)).await;
            }
        })
    };

    tokio::select! {
        _ = chrome_writer_handle => {
            tracing::info!("Chrome writer task stopped");
        }
        _ = chrome_reader_handle => {
            tracing::info!("Chrome reader task stopped");
        }
    }

    Ok(())
}
