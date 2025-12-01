use serde::{Deserialize, Serialize};
use specta::Type;

mod hotkey;
mod json;
mod persistence;
mod settings;
mod watch;

pub use hotkey::Hotkey;
pub use settings::{BackendSettings, BackendType, GeneralSettings, TelemetrySettings};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// General settings
    pub general: GeneralSettings,
    /// Telemetry settings
    pub telemetry: TelemetrySettings,
    /// Backend provider settings
    #[serde(default)]
    pub backend: BackendSettings,
}
