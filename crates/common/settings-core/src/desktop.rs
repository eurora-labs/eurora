use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::telemetry::TelemetryConsent;

/// Lower bound for interface and text scaling. Below this, the layout
/// drops below useful tap-target sizes on Linux / Windows.
pub const MIN_SCALE: f32 = 0.85;

/// Upper bound for interface and text scaling. Beyond this, fixed-size
/// chrome (titlebar, traffic lights) starts overlapping content.
pub const MAX_SCALE: f32 = 1.5;

/// Identity scale — the value the UI is designed against.
pub const DEFAULT_SCALE: f32 = 1.0;

/// Desktop-only cloud-synced settings. Mobile and web each have their
/// own platform sections to keep concepts that don't translate (window
/// chrome scaling, telemetry SDKs that don't run on the other
/// platforms) cleanly partitioned.
///
/// The custom `Default` impl here is the *wire fallback* used by
/// `#[serde(default)]` when a partial blob is read off the network.
/// Scales default to [`DEFAULT_SCALE`] rather than the derive-default
/// `0.0` because a zero-size UI is unrecoverable; an inert sensible
/// value is the only safe fallback. The product-blessed fresh-install
/// values live in `assets/defaults.jsonc` and are reached through
/// [`crate::CloudSettings::default()`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default, rename_all = "camelCase")]
pub struct DesktopSettings {
    /// Multiplier applied to the document's root font-size, scaling every
    /// rem-anchored design token (text, spacing, controls) together.
    pub interface_scale: f32,
    /// Additional multiplier layered on top of `interface_scale` that
    /// affects only typography utilities, leaving spacing and control
    /// sizes alone.
    pub text_scale: f32,
    /// Desktop-scoped telemetry consent. Covers what the desktop
    /// client collects (Sentry, PostHog) and nothing else; mobile and
    /// web each carry their own record because consent must be
    /// specific to the data actually collected.
    pub telemetry: TelemetryConsent,
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

impl Default for DesktopSettings {
    fn default() -> Self {
        Self {
            interface_scale: DEFAULT_SCALE,
            text_scale: DEFAULT_SCALE,
            telemetry: TelemetryConsent::default(),
            extras: Map::new(),
        }
    }
}

impl DesktopSettings {
    /// Clamp scale fields into the supported range and replace any
    /// non-finite values with [`DEFAULT_SCALE`]. Called at the API
    /// boundary so a corrupt cloud blob or a misbehaving client cannot
    /// push the UI into a state from which the user can't recover with
    /// the mouse.
    pub fn sanitize(&mut self) {
        self.interface_scale = sanitize_scale(self.interface_scale);
        self.text_scale = sanitize_scale(self.text_scale);
    }
}

/// Coerce a single scale value into the supported range. Non-finite
/// values collapse to [`DEFAULT_SCALE`]; finite values are clamped to
/// `[MIN_SCALE, MAX_SCALE]`.
pub fn sanitize_scale(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_SCALE, MAX_SCALE)
    } else {
        DEFAULT_SCALE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_replaces_nan_with_default() {
        let mut s = DesktopSettings {
            interface_scale: f32::NAN,
            text_scale: f32::INFINITY,
            ..DesktopSettings::default()
        };
        s.sanitize();
        assert_eq!(s.interface_scale, DEFAULT_SCALE);
        assert_eq!(s.text_scale, DEFAULT_SCALE);
    }

    #[test]
    fn sanitize_clamps_out_of_range() {
        let mut s = DesktopSettings {
            interface_scale: 0.1,
            text_scale: 9.0,
            ..DesktopSettings::default()
        };
        s.sanitize();
        assert_eq!(s.interface_scale, MIN_SCALE);
        assert_eq!(s.text_scale, MAX_SCALE);
    }

    #[test]
    fn sanitize_preserves_in_range_values() {
        let mut s = DesktopSettings {
            interface_scale: 1.15,
            text_scale: 0.9,
            ..DesktopSettings::default()
        };
        s.sanitize();
        assert_eq!(s.interface_scale, 1.15);
        assert_eq!(s.text_scale, 0.9);
    }

    #[test]
    fn sanitize_scale_pins_boundaries() {
        assert_eq!(sanitize_scale(MIN_SCALE - 0.01), MIN_SCALE);
        assert_eq!(sanitize_scale(MAX_SCALE + 0.01), MAX_SCALE);
        assert_eq!(sanitize_scale(MIN_SCALE), MIN_SCALE);
        assert_eq!(sanitize_scale(MAX_SCALE), MAX_SCALE);
        assert_eq!(sanitize_scale(f32::NEG_INFINITY), DEFAULT_SCALE);
    }

    #[test]
    fn round_trip_preserves_unknown_fields() {
        // Use scale values that are exactly representable in f32 so the
        // round-trip is bit-exact and the assertion exercises `extras`
        // preservation rather than float precision.
        let raw = serde_json::json!({
            "interfaceScale": 1.25,
            "textScale": 1.5,
            "telemetry": {
                "consentVersion": 1,
                "anonymousMetrics": true,
                "anonymousErrors": true,
                "nonAnonymousMetrics": false,
            },
            "futureKnob": { "nested": true },
        });
        let parsed: DesktopSettings = serde_json::from_value(raw.clone()).unwrap();
        let round_tripped = serde_json::to_value(&parsed).unwrap();
        assert_eq!(round_tripped, raw);
    }
}
