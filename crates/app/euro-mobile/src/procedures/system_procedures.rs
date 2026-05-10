//! System IPC commands for the mobile app.
//!
//! Only the telemetry surface is exposed today; everything else lives
//! on the desktop and isn't reachable on mobile. Commands mirror their
//! desktop counterparts in `euro-tauri::procedures::system_procedures`
//! and must stay shape-compatible so the mobile webview can share
//! types with the desktop one.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use thiserror::Error;

use crate::shared_types::SharedAppSettings;
use euro_telemetry::Controller as TelemetryController;

/// Typed error surface for the `system_*` IPC commands. Externally
/// tagged so the JS side gets `{ type: "Persistence", data: "..." }`
/// and can branch on `type` without parsing strings.
#[derive(Debug, Error, Serialize, Type)]
#[serde(tag = "type", content = "data")]
pub enum SystemError {
    #[error("persistence: {0}")]
    Persistence(String),
}

/// Single payload the mobile frontend fetches once at startup to bring
/// up its Sentry / PostHog SDKs. Bundles the user's persisted consent
/// state, the embedded build-time keys, and the release identity so the
/// SDKs can tag events with channel + version.
///
/// `None` on any field means "this surface is disabled in this build".
/// `euro-telemetry/build.rs` enforces all-or-nothing consistency: a
/// build with a DSN always carries a channel and a release, so the
/// frontend never has to defend against a half-configured payload.
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryBootstrap {
    pub settings: euro_settings::TelemetrySettings,
    pub sentry_dsn: Option<String>,
    pub posthog_key: Option<String>,
    pub posthog_host: Option<String>,
    pub channel: Option<String>,
    pub release: Option<String>,
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
        sentry_dsn: euro_telemetry::non_empty(euro_telemetry::SENTRY_DSN).map(str::to_owned),
        posthog_key: euro_telemetry::non_empty(euro_telemetry::POSTHOG_KEY).map(str::to_owned),
        posthog_host: euro_telemetry::non_empty(euro_telemetry::POSTHOG_HOST).map(str::to_owned),
        channel: euro_telemetry::non_empty(euro_telemetry::RELEASE_CHANNEL).map(str::to_owned),
        release: euro_telemetry::non_empty(euro_telemetry::RELEASE_VERSION).map(str::to_owned),
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
    let controller = app_handle.state::<Arc<TelemetryController>>();
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

    let controller = app_handle.state::<Arc<TelemetryController>>();
    controller.reapply(&telemetry);
    Ok(new_id)
}
