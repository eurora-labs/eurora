//! Local-only telemetry state: the anonymous `distinct_id`.
//!
//! The consent toggles themselves live in [`settings_core::TelemetryConsent`]
//! under each platform section of the cloud blob, and all consent-related
//! policy (whether the SDKs may run, whether the consent prompt is
//! required, monotonic recording) is implemented on the consent struct
//! itself — see `crates/common/settings-core/src/telemetry.rs`. The two
//! pieces are split deliberately: consent crosses the wire so a user's
//! choice follows them between devices, `distinct_id` does not because
//! rotating it must break cross-device linkage.

use serde::{Deserialize, Serialize};
use specta::Type;
use uuid::Uuid;

/// Local-only telemetry state. Persisted next to the rest of
/// [`crate::LocalSettings`] in `local.json`; never crosses the wire to
/// the cloud-sync backend.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct TelemetryLocal {
    /// Anonymous per-install identifier. `None` until the user accepts
    /// telemetry for the first time, then a fresh UUID v4 that survives
    /// until the user explicitly rotates it.
    pub distinct_id: Option<String>,
}

impl TelemetryLocal {
    /// Lazily fill `distinct_id` with a fresh UUID v4. Returns `true` if
    /// the field was mutated so callers can decide whether to persist.
    pub fn ensure_distinct_id(&mut self) -> bool {
        if self.distinct_id.is_none() {
            self.distinct_id = Some(Uuid::new_v4().to_string());
            true
        } else {
            false
        }
    }

    /// Replace the persisted distinct id with a fresh one. Used on logout
    /// or from the settings UI's "reset telemetry id" affordance to break
    /// linkage between sessions.
    pub fn rotate_distinct_id(&mut self) {
        self.distinct_id = Some(Uuid::new_v4().to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_distinct_id_is_idempotent_after_first_call() {
        let mut local = TelemetryLocal::default();
        assert!(local.ensure_distinct_id());
        let id = local.distinct_id.clone();
        assert!(!local.ensure_distinct_id());
        assert_eq!(local.distinct_id, id);
    }

    #[test]
    fn rotate_distinct_id_replaces_existing_value() {
        let mut local = TelemetryLocal::default();
        local.ensure_distinct_id();
        let first = local.distinct_id.clone();
        local.rotate_distinct_id();
        assert!(local.distinct_id.is_some());
        assert_ne!(local.distinct_id, first);
    }
}
