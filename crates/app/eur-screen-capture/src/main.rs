use anyhow::{Context, Result};
use eur_screen_capture::{
    capture_all_monitors, capture_monitor_by_index, generate_filename,
    list_monitors,
};
use std::{fs, path::PathBuf};
use tokio::signal;
use tracing::{Level, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Setup basic logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("primary");

    // Create output directories if they don't exist
    let output_dir = PathBuf::from("crates/eur-screen-capture/screen_captures");
    fs::create_dir_all(&output_dir)?;

    // Setup cancellation signal
    // let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
    // let cancel_tx = Arc::new(std::sync::Mutex::new(Some(cancel_tx)));

    // // Handle Ctrl+C
    // let cancel_tx_clone = cancel_tx.clone();
    // ctrlc::set_handler(move || {
    //     info!("Received Ctrl+C, shutting down...");
    //     if let Some(tx) = cancel_tx_clone.lock().unwrap().take() {
    //         let _ = tx.send(());
    //     }
    // })?;

    // Process command
    match command {
        "list" => {
            // List available monitors
            let monitors = list_monitors()?;
            println!("Available monitors:");
            for monitor in monitors {
                println!(
                    "Index: {}, Name: {}, Resolution: {}x{}, Primary: {}",
                    monitor.index, monitor.name, monitor.width, monitor.height, monitor.is_primary
                );
            }
        }
        "all" => {
            // Capture all monitors
            info!("Capturing all monitors...");
            let screenshots = capture_all_monitors()?;

            for screenshot in screenshots {
                let filename =
                    output_dir.join(generate_filename("screen", &screenshot.monitor_name));
                info!("Screenshot saved to {:?}", filename);
                screenshot.save(filename)?;
            }
        }
        "index" => {
            // Capture monitor by index
            let index = args
                .get(2)
                .and_then(|s| s.parse::<usize>().ok())
                .context("Please provide a valid monitor index")?;

            info!("Capturing monitor at index {}...", index);

            // Run continuously for specific monitor
            info!("Starting continuous capture (every 3 seconds). Press Ctrl+C to stop...");
            loop {
                let screenshot = capture_monitor_by_index(index)?;
                let filename =
                    output_dir.join(generate_filename("screen", &screenshot.monitor_name));
                info!("Screenshot saved to {:?}", filename);
                screenshot.save(filename)?;

                match signal::ctrl_c().await {
                    Ok(()) => {}
                    Err(err) => {
                        eprintln!("Unable to listen for shutdown signal: {}", err);
                        // we also shut down in case of error
                    }
                }

                // // Check for cancel signal
                // tokio::select! {
                //     _ = time::sleep(Duration::from_secs(3)) => {},
                //     _ = tokio::spawn(async move { let _ = cancel_rx; }) => {
                //         info!("Capture stopped");
                //         break;
                //     }
                // }
            }
        }
        _ => {
            // Default: capture primary monitor continuously
            info!(
                "Starting continuous capture of primary monitor (every 3 seconds). Press Ctrl+C to stop..."
            );

            // // Create a local cancel_rx variable
            // let mut local_cancel_rx = Some(cancel_rx);

            // loop {
            //     let screenshot = capture_primary_monitor()?;
            //     let filename =
            //         output_dir.join(generate_filename("screen", &screenshot.monitor_name));
            //     info!("Screenshot saved to {:?}", filename);
            //     // screenshot.save(filename)?;

            //     // Check for cancel signal or wait 3 seconds
            //     if let Some(rx) = local_cancel_rx.take() {
            //         tokio::select! {
            //             _ = time::sleep(Duration::from_secs(3)) => {},
            //             _ = rx => {
            //                 info!("Capture stopped");
            //                 break;
            //             }
            //         }
            //     } else {
            //         // If we've already taken the receiver, just sleep
            //         time::sleep(Duration::from_secs(3)).await;
            //     }
            // }
        }
    }

    Ok(())
}
