use std::net::TcpListener;
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
use tracing::{debug, error, info};

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

    async fn get_docker_compose_path<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<String, String>;

    async fn start_local_backend<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        ollama_model: String,
    ) -> Result<LocalBackendInfo, String>;
}

fn resolve_docker_compose_path<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docker-compose.yml");
        if path.exists() {
            return Ok(path);
        }
        return Err(format!(
            "docker-compose.yml not found at {}",
            path.display()
        ));
    }

    let resource_dir = app_handle
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to resolve resource directory: {}", e))?;

    let path = resource_dir.join("docker-compose.yml");
    if path.exists() {
        Ok(path)
    } else {
        Err(format!(
            "docker-compose.yml not found at {}",
            path.display()
        ))
    }
}

const DEFAULT_PORTS: [u16; 3] = [39051, 39080, 39432];

fn find_available_port(preferred: u16) -> Result<u16, String> {
    if TcpListener::bind(("localhost", preferred)).is_ok() {
        return Ok(preferred);
    }
    let listener = TcpListener::bind("localhost:0")
        .map_err(|e| format!("Failed to find available port: {}", e))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("Failed to get local address: {}", e))?
        .port();
    Ok(port)
}

fn host_ids() -> (String, String) {
    #[cfg(unix)]
    {
        let uid = std::process::Command::new("id")
            .arg("-u")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "1000".to_string());
        let gid = std::process::Command::new("id")
            .arg("-g")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "1000".to_string());
        (uid, gid)
    }
    #[cfg(not(unix))]
    {
        ("0".to_string(), "0".to_string())
    }
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

        debug!("Checking connection to gRPC server: {}", address);

        match tokio::net::TcpStream::connect(address.replace("http://", "").replace("https://", ""))
            .await
        {
            Ok(_) => {
                debug!("TCP connection successful");
                Ok("Server is reachable".to_string())
            }
            Err(e) => {
                let error_msg = format!("Failed to connect to server: {}", e);
                debug!("{}", error_msg);
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
        debug!("Checking for updates...");

        let updater = app_handle.updater().map_err(|e| {
            error!("Failed to get updater: {}", e);
            format!("Failed to get updater: {}", e)
        })?;

        match updater.check().await {
            Ok(Some(update)) => {
                debug!("Update available: {}", update.version);
                Ok(Some(UpdateInfo {
                    version: update.version.clone(),
                    body: update.body.clone(),
                }))
            }
            Ok(None) => {
                debug!("No update available");
                Ok(None)
            }
            Err(e) => {
                error!("Failed to check for updates: {}", e);
                Err(format!("Failed to check for updates: {}", e))
            }
        }
    }

    async fn install_update<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<(), String> {
        debug!("Installing update...");

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
            debug!(
                "Using outer app executable path for updater: {}",
                exe_inside_outer.display()
            );
            builder = builder.executable_path(&exe_inside_outer);
        }

        let updater = builder.build().map_err(|e| {
            error!("Failed to build updater: {}", e);
            format!("Failed to build updater: {}", e)
        })?;

        let update = updater.check().await.map_err(|e| {
            error!("Failed to check for updates: {}", e);
            format!("Failed to check for updates: {}", e)
        })?;

        if let Some(update) = update {
            debug!(
                "Downloading and installing update version: {}",
                update.version
            );

            update
                .download_and_install(
                    |chunk_length, content_length| {
                        debug!("Downloaded {} from {:?}", chunk_length, content_length);
                    },
                    || {
                        debug!("Download finished");
                    },
                )
                .await
                .map_err(|e| {
                    error!("Failed to download and install update: {}", e);
                    format!("Failed to download and install update: {}", e)
                })?;

            debug!("Update installed, restarting application");

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
                info!("Scheduling restart of outer app bundle: {}", outer_path);
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
            debug!("No update available to install");
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

    async fn get_docker_compose_path<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<String, String> {
        let path = resolve_docker_compose_path(&app_handle)?;
        debug!("Docker compose path: {}", path.display());
        Ok(path.to_string_lossy().to_string())
    }

    async fn start_local_backend<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        ollama_model: String,
    ) -> Result<LocalBackendInfo, String> {
        let compose_path = resolve_docker_compose_path(&app_handle)?;
        let compose_file = compose_path.to_string_lossy();

        info!("Stopping any existing eurora docker compose instances");
        let _ = tokio::process::Command::new("docker")
            .args(["compose", "-f", &compose_file, "down", "--remove-orphans"])
            .output()
            .await;

        let ps = tokio::process::Command::new("docker")
            .args(["ps", "-aq", "--filter", "name=eurora"])
            .output()
            .await;
        if let Ok(ps) = ps {
            let ids = String::from_utf8_lossy(&ps.stdout);
            let ids: Vec<&str> = ids.split_whitespace().collect();
            if !ids.is_empty() {
                info!("Stopping {} stray eurora container(s)", ids.len());
                let _ = tokio::process::Command::new("docker")
                    .arg("rm")
                    .arg("-f")
                    .args(&ids)
                    .output()
                    .await;
            }
        }

        let grpc_port = find_available_port(DEFAULT_PORTS[0])?;
        let http_port = find_available_port(DEFAULT_PORTS[1])?;
        let postgres_port = find_available_port(DEFAULT_PORTS[2])?;

        let model = if ollama_model.is_empty() {
            "llama3.2".to_string()
        } else {
            ollama_model
        };

        let asset_dir = dirs::data_dir()
            .ok_or_else(|| "Failed to resolve platform data directory".to_string())?
            .join("eurora")
            .join("assets");
        std::fs::create_dir_all(&asset_dir)
            .map_err(|e| format!("Failed to create asset directory: {}", e))?;

        let (uid, gid) = host_ids();

        info!(
            "Starting local backend via docker compose: {} (grpc={}, http={}, postgres={}, ollama_model={}, asset_dir={}, uid={}, gid={})",
            compose_path.display(),
            grpc_port,
            http_port,
            postgres_port,
            model,
            asset_dir.display(),
            uid,
            gid,
        );

        let output = tokio::process::Command::new("docker")
            .args(["compose", "-f", &compose_file, "up", "-d"])
            .env("EURORA_GRPC_PORT", grpc_port.to_string())
            .env("EURORA_HTTP_PORT", http_port.to_string())
            .env("EURORA_POSTGRES_PORT", postgres_port.to_string())
            .env("OLLAMA_MODEL", &model)
            .env("EURORA_ASSET_DIR", asset_dir.to_string_lossy().as_ref())
            .env("EURORA_UID", &uid)
            .env("EURORA_GID", &gid)
            .output()
            .await
            .map_err(|e| format!("Failed to run docker compose: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("docker compose failed: {}", stderr);
            return Err(format!("docker compose failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        info!("docker compose started: {}", stdout);

        let local_url = format!("http://localhost:{}", grpc_port);
        let endpoint_manager = app_handle.state::<SharedEndpointManager>();
        endpoint_manager
            .set_global_backend_url(&local_url)
            .map_err(|e| format!("Failed to switch API endpoint: {e}"))?;

        send_encryption_key(&local_url).await?;

        Ok(LocalBackendInfo {
            grpc_port,
            http_port,
            postgres_port,
        })
    }
}

async fn send_encryption_key(backend_url: &str) -> Result<(), String> {
    use backon::{ConstantBuilder, Retryable};
    use base64::prelude::*;
    use proto_gen::local_settings::SetEncryptionKeyRequest;
    use proto_gen::local_settings::proto_local_settings_service_client::ProtoLocalSettingsServiceClient;

    let main_key = euro_encrypt::MainKey::new()
        .map_err(|e| format!("Failed to retrieve encryption key from keyring: {e}"))?;

    let encoded = BASE64_STANDARD.encode(main_key.0);
    let url = backend_url.to_string();

    // The backend container needs time to start (postgres health check + boot).
    (|| {
        let encoded = encoded.clone();
        let url = url.clone();
        async move {
            let mut client = ProtoLocalSettingsServiceClient::connect(url).await?;
            client
                .set_encryption_key(SetEncryptionKeyRequest {
                    encryption_key: encoded,
                })
                .await?;
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        }
    })
    .retry(
        ConstantBuilder::default()
            .with_delay(std::time::Duration::from_secs(2))
            .with_max_times(30),
    )
    .sleep(tokio::time::sleep)
    .notify(|err, dur| {
        info!("Waiting for backend to be ready (retrying in {dur:?}): {err}");
    })
    .await
    .map_err(|e| format!("Backend did not become ready: {e}"))?;

    info!("Encryption key sent to local backend");
    Ok(())
}
