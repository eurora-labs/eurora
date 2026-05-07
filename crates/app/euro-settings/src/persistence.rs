use anyhow::Result;
use euro_fs::create_dirs_then_write;
use serde_json::json;
use serde_json_lenient::to_string_pretty;
use std::path::Path;

use crate::{
    AppSettings, ConnectionMode,
    json::{json_difference, merge_non_null_json_value},
    watch::SETTINGS_FILE,
};

pub(crate) static DEFAULTS: &str = include_str!("../assets/defaults.jsonc");

impl AppSettings {
    pub fn defaults() -> Self {
        let settings: serde_json::Value =
            serde_json_lenient::from_str(DEFAULTS).expect("embedded defaults.jsonc is invalid");
        serde_json::from_value(settings)
            .expect("embedded defaults.jsonc does not match AppSettings")
    }

    pub fn load(config_path: &Path) -> Result<Self> {
        if !config_path.exists() {
            create_dirs_then_write(config_path, "{}\n")?;
        }

        let customizations = serde_json_lenient::from_str(&std::fs::read_to_string(config_path)?)?;
        let mut settings: serde_json::Value = serde_json_lenient::from_str(DEFAULTS)?;

        merge_non_null_json_value(customizations, &mut settings);

        let mut app_settings: AppSettings = serde_json::from_value(settings)?;

        // `API_BASE_URL` is the developer escape hatch: setting it in the
        // shell forces the app to talk to that URL on this run regardless
        // of the persisted connection mode. Persisted setting is left
        // untouched — the override is in-memory only.
        if let Ok(api_base_url) = std::env::var("API_BASE_URL") {
            app_settings.api.mode = ConnectionMode::Custom { url: api_base_url };
        }

        Ok(app_settings)
    }

    pub fn load_from_default_path_creating() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .expect("missing config dir")
            .join("eurora");
        std::fs::create_dir_all(&config_dir).expect("failed to create config dir");
        let config_path = config_dir.join(SETTINGS_FILE);

        match AppSettings::load(config_path.as_path()) {
            Ok(settings) => Ok(settings),
            Err(e) => {
                tracing::warn!("Failed to load settings, resetting to defaults: {e}");
                let defaults = AppSettings::defaults();
                if let Err(write_err) = create_dirs_then_write(&config_path, "{}\n") {
                    tracing::warn!("Failed to reset settings file: {write_err}");
                }
                Ok(defaults)
            }
        }
    }

    /// Read settings without creating files or directories. Returns the
    /// embedded defaults on any I/O or parse failure. Used early in the
    /// Tauri startup sequence (e.g. to gate Sentry init) where we want to
    /// observe the user's last-saved choice without side effects.
    pub fn peek_from_default_path() -> Self {
        let Some(config_dir) = dirs::config_dir() else {
            return Self::defaults();
        };
        let config_path = config_dir.join("eurora").join(SETTINGS_FILE);
        if !config_path.exists() {
            return Self::defaults();
        }
        match AppSettings::load(config_path.as_path()) {
            Ok(settings) => settings,
            Err(e) => {
                tracing::warn!(
                    "Failed to read settings during early peek, falling back to defaults: {e}"
                );
                Self::defaults()
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AppearanceSettings, Theme};

    #[test]
    fn embedded_defaults_parse_into_app_settings() {
        let settings = AppSettings::defaults();
        assert_eq!(settings.appearance, AppearanceSettings::default());
    }

    #[test]
    fn legacy_settings_file_without_appearance_section_loads_with_defaults() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        // Pre-`appearance` users have config files lacking the section.
        std::fs::write(&path, r#"{"general": {"autostart": false}}"#)
            .expect("write legacy settings");

        let loaded = AppSettings::load(&path).expect("load legacy settings");
        assert!(!loaded.general.autostart);
        assert_eq!(loaded.appearance, AppearanceSettings::default());
    }

    #[test]
    fn legacy_considered_field_is_dropped_so_old_users_re_consent() {
        // Pre-`consentVersion` installs persisted `considered: true` because
        // the original defaults shipped that flag pre-flipped. After the
        // schema change those users must be re-prompted; the old field is
        // unknown to the new struct and falls through to the default
        // `consentVersion: 0`, which `needs_consent()` flags as required.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{"telemetry": {"considered": true, "anonymousMetrics": false}}"#,
        )
        .expect("write legacy settings");

        let loaded = AppSettings::load(&path).expect("load legacy telemetry settings");
        assert!(loaded.telemetry.needs_consent());
        assert_eq!(loaded.telemetry.consent_version, 0);
        // Custom toggles persist through the upgrade — only the consent
        // record itself is invalidated.
        assert!(!loaded.telemetry.anonymous_metrics);
    }

    #[test]
    fn appearance_round_trips_through_save_and_load() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.json");
        std::fs::write(&path, "{}\n").expect("seed empty settings");

        let mut settings = AppSettings::load(&path).expect("load defaults");
        settings.appearance.theme = Theme::Light;
        settings.appearance.dynamic_accent = false;
        settings.save(&path).expect("save");

        let reloaded = AppSettings::load(&path).expect("reload");
        assert_eq!(reloaded.appearance.theme, Theme::Light);
        assert!(!reloaded.appearance.dynamic_accent);
    }
}
