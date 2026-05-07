use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Type, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

/// Lower bound for interface and text scaling. Below this, the layout starts
/// dropping below useful tap-target sizes on Linux/Windows.
pub const MIN_SCALE: f32 = 0.85;
/// Upper bound for interface and text scaling. Beyond this, fixed-size chrome
/// (titlebar, traffic lights) starts overlapping content.
pub const MAX_SCALE: f32 = 1.5;
/// Identity scale — the value the UI is designed against.
pub const DEFAULT_SCALE: f32 = 1.0;

fn default_scale() -> f32 {
    DEFAULT_SCALE
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceSettings {
    pub theme: Theme,
    pub dynamic_accent: bool,
    /// Multiplier applied to the document's root font-size, scaling every
    /// rem-anchored design token (text, spacing, controls) together.
    #[serde(default = "default_scale")]
    pub interface_scale: f32,
    /// Additional multiplier layered on top of `interface_scale` that affects
    /// only typography utilities, leaving spacing and control sizes alone.
    #[serde(default = "default_scale")]
    pub text_scale: f32,
}

impl AppearanceSettings {
    /// Clamp scale fields into the supported range and replace any non-finite
    /// values with [`DEFAULT_SCALE`]. Called at the API boundary so a corrupt
    /// `settings.json` or a misbehaving client cannot push the UI into a
    /// state from which the user can't recover with the mouse.
    pub fn sanitize(&mut self) {
        self.interface_scale = sanitize_scale(self.interface_scale);
        self.text_scale = sanitize_scale(self.text_scale);
    }
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            dynamic_accent: true,
            interface_scale: DEFAULT_SCALE,
            text_scale: DEFAULT_SCALE,
        }
    }
}

fn sanitize_scale(value: f32) -> f32 {
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
        let mut s = AppearanceSettings {
            interface_scale: f32::NAN,
            text_scale: f32::INFINITY,
            ..AppearanceSettings::default()
        };
        s.sanitize();
        assert_eq!(s.interface_scale, DEFAULT_SCALE);
        assert_eq!(s.text_scale, DEFAULT_SCALE);
    }

    #[test]
    fn sanitize_clamps_out_of_range() {
        let mut s = AppearanceSettings {
            interface_scale: 0.1,
            text_scale: 9.0,
            ..AppearanceSettings::default()
        };
        s.sanitize();
        assert_eq!(s.interface_scale, MIN_SCALE);
        assert_eq!(s.text_scale, MAX_SCALE);
    }

    #[test]
    fn sanitize_preserves_in_range_values() {
        let mut s = AppearanceSettings {
            interface_scale: 1.15,
            text_scale: 0.9,
            ..AppearanceSettings::default()
        };
        s.sanitize();
        assert_eq!(s.interface_scale, 1.15);
        assert_eq!(s.text_scale, 0.9);
    }
}
