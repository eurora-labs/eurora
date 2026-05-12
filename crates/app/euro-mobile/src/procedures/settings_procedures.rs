//! Settings IPC commands for the mobile app.
//!
//! Only the telemetry-touching subset is exposed today — the rest of
//! the local + cloud sections (general, appearance, API endpoint)
//! doesn't yet have a mobile UI, so adding commands for them is
//! deferred until the mobile settings page lands. The commands below
//! mirror their desktop counterparts in
//! `euro-tauri::procedures::settings_procedures` and must stay
//! shape-compatible so the mobile webview's settings page can share
//! types with the desktop one.
//!
//! Today the desktop `TelemetryConsent` record (under
//! `cloud.settings.desktop.telemetry`) doubles as the mobile consent
//! source. Phase 10 partitions mobile into its own section once a
//! mobile telemetry stack ships.

use std::sync::Arc;

use euro_settings::{TelemetryConsent, TelemetryLocal, record_consent, wants_errors};
use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager};
use thiserror::Error;

use crate::shared_types::SharedSettingsState;
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
pub async fn settings_get_telemetry_consent(app_handle: AppHandle) -> TelemetryConsent {
    let state = app_handle.state::<SharedSettingsState>();
    state.lock().await.cache.settings.desktop.telemetry.clone()
}

#[tauri::command]
#[specta::specta]
pub async fn settings_get_local_telemetry(app_handle: AppHandle) -> TelemetryLocal {
    let state = app_handle.state::<SharedSettingsState>();
    state.lock().await.local.telemetry.clone()
}

/// Persist a fresh consent decision. Stamps the consent version, lazily
/// allocates a stable `distinct_id`, reapplies the native Sentry guard,
/// and returns the canonical post-stamp consent so the frontend's
/// optimistic state stays in sync.
#[tauri::command]
#[specta::specta]
pub async fn settings_set_telemetry_consent(
    app_handle: AppHandle,
    consent: TelemetryConsent,
) -> Result<TelemetryConsent, SettingsError> {
    let state = app_handle.state::<SharedSettingsState>();
    let mut settings = state.lock().await;

    settings.cache.settings.desktop.telemetry = consent;
    record_consent(&mut settings.cache.settings.desktop.telemetry);
    settings
        .save_cache_to_default_path()
        .map_err(|e| SettingsError::Persistence(e.to_string()))?;

    if settings.local.telemetry.ensure_distinct_id() {
        settings
            .save_local_to_default_path()
            .map_err(|e| SettingsError::Persistence(e.to_string()))?;
    }

    let consent_out = settings.cache.settings.desktop.telemetry.clone();
    let enabled = wants_errors(&consent_out);
    let distinct_id = settings.local.telemetry.distinct_id.clone();
    drop(settings);

    if let Some(controller) = app_handle.try_state::<Arc<TelemetryController>>() {
        controller.reapply(enabled, distinct_id.as_deref());
    }

    Ok(consent_out)
}
