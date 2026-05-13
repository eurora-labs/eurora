use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

use crate::telemetry::TelemetryConsent;

/// Identity scale — the value the UI is designed against. Used as the
/// `Default` for both [`InterfaceScale`] and [`TextScale`] so a missing
/// field on the wire never resolves to a zero-size UI.
pub const DEFAULT_SCALE: f32 = 1.0;

/// Interface-scale multiplier applied to the document's root font-size,
/// scaling every rem-anchored design token (text, spacing, controls)
/// together. Always finite and within
/// `[InterfaceScale::MIN, InterfaceScale::MAX]` by construction.
///
/// Bounds are chosen against window chrome: below `MIN` the layout
/// drops below useful tap-target sizes on Linux / Windows; above `MAX`
/// the fixed-size titlebar and traffic-light buttons start overlapping
/// content.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(transparent)]
#[cfg_attr(feature = "specta", specta(transparent))]
pub struct InterfaceScale(f32);

impl InterfaceScale {
    /// Below this, the layout drops below useful tap-target sizes on Linux / Windows.
    pub const MIN: Self = Self(0.85);
    /// Beyond this, fixed-size chrome (titlebar, traffic lights) overlaps content.
    pub const MAX: Self = Self(1.5);
    /// Identity scale — the value the UI is designed against.
    pub const DEFAULT: Self = Self(DEFAULT_SCALE);

    /// Build a value, clamping to `[MIN, MAX]`. Non-finite inputs
    /// (`NaN`, `±∞`) collapse to [`Self::DEFAULT`] — the rendering
    /// pipeline can't recover from a zero-size or undefined scale, and
    /// the user can't reach this screen to fix it once the chrome
    /// disappears.
    pub fn new(value: f32) -> Self {
        if value.is_finite() {
            Self(value.clamp(Self::MIN.0, Self::MAX.0))
        } else {
            Self::DEFAULT
        }
    }

    /// Underlying `f32`. The value is always finite and within
    /// `[MIN, MAX]` — no further checks are required at use sites.
    pub const fn get(self) -> f32 {
        self.0
    }
}

impl Default for InterfaceScale {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl From<f32> for InterfaceScale {
    fn from(value: f32) -> Self {
        Self::new(value)
    }
}

impl From<InterfaceScale> for f32 {
    fn from(value: InterfaceScale) -> Self {
        value.0
    }
}

impl<'de> Deserialize<'de> for InterfaceScale {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        f32::deserialize(deserializer).map(Self::new)
    }
}

/// Additional multiplier layered on top of [`InterfaceScale`] that
/// affects only typography utilities, leaving spacing and control
/// sizes alone. Always finite and within
/// `[TextScale::MIN, TextScale::MAX]` by construction.
///
/// Bounds currently mirror [`InterfaceScale`] but the type is separate
/// so they can diverge — text legibility may eventually want a wider
/// range than the chrome-anchored interface bounds allow, and the type
/// system already enforces that the two fields can't be swapped at
/// call sites.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(transparent)]
#[cfg_attr(feature = "specta", specta(transparent))]
pub struct TextScale(f32);

impl TextScale {
    /// Below this, body text falls below readable sizes.
    pub const MIN: Self = Self(0.85);
    /// Beyond this, body text grows large enough to break line wrapping in fixed panels.
    pub const MAX: Self = Self(1.5);
    /// Identity scale — the value the UI is designed against.
    pub const DEFAULT: Self = Self(DEFAULT_SCALE);

    pub fn new(value: f32) -> Self {
        if value.is_finite() {
            Self(value.clamp(Self::MIN.0, Self::MAX.0))
        } else {
            Self::DEFAULT
        }
    }

    pub const fn get(self) -> f32 {
        self.0
    }
}

impl Default for TextScale {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl From<f32> for TextScale {
    fn from(value: f32) -> Self {
        Self::new(value)
    }
}

impl From<TextScale> for f32 {
    fn from(value: TextScale) -> Self {
        value.0
    }
}

impl<'de> Deserialize<'de> for TextScale {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        f32::deserialize(deserializer).map(Self::new)
    }
}

