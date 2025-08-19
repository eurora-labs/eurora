use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct TelemetrySettings {
    /// Anonymous metrics
    pub anonymous_metrics: bool,
    /// Anonymous error reporting
    pub anonymous_errors: bool,
    /// Non-anonymous metrics
    pub non_anonymous_metrics: bool,
    /// Distinct ID, if non-anonymous metrics are enabled
    pub distinct_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct HoverSettings {
    /// Whether hover window is enabled
    pub enabled: bool,
    // /// Position of hover window
    // pub position: (i64, i64),
}
