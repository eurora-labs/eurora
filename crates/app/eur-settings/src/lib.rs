use serde::{Deserialize, Serialize};

mod hover;
mod telemetry;

pub use hover::HoverSettings;
pub use telemetry::TelemetrySettings;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Telemetry settings
    pub telemetry: TelemetrySettings,
    /// Hover settings
    pub hover: HoverSettings,
    /// Backend provider settings
    // TODO: Refactor prompt service to here
    pub backend_provider: String,
}
