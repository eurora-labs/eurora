use anyhow::Result;
use backon::{ConstantBuilder, Retryable};
use euro_native_messaging::PORT;
use euro_native_messaging::{
    parent_pid,
    server::{BrowserBridgeClient, Frame, FrameKind, RegisterFrame},
    utils::{generate_typescript_definitions, read_framed, write_framed},
};
use std::{env, time::Duration};
use tokio::io::{self};
use tokio::sync::{broadcast, mpsc};
use tonic::transport::Channel;
use tracing::{error, info, warn};
#[allow(unused_imports)]
use tracing_subscriber::prelude::*;

const RETRY_INTERVAL_SECS: u64 = 2;

async fn connect_with_retry(server_addr: &str) -> BrowserBridgeClient<Channel> {
    let addr = server_addr.to_string();
    (|| {
        let addr = addr.clone();
        async move {
            info!("Attempting to connect to euro-activity server at {}", addr);
            BrowserBridgeClient::connect(addr)
                .await
                .map_err(|e| e.to_string())
        }
    })
    .retry(ConstantBuilder::default().with_delay(Duration::from_secs(RETRY_INTERVAL_SECS)))
    .sleep(tokio::time::sleep)
    .notify(|err, dur| {
        warn!("Failed to connect to euro-activity server: {err}. Retrying in {dur:?}...");
    })
    .await
    .expect("infinite retry should never return Err")
}

#[tokio::main]
async fn main() -> Result<()> {
    parent_pid::capture_parent_pid();

    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && args[1] == "--generate_specta" {
        return generate_typescript_definitions();
    }

    let browser_pid = parent_pid::get_parent_pid();
    let host_pid = std::process::id();

    info!(
        "Starting native messaging client: host_pid={}, browser_pid={}",
        host_pid, browser_pid
    );

    let server_addr = format!("http://[::1]:{}", PORT);

    let (from_server_tx, mut from_server_rx) = mpsc::channel::<Frame>(1024);
    let (to_server_tx, _) = broadcast::channel::<Frame>(1024);

    let chrome_writer_handle = tokio::spawn(async move {
        let mut stdout = io::stdout();
        info!("Chrome writer task started");
        while let Some(frame) = from_server_rx.recv().await {
            info!("Writing frame to Chrome: {:?}", frame.kind);
            if let Err(err) = write_framed(&mut stdout, &frame).await {
                error!("Native host write error: {:?}", err);
                break;
            }
        }
        info!("Chrome writer task stopped");
    });

    let chrome_reader_handle = {
        let to_server_tx = to_server_tx.clone();
        tokio::spawn(async move {
            let mut stdin = io::stdin();
            info!("Chrome reader task started");
            loop {
                match read_framed(&mut stdin).await {
                    Ok(Some(frame)) => {
                        info!("Read frame from Chrome: {:?}", frame.kind);
                        let _ = to_server_tx.send(frame);
                    }
                    Ok(None) => {
                        info!("EOF from Chrome, connection closed");
                        break;
                    }
                    Err(e) => {
                        error!("Native host read error: {:?}", e);
                        break;
                    }
                }
            }
            info!("Chrome reader task stopped");
        })
    };

    let server_connection_handle = {
        let to_server_tx = to_server_tx.clone();
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

                let outbound_stream = async_stream::stream! {
                    info!("Sending registration frame: host_pid={}, browser_pid={}", host_pid, browser_pid);
                    yield register_frame;

                    loop {
                        match to_server_rx.recv().await {
                            Ok(frame) => {
                                info!("Forwarding frame to server: {:?}", frame);
                                yield frame;
                            }
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                warn!("Server connection lagged by {} frames", n);
                                continue;
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                info!("Chrome reader channel closed");
                                break;
                            }
                        }
                    }
                };

                let response = match client.open(outbound_stream).await {
                    Ok(response) => {
                        info!("Bidirectional stream opened successfully");
                        response
                    }
                    Err(e) => {
                        error!("Failed to open bidirectional stream: {}", e);
                        info!(
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
                            info!("Received frame from server: {:?}", frame);
                            if let Err(e) = from_server_tx.send(frame).await {
                                error!("Failed to forward frame from server: {}", e);
                                break;
                            }
                        }
                        Ok(None) => {
                            info!("Server stream ended");
                            break;
                        }
                        Err(e) => {
                            error!("Error receiving from server: {}", e);
                            break;
                        }
                    }
                }

                warn!("Server connection lost, reconnecting...");
                info!(
                    "Waiting {} seconds before reconnecting...",
                    RETRY_INTERVAL_SECS
                );
                tokio::time::sleep(Duration::from_secs(RETRY_INTERVAL_SECS)).await;
            }
        })
    };

    tokio::select! {
        _ = chrome_writer_handle => {
            info!("Chrome writer task stopped");
        }
        _ = chrome_reader_handle => {
            info!("Chrome reader task stopped");
        }
        _ = server_connection_handle => {
            info!("Server connection task stopped");
        }
    }

    Ok(())
}
