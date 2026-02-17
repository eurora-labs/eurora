use serde::{Deserialize, Serialize};
use specta::Type;

mod api_settings;
mod error;
mod general_settings;
mod json;
mod persistence;
mod telemetry_settings;
mod watch;

pub use api_settings::{APISettings, OpenAISettings, ProviderSettings};
pub use general_settings::GeneralSettings;
pub use telemetry_settings::TelemetrySettings;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub general: GeneralSettings,
    pub telemetry: TelemetrySettings,
    pub api: APISettings,
}
