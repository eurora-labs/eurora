use serde::{Deserialize, Serialize};
use specta::Type;
use uuid::Uuid;

/// Schema version of the telemetry consent prompt the user has agreed
/// to. Bumping this constant forces every user to revisit the prompt
/// the next time they launch the app — the canonical way to ask for
/// consent again when we expand what's collected. `0` means "never
/// asked"; any value below [`CURRENT_CONSENT_VERSION`] means "asked at
/// an earlier version, must be re-asked".
pub const CURRENT_CONSENT_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct TelemetrySettings {
    pub consent_version: u32,
    pub anonymous_metrics: bool,
    pub anonymous_errors: bool,
    pub non_anonymous_metrics: bool,
    pub distinct_id: Option<String>,
}

impl Default for TelemetrySettings {
    fn default() -> Self {
        Self {
            consent_version: 0,
            anonymous_metrics: true,
            anonymous_errors: true,
            non_anonymous_metrics: false,
            distinct_id: None,
        }
    }
}

impl TelemetrySettings {
    /// `true` when the user must be shown the consent prompt before any
    /// telemetry runs. Drives the onboarding redirect guard on the
    /// frontend.
    pub fn needs_consent(&self) -> bool {
        self.consent_version < CURRENT_CONSENT_VERSION
    }

    pub fn wants_errors(&self) -> bool {
        !self.needs_consent() && self.anonymous_errors
    }

    pub fn wants_metrics(&self) -> bool {
        !self.needs_consent() && self.anonymous_metrics
    }

    /// Anonymous metrics are a precondition for identification — turning
    /// on "non-anonymous metrics" without "anonymous metrics" would
    /// still produce zero events to identify against.
    pub fn wants_identified(&self) -> bool {
        self.wants_metrics() && self.non_anonymous_metrics
    }

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

    /// Replace the persisted distinct id with a fresh one. Used on
    /// logout or from the settings UI's "reset telemetry id" affordance
    /// to break linkage between sessions.
    pub fn rotate_distinct_id(&mut self) {
        self.distinct_id = Some(Uuid::new_v4().to_string());
    }

    /// Stamp the consent version to the current build's value.
    /// Idempotent for already-current consent. Called by the settings
    /// procedure whenever the user saves their telemetry choices —
    /// any save is by definition a recorded consent at the current
    /// schema version.
    pub fn record_consent(&mut self) {
        self.consent_version = CURRENT_CONSENT_VERSION;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_require_consent() {
        let settings = TelemetrySettings::default();
        assert!(settings.needs_consent());
        assert!(!settings.wants_errors());
        assert!(!settings.wants_metrics());
    }

    #[test]
    fn record_consent_unblocks_capture() {
        let mut settings = TelemetrySettings {
            consent_version: 0,
            anonymous_metrics: true,
            anonymous_errors: true,
            non_anonymous_metrics: true,
            distinct_id: None,
        };
        assert!(!settings.wants_errors());
        settings.record_consent();
        assert!(!settings.needs_consent());
        assert_eq!(settings.consent_version, CURRENT_CONSENT_VERSION);
        assert!(settings.wants_errors());
        assert!(settings.wants_identified());
    }

    #[test]
    fn legacy_consent_at_lower_version_re_prompts() {
        // A future release where CURRENT_CONSENT_VERSION > 1 would put
        // long-time users — saved at version 1 — into this state.
        let settings = TelemetrySettings {
            consent_version: CURRENT_CONSENT_VERSION.saturating_sub(1),
            anonymous_metrics: true,
            anonymous_errors: true,
            non_anonymous_metrics: false,
            distinct_id: Some("stable".to_owned()),
        };
        if CURRENT_CONSENT_VERSION > 0 && settings.consent_version < CURRENT_CONSENT_VERSION {
            assert!(settings.needs_consent());
            assert!(!settings.wants_metrics());
        }
    }

    #[test]
    fn ensure_distinct_id_is_idempotent_after_first_call() {
        let mut settings = TelemetrySettings::default();
        assert!(settings.ensure_distinct_id());
        let id = settings.distinct_id.clone();
        assert!(!settings.ensure_distinct_id());
        assert_eq!(settings.distinct_id, id);
    }

    #[test]
    fn rotate_distinct_id_replaces_existing_value() {
        let mut settings = TelemetrySettings::default();
        settings.ensure_distinct_id();
        let first = settings.distinct_id.clone();
        settings.rotate_distinct_id();
        assert!(settings.distinct_id.is_some());
        assert_ne!(settings.distinct_id, first);
    }

    #[test]
    fn identification_requires_anonymous_metrics() {
        let mut settings = TelemetrySettings {
            consent_version: CURRENT_CONSENT_VERSION,
            anonymous_metrics: false,
            anonymous_errors: true,
            non_anonymous_metrics: true,
            distinct_id: None,
        };
        assert!(!settings.wants_identified());
        settings.anonymous_metrics = true;
        assert!(settings.wants_identified());
    }
}
