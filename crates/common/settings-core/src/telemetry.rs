use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Per-platform telemetry consent record. Lives under each platform
/// section of [`crate::CloudSettings`] — never under `SharedSettings` —
/// because consent must be specific to the data actually collected,
/// and each platform ships a different telemetry stack (Sentry +
/// PostHog on desktop, platform-native SDKs on mobile, etc.). A user
/// agreeing on desktop has not seen, and cannot legally cover, what
/// mobile or web will collect.
///
/// `distinct_id` is intentionally absent — it is an anonymous
/// per-install identifier whose rotation must break cross-device
/// linkage, and so stays in the platform's local file rather than
/// crossing the wire.
///
/// `consent_version` records the schema version of the consent prompt
/// the user agreed to *on this platform*. Bumping
/// `CURRENT_CONSENT_VERSION` in the client forces a re-prompt; because
/// the record is per-platform, the bump propagates independently to
/// each device.
///
/// The derived `Default` here is the *wire fallback* used by
/// `#[serde(default)]` when a partial blob is read off the network
/// (every field collapses to `false` / `0` — the inert choice). The
/// product-blessed fresh-install values live in `assets/defaults.jsonc`
/// and are reached through [`crate::CloudSettings::default()`].
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default, rename_all = "camelCase")]
pub struct TelemetryConsent {
    pub consent_version: u32,
    pub anonymous_metrics: bool,
    pub anonymous_errors: bool,
    pub non_anonymous_metrics: bool,
    #[serde(flatten, skip_serializing_if = "Map::is_empty")]
    #[cfg_attr(
        feature = "specta",
        specta(type = std::collections::HashMap<String, specta_typescript::Unknown>)
    )]
    pub _extras: Map<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_fallback_default_is_inert() {
        // Derived `Default` is the partial-JSON fallback, not the
        // fresh-install default. It must be inert — every toggle off,
        // every counter zero — so an older client's missing fields
        // can't silently opt a user in to anything.
        let t = TelemetryConsent::default();
        assert_eq!(t.consent_version, 0);
        assert!(!t.anonymous_metrics);
        assert!(!t.anonymous_errors);
        assert!(!t.non_anonymous_metrics);
        assert!(t._extras.is_empty());
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
        assert!(!s.contains("_extras"), "got: {s}");
    }
}
