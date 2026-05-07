use serde::{Deserialize, Serialize};
use specta::Type;

mod api_settings;
mod appearance_settings;
mod general_settings;
mod json;
mod persistence;
mod telemetry_settings;
mod watch;

pub use api_settings::{APISettings, CLOUD_API_URL, ConnectionMode, LOCAL_API_URL};
pub use appearance_settings::{AppearanceSettings, DEFAULT_SCALE, MAX_SCALE, MIN_SCALE, Theme};
pub use general_settings::GeneralSettings;
pub use telemetry_settings::{CURRENT_CONSENT_VERSION, TelemetrySettings};

// `PartialEq` only — `AppearanceSettings` carries `f32` scale fields, which
// makes total equality (`Eq`) unsound. The struct only ever needs structural
// comparison (in tests), so `PartialEq` is sufficient.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub general: GeneralSettings,
    pub telemetry: TelemetrySettings,
    /// Falls back to [`APISettings::default`] when the persisted config has
    /// never been touched, which lets debug builds land on `localhost` and
    /// release builds on the cloud without the `defaults.jsonc` having to
    /// know about the build profile.
    #[serde(default)]
    pub api: APISettings,
    pub appearance: AppearanceSettings,
}
