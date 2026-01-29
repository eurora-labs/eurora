use std::{env, fs::File};
use tokio::io::{self};

use anyhow::Result;
use euro_native_messaging::PORT;
use euro_native_messaging::{
    parent_pid,
    server::{BrowserBridgeClient, Frame, FrameKind, RegisterFrame},
    utils::{ensure_single_instance, generate_typescript_definitions, read_framed, write_framed},
};
use tokio::sync::mpsc;
use tracing::{error, info};
// Need this import to succeed in prod builds
#[allow(unused_imports)]
use tracing_subscriber::prelude::*;
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    fmt,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Capture parent PID immediately at startup, before any other processing.
    // This records the PID of the browser process that started this native messaging host.
    parent_pid::capture_parent_pid();

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::WARN.into()) // anything not listed → WARN
        .parse_lossy("euro_=trace,hyper=off,tokio=off"); // keep yours, silence deps

    // Write only to file
    fmt()
        .with_env_filter(filter.clone())
        .with_writer(File::create("euro-native-messaging.log")?)
        .init();

    // Check for command line arguments
    let args: Vec<String> = env::args().collect();

    // Handle the generate_specta argument
    if args.len() > 1 && args[1] == "--generate_specta" {
        return generate_typescript_definitions();
    }

    // Ensure only one instance is running
    ensure_single_instance()?;

    let browser_pid = parent_pid::get_parent_pid();
    let host_pid = std::process::id();

    info!(
        "Starting native messaging client: host_pid={}, browser_pid={}",
        host_pid, browser_pid
    );

    // Channel for frames going to the gRPC server (will be forwarded from Chrome)
    let (to_server_tx, to_server_rx) = mpsc::unbounded_channel::<Frame>();

    // Channel for frames coming from the gRPC server (will be forwarded to Chrome)
    let (from_server_tx, mut from_server_rx) = mpsc::channel::<Frame>(1024);

    // Connect to the euro-activity gRPC server
    let server_addr = format!("http://[::1]:{}", PORT);
    info!("Connecting to euro-activity server at {}", server_addr);

    let mut client = match BrowserBridgeClient::connect(server_addr.clone()).await {
        Ok(client) => {
            info!("Connected to euro-activity server");
            client
        }
        Err(e) => {
            error!(
                "Failed to connect to euro-activity server at {}: {}",
                server_addr, e
            );
            return Err(e.into());
        }
    };

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
        let mut rx = to_server_rx;
        while let Some(frame) = rx.recv().await {
            info!("Forwarding frame to server: {:?}", frame);
            yield frame;
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
            return Err(e.into());
        }
    };

    let mut inbound_stream = response.into_inner();

    // Task: receive frames from the server and forward to Chrome
    let server_to_chrome_handle = {
        let from_server_tx = from_server_tx.clone();
        tokio::spawn(async move {
            info!("Server-to-Chrome forward task started");
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
            info!("Server-to-Chrome forward task ended");
        })
    };

    // Task: write frames to Chrome (stdout)
    let chrome_writer_handle = tokio::spawn(async move {
        let mut stdout = io::stdout();
        info!("Chrome writer task started");
        while let Some(frame) = from_server_rx.recv().await {
            info!("Writing frame to Chrome: {:?}", frame);
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
                        info!("Read frame from Chrome: {:?}", frame);
                        if let Err(e) = to_server_tx.send(frame) {
                            error!("Failed to forward frame to server: {}", e);
                            break;
                        }
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

    tokio::select! {
        _ = chrome_writer_handle => {
            info!("Chrome writer task stopped");
        }
        _ = chrome_reader_handle => {
            info!("Chrome reader task stopped");
        }
        _ = server_to_chrome_handle => {
            info!("Server-to-Chrome forward task stopped");
        }
    }

    Ok(())
}
