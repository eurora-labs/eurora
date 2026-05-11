use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Web-only cloud-synced settings. Empty in v1 — the section exists
/// so the web client can ship fields onto it without disturbing
/// desktop's wire shape.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(default, rename_all = "camelCase")]
pub struct WebSettings {
    #[serde(flatten, skip_serializing_if = "Map::is_empty")]
    #[cfg_attr(
        feature = "specta",
        specta(type = std::collections::HashMap<String, specta_typescript::Unknown>)
    )]
    pub extras: Map<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_unknown_fields() {
        let raw = serde_json::json!({ "futureField": 42 });
        let parsed: WebSettings = serde_json::from_value(raw.clone()).unwrap();
        let round_tripped = serde_json::to_value(&parsed).unwrap();
        assert_eq!(round_tripped, raw);
    }

    #[test]
    fn default_serializes_to_empty_object() {
        let s = serde_json::to_value(WebSettings::default()).unwrap();
        assert_eq!(s, serde_json::json!({}));
    }
}
