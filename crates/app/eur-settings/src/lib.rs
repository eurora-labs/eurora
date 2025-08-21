use serde::{Deserialize, Serialize};

mod hotkey;
mod json;
mod persistence;
mod settings;
mod watch;

pub use hotkey::Hotkey;
pub use settings::BackendSettings;
pub use settings::GeneralSettings;
pub use settings::HoverSettings;
pub use settings::LauncherSettings;
pub use settings::TelemetrySettings;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// General settings
    pub general: GeneralSettings,
    /// Telemetry settings
    pub telemetry: TelemetrySettings,
    /// Hover settings
    pub hover: HoverSettings,
    /// Launcher settings
    pub launcher: LauncherSettings,
    /// Backend provider settings
    #[serde(default)]
    pub backend: BackendSettings,
}
