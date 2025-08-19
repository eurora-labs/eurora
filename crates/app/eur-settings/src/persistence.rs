use crate::Settings;
use crate::json::{json_difference, merge_non_null_json_value};
use crate::watch::SETTINGS_FILE;
use anyhow::Result;
use eur_fs::create_dirs_then_write;
use serde_json::json;
use serde_json_lenient::to_string_pretty;
use std::path::Path;

pub(crate) static DEFAULTS: &str = include_str!("../assets/defaults.jsonc");

impl Settings {
    pub fn load(config_path: &Path) -> Result<Self> {
        if !config_path.exists() {
            create_dirs_then_write(config_path, "{}\n")?;
        }

        let customizations = serde_json_lenient::from_str(&std::fs::read_to_string(config_path)?)?;
        let mut settings: serde_json::Value = serde_json_lenient::from_str(DEFAULTS)?;

        merge_non_null_json_value(customizations, &mut settings);
        Ok(serde_json::from_value(settings)?)
    }

    pub fn load_from_default_path_creating() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .expect("missing config dir")
            .join("eurora");
        std::fs::create_dir_all(&config_dir).expect("failed to create config dir");
        Settings::load(config_dir.join(SETTINGS_FILE).as_path())
    }

    /// Save all value in this instance to the custom configuration file *if they differ* from the defaults.
    pub fn save(&self, config_path: &Path) -> Result<()> {
        // Load the current settings
        let current = serde_json::to_value(Settings::load(config_path)?)?;

        // Derive changed values only compared to the current settings
        let update = serde_json::to_value(self)?;
        let diff = json_difference(current, &update);

        // If there are no changes, do nothing
        if diff == json!({}) {
            return Ok(());
        }

        // Load the existing customizations only
        let mut customizations =
            serde_json_lenient::from_str(&std::fs::read_to_string(config_path)?)?;

        // Merge the new customizations into the existing ones
        // TODO: This will nuke any comments in the file
        merge_non_null_json_value(diff, &mut customizations);
        eur_fs::create_dirs_then_write(config_path, to_string_pretty(&customizations)?)?;
        Ok(())
    }
}
