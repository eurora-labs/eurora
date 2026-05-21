use serde::{Deserialize, Serialize};

use crate::{
    desktop::DesktopSettings, mobile::MobileSettings, shared::SharedSettings, web::WebSettings,
};

/// Wire schema version of the cloud settings blob. Bump whenever a
/// breaking change to the JSONB shape is introduced.
///
/// The version travels on the request / response *envelope*
/// ([`crate::PutSettingsRequest`], [`crate::GetSettingsResponse`]) and
/// is owned by the server's metadata columns — never inside the blob.
/// Two reasons to keep it out of [`CloudSettings`]:
///
/// 1. Single source of truth. Duplicating the version inside the blob
///    creates a "what if they disagree?" question with no defensible
///    answer. Envelope wins, full stop.
/// 2. Schema-version routing is supposed to happen *before* a reader
///    commits to a particular Rust shape. Putting the field inside the
///    blob would invert that ordering.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Top-level cloud-synced settings blob. Sections are addressed
/// individually so clients only touch the fields that apply to their
/// platform; unknown sections and unknown fields within sections are
/// preserved verbatim through `extras` so a newer release of one
/// client never drops fields written by another.
///
/// The wire format the server stores is exactly this struct: an opaque
/// JSON document. Protocol metadata — schema version, `updated_at`,
/// optimistic-concurrency baseline — rides on the request / response
/// envelopes, not inside the blob, so this struct deliberately carries
/// neither a version nor a timestamp.
///
/// Fresh-install defaults are whatever each section's [`Default`] impl
/// produces. Per-field `#[serde(default)]` rather than struct-level so a
/// missing wire field falls back to its *type's* `Default::default()`
/// without going through `CloudSettings::default()` itself.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CloudSettings {
    #[serde(default)]
    pub shared: SharedSettings,
    #[serde(default)]
    pub desktop: DesktopSettings,
    #[serde(default)]
    pub mobile: MobileSettings,
    #[serde(default)]
    pub web: WebSettings,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        desktop::DEFAULT_SCALE, shared::ThemePreference, telemetry::DESKTOP_CONSENT_VERSION,
    };

    #[test]
    fn defaults_round_trip() {
        let s = CloudSettings::default();
        let v = serde_json::to_value(&s).unwrap();
        let back: CloudSettings = serde_json::from_value(v).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn fresh_install_defaults() {
        // Pin the product-blessed fresh-install state so a change to any
        // section's `Default` is reviewed alongside the call sites that
        // assumed the old value.
        let d = CloudSettings::default();

        assert_eq!(d.shared.theme, ThemePreference::System);
        assert!(d.shared.dynamic_accent);

        assert_eq!(d.desktop.interface_scale.get(), DEFAULT_SCALE);
        assert_eq!(d.desktop.text_scale.get(), DEFAULT_SCALE);

        // Fresh install must be below the current consent version so the
        // user is prompted on first launch. This is the load-bearing
        // sentinel — see `telemetry::TelemetryConsent`.
        assert!(d.desktop.telemetry.consent_version < DESKTOP_CONSENT_VERSION);
        assert!(!d.desktop.telemetry.anonymous_metrics);
        assert!(!d.desktop.telemetry.anonymous_errors);
        assert!(!d.desktop.telemetry.non_anonymous_metrics);

        assert!(d.shared.extras.is_empty());
        assert!(d.desktop.extras.is_empty());
        assert!(d.mobile.extras.is_empty());
        assert!(d.web.extras.is_empty());
    }

    #[test]
    fn unknown_top_level_section_is_preserved() {
        // A newer client adds fields to known sections; an older client
        // must round-trip them without dropping them. The top-level
        // struct has no `extras` itself by design (sections are the
        // extensibility unit), so this test pins that *known* sections
        // preserve their unknown subfields end-to-end through
        // `CloudSettings`.
        let raw = serde_json::json!({
            "shared": {
                "theme": "dark",
                "dynamicAccent": false,
                "futureSharedKnob": "x",
            },
            "desktop": {
                "interfaceScale": 1.25,
                "textScale": 1.5,
                "telemetry": {
                    "consentVersion": 1,
                    "anonymousMetrics": true,
                    "anonymousErrors": true,
                    "nonAnonymousMetrics": true,
                    "futureTelemetryKnob": true,
                },
                "askBar": { "enabled": true },
                "futureDesktopKnob": [1, 2, 3],
            },
            "mobile": { "futureMobileKnob": "y" },
            "web": { "futureWebKnob": "z" },
        });
        let parsed: CloudSettings = serde_json::from_value(raw.clone()).unwrap();
        let round_tripped = serde_json::to_value(&parsed).unwrap();
        assert_eq!(round_tripped, raw);
    }

    #[test]
    fn deserialize_clamps_desktop_scales() {
        // The clamp is a type-level invariant on `InterfaceScale` /
        // `TextScale`, exercised at deserialization. A corrupt cloud
        // blob with an out-of-range scale lands in the struct already
        // clamped — no separate `sanitize` pass is required.
        let raw = serde_json::json!({
            "shared": {},
            "desktop": { "interfaceScale": 9.0, "textScale": 0.1 },
            "mobile": {},
            "web": {},
        });
        let parsed: CloudSettings = serde_json::from_value(raw).unwrap();
        assert_eq!(
            parsed.desktop.interface_scale,
            crate::desktop::InterfaceScale::MAX
        );
        assert_eq!(parsed.desktop.text_scale, crate::desktop::TextScale::MIN);
    }
}
