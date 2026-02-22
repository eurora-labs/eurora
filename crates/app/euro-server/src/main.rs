#![cfg_attr(
    all(windows, not(test), not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod postgres;

use std::net::SocketAddr;
use std::path::PathBuf;

use tauri::{
    Manager,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use tokio::sync::watch;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

const GRPC_PORT: u16 = 39051;
const HTTP_PORT: u16 = 39080;
const PG_PORT: u16 = 39432;

fn main() {
    // --- Tracing ---
    let global_filter = Targets::new()
        .with_default(LevelFilter::WARN)
        .with_target("euro_server", LevelFilter::DEBUG)
        .with_target("be_", LevelFilter::INFO)
        .with_target("agent_chain", LevelFilter::INFO);

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(global_filter)
        .try_init()
        .unwrap();

    // --- Tokio runtime ---
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            tauri::async_runtime::set(tokio::runtime::Handle::current());

            let builder = tauri::Builder::default().setup(|app| {
                // --- Resolve paths ---
                let eurora_dir = base_data_dir();
                let pg_data_dir = eurora_dir.join("pgdata");
                let log_dir = eurora_dir.join("logs");
                let asset_dir = eurora_dir.join("assets");
                std::fs::create_dir_all(&asset_dir).ok();
                std::fs::create_dir_all(&log_dir).ok();

                // --- PostgreSQL binary directory ---
                let pg_bin_dir = resolve_postgres_bin_dir(app);

                // --- Authz config ---
                let (authz_model, authz_policy) = resolve_authz_config(app);

                // --- Encryption key from OS keyring ---
                let euro_key = euro_encrypt::MainKey::new()
                    .expect("Failed to retrieve encryption key from OS keyring");

                // Initialize the file-based secret store (same as euro-tauri)
                let app_data_dir = app
                    .path()
                    .app_data_dir()
                    .expect("Failed to resolve app data directory");
                std::fs::create_dir_all(&app_data_dir).ok();
                euro_secret::secret::init_file_store(*euro_key.as_bytes(), app_data_dir)
                    .expect("Failed to initialise secret store");

                // Bridge euro_encrypt::MainKey → be_encrypt::MainKey
                let be_key = be_encrypt::MainKey(*euro_key.as_bytes());

                // --- Shutdown channel ---
                let (shutdown_tx, shutdown_rx) = watch::channel(());

                // --- System Tray ---
                let logs_item = MenuItem::with_id(app, "logs", "Open Logs", true, None::<&str>)?;
                let quit_item =
                    MenuItem::with_id(app, "quit", "Quit Eurora Server", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&logs_item, &quit_item])?;

                let log_dir_for_tray = log_dir.clone();
                TrayIconBuilder::new()
                    .icon(app.default_window_icon().unwrap().clone())
                    .tooltip("Eurora Server")
                    .menu(&menu)
                    .show_menu_on_left_click(true)
                    .on_menu_event(move |app, event| match event.id.as_ref() {
                        "quit" => {
                            let _ = shutdown_tx.send(());
                            // Give the server a moment to shut down, then exit
                            let handle = app.clone();
                            tauri::async_runtime::spawn(async move {
                                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                handle.exit(0);
                            });
                        }
                        "logs" => {
                            let _ = open_path(&log_dir_for_tray);
                        }
                        _ => {}
                    })
                    .build(app)?;

                // --- Spawn the server orchestration ---
                let pg_manager =
                    postgres::PostgresManager::new(pg_data_dir, log_dir, pg_bin_dir, PG_PORT);

                // Set ASSET_STORAGE_FS_ROOT for be-storage
                // SAFETY: Called during single-threaded setup before spawning
                // any tasks that read this variable.
                unsafe { std::env::set_var("ASSET_STORAGE_FS_ROOT", &asset_dir) };

                let server_json_path = eurora_dir.join("server.json");

                tauri::async_runtime::spawn(async move {
                    if let Err(e) = run_backend(
                        pg_manager,
                        be_key,
                        shutdown_rx,
                        authz_model,
                        authz_policy,
                        &server_json_path,
                    )
                    .await
                    {
                        tracing::error!("Backend failed: {}", e);
                    }
                });

                Ok(())
            });

            // Autostart plugin (non-macOS, release only — matches euro-tauri pattern)
            #[cfg(not(target_os = "macos"))]
            let builder = if !cfg!(debug_assertions) {
                builder.plugin(tauri_plugin_autostart::init(
                    tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                    Some(vec!["--startup-launch"]),
                ))
            } else {
                builder
            };

            builder
                .build(tauri::generate_context!())
                .expect("Failed to build Eurora Server")
                .run(|_app_handle, _event| {});
        });
}

