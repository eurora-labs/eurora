use std::sync::Arc;

use euro_settings::{
    APISettings, AppSettings, AppearanceSettings, GeneralSettings, TelemetrySettings,
};
use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager};
use thiserror::Error;

use crate::shared_types::{SharedAppSettings, SharedEndpointManager};
use crate::telemetry;

/// Typed error surface for the `settings_*` IPC commands. Externally
/// tagged so the JS side gets `{ type: "Persistence", data: "..." }`
/// and can branch on `type` instead of parsing strings. Variants are
/// grouped by failure mode rather than by command, since several
/// commands share `Persistence` (any setter that hits the on-disk
/// settings file).
#[derive(Debug, Error, Serialize, Type)]
#[serde(tag = "type", content = "data")]
pub enum SettingsError {
    #[error("persistence: {0}")]
    Persistence(String),
    #[error("endpoint switch: {0}")]
    EndpointSwitch(String),
}

#[tauri::command]
#[specta::specta]
pub async fn settings_get_all(app_handle: AppHandle) -> AppSettings {
    let state = app_handle.state::<SharedAppSettings>();
    let settings = state.lock().await;
    settings.clone()
}

#[tauri::command]
#[specta::specta]
pub async fn settings_get_telemetry(app_handle: AppHandle) -> TelemetrySettings {
    let state = app_handle.state::<SharedAppSettings>();
    let settings = state.lock().await;
    settings.telemetry.clone()
}

#[tauri::command]
#[specta::specta]
pub async fn settings_set_telemetry(
    app_handle: AppHandle,
    telemetry_settings: TelemetrySettings,
) -> Result<TelemetrySettings, SettingsError> {
    let state = app_handle.state::<SharedAppSettings>();
    let mut settings = state.lock().await;

    settings.telemetry = telemetry_settings;
    // Any save through this procedure is by definition a recorded
    // consent at the current schema version. Stamping it here
    // (rather than trusting the frontend) prevents an old client
    // from accidentally pinning the user at a stale version.
    settings.telemetry.record_consent();
    // Allocate a stable id alongside the first consent so that
    // an exit between this call and the next save doesn't drop
    // the id.
    settings.telemetry.ensure_distinct_id();
    settings
        .save_to_default_path()
        .map_err(|e| SettingsError::Persistence(e.to_string()))?;

    let new_telemetry = settings.telemetry.clone();
    drop(settings);

    if let Some(controller) = app_handle.try_state::<Arc<telemetry::Controller>>() {
        controller.reapply(&new_telemetry);
    }

    Ok(new_telemetry)
}

#[tauri::command]
#[specta::specta]
pub async fn settings_get_general(app_handle: AppHandle) -> GeneralSettings {
    let state = app_handle.state::<SharedAppSettings>();
    let settings = state.lock().await;
    settings.general.clone()
}

#[tauri::command]
#[specta::specta]
pub async fn settings_set_general(
    app_handle: AppHandle,
    general_settings: GeneralSettings,
) -> Result<GeneralSettings, SettingsError> {
    let state = app_handle.state::<SharedAppSettings>();
    let mut settings = state.lock().await;

    settings.general = general_settings;
    settings
        .save_to_default_path()
        .map_err(|e| SettingsError::Persistence(e.to_string()))?;

    Ok(settings.general.clone())
}

#[tauri::command]
#[specta::specta]
pub async fn settings_get_api(app_handle: AppHandle) -> APISettings {
    let state = app_handle.state::<SharedAppSettings>();
    let settings = state.lock().await;
    settings.api.clone()
}

#[tauri::command]
#[specta::specta]
pub async fn settings_set_api(
    app_handle: AppHandle,
    api_settings: APISettings,
) -> Result<APISettings, SettingsError> {
    let state = app_handle.state::<SharedAppSettings>();
    let mut settings = state.lock().await;

    settings.api = api_settings;
    settings
        .save_to_default_path()
        .map_err(|e| SettingsError::Persistence(e.to_string()))?;

    // Translate the persisted connection mode into a concrete URL and
    // hand it to the shared endpoint manager so in-flight requests pick
    // up the change without a restart.
    let endpoint = settings.api.endpoint().to_string();
    let endpoint_manager = app_handle.state::<SharedEndpointManager>();
    endpoint_manager
        .set_global_backend_url(&endpoint)
        .map_err(|e| SettingsError::EndpointSwitch(e.to_string()))?;

    Ok(settings.api.clone())
}

#[tauri::command]
#[specta::specta]
pub async fn settings_get_appearance(app_handle: AppHandle) -> AppearanceSettings {
    let state = app_handle.state::<SharedAppSettings>();
    let settings = state.lock().await;
    settings.appearance.clone()
}

#[tauri::command]
#[specta::specta]
pub async fn settings_set_appearance(
    app_handle: AppHandle,
    appearance_settings: AppearanceSettings,
) -> Result<AppearanceSettings, SettingsError> {
    let state = app_handle.state::<SharedAppSettings>();
    let mut settings = state.lock().await;

    // Clamp scales and reject NaN/inf at the API boundary so a buggy
    // client or hand-edited config can't push the UI into an unusable
    // state. The frontend already constrains its sliders, so this is a
    // defensive backstop, not the primary validation path.
    let mut appearance_settings = appearance_settings;
    appearance_settings.sanitize();

    settings.appearance = appearance_settings;
    settings
        .save_to_default_path()
        .map_err(|e| SettingsError::Persistence(e.to_string()))?;

    Ok(settings.appearance.clone())
}
