use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use euro_activity::ContextChip;
use euro_browser::BundledExtensionState;
use euro_process::{Browser, BrowserStore};
use euro_timeline::TimelineManager;
use llm_core::RedactedLlmConfig;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{Manager, Runtime};
use tauri_plugin_updater::UpdaterExt;
use tokio::sync::Mutex;
use url::Url;

use crate::error::ResultExt;
use crate::shared_types::{SharedAppSettings, SharedEndpointManager};
use crate::telemetry;

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

/// Single payload the desktop frontend fetches once at startup to bring
/// up its Sentry / PostHog SDKs. Bundles the user's persisted consent
/// state, the embedded build-time keys, and the release identity so the
/// SDKs can tag events with channel + version. Empty strings mean
/// "disabled" — keeps dev builds quiet without forcing a separate
/// nullable type per field.
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryBootstrap {
    pub settings: euro_settings::TelemetrySettings,
    pub sentry_dsn: String,
    pub posthog_key: String,
    pub posthog_host: String,
    pub channel: String,
    pub release: String,
}

// `LocalBackendInfo` and the `start_local_backend` procedure that returned
// it have been removed. The desktop app no longer manages a docker-compose
// instance for the user — the backend is expected to be running separately
// (`just dev:backend` or a deployed instance), and the user picks how to
// reach it via the connection mode in `APISettings`.

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
    async fn check_backend_connection<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        server_address: Option<String>,
    ) -> Result<String, String>;

    async fn get_llm_info<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<RedactedLlmConfig, String>;

    // Probe an arbitrary URL by hitting /llm/info — used by the connection
    // picker's "Test connection" button before persisting the URL.
    async fn test_backend_url<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        url: String,
    ) -> Result<RedactedLlmConfig, String>;

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

    async fn get_browser_extension_state(
        process_name: String,
    ) -> Result<BrowserExtensionState, String>;

    async fn open_browser_extension_settings(process_name: String) -> Result<(), String>;

    async fn open_url_in_browser(process_id: u32, url: String) -> Result<(), String>;

    #[taurpc(event)]
    async fn browser_extension_status_changed(status: BrowserExtensionStatus);

    async fn focus_main_window<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<(), String>;

    async fn get_telemetry_bootstrap<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<TelemetryBootstrap, String>;

    async fn needs_telemetry_consent<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<bool, String>;

    async fn reinit_telemetry<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<(), String>;

    async fn rotate_telemetry_distinct_id<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<String, String>;
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

/// Hit `/llm/info` on `base_url` and return the redacted configuration.
/// Shared by `get_llm_info` (which uses the configured endpoint) and
/// `test_backend_url` (which lets the connection picker probe an arbitrary
/// URL before persisting it).
async fn fetch_llm_info(base_url: &str) -> Result<RedactedLlmConfig, String> {
    let parsed = Url::parse(base_url).map_err(|e| format!("Invalid URL `{base_url}`: {e}"))?;
    let info_url = parsed
        .join("llm/info")
        .map_err(|e| format!("Failed to derive /llm/info URL: {e}"))?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let response = client
        .get(info_url.clone())
        .send()
        .await
        .map_err(|e| format!("Request to {info_url} failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Backend at {base_url} returned {} on /llm/info",
            response.status()
        ));
    }

    response
        .json::<RedactedLlmConfig>()
        .await
        .map_err(|e| format!("Failed to parse /llm/info response: {e}"))
}

#[derive(Clone)]
pub struct SystemApiImpl;

#[taurpc::resolvers]
impl SystemApi for SystemApiImpl {
    async fn check_backend_connection<R: Runtime>(
        self,
        _app_handle: tauri::AppHandle<R>,
        server_address: Option<String>,
    ) -> Result<String, String> {
        // Default to the build-time-baked Local-mode endpoint when
        // the frontend doesn't pass an explicit address (e.g., the
        // very first reachability probe before connection mode is
        // resolved). Strip the scheme so the result is a `host:port`
        // suitable for `TcpStream::connect`.
        let address = server_address.unwrap_or_else(|| euro_settings::LOCAL_API_URL.to_string());
        let host_port = address.replace("http://", "").replace("https://", "");

        tracing::debug!("Checking TCP reachability of {host_port}");

        match tokio::net::TcpStream::connect(&host_port).await {
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

    async fn get_llm_info<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<RedactedLlmConfig, String> {
        let endpoint_manager = app_handle.state::<SharedEndpointManager>();
        let url = endpoint_manager.current_url().to_string();
        fetch_llm_info(&url).await
    }

    async fn test_backend_url<R: Runtime>(
        self,
        _app_handle: tauri::AppHandle<R>,
        url: String,
    ) -> Result<RedactedLlmConfig, String> {
        fetch_llm_info(&url).await
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

    async fn get_telemetry_bootstrap<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<TelemetryBootstrap, String> {
        let state = app_handle.state::<SharedAppSettings>();
        let mut settings = state.lock().await;

        // Lazily allocate the distinct id the first time the frontend
        // bootstraps after consent. Persist immediately so a crash
        // before the next save doesn't lose the id and accidentally
        // generate a fresh one on the next run.
        let id_changed = if settings.telemetry.needs_consent() {
            false
        } else {
            settings.telemetry.ensure_distinct_id()
        };
        if id_changed {
            settings
                .save_to_default_path()
                .ctx("Failed to persist telemetry distinct id")?;
        }

        let telemetry = settings.telemetry.clone();
        drop(settings);

        Ok(TelemetryBootstrap {
            settings: telemetry,
            sentry_dsn: env!("EURORA_DESKTOP_SENTRY_DSN").to_owned(),
            posthog_key: env!("EURORA_DESKTOP_POSTHOG_KEY").to_owned(),
            posthog_host: env!("EURORA_DESKTOP_POSTHOG_HOST").to_owned(),
            channel: env!("EURORA_RELEASE_CHANNEL").to_owned(),
            release: env!("CARGO_PKG_VERSION").to_owned(),
        })
    }

    async fn needs_telemetry_consent<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<bool, String> {
        let state = app_handle.state::<SharedAppSettings>();
        let settings = state.lock().await;
        Ok(settings.telemetry.needs_consent())
    }

    async fn reinit_telemetry<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<(), String> {
        let settings_state = app_handle.state::<SharedAppSettings>();
        let telemetry = {
            let settings = settings_state.lock().await;
            settings.telemetry.clone()
        };
        let controller = app_handle
            .try_state::<Arc<telemetry::Controller>>()
            .ok_or_else(|| "Telemetry controller not available".to_string())?;
        controller.reapply(&telemetry);
        Ok(())
    }

    async fn rotate_telemetry_distinct_id<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<String, String> {
        let settings_state = app_handle.state::<SharedAppSettings>();
        let mut settings = settings_state.lock().await;
        settings.telemetry.rotate_distinct_id();
        settings
            .save_to_default_path()
            .ctx("Failed to persist rotated telemetry id")?;
        let new_id = settings
            .telemetry
            .distinct_id
            .clone()
            .expect("rotate_distinct_id always populates the id");
        let telemetry = settings.telemetry.clone();
        drop(settings);

        if let Some(controller) = app_handle.try_state::<Arc<telemetry::Controller>>() {
            controller.reapply(&telemetry);
        }
        Ok(new_id)
    }
}
