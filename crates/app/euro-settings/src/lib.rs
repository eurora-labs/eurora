use serde::{Deserialize, Serialize};
use specta::Type;

mod json;
mod persistence;
mod settings;
mod watch;

pub use settings::{ApiSettings, GeneralSettings, TelemetrySettings};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub general: GeneralSettings,
    pub telemetry: TelemetrySettings,
    pub api: ApiSettings,
}
