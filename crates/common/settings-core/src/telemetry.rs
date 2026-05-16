use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Version of the desktop telemetry consent prompt. Bumping forces every
/// stored consent record on desktop to re-prompt: the gate compares the
/// user's stored [`TelemetryConsent::consent_version`] against this
/// value and the user is sent through the prompt again whenever stored
/// is below current.
///
/// Lives alongside the field it ratchets against so a bump can't drift
/// from the comparison. When mobile and web ship their own consent
/// flows, declare their own `MOBILE_CONSENT_VERSION` / `WEB_CONSENT_VERSION`
/// constants in this module — the [`TelemetryConsent`] struct is shared
/// but the "current version" is per-platform because each platform's
/// consent prompt is a separate document.
pub const DESKTOP_CONSENT_VERSION: u32 = 1;

/// Per-platform telemetry consent record. Lives under each platform
/// section of [`crate::CloudSettings`] — never under
/// [`crate::SharedSettings`] — because consent must be specific to the
/// data actually collected, and each platform ships a different
/// telemetry stack (Sentry + PostHog on desktop, platform-native SDKs
/// on mobile, etc.). A user agreeing on desktop has not seen, and
/// cannot legally cover, what mobile or web will collect.
///
/// `distinct_id` is intentionally absent — it is an anonymous
/// per-install identifier whose rotation must break cross-device
/// linkage, and so stays in the platform's local file rather than
/// crossing the wire.
///
/// ## `consent_version` semantics
///
/// `consent_version` records the schema version of the consent prompt
/// the user agreed to *on this platform*.
///
/// - `0` (the `Default`) means the user has **never** seen the prompt.
/// - Any value `>= DESKTOP_CONSENT_VERSION` (or the analogue for other
///   platforms) means the user has consented to the current schema.
/// - A value between `1` and `current - 1` means the user consented to
///   an older prompt; the client must re-prompt.
///
/// The value is **monotonically non-decreasing** by contract. The only
/// writer is the platform's consent procedure, which uses
/// [`Self::record_for_desktop`] (analogous methods for other platforms
/// will land when those clients ship). Regular settings-page saves
/// must *not* touch this field; the IPC layer enforces this by
/// clamping incoming values against the stored version.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default, rename_all = "camelCase")]
pub struct TelemetryConsent {
    pub consent_version: u32,
    pub anonymous_metrics: bool,
    pub anonymous_errors: bool,
    pub non_anonymous_metrics: bool,
    // `flatten` of an empty Map already emits nothing — no
    // `skip_serializing_if` needed, and using it here would force
    // tauri-specta out of unified mode where the IPC surface lives.
    #[serde(flatten)]
    #[cfg_attr(
        feature = "specta",
        specta(type = std::collections::HashMap<String, specta_typescript::Unknown>)
    )]
    pub extras: Map<String, Value>,
}

impl TelemetryConsent {
    /// `true` when the user has agreed to the current desktop consent
    /// prompt and the SDKs may run subject to the individual toggles.
    /// `false` while the user has either never seen the prompt
    /// (`consent_version == 0`) or only agreed to an older one.
    #[must_use]
    pub fn is_recorded_for_desktop(&self) -> bool {
        self.consent_version >= DESKTOP_CONSENT_VERSION
    }

    /// Inverse of [`Self::is_recorded_for_desktop`]. Drives the gate
    /// that redirects users to the consent prompt before any other
    /// route can render.
    #[must_use]
    pub fn needs_prompt_for_desktop(&self) -> bool {
        !self.is_recorded_for_desktop()
    }

    /// `true` when anonymous error reporting may run on desktop.
    /// Requires both a recorded consent at the current version and the
    /// user's explicit opt-in.
    #[must_use]
    pub fn allows_errors_on_desktop(&self) -> bool {
        self.is_recorded_for_desktop() && self.anonymous_errors
    }

    /// `true` when anonymous usage metrics may run on desktop.
    #[must_use]
    pub fn allows_metrics_on_desktop(&self) -> bool {
        self.is_recorded_for_desktop() && self.anonymous_metrics
    }

