use std::{env, fs::File, net::ToSocketAddrs, process};

use anyhow::{Result, anyhow};
use eur_native_messaging::PORT;
use eur_native_messaging::server;
use tonic::transport::Server;
use tracing::debug;
// Need this import to succeed in prod builds
#[allow(unused_imports)]
use tracing_subscriber::prelude::*;
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    fmt,
};

/// Find processes by name and return their PIDs
fn find_processes_by_name(process_name: &str) -> Result<Vec<u32>> {
    let mut pids = Vec::new();
    let current_pid = process::id();

    #[cfg(target_family = "unix")]
    {
        use std::process::Command;
        // On Unix-like systems, use pgrep to find processes by name
        let output = Command::new("pgrep").args(["-f", process_name]).output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    for line in output_str.lines() {
                        if let Ok(pid) = line.trim().parse::<u32>() {
                            // Don't include our own process
                            if pid != current_pid {
                                pids.push(pid);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                debug!("Failed to run pgrep: {}", e);
                // Fallback: try using ps
                let output = Command::new("ps").args(["aux"]).output();

                if let Ok(output) = output {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    for line in output_str.lines() {
                        if line.contains(process_name) {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() > 1
                                && let Ok(pid) = parts[1].parse::<u32>()
                                && pid != current_pid
                            {
                                pids.push(pid);
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_family = "windows")]
    {
        use std::process::Command;
        // On Windows, use tasklist to find processes by name
        let output = Command::new("tasklist")
            .args([
                "/FI",
                &format!("IMAGENAME eq {}.exe", process_name),
                "/FO",
                "CSV",
                "/NH",
            ])
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    for line in output_str.lines() {
                        if !line.trim().is_empty() {
                            // Parse CSV format: "process.exe","PID","Session Name","Session#","Mem Usage"
                            let parts: Vec<&str> = line.split(',').collect();
                            if parts.len() > 1 {
                                let pid_str = parts[1].trim_matches('"');
                                if let Ok(pid) = pid_str.parse::<u32>() {
                                    if pid != current_pid {
                                        pids.push(pid);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                debug!("Failed to run tasklist: {}", e);
            }
        }
    }

    Ok(pids)
}

/// Kill a process with the given PID
fn kill_process(pid: u32) -> Result<()> {
    #[cfg(target_family = "unix")]
    {
        use std::process::Command;
        // On Unix-like systems, we can use kill to terminate the process
        let status = Command::new("kill").args([&pid.to_string()]).status()?;

        if !status.success() {
            return Err(anyhow!("Failed to kill process {}", pid));
        }
    }

    #[cfg(target_family = "windows")]
    {
        use std::process::Command;
        // On Windows, we can use taskkill to terminate the process
        let status = Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to kill process {}", pid));
        }
    }

    // Wait a moment for the process to terminate
    std::thread::sleep(std::time::Duration::from_millis(500));

    Ok(())
}

/// Ensure only one instance is running
fn ensure_single_instance() -> Result<()> {
    // Define the process name to search for
    let process_name = "eur-native-messaging";

    // Find any existing instances of this process
    let existing_pids = find_processes_by_name(process_name)?;

    // Kill all existing instances
    for pid in existing_pids {
        debug!("Found existing instance with PID {}. Killing it...", pid);
        if let Err(e) = kill_process(pid) {
            debug!("Failed to kill process {}: {}", pid, e);
            // Continue trying to kill other processes even if one fails
        }
    }

    // Register a shutdown handler for clean exit
    ctrlc::set_handler(move || {
        debug!("Received shutdown signal. Exiting...");
        process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    Ok(())
}

/// Generate TypeScript definitions using Specta
fn generate_typescript_definitions() -> Result<()> {
    use specta_typescript::Typescript;

    Typescript::default()
        .export_to(
            "packages/browser-shared/src/content/bindings.ts",
            &specta::export(),
        )
        .unwrap();

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Check for command line arguments
    let args: Vec<String> = env::args().collect();

    // Handle the generate_specta argument
    if args.len() > 1 && args[1] == "--generate_specta" {
        return generate_typescript_definitions();
    }

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::WARN.into()) // anything not listed → WARN
        .parse_lossy("eur_=trace,hyper=off,tokio=off"); // keep yours, silence deps

    // Write only to file
    fmt()
        .with_env_filter(filter.clone())
        .with_writer(File::create("eur-native-messaging.log")?)
        .init();

    // Ensure only one instance is running
    ensure_single_instance()?;

    // Create the gRPC server with channels for both unsolicited and response messages
    let (grpc_server, native_tx, stdin_tx) = server::TauriIpcServer::new().await;
    let server_clone = grpc_server.clone();

    // Start background task to read from stdin and route messages
    tokio::spawn(async move {
        use eur_native_messaging::types::{ChromeMessage, NativeMessage};
        use serde_json::Value;
        use tokio::io::{AsyncReadExt, stdin};

        let mut stdin = stdin();
        loop {
            // Read message size (4 bytes)
            let mut size_bytes = [0u8; 4];
            if let Err(e) = stdin.read_exact(&mut size_bytes).await {
                debug!(
                    "Failed to read message size from stdin: {}, exiting stdin reader",
                    e
                );
                break;
            }

            let message_size = u32::from_ne_bytes(size_bytes) as usize;

            // Read message body
            let mut buffer = vec![0u8; message_size];
            if let Err(e) = stdin.read_exact(&mut buffer).await {
                debug!(
                    "Failed to read message body from stdin: {}, exiting stdin reader",
                    e
                );
                break;
            }

            // First parse as generic JSON to determine message type
            match serde_json::from_slice::<Value>(&buffer) {
                Ok(json_value) => {
                    // Try to parse as ChromeMessage (unsolicited messages)
                    if let Ok(chrome_message) =
                        serde_json::from_value::<ChromeMessage>(json_value.clone())
                    {
                        debug!("Received unsolicited chrome message from stdin");
                        if native_tx.send(chrome_message).await.is_err() {
                            debug!("Failed to send chrome message to channel, receiver dropped");
                            break;
                        }
                    }
                    // Try to parse as NativeMessage (response to commands)
                    else if let Ok(native_message) =
                        serde_json::from_value::<NativeMessage>(json_value.clone())
                    {
                        debug!("Received native response message from stdin");
                        if stdin_tx.send(native_message).await.is_err() {
                            debug!("Failed to send native response to channel, receiver dropped");
                            break;
                        }
                    } else {
                        debug!(
                            "Received message from stdin that doesn't match ChromeMessage or NativeMessage format"
                        );
                        if let Ok(raw_str) = serde_json::to_string_pretty(&json_value) {
                            debug!("Raw JSON: {}", raw_str);
                        }
                    }
                }
                Err(e) => {
                    debug!("Failed to parse JSON from stdin: {}", e);
                    if let Ok(raw_str) = String::from_utf8(buffer.clone()) {
                        debug!("Raw message: {}", raw_str);
                    }
                }
            }
        }
    });

    // Start the gRPC server
    tokio::spawn(async move {
        Server::builder()
            // Use the server module's implementation directly
            .add_service(eur_proto::ipc::tauri_ipc_server::TauriIpcServer::new(
                grpc_server,
            ))
            .serve(
                format!("[::1]:{}", PORT)
                    .to_socket_addrs()
                    .unwrap()
                    .next()
                    .unwrap(),
            )
            .await
            .unwrap();
    });

    // Handle stdio in the main thread
    server_clone.handle_stdio().await?;

    Ok(())
}
