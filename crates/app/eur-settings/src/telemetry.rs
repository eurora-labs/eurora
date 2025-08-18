use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
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
