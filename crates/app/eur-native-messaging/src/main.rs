use std::{env, fs::File, net::ToSocketAddrs, process};

use anyhow::{Result, anyhow};
// Import the PORT constant from lib.rs
use eur_native_messaging::PORT;
use eur_native_messaging::server;
use tonic::transport::Server;
use tracing::info;
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
                info!("Failed to run pgrep: {}", e);
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
                info!("Failed to run tasklist: {}", e);
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
        info!("Found existing instance with PID {}. Killing it...", pid);
        if let Err(e) = kill_process(pid) {
            info!("Failed to kill process {}: {}", pid, e);
            // Continue trying to kill other processes even if one fails
        }
    }

    // Register a shutdown handler for clean exit
    ctrlc::set_handler(move || {
        info!("Received shutdown signal. Exiting...");
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
            "packages/chrome-ext-shared/src/lib/bindings.ts",
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

    #[cfg(debug_assertions)]
    {
        // Write only to file
        fmt()
            .with_env_filter(filter.clone())
            .with_writer(File::create("eur-native-messaging.log")?)
            .init();
    }

    #[cfg(not(debug_assertions))]
    {
        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_filter(filter.clone()))
            .with(sentry::integrations::tracing::layer().with_filter(filter))
            .try_init()
            .unwrap();

        let _guard = sentry::init((
            "https://ff55ae34aa53740318b8f1beace59031@o4508907847352320.ingest.de.sentry.io/4510096917725264",
            sentry::ClientOptions {
                release: sentry::release_name!(),
                traces_sample_rate: 0.0,
                enable_logs: true,
                send_default_pii: true, // during closed beta all metrics are non-anonymous
                debug: true,
                ..Default::default()
            },
        ));
    }

    // Ensure only one instance is running
    ensure_single_instance()?;

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
