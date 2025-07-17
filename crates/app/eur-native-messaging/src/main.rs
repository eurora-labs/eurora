use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    net::ToSocketAddrs,
    path::PathBuf,
    process,
};

use anyhow::{anyhow, Result};
// Import the PORT constant from lib.rs
use eur_native_messaging::PORT;
use tonic::transport::Server;
use tracing::info;
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    fmt,
};

mod asset_context;
mod asset_converter;
mod server;
mod snapshot_context;
mod snapshot_converter;

/// Get the path to the lock file
fn get_lock_file_path() -> Result<PathBuf> {
    let config_dir =
        dirs::config_dir().ok_or_else(|| anyhow!("Could not determine config directory"))?;
    let app_config_dir = config_dir.join("eurora");

    // Create the directory if it doesn't exist
    if !app_config_dir.exists() {
        fs::create_dir_all(&app_config_dir)?;
    }

    Ok(app_config_dir.join("native-messaging.lock"))
}

/// Check if a process with the given PID is running
fn is_process_running(pid: u32) -> bool {
    #[cfg(target_family = "unix")]
    {
        use std::process::Command;
        // On Unix-like systems, we can check if the process exists by sending signal 0
        // This doesn't actually send a signal, but checks if the process exists
        let output = Command::new("kill").args(["-0", &pid.to_string()]).output();

        match output {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    #[cfg(target_family = "windows")]
    {
        use std::process::Command;
        // On Windows, we can use tasklist to check if the process exists
        let output = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH"])
            .output();

        match output {
            Ok(output) => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                output_str.contains(&pid.to_string())
            }
            Err(_) => false,
        }
    }
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
    let lock_file_path = get_lock_file_path()?;

    // Check if the lock file exists
    if lock_file_path.exists() {
        // Read the PID from the lock file
        let mut file = File::open(&lock_file_path)?;
        let mut pid_str = String::new();
        file.read_to_string(&mut pid_str)?;

        // Parse the PID
        let pid = pid_str
            .trim()
            .parse::<u32>()
            .map_err(|_| anyhow!("Invalid PID in lock file: {}", pid_str))?;

        // Check if the process is still running
        if is_process_running(pid) {
            info!("Found existing instance with PID {}. Killing it...", pid);
            // Kill the existing process
            kill_process(pid)?;
        } else {
            info!("Found stale lock file. Removing it...");
        }

        // Remove the lock file (whether it was stale or we killed the process)
        fs::remove_file(&lock_file_path)?;
    }

    // Create a new lock file with our PID
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&lock_file_path)?;

    let current_pid = process::id();
    file.write_all(current_pid.to_string().as_bytes())?;

    // Register a shutdown handler to remove the lock file when the process exits
    let lock_file_path_clone = lock_file_path.clone();
    ctrlc::set_handler(move || {
        info!("Received shutdown signal. Cleaning up...");
        let _ = fs::remove_file(&lock_file_path_clone);
        process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Ensure only one instance is running
    ensure_single_instance()?;

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::WARN.into()) // anything not listed → WARN
        .parse_lossy("eur_=trace,hyper=off,tokio=off"); // keep yours, silence deps

    // Write only to file
    fmt()
        .with_env_filter(filter)
        .with_writer(File::create("eur-native-messaging.log")?)
        .init();

    // Create the gRPC server
    let (grpc_server, _) = server::TauriIpcServer::new();
    let server_clone = grpc_server.clone();

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
