use std::net::TcpListener;
use std::path::PathBuf;

use euro_activity::ContextChip;
use euro_browser::BundledExtensionState;
use euro_process::{Browser, BrowserStore};
use euro_timeline::TimelineManager;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{Manager, Runtime};
use tauri_plugin_updater::UpdaterExt;
use tokio::sync::Mutex;

use crate::error::ResultExt;
use crate::shared_types::SharedEndpointManager;

/// `RequestFrame.action` the desktop sends to the macOS launcher to
/// deep-link the user into Safari's extension settings. Mirrors the
/// constant on the Swift side; kept here rather than in
/// `euro-bridge-protocol` because only this crate originates the request.
pub const OPEN_BROWSER_EXTENSION_SETTINGS_ACTION: &str = "OPEN_BROWSER_EXTENSION_SETTINGS";

/// Logical client identifier the macOS launcher registers with on the
/// bridge — the `app_kind` field of its `RegisterFrame` and the key its
/// [`BundledExtensionState`] reports use. Mirrors the Swift launcher's
/// `appKind` value passed to `BridgeWebSocketClient`.
pub const SAFARI_BRIDGE_APP_KIND: &str = "safari";

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

/// What the desktop frontend should render for a given browser when its
/// Eurora extension isn't currently driving messages back to the app.
///
/// Resolution combines two signals — whether a native messenger is
/// registered on the bridge for that browser, and (for browsers whose
/// extension ships bundled with a host app) the latest
/// [`BundledExtensionState`] the host has published. The variants are
/// ordered from "everything works" to "we don't recognize this process".
///
/// Snake-case Specta tags so the externally-tagged JSON matches what the
/// existing TS bindings file consumes for tagged unions elsewhere in the
/// app (`{ "kind": "not_installed", "install_url": "…" }`).
#[derive(Clone, Debug, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BrowserExtensionState {
    /// Extension is connected to the bridge. The frontend should render
    /// no install affordance for this browser.
    Connected,
    /// No client is registered for this browser, but the extension is
    /// distributed via a public store. The frontend should offer to open
    /// `install_url` in the focused browser.
    NotInstalled { install_url: String },
    /// The extension is installed but the user has it disabled in the
    /// browser's settings. Bundled browsers (Safari) only — there is no
    /// API to flip this state programmatically, so the frontend should
    /// deep-link the user into the relevant settings page.
    Disabled,
    /// The browser has no record of the bundled extension. Typically
    /// means the host app has never been launched on this machine.
    NotDiscovered,
    /// We can't determine the extension's state right now. The frontend
    /// should hide the affordance entirely rather than showing stale or
    /// misleading guidance.
    Unknown,
    /// Process name doesn't match any browser Eurora knows about. The
    /// frontend should hide the affordance.
    Unsupported,
}

/// Push payload describing the current [`BrowserExtensionState`] for a
/// given browser process. Emitted whenever the underlying signal
/// transitions — a native messenger registers/disconnects, or a bundled
/// host publishes a fresh state. The frontend uses this to update the
/// install affordance without polling.
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct BrowserExtensionStatus {
    pub process_name: String,
    pub state: BrowserExtensionState,
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

    async fn get_browser_extension_state(
        process_name: String,
    ) -> Result<BrowserExtensionState, String>;

    async fn open_browser_extension_settings(process_name: String) -> Result<(), String>;

    async fn open_url_in_browser(process_id: u32, url: String) -> Result<(), String>;

    #[taurpc(event)]
    async fn browser_extension_status_changed(status: BrowserExtensionStatus);

    async fn focus_main_window<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<(), String>;
}

