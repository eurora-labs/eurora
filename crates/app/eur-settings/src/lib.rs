use serde::{Deserialize, Serialize};

mod hover;
mod telemetry;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Telemetry settings
    pub telemetry: telemetry::TelemetrySettings,
    /// Hover settings
    pub hover: hover::HoverSettings,

    /// Backend provider settings
    // TODO: Refactor prompt service to here
    pub backend_provider: String,
}
