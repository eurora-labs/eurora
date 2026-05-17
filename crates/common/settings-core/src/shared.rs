use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// User's preferred colour scheme. `System` defers to the OS-level
/// setting; `Light` / `Dark` pin it explicitly across devices.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

/// Cross-platform cloud-synced settings. Anything in this section
/// applies identically on desktop, mobile, and web; per-platform
/// concepts live in the platform sections of [`crate::CloudSettings`].
///
/// Telemetry consent is deliberately *not* here: each platform ships
/// a different telemetry stack and so collects different categories
/// of data, which means consent has to be platform-specific (see
/// [`crate::TelemetryConsent`]).
///
/// `dynamic_accent` defaults to `true` — the design pulls the OS / wallpaper
/// accent by default and a user who has never touched the toggle should see
/// the dynamic behaviour. This is the one field where the product default
/// differs from `bool::default()`, which is why this struct has a hand-rolled
/// `Default` instead of `#[derive]`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default, rename_all = "camelCase")]
pub struct SharedSettings {
    pub theme: ThemePreference,
    pub dynamic_accent: bool,
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

impl Default for SharedSettings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::default(),
            dynamic_accent: true,
            extras: Map::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_round_trip() {
        let s = SharedSettings::default();
        let v = serde_json::to_value(&s).unwrap();
        let back: SharedSettings = serde_json::from_value(v).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn default_enables_dynamic_accent() {
        // The one field where the product default diverges from
        // `bool::default()`. If you change this, audit the
        // appearance-page UI for assumptions about the initial toggle.
        assert!(SharedSettings::default().dynamic_accent);
    }

    #[test]
    fn round_trip_preserves_unknown_fields() {
        let raw = serde_json::json!({
            "theme": "dark",
            "dynamicAccent": true,
            "futureSharedKnob": "preserve me",
        });
        let parsed: SharedSettings = serde_json::from_value(raw.clone()).unwrap();
        assert_eq!(parsed.theme, ThemePreference::Dark);
        let round_tripped = serde_json::to_value(&parsed).unwrap();
        assert_eq!(round_tripped, raw);
    }

    #[test]
    fn theme_serializes_lowercase() {
        let s = serde_json::to_value(ThemePreference::Light).unwrap();
        assert_eq!(s, serde_json::json!("light"));
    }
}
