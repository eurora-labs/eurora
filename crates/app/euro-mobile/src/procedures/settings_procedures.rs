//! Settings IPC commands for the mobile app.
//!
//! Only the telemetry-touching subset is exposed today — the rest of
//! `AppSettings` (general, appearance, API endpoint) doesn't yet have a
//! mobile UI, so adding commands for them is deferred until the
//! settings refactor lands. The two commands below mirror their desktop
//! counterparts in `euro-tauri::procedures::settings_procedures` and
//! must stay shape-compatible so the mobile webview's settings page
//! can share types with the desktop one.

use std::sync::Arc;

use euro_settings::TelemetrySettings;
use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager};
use thiserror::Error;

use crate::shared_types::SharedAppSettings;
use euro_telemetry::Controller as TelemetryController;

/// Typed error surface for the `settings_*` IPC commands. Externally
/// tagged so the JS side gets `{ type: "Persistence", data: "..." }`
/// and can branch on `type` instead of parsing strings.
#[derive(Debug, Error, Serialize, Type)]
#[serde(tag = "type", content = "data")]
pub enum SettingsError {
    #[error("persistence: {0}")]
    Persistence(String),
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
    // consent at the current schema version. Stamping it here (rather
    // than trusting the frontend) prevents an old client from
    // accidentally pinning the user at a stale version.
    settings.telemetry.record_consent();
    // Allocate a stable id alongside the first consent so an exit
    // between this call and the next save doesn't drop the id.
    settings.telemetry.ensure_distinct_id();
    settings
        .save_to_default_path()
        .map_err(|e| SettingsError::Persistence(e.to_string()))?;

    let new_telemetry = settings.telemetry.clone();
    drop(settings);

    if let Some(controller) = app_handle.try_state::<Arc<TelemetryController>>() {
        controller.reapply(&new_telemetry);
    }

    Ok(new_telemetry)
}