/// Authoritative resolver behind [`SystemApi::get_browser_extension_state`]
/// and the push-event side of [`spawn_browser_status_bridge`]. Pure
/// function over the bridge registry, the bundled-extension state map,
/// and the [`Browser`] catalog — no I/O — so it's straightforward to
/// unit-test and reuse from the event loop.
pub fn resolve_browser_extension_state(process_name: &str) -> BrowserExtensionState {
    let Some(browser) = Browser::from_process_name(process_name) else {
        return BrowserExtensionState::Unsupported;
    };
    let bridge = euro_browser::BridgeService::get_or_init();

    match browser.store() {
        BrowserStore::Bundled => resolve_bundled_state(browser, bridge),
        store => {
            // For store-distributed browsers, "is the extension connected"
            // is itself the question — a disabled extension simply stops
            // talking to the native messaging host, indistinguishable from
            // not being installed at all.
            if bridge.is_connected_by_app_name(process_name) {
                BrowserExtensionState::Connected
            } else if let Some(install_url) = store.extension_url() {
                BrowserExtensionState::NotInstalled {
                    install_url: install_url.to_owned(),
                }
            } else {
                BrowserExtensionState::Unknown
            }
        }
    }
}

fn resolve_bundled_state(
    browser: Browser,
    bridge: &euro_browser::BridgeService,
) -> BrowserExtensionState {
    // Today the only bundled host is the macOS Safari launcher. Other
    // `BrowserStore::Bundled` browsers (PaleMoon) have no first-party host
    // app, so we have no signal — surface that explicitly rather than
    // pretending Safari's "Disabled" maps onto them.
    if browser != Browser::Safari {
        return BrowserExtensionState::Unknown;
    }
    match bridge.bundled_extension_state(SAFARI_BRIDGE_APP_KIND) {
        BundledExtensionState::Enabled => BrowserExtensionState::Connected,
        BundledExtensionState::Disabled => BrowserExtensionState::Disabled,
        BundledExtensionState::NotDiscovered => BrowserExtensionState::NotDiscovered,
        BundledExtensionState::Unknown => BrowserExtensionState::Unknown,
    }
}

#[cfg(target_os = "macos")]
async fn open_safari_extension_settings() -> Result<(), String> {
    use euro_bridge_protocol::BridgeError;

    let bridge = euro_browser::BridgeService::get_or_init();
    let pid = bridge
        .find_pid_by_app_name(SAFARI_BRIDGE_APP_KIND)
        .ok_or_else(|| "Safari launcher is not connected".to_string())?;

    match bridge
        .send_request(pid, OPEN_BROWSER_EXTENSION_SETTINGS_ACTION, None)
        .await
    {
        Ok(_) => Ok(()),
        Err(BridgeError::Client { message, .. }) => Err(format!(
            "Launcher refused to open Safari settings: {message}"
        )),
        Err(err) => Err(format!("Bridge error opening Safari settings: {err}")),
    }
}

#[cfg(not(target_os = "macos"))]
async fn open_safari_extension_settings() -> Result<(), String> {
    Err("Safari is only supported on macOS".to_string())
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
        .ctx("Failed to resolve resource directory")?;

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
    let listener = TcpListener::bind("localhost:0").ctx("Failed to find available port")?;
    let port = listener
        .local_addr()
        .ctx("Failed to get local address")?
        .port();
    Ok(port)
}

