use std::sync::Arc;

use euro_settings::{
    APISettings, DesktopSettings, GeneralSettings, SharedSettings, SyncEngine, TelemetryConsent,
    TelemetryLocal, record_consent, wants_errors,
};
use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager};
use thiserror::Error;

use crate::shared_types::{SharedEndpointManager, SharedSettingsState};
use euro_telemetry::Controller as TelemetryController;

/// Typed error surface for the `settings_*` IPC commands. Externally
/// tagged so the JS side gets `{ type: "Persistence", data: "..." }`
/// and can branch on `type` instead of parsing strings. Variants are
/// grouped by failure mode rather than by command, since several
/// commands share `Persistence` (any setter that hits the on-disk
/// settings files).
#[derive(Debug, Error, Serialize, Type)]
#[serde(tag = "type", content = "data")]
pub enum SettingsError {
    #[error("persistence: {0}")]
    Persistence(String),
    #[error("endpoint switch: {0}")]
    EndpointSwitch(String),
}

// --- General (local) ------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn settings_get_general(app_handle: AppHandle) -> GeneralSettings {
    let state = app_handle.state::<SharedSettingsState>();
    state.lock().await.local.general.clone()
}

#[tauri::command]
#[specta::specta]
pub async fn settings_set_general(
    app_handle: AppHandle,
    general_settings: GeneralSettings,
) -> Result<GeneralSettings, SettingsError> {
    let state = app_handle.state::<SharedSettingsState>();
    let mut settings = state.lock().await;

    settings.local.general = general_settings;
    settings
        .save_local_to_default_path()
        .map_err(|e| SettingsError::Persistence(e.to_string()))?;

    Ok(settings.local.general.clone())
}

// --- API endpoint (local) -------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn settings_get_api(app_handle: AppHandle) -> APISettings {
    let state = app_handle.state::<SharedSettingsState>();
    state.lock().await.local.api.clone()
}

#[tauri::command]
#[specta::specta]
pub async fn settings_set_api(
    app_handle: AppHandle,
    api_settings: APISettings,
) -> Result<APISettings, SettingsError> {
    let state = app_handle.state::<SharedSettingsState>();
    let mut settings = state.lock().await;

    settings.local.api = api_settings;
    settings
        .save_local_to_default_path()
        .map_err(|e| SettingsError::Persistence(e.to_string()))?;

    // Translate the persisted connection mode into a concrete URL and
    // hand it to the shared endpoint manager so in-flight requests pick
    // up the change without a restart.
    let endpoint = settings.local.api.endpoint().to_string();
    let endpoint_manager = app_handle.state::<SharedEndpointManager>();
    endpoint_manager
        .set_global_backend_url(&endpoint)
        .map_err(|e| SettingsError::EndpointSwitch(e.to_string()))?;

    Ok(settings.local.api.clone())
}

// --- Shared cloud section -------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn settings_get_shared(app_handle: AppHandle) -> SharedSettings {
    let state = app_handle.state::<SharedSettingsState>();
    state.lock().await.cache.settings.shared.clone()
}

#[tauri::command]
#[specta::specta]
pub async fn settings_set_shared(
    app_handle: AppHandle,
    shared: SharedSettings,
) -> Result<SharedSettings, SettingsError> {
    let state = app_handle.state::<SharedSettingsState>();
    let mut settings = state.lock().await;

    settings.cache.settings.shared = shared;
    settings.cache.settings.sanitize();
    settings
        .save_cache_to_default_path()
        .map_err(|e| SettingsError::Persistence(e.to_string()))?;

    app_handle.state::<SyncEngine>().request_push();

    Ok(settings.cache.settings.shared.clone())
}

// --- Desktop cloud section ------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn settings_get_desktop(app_handle: AppHandle) -> DesktopSettings {
    let state = app_handle.state::<SharedSettingsState>();
    state.lock().await.cache.settings.desktop.clone()
}

/// Write the entire desktop section. The frontend's appearance and
/// telemetry pages each operate on a subset of fields; both target this
/// one command, so a partial-section write is achieved by reading the
/// current section, patching the relevant fields, and writing the whole
/// thing back.
///
/// Side effects: clamps scales via [`DesktopSettings::sanitize`], stamps
/// `telemetry.consent_version` to the current build's value, lazily
/// allocates a local `distinct_id`, and reapplies the native Sentry
/// guard to match the new consent decision.
#[tauri::command]
#[specta::specta]
pub async fn settings_set_desktop(
    app_handle: AppHandle,
    desktop: DesktopSettings,
) -> Result<DesktopSettings, SettingsError> {
    let state = app_handle.state::<SharedSettingsState>();
    let mut settings = state.lock().await;

    settings.cache.settings.desktop = desktop;
    settings.cache.settings.sanitize();
    // Any save through this procedure is by definition a recorded
    // consent at the current schema version. Stamping it here (rather
    // than trusting the frontend) prevents an older client from
    // accidentally pinning the user at a stale version.
    record_consent(&mut settings.cache.settings.desktop.telemetry);
    settings
        .save_cache_to_default_path()
        .map_err(|e| SettingsError::Persistence(e.to_string()))?;

    // Queue the push before the local-only `distinct_id` work below:
    // a failure persisting `local.json` must not suppress propagation
    // of the cache change we have already committed to `cloud.json`.
    app_handle.state::<SyncEngine>().request_push();

    // Lazily allocate a stable distinct id alongside the first consent
    // so that an exit between this call and the next save doesn't drop
    // the id. Touches only `local.json`, so it's a separate write.
    if settings.local.telemetry.ensure_distinct_id() {
        settings
            .save_local_to_default_path()
            .map_err(|e| SettingsError::Persistence(e.to_string()))?;
    }

    let desktop_clone = settings.cache.settings.desktop.clone();
    let enabled = wants_errors(&desktop_clone.telemetry);
    let distinct_id = settings.local.telemetry.distinct_id.clone();
    drop(settings);

    if let Some(controller) = app_handle.try_state::<Arc<TelemetryController>>() {
        controller.reapply(enabled, distinct_id.as_deref());
    }

    Ok(desktop_clone)
}

// --- Telemetry (local) ----------------------------------------------------

/// Returns the per-install telemetry state — currently just the
/// anonymous distinct id. The cross-device consent toggles live under
/// the cloud `desktop` section and are surfaced via
/// [`settings_get_desktop`].
#[tauri::command]
#[specta::specta]
pub async fn settings_get_local_telemetry(app_handle: AppHandle) -> TelemetryLocal {
    let state = app_handle.state::<SharedSettingsState>();
    state.lock().await.local.telemetry.clone()
}

/// Convenience read for the early-boot / pre-auth path where the
/// frontend needs only the consent toggles. Equivalent to
/// `settings_get_desktop().telemetry`, but typed so the IPC surface
/// documents that *only* the consent block crosses this boundary.
#[tauri::command]
#[specta::specta]
pub async fn settings_get_telemetry_consent(app_handle: AppHandle) -> TelemetryConsent {
    let state = app_handle.state::<SharedSettingsState>();
    state.lock().await.cache.settings.desktop.telemetry.clone()
}
