use std::path::PathBuf;

use euro_activity::ContextChip;
use euro_timeline::TimelineManager;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{Manager, Runtime};
use tauri_plugin_updater::UpdaterExt;
use tokio::sync::Mutex;

#[cfg(target_os = "macos")]
fn find_outermost_app_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let mut outermost: Option<PathBuf> = None;
    let mut count = 0u32;
    for ancestor in exe.ancestors() {
        if ancestor.extension().is_some_and(|ext| ext == "app") {
            outermost = Some(ancestor.to_path_buf());
            count += 1;
        }
    }
    if count >= 2 { outermost } else { None }
}

use crate::shared_types::SharedEndpointManager;

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct UpdateInfo {
    pub version: String,
    pub body: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct LocalBackendInfo {
    pub grpc_port: u16,
    pub http_port: u16,
    pub postgres_port: u16,
}

#[taurpc::procedures(path = "system")]
pub trait SystemApi {
    async fn check_grpc_server_connection(server_address: Option<String>)
    -> Result<String, String>;

    async fn list_activities<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<Vec<ContextChip>, String>;

    async fn check_for_update<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<Option<UpdateInfo>, String>;

    async fn install_update<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<(), String>;

    async fn quit<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<(), String>;

    async fn check_accessibility_permission() -> Result<bool, String>;

    async fn request_accessibility_permission() -> Result<(), String>;

    async fn connect_local_server<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<LocalBackendInfo, String>;
}

/// Resolve the path to the server.json file written by euro-server.
fn server_json_path() -> Result<PathBuf, String> {
    let path = dirs::data_dir()
        .ok_or("Failed to resolve platform data directory")?
        .join("eurora")
        .join("server.json");
    Ok(path)
}

#[derive(Clone)]
pub struct SystemApiImpl;

#[taurpc::resolvers]
impl SystemApi for SystemApiImpl {
    async fn check_grpc_server_connection(
        self,
        server_address: Option<String>,
    ) -> Result<String, String> {
        let address = server_address.unwrap_or_else(|| "localhost:50051".to_string());

        tracing::debug!("Checking connection to gRPC server: {}", address);

        match tokio::net::TcpStream::connect(address.replace("http://", "").replace("https://", ""))
            .await
        {
            Ok(_) => {
                tracing::debug!("TCP connection successful");
                Ok("Server is reachable".to_string())
            }
            Err(e) => {
                let error_msg = format!("Failed to connect to server: {}", e);
                tracing::debug!("{}", error_msg);
                Err(error_msg)
            }
        }
    }

    async fn list_activities<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<Vec<ContextChip>, String> {
        let timeline_state: tauri::State<Mutex<TimelineManager>> = app_handle.state();
        let timeline = timeline_state.lock().await;
        let activities = timeline.get_context_chips().await;
        let limited_activities = activities.into_iter().take(5).collect::<Vec<ContextChip>>();

        Ok(limited_activities)
    }

    async fn check_for_update<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<Option<UpdateInfo>, String> {
        tracing::debug!("Checking for updates...");

        let updater = app_handle.updater().map_err(|e| {
            tracing::error!("Failed to get updater: {}", e);
            format!("Failed to get updater: {}", e)
        })?;

        match updater.check().await {
            Ok(Some(update)) => {
                tracing::debug!("Update available: {}", update.version);
                Ok(Some(UpdateInfo {
                    version: update.version.clone(),
                    body: update.body.clone(),
                }))
            }
            Ok(None) => {
                tracing::debug!("No update available");
                Ok(None)
            }
            Err(e) => {
                tracing::error!("Failed to check for updates: {}", e);
                Err(format!("Failed to check for updates: {}", e))
            }
        }
    }

    async fn install_update<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<(), String> {
        tracing::debug!("Installing update...");

        // On macOS, when running inside a nested bundle (Eurora.app wraps
        // Eurora.app), we must tell the updater to target the
        // *outermost* .app so it replaces the whole bundle — keeping the
        // Safari extension and its code signature intact.
        #[cfg(target_os = "macos")]
        let outer_app: Option<PathBuf> = find_outermost_app_bundle();
        #[cfg(not(target_os = "macos"))]
        let outer_app: Option<PathBuf> = None;

        let mut builder = app_handle.updater_builder();

        if let Some(ref outer) = outer_app {
            // Point the updater at a path inside the outer bundle so it
            // resolves EuroraMacOS.app (not Eurora.app) as the bundle
            // to replace.
            let exe_inside_outer = outer.join("Contents").join("MacOS").join("Eurora");
            tracing::debug!(
                "Using outer app executable path for updater: {}",
                exe_inside_outer.display()
            );
            builder = builder.executable_path(&exe_inside_outer);
        }

        let updater = builder.build().map_err(|e| {
            tracing::error!("Failed to build updater: {}", e);
            format!("Failed to build updater: {}", e)
        })?;

        let update = updater.check().await.map_err(|e| {
            tracing::error!("Failed to check for updates: {}", e);
            format!("Failed to check for updates: {}", e)
        })?;

        if let Some(update) = update {
            tracing::debug!(
                "Downloading and installing update version: {}",
                update.version
            );

            update
                .download_and_install(
                    |chunk_length, content_length| {
                        tracing::debug!("Downloaded {} from {:?}", chunk_length, content_length);
                    },
                    || {
                        tracing::debug!("Download finished");
                    },
                )
                .await
                .map_err(|e| {
                    tracing::error!("Failed to download and install update: {}", e);
                    format!("Failed to download and install update: {}", e)
                })?;

            tracing::debug!("Update installed, restarting application");

            // On macOS with the nested wrapper bundle we must restart the
            // *outer* Eurora.app (which hosts the launcher, bridge server
            // and Safari extension).  A plain `app_handle.restart()` would
            // only relaunch the inner Tauri binary — the launcher and its
            // TCP bridge would stay dead.
            //
            // Strategy: spawn a background shell that waits for the old
            // processes to exit, then `open`s the new outer app.  We then
            // exit the current process; the launcher will observe the
            // termination and shut itself down cleanly.
            if let Some(ref outer) = outer_app {
                let outer_path = outer.to_string_lossy().to_string();
                tracing::info!("Scheduling restart of outer app bundle: {}", outer_path);
                let _ = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(format!("sleep 2 && open {:?}", outer_path))
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
                // Exit immediately so the old launcher can terminate too.
                std::process::exit(0);
            }

            app_handle.restart();
            #[allow(unreachable_code)]
            Ok(())
        } else {
            tracing::debug!("No update available to install");
            Err("No update available to install".to_string())
        }
    }

    async fn quit<R: Runtime>(self, app_handle: tauri::AppHandle<R>) -> Result<(), String> {
        app_handle.exit(0);
        Ok(())
    }

    async fn check_accessibility_permission(self) -> Result<bool, String> {
        #[cfg(target_os = "macos")]
        {
            use core_foundation::base::TCFType;
            use core_foundation::boolean::CFBoolean;
            use core_foundation::dictionary::CFDictionary;
            use core_foundation::string::CFString;

            unsafe extern "C" {
                fn AXIsProcessTrustedWithOptions(
                    options: core_foundation::dictionary::CFDictionaryRef,
                ) -> bool;
            }

            let key = CFString::new("AXTrustedCheckOptionPrompt");
            let value = CFBoolean::false_value();
            let options = CFDictionary::from_CFType_pairs(&[(key, value)]);

            let trusted = unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) };

            Ok(trusted)
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(true)
        }
    }

    async fn request_accessibility_permission(self) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            use core_foundation::base::TCFType;
            use core_foundation::boolean::CFBoolean;
            use core_foundation::dictionary::CFDictionary;
            use core_foundation::string::CFString;

            unsafe extern "C" {
                fn AXIsProcessTrustedWithOptions(
                    options: core_foundation::dictionary::CFDictionaryRef,
                ) -> bool;
            }

            let key = CFString::new("AXTrustedCheckOptionPrompt");
            let value = CFBoolean::true_value();
            let options = CFDictionary::from_CFType_pairs(&[(key, value)]);

            let _ = unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) };

            Ok(())
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(())
        }
    }

    async fn connect_local_server<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<LocalBackendInfo, String> {
        let path = server_json_path()?;

        let contents = tokio::fs::read_to_string(&path).await.map_err(|_| {
            "Eurora Server is not running. Please install and start Eurora Server first."
                .to_string()
        })?;

        let info: serde_json::Value =
            serde_json::from_str(&contents).map_err(|e| format!("Invalid server.json: {e}"))?;

        let grpc_port = info["grpc_port"].as_u64().unwrap_or(39051) as u16;
        let http_port = info["http_port"].as_u64().unwrap_or(39080) as u16;
        let postgres_port = info["pg_port"].as_u64().unwrap_or(39432) as u16;

        // Probe the gRPC endpoint to confirm the server is reachable
        let grpc_addr = format!("127.0.0.1:{grpc_port}");
        tokio::net::TcpStream::connect(&grpc_addr)
            .await
            .map_err(|e| format!("Cannot connect to Eurora Server at {grpc_addr}: {e}"))?;

        let local_url = format!("http://localhost:{grpc_port}");
        let endpoint_manager = app_handle.state::<SharedEndpointManager>();
        endpoint_manager
            .set_global_backend_url(&local_url)
            .map_err(|e| format!("Failed to switch API endpoint: {e}"))?;

        tracing::info!("Connected to local Eurora Server at {local_url}");

        Ok(LocalBackendInfo {
            grpc_port,
            http_port,
            postgres_port,
        })
    }
}