/// Desktop-only cloud-synced settings. Mobile and web each have their
/// own platform sections to keep concepts that don't translate (window
/// chrome scaling, telemetry SDKs that don't run on the other
/// platforms) cleanly partitioned.
///
/// The derived `Default` here is the *wire fallback* used by
/// `#[serde(default)]` when a partial blob is read off the network.
/// Scales default to [`DEFAULT_SCALE`] (via [`InterfaceScale::DEFAULT`]
/// / [`TextScale::DEFAULT`]) rather than `0.0`, because a zero-size UI
/// is unrecoverable; an inert sensible value is the only safe
/// fallback. The product-blessed fresh-install values live in
/// `assets/defaults.jsonc` and are reached through
/// [`crate::CloudSettings::default()`].
///
/// `interface_scale` and `text_scale` carry their invariants in the
/// type, not in a separate validation pass: any value of type
/// [`InterfaceScale`] or [`TextScale`] is by construction finite and
/// within `[MIN, MAX]`. Deserialization, `From<f32>`, and the
/// `bon`-generated builder all route through the clamping
/// constructor, so a corrupt cloud blob, a misbehaving client, or a
/// hand-rolled patch via `extras` all collapse to safe values before
/// they ever land in the struct.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, bon::Builder)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default, rename_all = "camelCase")]
pub struct DesktopSettings {
    /// Multiplier applied to the document's root font-size, scaling every
    /// rem-anchored design token (text, spacing, controls) together.
    #[builder(into, default)]
    pub interface_scale: InterfaceScale,
    /// Additional multiplier layered on top of `interface_scale` that
    /// affects only typography utilities, leaving spacing and control
    /// sizes alone.
    #[builder(into, default)]
    pub text_scale: TextScale,
    /// Desktop-scoped telemetry consent. Covers what the desktop
    /// client collects (Sentry, PostHog) and nothing else; mobile and
    /// web each carry their own record because consent must be
    /// specific to the data actually collected.
    #[builder(default)]
    pub telemetry: TelemetryConsent,
    // `flatten` of an empty Map already emits nothing — no
    // `skip_serializing_if` needed, and using it here would force
    // tauri-specta out of unified mode where the IPC surface lives.
    #[serde(flatten)]
    #[cfg_attr(
        feature = "specta",
        specta(type = std::collections::HashMap<String, specta_typescript::Unknown>)
    )]
    #[builder(default)]
    pub extras: Map<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interface_scale_clamps_out_of_range() {
        assert_eq!(InterfaceScale::new(0.1), InterfaceScale::MIN);
        assert_eq!(InterfaceScale::new(9.0), InterfaceScale::MAX);
        assert_eq!(
            InterfaceScale::new(InterfaceScale::MIN.get() - 0.01),
            InterfaceScale::MIN
        );
        assert_eq!(
            InterfaceScale::new(InterfaceScale::MAX.get() + 0.01),
            InterfaceScale::MAX
        );
    }

    #[test]
    fn interface_scale_preserves_in_range_values() {
        assert_eq!(InterfaceScale::new(1.15).get(), 1.15);
        assert_eq!(
            InterfaceScale::new(InterfaceScale::MIN.get()),
            InterfaceScale::MIN
        );
        assert_eq!(
            InterfaceScale::new(InterfaceScale::MAX.get()),
            InterfaceScale::MAX
        );
    }

    #[test]
    fn interface_scale_collapses_non_finite_to_default() {
        assert_eq!(InterfaceScale::new(f32::NAN), InterfaceScale::DEFAULT);
        assert_eq!(InterfaceScale::new(f32::INFINITY), InterfaceScale::DEFAULT);
        assert_eq!(
            InterfaceScale::new(f32::NEG_INFINITY),
            InterfaceScale::DEFAULT
        );
    }

    #[test]
    fn text_scale_clamps_out_of_range() {
        assert_eq!(TextScale::new(0.1), TextScale::MIN);
        assert_eq!(TextScale::new(9.0), TextScale::MAX);
    }

    #[test]
    fn text_scale_collapses_non_finite_to_default() {
        assert_eq!(TextScale::new(f32::NAN), TextScale::DEFAULT);
    }

    #[test]
    fn deserialize_clamps_out_of_range_scales() {
        let raw = serde_json::json!({
            "interfaceScale": 9.0,
            "textScale": 0.1,
        });
        let parsed: DesktopSettings = serde_json::from_value(raw).unwrap();
        assert_eq!(parsed.interface_scale, InterfaceScale::MAX);
        assert_eq!(parsed.text_scale, TextScale::MIN);
    }

    #[test]
    fn from_f32_clamps_at_the_type_boundary() {
        // The IPC surface and tests both hand the type raw `f32`s
        // through `From<f32>`; pin that the clamping path matches the
        // explicit `::new` constructor.
        let nan: InterfaceScale = f32::NAN.into();
        let inf: TextScale = f32::INFINITY.into();
        assert_eq!(nan, InterfaceScale::DEFAULT);
        assert_eq!(inf, TextScale::DEFAULT);
    }

    #[test]
    fn builder_clamps_via_into_setter() {
        // `#[builder(into)]` routes raw `f32` through `From<f32>`,
        // which clamps. Callers writing tests or migrators can hand
        // the builder a raw `f32` and rely on the same invariant as
        // the deserialize path.
        let s = DesktopSettings::builder()
            .interface_scale(9.0_f32)
            .text_scale(0.1_f32)
            .build();
        assert_eq!(s.interface_scale, InterfaceScale::MAX);
        assert_eq!(s.text_scale, TextScale::MIN);
    }

    #[test]
    fn builder_defaults_match_wire_fallback() {
        // An empty builder must reproduce the wire-fallback `Default`
        // exactly — otherwise the partial-JSON and explicit-builder
        // paths would diverge.
        assert_eq!(
            DesktopSettings::builder().build(),
            DesktopSettings::default()
        );
    }

    #[test]
    fn default_uses_default_scale() {
        let s = DesktopSettings::default();
        assert_eq!(s.interface_scale.get(), DEFAULT_SCALE);
        assert_eq!(s.text_scale.get(), DEFAULT_SCALE);
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
