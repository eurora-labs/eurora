//! Desktop / mobile app-side settings.
//!
//! The crate owns three pieces:
//!
//! - [`LocalSettings`] — per-install state persisted to `local.json`
//!   (autostart, API endpoint, telemetry distinct id).
//! - [`CloudSettingsCache`] — last-pulled mirror of the user-scoped cloud
//!   settings blob, persisted to `cloud.json`. The wire shape lives in
//!   the `settings-core` crate; this file is just a local cache plus
//!   the `last_user_id` the sync engine uses for account-isolation
//!   and the OCC baseline (`base_updated_at`) it sends on the next PUT.
//! - [`SettingsState`] — combined owner held inside `tauri::Manager`
//!   state under a `tokio::sync::Mutex`. IPC handlers lock it, read or
//!   mutate, and persist the affected file via [`SettingsState::save_local`]
//!   or [`SettingsState::save_cache`].
//!
//! The wire-format types ([`settings_core::SharedSettings`],
//! [`settings_core::DesktopSettings`], [`settings_core::TelemetryConsent`])
//! are re-exported here as a convenience; consumers can also `use
//! settings_core::...` directly. The crate deliberately does **not**
//! define any "page-shaped" composite (`AppearanceSettings`,
//! `TelemetrySettings`) — the frontend composes per-page state from the
//! section types so adding a field to a section in `settings-core`
//! propagates without an app-side duplicate to also update.

pub mod api;
pub mod cloud_cache;
pub mod effective;
pub mod general;
pub mod local;
pub mod persistence;
pub mod state;
pub mod sync;
pub mod telemetry;

pub use api::{APISettings, ConnectionMode, DEFAULT_API_URL};
pub use cloud_cache::CloudSettingsCache;
pub use effective::EffectiveSettings;
pub use general::GeneralSettings;
pub use local::LocalSettings;
pub use persistence::default_config_dir;
pub use state::SettingsState;
pub use sync::{
    BackoffConfig, PullOutcome, PushOutcome, ReqwestTransport, SettingsTransport, SyncEngine,
    SyncError, SyncResult, SyncStatus,
};
pub use telemetry::{
    CURRENT_CONSENT_VERSION, TelemetryLocal, needs_consent, record_consent, wants_errors,
    wants_identified, wants_metrics,
};

// Wire types from settings-core that IPC handlers and the frontend
// bindings consume directly. Re-exported so app crates can take a
// single dependency on `euro-settings` without having to also add
// `settings-core` to their Cargo.toml.
pub use settings_core::{
    CURRENT_SCHEMA_VERSION, CloudSettings, DEFAULT_SCALE, DesktopSettings, MAX_SCALE, MIN_SCALE,
    MobileSettings, SharedSettings, TelemetryConsent, ThemePreference, WebSettings, sanitize_scale,
};