fn host_ids() -> (String, String) {
    #[cfg(unix)]
    {
        let uid = unsafe { libc::getuid() };
        let gid = unsafe { libc::getgid() };
        (uid.to_string(), gid.to_string())
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

        tracing::debug!("Checking connection to gRPC server");

        match tokio::net::TcpStream::connect(address.replace("http://", "").replace("https://", ""))
            .await
        {
            Ok(_) => {
                tracing::debug!("TCP connection successful");
                Ok("Server is reachable".to_string())
            }
            Err(e) => {
                let error_msg = format!("Failed to connect to server: {e}");
                tracing::debug!("{error_msg}");
                Err(error_msg)
            }
        }
    }

    async fn list_activities<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<Vec<ContextChip>, String> {
        let timeline_state: tauri::State<Mutex<TimelineManager>> = app_handle
            .try_state()
            .ok_or_else(|| "Timeline not available".to_string())?;
        let timeline = timeline_state.lock().await;
        Ok(timeline.get_context_chip().await.into_iter().collect())
    }

    async fn check_for_update<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<Option<UpdateInfo>, String> {
        tracing::debug!("Checking for updates...");

        let updater = app_handle.updater().map_err(|e| {
            tracing::error!("Failed to get updater: {e}");
            format!("Failed to get updater: {e}")
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
                tracing::error!("Failed to check for updates: {e}");
                Err(format!("Failed to check for updates: {e}"))
            }
        }
    }

    async fn install_update<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<(), String> {
        tracing::debug!("Installing update...");

        #[cfg(target_os = "macos")]
        let outer_app: Option<PathBuf> = find_outermost_app_bundle();
        #[cfg(not(target_os = "macos"))]
        let outer_app: Option<PathBuf> = None;

        let mut builder = app_handle.updater_builder();

        if let Some(ref outer) = outer_app {
            let exe_inside_outer = outer.join("Contents").join("MacOS").join("Eurora");
            tracing::debug!(
                "Using outer app executable path for updater: {}",
                exe_inside_outer.display()
            );
            builder = builder.executable_path(&exe_inside_outer);
        }

        let updater = builder.build().map_err(|e| {
            tracing::error!("Failed to build updater: {e}");
            format!("Failed to build updater: {e}")
        })?;

        let update = updater.check().await.map_err(|e| {
            tracing::error!("Failed to check for updates: {e}");
            format!("Failed to check for updates: {e}")
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
                    tracing::error!("Failed to download and install update: {e}");
                    format!("Failed to download and install update: {e}")
                })?;

            tracing::debug!("Update installed, restarting application");

            if let Some(ref outer) = outer_app {
                tracing::info!(
                    "Scheduling restart of outer app bundle: {}",
                    outer.display()
                );
                let _ = std::process::Command::new("sh")
                    .args(["-c", "sleep 2 && exec open \"$1\"", "--"])
                    .arg(outer)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
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

    async fn get_docker_compose_path<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<String, String> {
        let path = resolve_docker_compose_path(&app_handle)?;
        tracing::debug!("Docker compose path: {}", path.display());
        Ok(path.to_string_lossy().to_string())
    }

    async fn start_local_backend<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        ollama_model: String,
    ) -> Result<LocalBackendInfo, String> {
        let compose_path = resolve_docker_compose_path(&app_handle)?;
        let compose_file = compose_path.to_string_lossy();

        tracing::info!("Stopping any existing eurora docker compose instances");
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
                tracing::info!("Stopping {} stray eurora container(s)", ids.len());
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
        tokio::fs::create_dir_all(&asset_dir)
            .await
            .ctx("Failed to create asset directory")?;

        let (uid, gid) = host_ids();

        tracing::info!(
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
            .ctx("Failed to run docker compose")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!("docker compose failed: {stderr}");
            return Err(format!("docker compose failed: {stderr}"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        tracing::info!("docker compose started: {stdout}");

        let local_url = format!("http://localhost:{grpc_port}");
        let endpoint_manager = app_handle.state::<SharedEndpointManager>();
        endpoint_manager
            .set_global_backend_url(&local_url)
            .ctx("Failed to switch API endpoint")?;

        Ok(LocalBackendInfo {
            grpc_port,
            http_port,
            postgres_port,
        })
    }

    async fn get_browser_extension_state(
        self,
        process_name: String,
    ) -> Result<BrowserExtensionState, String> {
        Ok(resolve_browser_extension_state(&process_name))
    }

    async fn open_browser_extension_settings(self, process_name: String) -> Result<(), String> {
        let browser = Browser::from_process_name(&process_name)
            .ok_or_else(|| format!("Unsupported browser: {process_name:?}"))?;
        match browser {
            Browser::Safari => open_safari_extension_settings().await,
            // No other browser exposes a stable deep-link to its extension
            // settings page, and store-distributed extensions can't be in
            // an "installed-but-disabled" state from the desktop's
            // perspective anyway (a disabled extension stops connecting,
            // which we surface as `NotInstalled`). Failing loudly here is
            // cheaper than silently hiding a frontend bug.
            other => Err(format!(
                "open_browser_extension_settings is not supported for {other:?}"
            )),
        }
    }

    async fn open_url_in_browser(self, process_id: u32, url: String) -> Result<(), String> {
        crate::browser_launcher::open_url_in_process(process_id, &url)
    }

    async fn focus_main_window<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<(), String> {
        crate::window::show_and_focus_main(&app_handle).map_err(|e| e.to_string())
    }
}