/// Run the full backend stack: PostgreSQL + be-monolith.
async fn run_backend(
    pg_manager: postgres::PostgresManager,
    encryption_key: be_encrypt::MainKey,
    shutdown_rx: watch::Receiver<()>,
    authz_model_path: String,
    authz_policy_path: String,
    server_json_path: &std::path::Path,
) -> anyhow::Result<()> {
    // 1. Initialize + start PostgreSQL
    pg_manager.init_db_if_needed().await?;
    pg_manager.start().await?;
    pg_manager.ensure_database().await?;

    // 2. Write server.json for euro-tauri discovery
    let server_info = serde_json::json!({
        "grpc_port": GRPC_PORT,
        "http_port": HTTP_PORT,
        "pg_port": PG_PORT,
    });
    std::fs::write(
        server_json_path,
        serde_json::to_string_pretty(&server_info)?,
    )?;
    tracing::info!(
        path = %server_json_path.display(),
        "Wrote server.json for desktop app discovery"
    );

    // 3. Run be-monolith in-process
    let config = be_monolith::ServerConfig {
        database_url: pg_manager.connection_url(),
        grpc_addr: SocketAddr::from(([127, 0, 0, 1], GRPC_PORT)),
        http_addr: SocketAddr::from(([127, 0, 0, 1], HTTP_PORT)),
        local_mode: true,
        authz_model_path,
        authz_policy_path,
        encryption_key: Some(encryption_key),
        shutdown: shutdown_rx,
    };

    let server_result = be_monolith::run_server(config).await;

    // 4. Cleanup: stop PostgreSQL and remove server.json
    tracing::info!("Server shutting down, stopping PostgreSQL...");
    if let Err(e) = pg_manager.stop().await {
        tracing::error!("Failed to stop PostgreSQL: {}", e);
    }
    let _ = std::fs::remove_file(server_json_path);

    server_result.map_err(|e| anyhow::anyhow!("{e}"))
}

/// Resolve the base data directory for eurora.
fn base_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap().join(".local").join("share"))
        .join("eurora")
}

/// Resolve the directory containing PostgreSQL binaries.
fn resolve_postgres_bin_dir(app: &tauri::App) -> PathBuf {
    if cfg!(debug_assertions) {
        // Dev: use system-installed PostgreSQL or EURORA_PG_BIN_DIR
        PathBuf::from(std::env::var("EURORA_PG_BIN_DIR").unwrap_or_else(|_| {
            // Try common locations
            for candidate in &["/usr/local/bin", "/opt/homebrew/bin", "/usr/bin"] {
                let path = PathBuf::from(candidate);
                if path.join("initdb").exists() || path.join("pg_ctl").exists() {
                    return candidate.to_string();
                }
            }
            "/usr/local/bin".to_string()
        }))
    } else {
        // Release: bundled in resources/postgres/bin/
        app.path()
            .resource_dir()
            .expect("Failed to resolve resource dir")
            .join("postgres")
            .join("bin")
    }
}

/// Open a path in the platform's default file manager.
fn open_path(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(path).spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer").arg(path).spawn()?;
    }
    Ok(())
}

/// Resolve the authz model and policy file paths.
fn resolve_authz_config(app: &tauri::App) -> (String, String) {
    if cfg!(debug_assertions) {
        // Dev: use the workspace config directory
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("config")
            .join("authz");
        (
            workspace_root
                .join("model.conf")
                .to_string_lossy()
                .to_string(),
            workspace_root
                .join("policy.csv")
                .to_string_lossy()
                .to_string(),
        )
    } else {
        // Release: bundled as resources
        let resource_dir = app
            .path()
            .resource_dir()
            .expect("Failed to resolve resource dir");
        (
            resource_dir
                .join("config/authz/model.conf")
                .to_string_lossy()
                .to_string(),
            resource_dir
                .join("config/authz/policy.csv")
                .to_string_lossy()
                .to_string(),
        )
    }
}
