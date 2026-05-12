//! Desktop / mobile telemetry: local-only `distinct_id` storage plus the
//! product-side consent policy (current version, opt-in derivations).
//!
//! The consent toggles themselves live in [`settings_core::TelemetryConsent`]
//! under each platform section of the cloud blob — see
//! `crates/common/settings-core/src/telemetry.rs`. The two pieces are
//! deliberately split: consent crosses the wire so a user's choice follows
//! them between devices, `distinct_id` does not because rotating it must
//! break cross-device linkage.

use serde::{Deserialize, Serialize};
use settings_core::TelemetryConsent;
use specta::Type;
use uuid::Uuid;

/// Schema version of the telemetry consent prompt the user has agreed to.
/// Bumping this constant forces every user to revisit the prompt the next
/// time they launch the app — the canonical way to ask for consent again
/// when we expand what's collected. `0` means "never asked"; any value
/// below this constant means "asked at an earlier version, must be
/// re-asked".
pub const CURRENT_CONSENT_VERSION: u32 = 1;

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

/// `true` when the user must be shown the consent prompt before any
/// telemetry runs. Drives the onboarding redirect guard on the frontend.
#[must_use]
pub fn needs_consent(consent: &TelemetryConsent) -> bool {
    consent.consent_version < CURRENT_CONSENT_VERSION
}

#[must_use]
pub fn wants_errors(consent: &TelemetryConsent) -> bool {
    !needs_consent(consent) && consent.anonymous_errors
}

#[must_use]
pub fn wants_metrics(consent: &TelemetryConsent) -> bool {
    !needs_consent(consent) && consent.anonymous_metrics
}

/// Anonymous metrics are a precondition for identification — turning on
/// "non-anonymous metrics" without "anonymous metrics" would still
/// produce zero events to identify against.
#[must_use]
pub fn wants_identified(consent: &TelemetryConsent) -> bool {
    wants_metrics(consent) && consent.non_anonymous_metrics
}

/// Stamp the consent version to the current build's value. Idempotent
/// for already-current consent. Called by the settings procedure
/// whenever the user saves their telemetry choices — any save is by
/// definition a recorded consent at the current schema version.
pub fn record_consent(consent: &mut TelemetryConsent) {
    consent.consent_version = CURRENT_CONSENT_VERSION;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_fallback_defaults_require_consent() {
        let c = TelemetryConsent::default();
        assert!(needs_consent(&c));
        assert!(!wants_errors(&c));
        assert!(!wants_metrics(&c));
    }

    #[test]
    fn record_consent_unblocks_capture() {
        let mut c = TelemetryConsent {
            consent_version: 0,
            anonymous_metrics: true,
            anonymous_errors: true,
            non_anonymous_metrics: true,
            ..TelemetryConsent::default()
        };
        assert!(!wants_errors(&c));
        record_consent(&mut c);
        assert!(!needs_consent(&c));
        assert_eq!(c.consent_version, CURRENT_CONSENT_VERSION);
        assert!(wants_errors(&c));
        assert!(wants_identified(&c));
    }

    #[test]
    fn identification_requires_anonymous_metrics() {
        let mut c = TelemetryConsent {
            consent_version: CURRENT_CONSENT_VERSION,
            anonymous_metrics: false,
            anonymous_errors: true,
            non_anonymous_metrics: true,
            ..TelemetryConsent::default()
        };
        assert!(!wants_identified(&c));
        c.anonymous_metrics = true;
        assert!(wants_identified(&c));
    }

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
