use std::sync::LazyLock;

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

/// Embedded fresh-install defaults. Product owns this document; the
/// crate parses it once at first access.
const DEFAULTS_JSONC: &str = include_str!("../assets/defaults.jsonc");

/// Parsed singleton of [`DEFAULTS_JSONC`].
static FRESH_INSTALL_DEFAULTS: LazyLock<CloudSettings> = LazyLock::new(|| {
    serde_json_lenient::from_str(DEFAULTS_JSONC)
        .expect("embedded settings-core defaults.jsonc must parse into CloudSettings")
});

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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CloudSettings {
    // Per-field `#[serde(default)]` rather than struct-level: a missing
    // wire field falls back to its *type's* `Default::default()`, never
    // to `CloudSettings::default()`. The latter is backed by the JSONC
    // singleton — re-entering it during serde fill-in would deadlock
    // [`FRESH_INSTALL_DEFAULTS`] mid-initialization.
    #[serde(default)]
    pub shared: SharedSettings,
    #[serde(default)]
    pub desktop: DesktopSettings,
    #[serde(default)]
    pub mobile: MobileSettings,
    #[serde(default)]
    pub web: WebSettings,
}

impl Default for CloudSettings {
    /// Fresh-install defaults: a clone of the values baked into
    /// `assets/defaults.jsonc`.
    fn default() -> Self {
        FRESH_INSTALL_DEFAULTS.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{desktop::DEFAULT_SCALE, shared::ThemePreference};

    #[test]
    fn defaults_round_trip() {
        let s = CloudSettings::default();
        let v = serde_json::to_value(&s).unwrap();
        let back: CloudSettings = serde_json::from_value(v).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn defaults_match_embedded_jsonc() {
        // Pin the JSONC content: if product changes a value here, this
        // test fails loudly so the change is reviewed alongside any code
        // that assumed the old defaults.
        let d = CloudSettings::default();

        assert_eq!(d.shared.theme, ThemePreference::System);
        assert!(d.shared.dynamic_accent);

        assert_eq!(d.desktop.interface_scale.get(), DEFAULT_SCALE);
        assert_eq!(d.desktop.text_scale.get(), DEFAULT_SCALE);

        assert_eq!(d.desktop.telemetry.consent_version, 1);
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
