use serde::{Deserialize, Serialize};
use specta::Type;

use crate::{api::APISettings, general::GeneralSettings, telemetry::TelemetryLocal};

/// On-disk shape of `~/.config/eurora/local.json`.
///
/// Carries every setting that must stay tied to *this install*:
/// - OS-level autostart registration is per-install,
/// - the API endpoint is the transport the sync engine itself uses
///   (chicken/egg if synced),
/// - the anonymous telemetry distinct id, whose rotation must break
///   cross-device linkage.
///
/// Per-field `#[serde(default)]` ensures partial files written by an
/// older build round-trip through deserialise.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct LocalSettings {
    pub general: GeneralSettings,
    pub api: APISettings,
    pub telemetry: TelemetryLocal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ConnectionMode;

    #[test]
    fn defaults_round_trip() {
        let s = LocalSettings::default();
        let v = serde_json::to_value(&s).unwrap();
        let back: LocalSettings = serde_json::from_value(v).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn fresh_defaults_enable_autostart_and_default_endpoint() {
        let s = LocalSettings::default();
        assert!(s.general.autostart);
        assert!(matches!(s.api.mode, ConnectionMode::Default));
        assert!(s.telemetry.distinct_id.is_none());
    }

    #[test]
    fn empty_object_deserialises_to_defaults() {
        let s: LocalSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(s, LocalSettings::default());
    }

    #[test]
    fn partial_object_fills_missing_sections_with_defaults() {
        let raw = serde_json::json!({ "general": { "autostart": false } });
        let s: LocalSettings = serde_json::from_value(raw).unwrap();
        assert!(!s.general.autostart);
        assert!(matches!(s.api.mode, ConnectionMode::Default));
        assert!(s.telemetry.distinct_id.is_none());
    }
}
