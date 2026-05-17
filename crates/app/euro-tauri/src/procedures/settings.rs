use std::sync::Arc;

use euro_settings::{
    APISettings, DesktopSettings, GeneralSettings, SharedSettings, SyncEngine, TelemetryConsent,
    TelemetryLocal,
};
use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use thiserror::Error;

use crate::procedures::system::ConsentGate;
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

/// Write the entire desktop section. Used by the appearance page and
/// anything else that mutates non-consent desktop state — the consent
/// prompt has its own dedicated procedure
/// ([`settings_record_telemetry_consent`]) so this one does **not** stamp
/// a consent version.
///
/// `consent_version` is monotonically non-decreasing: incoming values
/// below the stored version are clamped up. This protects against a
/// misbehaving caller (or an older client) accidentally rolling back
/// the user's recorded consent while editing unrelated fields. Scale
/// fields are clamped at the IPC boundary by their newtypes
/// (`InterfaceScale` / `TextScale`), so no extra validation pass is
/// required here either.
#[tauri::command]
#[specta::specta]
pub async fn settings_set_desktop(
    app_handle: AppHandle,
    desktop: DesktopSettings,
) -> Result<DesktopSettings, SettingsError> {
    let state = app_handle.state::<SharedSettingsState>();
    let mut settings = state.lock().await;

    let prior_consent_version = settings.cache.settings.desktop.telemetry.consent_version;
    settings.cache.settings.desktop = desktop;
    settings.cache.settings.desktop.telemetry.consent_version = settings
        .cache
        .settings
        .desktop
        .telemetry
        .consent_version
        .max(prior_consent_version);

    settings
        .save_cache_to_default_path()
        .map_err(|e| SettingsError::Persistence(e.to_string()))?;

    app_handle.state::<SyncEngine>().request_push();

    Ok(settings.cache.settings.desktop.clone())
}

/// Persist the user's response to the desktop telemetry consent prompt.
///
/// The frontend hands the desired toggle state in `consent`; the backend
/// authoritatively stamps the consent version via
/// [`TelemetryConsent::record_for_desktop`] (monotonic — a newer
/// recorded version from another client is left alone). Lazily allocates
/// the local anonymous `distinct_id`, reapplies the native Sentry guard
/// so the new choice takes effect immediately, and emits
/// [`ConsentGate`] with `required: false` so the frontend can route
/// away from the consent page.
#[tauri::command]
#[specta::specta]
pub async fn settings_record_telemetry_consent(
    app_handle: AppHandle,
    consent: TelemetryConsent,
) -> Result<TelemetryConsent, SettingsError> {
    let state = app_handle.state::<SharedSettingsState>();
    let mut settings = state.lock().await;

    settings.cache.settings.desktop.telemetry = consent;
    settings
        .cache
        .settings
        .desktop
        .telemetry
        .record_for_desktop();
    settings
        .save_cache_to_default_path()
        .map_err(|e| SettingsError::Persistence(e.to_string()))?;

    app_handle.state::<SyncEngine>().request_push();

    if settings.local.telemetry.ensure_distinct_id() {
        settings
            .save_local_to_default_path()
            .map_err(|e| SettingsError::Persistence(e.to_string()))?;
    }

    let consent_out = settings.cache.settings.desktop.telemetry.clone();
    let enabled = consent_out.allows_errors_on_desktop();
    let distinct_id = settings.local.telemetry.distinct_id.clone();
    drop(settings);

    if let Some(controller) = app_handle.try_state::<Arc<TelemetryController>>() {
        controller.reapply(enabled, distinct_id.as_deref());
    }

    // Recording consent always clears the gate. Emitted after persistence
    // so a listener that immediately re-queries observes the new state.
    let _ = ConsentGate { required: false }.emit(&app_handle);

    Ok(consent_out)
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
