// use std::fs::File;
use anyhow::Result;
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
// Need this import to succeed in prod builds
#[allow(unused_imports)]
use tracing_subscriber::prelude::*;
// use tracing_subscriber::{
//     filter::{EnvFilter, LevelFilter},
//     fmt,
// };

/// Retry interval for connecting to the server
const RETRY_INTERVAL_SECS: u64 = 2;

/// Connect to the gRPC server with retry logic
async fn connect_with_retry(server_addr: &str) -> BrowserBridgeClient<Channel> {
    loop {
        info!(
            "Attempting to connect to euro-activity server at {}",
            server_addr
        );

        match BrowserBridgeClient::connect(server_addr.to_string()).await {
            Ok(client) => {
                info!("Connected to euro-activity server");
                return client;
            }
            Err(e) => {
                warn!(
                    "Failed to connect to euro-activity server: {}. Retrying in {} seconds...",
                    e, RETRY_INTERVAL_SECS
                );
                tokio::time::sleep(Duration::from_secs(RETRY_INTERVAL_SECS)).await;
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Capture parent PID immediately at startup, before any other processing.
    // This records the PID of the browser process that started this native messaging host.
    parent_pid::capture_parent_pid();

    // let filter = EnvFilter::builder()
    //     .with_default_directive(LevelFilter::WARN.into()) // anything not listed → WARN
    //     .parse_lossy("euro_=trace,hyper=off,tokio=off"); // keep yours, silence deps

    // // Write only to file
    // fmt()
    //     .with_env_filter(filter.clone())
    //     .with_writer(File::create("euro-native-messaging.log")?)
    //     .init();

    // Check for command line arguments
    let args: Vec<String> = env::args().collect();

    // Handle the generate_specta argument
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

    // Channel for frames coming from the gRPC server (will be forwarded to Chrome)
    let (from_server_tx, mut from_server_rx) = mpsc::channel::<Frame>(1024);

    // Broadcast channel for frames going to the gRPC server (allows resubscribing on reconnect)
    let (to_server_tx, _) = broadcast::channel::<Frame>(1024);

    // Task: write frames to Chrome (stdout)
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

    // Task: read frames from Chrome (stdin) and forward to server
    let chrome_reader_handle = {
        let to_server_tx = to_server_tx.clone();
        tokio::spawn(async move {
            let mut stdin = io::stdin();
            info!("Chrome reader task started");
            loop {
                match read_framed(&mut stdin).await {
                    Ok(Some(frame)) => {
                        info!("Read frame from Chrome: {:?}", frame.kind);
                        // Broadcast to any active server connection
                        // It's okay if there are no receivers (server disconnected)
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

    // Main connection loop with retry logic
    let server_connection_handle = {
        let to_server_tx = to_server_tx.clone();
        tokio::spawn(async move {
            loop {
                // Connect to the server (with retry)
                let mut client = connect_with_retry(&server_addr).await;

                // Subscribe to frames from Chrome for this connection
                let mut to_server_rx = to_server_tx.subscribe();

                // Send registration frame first
                let register_frame = Frame {
                    kind: Some(FrameKind::Register(RegisterFrame {
                        host_pid,
                        browser_pid,
                    })),
                };

                // Create a stream that starts with the register frame followed by forwarded frames
                let outbound_stream = async_stream::stream! {
                    // Send registration frame first
                    info!("Sending registration frame: host_pid={}, browser_pid={}", host_pid, browser_pid);
                    yield register_frame;

                    // Then forward all frames from Chrome to the server
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

                // Open bidirectional stream with the server
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

                // Receive frames from the server and forward to Chrome
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
