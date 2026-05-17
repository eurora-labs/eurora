//! System IPC commands for the mobile app.
//!
//! Only the telemetry surface is exposed today; everything else lives
//! on the desktop and isn't reachable on mobile. Commands mirror their
//! desktop counterparts in `euro-tauri::procedures::system`
//! and must stay shape-compatible so the mobile webview can share
//! types with the desktop one.

use std::sync::Arc;

use euro_settings::TelemetryConsent;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use thiserror::Error;

use crate::shared_types::SharedSettingsState;
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
/// up its Sentry / PostHog SDKs. Bundles the user's persisted consent,
/// the local anonymous identifier, the embedded build-time keys, the
/// release identity, and **precomputed policy decisions** so the
/// frontend doesn't reproduce the consent-gating rules in TypeScript.
///
/// `None` on any build-time field means "this surface is disabled in
/// this build". `euro-telemetry/build.rs` enforces all-or-nothing
/// consistency: a build with a DSN always carries a channel and a
/// release, so the frontend never has to defend against a
/// half-configured payload.
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryBootstrap {
    pub consent: TelemetryConsent,
    pub distinct_id: Option<String>,
    pub sentry_dsn: Option<String>,
    pub posthog_key: Option<String>,
    pub posthog_host: Option<String>,
    pub channel: Option<String>,
    pub release: Option<String>,
    pub allows_errors: bool,
    pub allows_metrics: bool,
    pub allows_identification: bool,
}

/// Pushed from Rust to the frontend whenever the mobile telemetry
/// consent gate flips. Fired once during startup (in response to
/// [`frontend_ready`]) and again whenever the gate changes (e.g. after
/// [`crate::procedures::settings::settings_record_telemetry_consent`]).
///
/// Today mobile shares the `desktop.telemetry` cloud record with the
/// desktop client; Phase 10 partitions mobile into its own section
/// once a mobile telemetry stack ships.
#[derive(Clone, Debug, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct ConsentGate {
    pub required: bool,
}

#[tauri::command]
#[specta::specta]
pub async fn system_get_telemetry_bootstrap(
    app_handle: AppHandle,
) -> Result<TelemetryBootstrap, SystemError> {
    let state = app_handle.state::<SharedSettingsState>();
    let mut settings = state.lock().await;

    // Lazily allocate the distinct id the first time the frontend
    // bootstraps after consent. Persist immediately so a crash before
    // the next save doesn't lose the id and accidentally generate a
    // fresh one on the next run.
    let consent_recorded = settings
        .cache
        .settings
        .desktop
        .telemetry
        .is_recorded_for_desktop();
    let id_changed = consent_recorded && settings.local.telemetry.ensure_distinct_id();
    if id_changed {
        settings.save_local_to_default_path().map_err(|e| {
            SystemError::Persistence(format!("Failed to persist telemetry distinct id: {e}"))
        })?;
    }

    let consent = settings.cache.settings.desktop.telemetry.clone();
    let distinct_id = settings.local.telemetry.distinct_id.clone();
    drop(settings);

    Ok(TelemetryBootstrap {
        allows_errors: consent.allows_errors_on_desktop(),
        allows_metrics: consent.allows_metrics_on_desktop(),
        allows_identification: consent.allows_identification_on_desktop(),
        consent,
        distinct_id,
        sentry_dsn: euro_telemetry::non_empty(euro_telemetry::SENTRY_DSN).map(str::to_owned),
        posthog_key: euro_telemetry::non_empty(euro_telemetry::POSTHOG_KEY).map(str::to_owned),
        posthog_host: euro_telemetry::non_empty(euro_telemetry::POSTHOG_HOST).map(str::to_owned),
        channel: euro_telemetry::non_empty(euro_telemetry::RELEASE_CHANNEL).map(str::to_owned),
        release: euro_telemetry::non_empty(euro_telemetry::RELEASE_VERSION).map(str::to_owned),
    })
}

/// Frontend handshake: called from the root layout once event listeners
/// are attached. The backend reads the current consent state and emits
/// [`ConsentGate`] with `required` set accordingly. See the desktop
/// equivalent for the design rationale.
#[tauri::command]
#[specta::specta]
pub async fn frontend_ready(app_handle: AppHandle) -> Result<(), SystemError> {
    let state = app_handle.state::<SharedSettingsState>();
    let required = {
        let settings = state.lock().await;
        settings
            .cache
            .settings
            .desktop
            .telemetry
            .needs_prompt_for_desktop()
    };
    let _ = ConsentGate { required }.emit(&app_handle);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn system_reinit_telemetry(app_handle: AppHandle) {
    let state = app_handle.state::<SharedSettingsState>();
    let (enabled, distinct_id) = {
        let settings = state.lock().await;
        (
            settings
                .cache
                .settings
                .desktop
                .telemetry
                .allows_errors_on_desktop(),
            settings.local.telemetry.distinct_id.clone(),
        )
    };
    let controller = app_handle.state::<Arc<TelemetryController>>();
    controller.reapply(enabled, distinct_id.as_deref());
}

#[tauri::command]
#[specta::specta]
pub async fn system_rotate_telemetry_distinct_id(
    app_handle: AppHandle,
) -> Result<String, SystemError> {
    let state = app_handle.state::<SharedSettingsState>();
    let mut settings = state.lock().await;
    settings.local.telemetry.rotate_distinct_id();
    settings.save_local_to_default_path().map_err(|e| {
        SystemError::Persistence(format!("Failed to persist rotated telemetry id: {e}"))
    })?;
    let new_id = settings
        .local
        .telemetry
        .distinct_id
        .clone()
        .expect("rotate_distinct_id always populates the id");
    let enabled = settings
        .cache
        .settings
        .desktop
        .telemetry
        .allows_errors_on_desktop();
    drop(settings);

    let controller = app_handle.state::<Arc<TelemetryController>>();
    controller.reapply(enabled, Some(new_id.as_str()));
    Ok(new_id)
}
