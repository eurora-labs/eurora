//! Borrowed read-only view over the split [`LocalSettings`] +
//! [`CloudSettingsCache`] state.
//!
//! IPC handlers lock the [`crate::SettingsState`] mutex once, construct
//! an [`EffectiveSettings`], read whichever sections they need, and
//! drop the lock — no clones until the handler decides to hand a value
//! back across the wire.

use settings_core::{DesktopSettings, SharedSettings, TelemetryConsent};

use crate::{
    cloud_cache::CloudSettingsCache, local::LocalSettings, telemetry as telemetry_policy,
};

/// Composite read-only view over a [`LocalSettings`] / [`CloudSettingsCache`]
/// pair held inside a [`crate::SettingsState`].
pub struct EffectiveSettings<'a> {
    pub local: &'a LocalSettings,
    pub cache: &'a CloudSettingsCache,
}

impl<'a> EffectiveSettings<'a> {
    pub fn new(local: &'a LocalSettings, cache: &'a CloudSettingsCache) -> Self {
        Self { local, cache }
    }

    pub fn shared(&self) -> &'a SharedSettings {
        &self.cache.settings.shared
    }

    pub fn desktop(&self) -> &'a DesktopSettings {
        &self.cache.settings.desktop
    }

    pub fn telemetry_consent(&self) -> &'a TelemetryConsent {
        &self.cache.settings.desktop.telemetry
    }

    pub fn telemetry_distinct_id(&self) -> Option<&'a str> {
        self.local.telemetry.distinct_id.as_deref()
    }

    /// `true` when the user must be shown the consent prompt before any
    /// telemetry runs on this platform.
    pub fn needs_telemetry_consent(&self) -> bool {
        telemetry_policy::needs_consent(self.telemetry_consent())
    }

    /// `true` when the user has consented and opted into anonymous error
    /// reporting. Drives the native Sentry guard.
    pub fn wants_telemetry_errors(&self) -> bool {
        telemetry_policy::wants_errors(self.telemetry_consent())
    }
}