    /// `true` when the user has opted into non-anonymous identification
    /// on top of anonymous metrics. The anonymous metrics check is a
    /// precondition because turning identification on without metrics
    /// would still produce zero events to identify against.
    #[must_use]
    pub fn allows_identification_on_desktop(&self) -> bool {
        self.allows_metrics_on_desktop() && self.non_anonymous_metrics
    }

    /// Stamp the consent version for desktop, monotonically. Called by
    /// the desktop consent procedure after writing the user's toggle
    /// choices. Never downgrades: a stored value above
    /// [`DESKTOP_CONSENT_VERSION`] (written by a newer client) is left
    /// alone, so an older client running the same procedure can't roll
    /// back the user's recorded consent.
    pub fn record_for_desktop(&mut self) {
        self.consent_version = self.consent_version.max(DESKTOP_CONSENT_VERSION);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_needs_prompt() {
        // The load-bearing sentinel: a fresh `TelemetryConsent` must
        // require the prompt. `consent_version` defaults to 0 via
        // `u32::default()` and 0 is below every defined `*_CONSENT_VERSION`.
        let c = TelemetryConsent::default();
        assert_eq!(c.consent_version, 0);
        assert!(c.needs_prompt_for_desktop());
        assert!(!c.is_recorded_for_desktop());
        assert!(!c.allows_errors_on_desktop());
        assert!(!c.allows_metrics_on_desktop());
        assert!(!c.allows_identification_on_desktop());
        assert!(c.extras.is_empty());
    }

    #[test]
    fn record_advances_to_current_version() {
        let mut c = TelemetryConsent::default();
        c.record_for_desktop();
        assert_eq!(c.consent_version, DESKTOP_CONSENT_VERSION);
        assert!(c.is_recorded_for_desktop());
    }

    #[test]
    fn record_is_monotonic() {
        // A newer client stamped a higher version; the older client's
        // record call must not downgrade it.
        let mut c = TelemetryConsent {
            consent_version: DESKTOP_CONSENT_VERSION + 5,
            ..Default::default()
        };
        c.record_for_desktop();
        assert_eq!(c.consent_version, DESKTOP_CONSENT_VERSION + 5);
    }

    #[test]
    fn allows_gates_require_recorded_consent() {
        // Toggles set to `true` are inert until consent is recorded.
        // Prevents an older client with stale toggles from silently
        // enabling capture before the user re-confirms.
        let mut c = TelemetryConsent {
            consent_version: 0,
            anonymous_metrics: true,
            anonymous_errors: true,
            non_anonymous_metrics: true,
            ..Default::default()
        };
        assert!(!c.allows_errors_on_desktop());
        assert!(!c.allows_metrics_on_desktop());
        assert!(!c.allows_identification_on_desktop());

        c.record_for_desktop();
        assert!(c.allows_errors_on_desktop());
        assert!(c.allows_metrics_on_desktop());
        assert!(c.allows_identification_on_desktop());
    }

    #[test]
    fn identification_requires_anonymous_metrics() {
        let mut c = TelemetryConsent {
            anonymous_metrics: false,
            anonymous_errors: true,
            non_anonymous_metrics: true,
            ..Default::default()
        };
        c.record_for_desktop();
        assert!(!c.allows_identification_on_desktop());
        c.anonymous_metrics = true;
        assert!(c.allows_identification_on_desktop());
    }

    #[test]
    fn round_trip_preserves_unknown_fields() {
        let raw = serde_json::json!({
            "consentVersion": 1,
            "anonymousMetrics": true,
            "anonymousErrors": false,
            "nonAnonymousMetrics": false,
            "futureField": "from a later release",
            "anotherFutureField": 42,
        });
        let parsed: TelemetryConsent = serde_json::from_value(raw.clone()).unwrap();
        assert_eq!(parsed.consent_version, 1);
        assert!(!parsed.anonymous_errors);
        let round_tripped = serde_json::to_value(&parsed).unwrap();
        assert_eq!(round_tripped, raw);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let raw = serde_json::json!({});
        let parsed: TelemetryConsent = serde_json::from_value(raw).unwrap();
        assert_eq!(parsed, TelemetryConsent::default());
    }

    #[test]
    fn empty_extras_are_not_emitted() {
        let t = TelemetryConsent::default();
        let s = serde_json::to_string(&t).unwrap();
        assert!(!s.contains("extras"), "got: {s}");
    }
}
