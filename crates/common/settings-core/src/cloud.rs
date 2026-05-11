use std::sync::LazyLock;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    desktop::DesktopSettings, mobile::MobileSettings, shared::SharedSettings, web::WebSettings,
};

/// Wire schema version of the cloud settings blob. Bump whenever a
/// breaking change to the JSONB shape is introduced; the server keeps
/// the value as written so older clients can detect they're behind.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Embedded fresh-install defaults. Product owns this document; the
/// crate parses it once at first access.
const DEFAULTS_JSONC: &str = include_str!("../assets/defaults.jsonc");

/// Parsed singleton of [`DEFAULTS_JSONC`] with structural fields
/// (`schema_version`, `updated_at`) stamped from code constants so the
/// JSONC document stays free of plumbing concerns.
static FRESH_INSTALL_DEFAULTS: LazyLock<CloudSettings> = LazyLock::new(|| {
    let mut s: CloudSettings = serde_json_lenient::from_str(DEFAULTS_JSONC)
        .expect("embedded settings-core defaults.jsonc must parse into CloudSettings");
    s.schema_version = CURRENT_SCHEMA_VERSION;
    s.updated_at = DateTime::<Utc>::UNIX_EPOCH;
    s
});

/// Top-level cloud-synced settings blob. Sections are addressed
/// individually so clients only touch the fields that apply to their
/// platform; unknown sections and unknown fields within sections are
/// preserved verbatim through `_extras` so a newer release of one
/// client never drops fields written by another.
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
    pub schema_version: u32,
    #[serde(default)]
    pub shared: SharedSettings,
    #[serde(default)]
    pub desktop: DesktopSettings,
    #[serde(default)]
    pub mobile: MobileSettings,
    #[serde(default)]
    pub web: WebSettings,
    #[serde(default)]
    pub updated_at: DateTime<Utc>,
}

impl Default for CloudSettings {
    /// Fresh-install defaults: a clone of the values baked into
    /// `assets/defaults.jsonc`. `updated_at` is [`DateTime::UNIX_EPOCH`]
    /// so the very first server pull always wins by timestamp
    /// comparison.
    fn default() -> Self {
        FRESH_INSTALL_DEFAULTS.clone()
    }
}

impl CloudSettings {
    /// Coerce out-of-range or non-finite numeric fields into their safe
    /// ranges across every section. Run on the server before write and
    /// on the client after deserialize.
    pub fn sanitize(&mut self) {
        self.desktop.sanitize();
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
        assert_eq!(d.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(d.updated_at, DateTime::<Utc>::UNIX_EPOCH);

        assert_eq!(d.shared.theme, ThemePreference::System);
        assert!(d.shared.dynamic_accent);

        assert_eq!(d.desktop.interface_scale, DEFAULT_SCALE);
        assert_eq!(d.desktop.text_scale, DEFAULT_SCALE);

        assert_eq!(d.desktop.telemetry.consent_version, 1);
        assert!(!d.desktop.telemetry.anonymous_metrics);
        assert!(!d.desktop.telemetry.anonymous_errors);
        assert!(!d.desktop.telemetry.non_anonymous_metrics);

        assert!(d.shared._extras.is_empty());
        assert!(d.desktop._extras.is_empty());
        assert!(d.mobile._extras.is_empty());
        assert!(d.web._extras.is_empty());
    }

    #[test]
    fn unknown_top_level_section_is_preserved() {
        // A newer client adds fields to known sections; an older client
        // must round-trip them without dropping them. The top-level
        // struct has no `_extras` itself by design (sections are the
        // extensibility unit), so this test pins that *known* sections
        // preserve their unknown subfields end-to-end through
        // `CloudSettings`.
        let raw = serde_json::json!({
            "schemaVersion": CURRENT_SCHEMA_VERSION,
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
            "updatedAt": "2026-05-11T10:00:00Z",
        });
        let parsed: CloudSettings = serde_json::from_value(raw.clone()).unwrap();
        let round_tripped = serde_json::to_value(&parsed).unwrap();
        assert_eq!(round_tripped, raw);
    }

    #[test]
    fn sanitize_clamps_desktop_scales() {
        let mut s = CloudSettings::default();
        s.desktop.interface_scale = 9.0;
        s.desktop.text_scale = f32::NAN;
        s.sanitize();
        assert_eq!(s.desktop.interface_scale, crate::desktop::MAX_SCALE);
        assert_eq!(s.desktop.text_scale, crate::desktop::DEFAULT_SCALE);
    }
}
