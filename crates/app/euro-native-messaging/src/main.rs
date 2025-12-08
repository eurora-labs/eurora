use std::{env, fs::File, net::ToSocketAddrs};
use tokio::io::{self};

use anyhow::Result;
use euro_native_messaging::PORT;
// use euro_native_messaging::server_o;
use euro_native_messaging::{
    server::{self, Frame},
    utils::{ensure_single_instance, generate_typescript_definitions, read_framed, write_framed},
};
use tokio::sync::{broadcast, mpsc};
use tonic::transport::Server;
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

    // Frame host → Chrome
    let (chrome_tx, mut chrome_rx) = mpsc::unbounded_channel::<Frame>();

    // Frames Chrome → host (broadcast to all gRPC clients)
    let (chrome_from_tx, _) = broadcast::channel::<Frame>(1024);

    // Native messaging writer: host → Chrome
    let writer_handle = tokio::spawn(async move {
        let mut stdout = io::stdout();
        info!("Native messaging writer task started");
        while let Some(frame) = chrome_rx.recv().await {
            info!("Writing frame to Chrome: {:?}", frame);
            if let Err(err) = write_framed(&mut stdout, &frame).await {
                info!("Native host write error: {err:?}");
                break;
            }
        }
        info!("Native messaging writer task stopped");
    });

    let reader_handle = {
        let chrome_from_tx = chrome_from_tx.clone();
        tokio::spawn(async move {
            let mut stdin = io::stdin();
            info!("Native messaging reader task started");
            loop {
                match read_framed(&mut stdin).await {
                    Ok(Some(frame)) => {
                        if let Err(err) = chrome_from_tx.send(frame) {
                            info!("Chrome sender error: {err:?}");
                        }
                    }
                    Ok(None) => {
                        info!("EOF from Chrome, connection closed");
                        break;
                    }
                    Err(e) => {
                        info!("Native host read error: {e:?}");
                        break;
                    }
                }
            }
            info!("Native messaging reader task stopped");
        })
    };

    // gRPC server
    let ipc_server = server::BrowserBridgeService {
        chrome_tx,
        chrome_from_tx,
    };

    let grpc_handle = tokio::spawn(async move {
        let addr = format!("[::1]:{}", PORT)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap();

        info!("Starting gRPC server at {}", addr);

        if let Err(e) = Server::builder()
            .add_service(server::BrowserBridgeServer::new(ipc_server))
            .serve(addr)
            .await
        {
            error!("Failed to start gRPC server: {}", e);
        }
        info!("gRPC server ended");
    });

    tokio::select! {
        _ = writer_handle => {
            info!("Native messaging writer task stopped");
            }
        _ = reader_handle => {
            info!("Native messaging reader task stopped");
            }
        _ = grpc_handle => {
            info!("gRPC server ended");
            }
    }

    Ok(())
}
