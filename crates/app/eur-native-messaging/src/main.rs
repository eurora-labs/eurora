use std::{env, fs::File, net::ToSocketAddrs, process};

use anyhow::{Context, Result, anyhow};
use eur_native_messaging::PORT;
use specta_typescript::BigIntExportBehavior;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
// use eur_native_messaging::server_o;
use eur_native_messaging::server::{self, Frame};
use tokio::sync::{broadcast, mpsc};
use tonic::transport::Server;
use tracing::{debug, error, info};
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

    if let Err(e) = Typescript::default()
        .bigint(BigIntExportBehavior::BigInt)
        .export_to(
            "packages/browser-shared/src/content/bindings.ts",
            &specta::export(),
        )
    {
        debug!("Failed to generate TypeScript definitions: {}", e);
    }

    Ok(())
}

async fn read_framed<R>(reader: &mut R) -> anyhow::Result<Option<Frame>>
where
    R: AsyncReadExt + Unpin,
{
    let mut len_buf = [0u8; 4];

    match reader.read_exact(&mut len_buf).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            return Ok(None);
        }
        Err(e) => return Err(e).context("reading message length"),
    }

    let len = u32::from_le_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    reader
        .read_exact(&mut buf)
        .await
        .context("reading message body")?;

    let frame: Frame = serde_json::from_slice(&buf).context("parsing Frame from JSON")?;

    Ok(Some(frame))
}

async fn write_framed<W>(writer: &mut W, frame: &Frame) -> anyhow::Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    let json = serde_json::to_vec(frame).context("serializing Frame to JSON")?;
    let len = json.len() as u32;

    writer
        .write_all(&len.to_le_bytes())
        .await
        .context("writing length")?;
    writer.write_all(&json).await.context("writing body")?;
    writer.flush().await.context("flushing stdout")?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::WARN.into()) // anything not listed → WARN
        .parse_lossy("eur_=trace,hyper=off,tokio=off"); // keep yours, silence deps

    // Write only to file
    fmt()
        .with_env_filter(filter.clone())
        .with_writer(File::create("eur-native-messaging.log")?)
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
            info!(
                "Writing frame to Chrome: kind={} id={} action={}",
                frame.kind, frame.id, frame.action,
            );
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
