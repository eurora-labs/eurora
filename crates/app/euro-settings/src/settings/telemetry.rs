use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct TelemetrySettings {
    pub considered: bool,
    pub anonymous_metrics: bool,
    pub anonymous_errors: bool,
    pub non_anonymous_metrics: bool,
    pub distinct_id: Option<String>,
}

impl Default for TelemetrySettings {
    fn default() -> Self {
        Self {
            considered: false,
            anonymous_metrics: true,
            anonymous_errors: true,
            non_anonymous_metrics: false,
            distinct_id: None,
        }
    }
}
