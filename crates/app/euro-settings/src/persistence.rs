use anyhow::Result;
use euro_fs::create_dirs_then_write;
use serde_json::json;
use serde_json_lenient::to_string_pretty;
use std::path::Path;
use tracing::debug;

use crate::{
    AppSettings,
    json::{json_difference, merge_non_null_json_value},
    watch::SETTINGS_FILE,
};

pub(crate) static DEFAULTS: &str = include_str!("../assets/defaults.jsonc");

impl AppSettings {
    pub fn load(config_path: &Path) -> Result<Self> {
        if !config_path.exists() {
            create_dirs_then_write(config_path, "{}\n")?;
        }

        let customizations = serde_json_lenient::from_str(&std::fs::read_to_string(config_path)?)?;
        let mut settings: serde_json::Value = serde_json_lenient::from_str(DEFAULTS)?;

        merge_non_null_json_value(customizations, &mut settings);

        let mut app_settings: AppSettings = serde_json::from_value(settings)?;

        // Normal user login flows won't work during development
        // if you have the variable set in the .env file
        if let Ok(api_base_url) = std::env::var("API_BASE_URL") {
            app_settings.api.endpoint = api_base_url;
        } else if cfg!(debug_assertions) {
            // This is handy for development so that
            // the Tauri app connects after running
            // pnpm docker:monolith
            if let Some(endpoint) = euro_debug::detect_local_backend_endpoint() {
                debug!("Detected local backend at {}", endpoint);
                app_settings.api.endpoint = endpoint;
            }
        }

        Ok(app_settings)
    }

    pub fn load_from_default_path_creating() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .expect("missing config dir")
            .join("eurora");
        std::fs::create_dir_all(&config_dir).expect("failed to create config dir");
        AppSettings::load(config_dir.join(SETTINGS_FILE).as_path())
    }

    pub fn save(&self, config_path: &Path) -> Result<()> {
        let current = serde_json::to_value(AppSettings::load(config_path)?)?;
        let update = serde_json::to_value(self)?;
        let diff = json_difference(current, &update);

        if diff == json!({}) {
            return Ok(());
        }

        let mut customizations =
            serde_json_lenient::from_str(&std::fs::read_to_string(config_path)?)?;

        // TODO: This will nuke any comments in the file
        merge_non_null_json_value(diff, &mut customizations);
        euro_fs::create_dirs_then_write(config_path, to_string_pretty(&customizations)?)?;
        Ok(())
    }

    pub fn save_to_default_path(&self) -> Result<()> {
        self.save(
            dirs::config_dir()
                .expect("missing config dir")
                .join("eurora")
                .join(SETTINGS_FILE)
                .as_path(),
        )
    }
}
