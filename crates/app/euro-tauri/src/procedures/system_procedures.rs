use std::path::PathBuf;
use std::sync::Arc;

use euro_activity::ContextChip;
use euro_bridge::BundledExtensionState;
use euro_process::{Browser, BrowserStore};
use euro_timeline::TimelineManager;
use llm_core::RedactedLlmConfig;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;
use tauri_specta::Event;
use thiserror::Error;
use tokio::sync::Mutex;
use url::Url;

use crate::shared_types::{SharedAppSettings, SharedEndpointManager, SharedHttpClient};
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

/// Typed error surface for the `system_*` IPC commands. Externally tagged
/// so the JS side gets `{ type: "BackendUnreachable", data: "..." }` and
/// can branch on `type` without parsing strings. Variants are grouped by
/// failure mode rather than by command, since several commands share a
/// failure mode (e.g. anything that touches the LLM endpoint can hit
/// `BackendUnreachable` / `BadResponse`).
#[derive(Debug, Error, Serialize, Type)]
#[serde(tag = "type", content = "data")]
pub enum SystemError {
    #[error("backend unreachable: {0}")]
    BackendUnreachable(String),
    #[error("invalid url: {0}")]
    InvalidUrl(String),
    #[error("bad response: {0}")]
    BadResponse(String),
    #[error("updater: {0}")]
    Updater(String),
    #[error("no update available")]
    NoUpdateAvailable,
    #[error("unsupported browser: {0}")]
    UnsupportedBrowser(String),
    #[error("bridge: {0}")]
    Bridge(String),
    #[error("state unavailable: {0}")]
    StateUnavailable(&'static str),
    #[error("persistence: {0}")]
    Persistence(String),
    #[error("window: {0}")]
    Window(String),
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

/// Push event fired whenever a browser's extension state transitions —
/// a native messenger registers/disconnects, or a bundled host publishes
/// a fresh state. The frontend uses this to update the install
/// affordance without polling.
#[derive(Clone, Debug, Serialize, Deserialize, Type, Event)]
pub struct BrowserExtensionStatusChanged {
    pub process_name: String,
    pub state: BrowserExtensionState,
}

/// Authoritative resolver behind `system_get_browser_extension_state`
/// and the push-event side of `spawn_browser_status_bridge`. Pure
/// function over the bridge registry, the bundled-extension state map,
/// and the [`Browser`] catalog — no I/O — so it's straightforward to
/// unit-test and reuse from the event loop.
pub fn resolve_browser_extension_state(process_name: &str) -> BrowserExtensionState {
    let Some(browser) = Browser::from_process_name(process_name) else {
        return BrowserExtensionState::Unsupported;
    };
    let bridge = euro_bridge::BridgeService::get_or_init();

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
    bridge: &euro_bridge::BridgeService,
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
async fn open_safari_extension_settings() -> Result<(), SystemError> {
    use euro_bridge_protocol::BridgeError;

    let bridge = euro_bridge::BridgeService::get_or_init();
    let pid = bridge
        .find_pid_by_app_name(SAFARI_BRIDGE_APP_KIND)
        .ok_or_else(|| SystemError::Bridge("Safari launcher is not connected".to_string()))?;

    match bridge
        .send_request(pid, OPEN_BROWSER_EXTENSION_SETTINGS_ACTION, None)
        .await
    {
        Ok(_) => Ok(()),
        Err(BridgeError::Client { message, .. }) => Err(SystemError::Bridge(format!(
            "Launcher refused to open Safari settings: {message}"
        ))),
        Err(err) => Err(SystemError::Bridge(format!(
            "Bridge error opening Safari settings: {err}"
        ))),
    }
}

#[cfg(not(target_os = "macos"))]
async fn open_safari_extension_settings() -> Result<(), SystemError> {
    Err(SystemError::UnsupportedBrowser(
        "Safari is only supported on macOS".to_string(),
    ))
}

/// Hit `/llm/info` on `base_url` and return the redacted configuration.
/// Shared by `system_get_llm_info` (which uses the configured endpoint)
/// and `system_test_backend_url` (which lets the connection picker probe
/// an arbitrary URL before persisting it).
async fn fetch_llm_info(
    client: &reqwest::Client,
    base_url: &str,
) -> Result<RedactedLlmConfig, SystemError> {
    let parsed = Url::parse(base_url)
        .map_err(|e| SystemError::InvalidUrl(format!("Invalid URL `{base_url}`: {e}")))?;
    let info_url = parsed
        .join("llm/info")
        .map_err(|e| SystemError::InvalidUrl(format!("Failed to derive /llm/info URL: {e}")))?;

    let response = client.get(info_url.clone()).send().await.map_err(|e| {
        SystemError::BackendUnreachable(format!("Request to {info_url} failed: {e}"))
    })?;

    if !response.status().is_success() {
        return Err(SystemError::BadResponse(format!(
            "Backend at {base_url} returned {} on /llm/info",
            response.status()
        )));
    }

    response
        .json::<RedactedLlmConfig>()
        .await
        .map_err(|e| SystemError::BadResponse(format!("Failed to parse /llm/info response: {e}")))
}

#[tauri::command]
#[specta::specta]
pub async fn system_check_backend_connection(
    server_address: Option<String>,
) -> Result<String, SystemError> {
    // Default to the build-time-baked endpoint when the frontend
    // doesn't pass an explicit address (e.g., the very first
    // reachability probe before connection mode is resolved).
    // Strip the scheme so the result is a `host:port` suitable
    // for `TcpStream::connect`.
    let address = server_address.unwrap_or_else(|| euro_settings::DEFAULT_API_URL.to_string());
    let host_port = address.replace("http://", "").replace("https://", "");

    tracing::debug!("Checking TCP reachability of {host_port}");

    match tokio::net::TcpStream::connect(&host_port).await {
        Ok(_) => {
            tracing::debug!("TCP connection successful");
            Ok("Server is reachable".to_string())
        }
        Err(e) => {
            tracing::debug!("Failed to connect to server: {e}");
            Err(SystemError::BackendUnreachable(format!(
                "Failed to connect to server: {e}"
            )))
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn system_get_llm_info(app_handle: AppHandle) -> Result<RedactedLlmConfig, SystemError> {
    let endpoint_manager = app_handle.state::<SharedEndpointManager>();
    let url = endpoint_manager.current_url().to_string();
    let client = app_handle.state::<SharedHttpClient>().inner().clone();
    fetch_llm_info(&client, &url).await
}

/// Probe an arbitrary URL by hitting /llm/info — used by the connection
/// picker's "Test connection" button before persisting the URL.
#[tauri::command]
#[specta::specta]
pub async fn system_test_backend_url(
    app_handle: AppHandle,
    url: String,
) -> Result<RedactedLlmConfig, SystemError> {
    let client = app_handle.state::<SharedHttpClient>().inner().clone();
    fetch_llm_info(&client, &url).await
}

/// The backend URL the desktop binary was built against. Surfaced to
/// the frontend so the connection picker can show what `Default` mode
/// resolves to — the value differs per release (dev builds point at
/// localhost, end-user binaries at the shipping organisation's hosted
/// backend).
#[tauri::command]
#[specta::specta]
pub async fn system_get_default_backend_url() -> String {
    euro_settings::DEFAULT_API_URL.to_string()
}

#[tauri::command]
#[specta::specta]
pub async fn system_list_activities(
    app_handle: AppHandle,
) -> Result<Vec<ContextChip>, SystemError> {
    let timeline_state: tauri::State<'_, Mutex<TimelineManager>> = app_handle
        .try_state()
        .ok_or(SystemError::StateUnavailable("timeline"))?;
    let timeline = timeline_state.lock().await;
    Ok(timeline.get_context_chip().await.into_iter().collect())
}

#[tauri::command]
#[specta::specta]
pub async fn system_check_for_update(
    app_handle: AppHandle,
) -> Result<Option<UpdateInfo>, SystemError> {
    tracing::debug!("Checking for updates...");

    let updater = app_handle.updater().map_err(|e| {
        tracing::error!("Failed to get updater: {e}");
        SystemError::Updater(format!("Failed to get updater: {e}"))
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
            Err(SystemError::Updater(format!(
                "Failed to check for updates: {e}"
            )))
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn system_install_update(app_handle: AppHandle) -> Result<(), SystemError> {
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
        SystemError::Updater(format!("Failed to build updater: {e}"))
    })?;

    let update = updater.check().await.map_err(|e| {
        tracing::error!("Failed to check for updates: {e}");
        SystemError::Updater(format!("Failed to check for updates: {e}"))
    })?;

    let Some(update) = update else {
        tracing::debug!("No update available to install");
        return Err(SystemError::NoUpdateAvailable);
    };

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
            SystemError::Updater(format!("Failed to download and install update: {e}"))
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
}

#[tauri::command]
#[specta::specta]
pub async fn system_quit(app_handle: AppHandle) {
    app_handle.exit(0);
}

#[tauri::command]
#[specta::specta]
pub async fn system_check_accessibility_permission() -> bool {
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

        unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) }
    }

    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

#[tauri::command]
#[specta::specta]
pub async fn system_request_accessibility_permission() {
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
    }
}

#[tauri::command]
#[specta::specta]
pub async fn system_get_browser_extension_state(process_name: String) -> BrowserExtensionState {
    resolve_browser_extension_state(&process_name)
}

#[tauri::command]
#[specta::specta]
pub async fn system_open_browser_extension_settings(
    process_name: String,
) -> Result<(), SystemError> {
    let browser = Browser::from_process_name(&process_name).ok_or_else(|| {
        SystemError::UnsupportedBrowser(format!("Unknown browser: {process_name:?}"))
    })?;
    match browser {
        Browser::Safari => open_safari_extension_settings().await,
        // No other browser exposes a stable deep-link to its extension
        // settings page, and store-distributed extensions can't be in
        // an "installed-but-disabled" state from the desktop's
        // perspective anyway (a disabled extension stops connecting,
        // which we surface as `NotInstalled`). Failing loudly here is
        // cheaper than silently hiding a frontend bug.
        other => Err(SystemError::UnsupportedBrowser(format!(
            "open_browser_extension_settings is not supported for {other:?}"
        ))),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn system_open_url_in_browser(process_id: u32, url: String) -> Result<(), SystemError> {
    crate::browser_launcher::open_url_in_process(process_id, &url).map_err(SystemError::Bridge)
}

#[tauri::command]
#[specta::specta]
pub async fn system_focus_main_window(app_handle: AppHandle) -> Result<(), SystemError> {
    crate::window::show_and_focus_main(&app_handle).map_err(|e| SystemError::Window(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn system_get_telemetry_bootstrap(
    app_handle: AppHandle,
) -> Result<TelemetryBootstrap, SystemError> {
    let state = app_handle.state::<SharedAppSettings>();
    let mut settings = state.lock().await;

    // Lazily allocate the distinct id the first time the frontend
    // bootstraps after consent. Persist immediately so a crash before
    // the next save doesn't lose the id and accidentally generate a
    // fresh one on the next run.
    let id_changed = if settings.telemetry.needs_consent() {
        false
    } else {
        settings.telemetry.ensure_distinct_id()
    };
    if id_changed {
        settings.save_to_default_path().map_err(|e| {
            SystemError::Persistence(format!("Failed to persist telemetry distinct id: {e}"))
        })?;
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

#[tauri::command]
#[specta::specta]
pub async fn system_needs_telemetry_consent(app_handle: AppHandle) -> bool {
    let state = app_handle.state::<SharedAppSettings>();
    let settings = state.lock().await;
    settings.telemetry.needs_consent()
}

#[tauri::command]
#[specta::specta]
pub async fn system_reinit_telemetry(app_handle: AppHandle) {
    let settings_state = app_handle.state::<SharedAppSettings>();
    let telemetry = {
        let settings = settings_state.lock().await;
        settings.telemetry.clone()
    };
    let controller = app_handle.state::<Arc<telemetry::Controller>>();
    controller.reapply(&telemetry);
}

#[tauri::command]
#[specta::specta]
pub async fn system_rotate_telemetry_distinct_id(
    app_handle: AppHandle,
) -> Result<String, SystemError> {
    let settings_state = app_handle.state::<SharedAppSettings>();
    let mut settings = settings_state.lock().await;
    settings.telemetry.rotate_distinct_id();
    settings.save_to_default_path().map_err(|e| {
        SystemError::Persistence(format!("Failed to persist rotated telemetry id: {e}"))
    })?;
    let new_id = settings
        .telemetry
        .distinct_id
        .clone()
        .expect("rotate_distinct_id always populates the id");
    let telemetry = settings.telemetry.clone();
    drop(settings);

    let controller = app_handle.state::<Arc<telemetry::Controller>>();
    controller.reapply(&telemetry);
    Ok(new_id)
}
