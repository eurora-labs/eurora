//! Disk I/O for the split settings state, including the one-shot legacy
//! `settings.json` migration.
//!
//! Two files live under `~/.config/eurora/`:
//! - `local.json` — owned by [`crate::LocalSettings`]
//! - `cloud.json` — owned by [`crate::CloudSettingsCache`]
//!
//! On first launch of the new build we read the legacy combined file
//! `settings.json`, split it across the two new files, and rename the
//! source to `settings.json.legacy` so the migration is idempotent and
//! the original payload is recoverable for one release cycle.

use std::path::{Path, PathBuf};

use anyhow::Result;
use euro_fs::create_dirs_then_write;
use serde_json::Value;
use settings_core::{CURRENT_SCHEMA_VERSION, CloudSettings};

use crate::{cloud_cache::CloudSettingsCache, local::LocalSettings, state::SettingsState};

pub(crate) const LOCAL_FILE: &str = "local.json";
pub(crate) const CLOUD_FILE: &str = "cloud.json";
pub(crate) const LEGACY_FILE: &str = "settings.json";
pub(crate) const LEGACY_BACKUP_FILE: &str = "settings.json.legacy";

fn default_config_dir() -> Result<PathBuf> {
    Ok(dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("no platform config dir"))?
        .join("eurora"))
}

fn write_json_atomic<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value)?;
    create_dirs_then_write(path, json)?;
    Ok(())
}

impl SettingsState {
    /// Load both files from the given config directory, migrating the
    /// legacy `settings.json` if it's the only thing present. Always
    /// returns a usable [`SettingsState`]; malformed JSON is logged and
    /// replaced with defaults rather than surfacing as an error so a
    /// hand-edit can't lock the user out of the settings UI.
    pub fn load_or_migrate(config_dir: &Path) -> Result<Self> {
        let local_path = config_dir.join(LOCAL_FILE);
        let cloud_path = config_dir.join(CLOUD_FILE);
        let legacy_path = config_dir.join(LEGACY_FILE);

        if local_path.exists() {
            // Fast path: split files already exist. Cloud cache is
            // optional — a fresh-install state with no cloud writes yet
            // is legitimate.
            let local = read_or_default::<LocalSettings>(&local_path);
            let cache = if cloud_path.exists() {
                read_or_default::<CloudSettingsCache>(&cloud_path)
            } else {
                CloudSettingsCache::default()
            };
            return Ok(Self::new(local, cache));
        }

        if legacy_path.exists() {
            let (local, cache) = match split_legacy(&legacy_path) {
                Ok(parts) => parts,
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse legacy settings.json ({e}); using defaults"
                    );
                    (LocalSettings::default(), CloudSettingsCache::default())
                }
            };
            write_json_atomic(&local_path, &local)?;
            write_json_atomic(&cloud_path, &cache)?;

            // Rename rather than delete: the .legacy file is the
            // recovery artifact if migration drops a field somebody
            // notices in the next release cycle.
            let backup_path = config_dir.join(LEGACY_BACKUP_FILE);
            if let Err(e) = std::fs::rename(&legacy_path, &backup_path) {
                tracing::warn!(
                    "Failed to rename legacy settings.json to .legacy: {e}; \
                     leaving original in place"
                );
            }

            return Ok(Self::new(local, cache));
        }

        // Fresh install — persist defaults eagerly so subsequent runs
        // hit the fast path and we don't accidentally re-migrate from a
        // stray `settings.json` that lands later.
        let state = Self::default();
        write_json_atomic(&local_path, &state.local)?;
        write_json_atomic(&cloud_path, &state.cache)?;
        Ok(state)
    }

    /// Convenience wrapper around [`Self::load_or_migrate`] using the
    /// platform-default config directory.
    pub fn load_or_migrate_from_default_path() -> Result<Self> {
        Self::load_or_migrate(&default_config_dir()?)
    }

    pub fn save_local(&self, config_dir: &Path) -> Result<()> {
        write_json_atomic(&config_dir.join(LOCAL_FILE), &self.local)
    }

    pub fn save_cache(&self, config_dir: &Path) -> Result<()> {
        write_json_atomic(&config_dir.join(CLOUD_FILE), &self.cache)
    }

    pub fn save_local_to_default_path(&self) -> Result<()> {
        self.save_local(&default_config_dir()?)
    }

    pub fn save_cache_to_default_path(&self) -> Result<()> {
        self.save_cache(&default_config_dir()?)
    }
}

impl CloudSettingsCache {
    /// Read the cache without creating files or directories. Returns
    /// defaults on any I/O or parse failure. Used early in the Tauri
    /// startup sequence (to gate Sentry init) where side effects are
    /// undesirable.
    #[must_use]
    pub fn peek_from_default_path() -> Self {
        let Ok(config_dir) = default_config_dir() else {
            return Self::default();
        };
        let cloud_path = config_dir.join(CLOUD_FILE);
        if cloud_path.exists() {
            return read_or_default::<CloudSettingsCache>(&cloud_path);
        }
        // Pre-migration install: the legacy combined file may still
        // exist and carry the user's consent record. Pull just enough
        // out of it to seed the cache, but don't persist anything —
        // this is a peek.
        let legacy_path = config_dir.join(LEGACY_FILE);
        if legacy_path.exists()
            && let Ok((_, cache)) = split_legacy(&legacy_path)
        {
            return cache;
        }
        Self::default()
    }
}

fn read_or_default<T>(path: &Path) -> T
where
    T: Default + serde::de::DeserializeOwned,
{
    let bytes = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(?path, "Failed to read settings file ({e}); using defaults");
            return T::default();
        }
    };
    match serde_json_lenient::from_str::<T>(&bytes) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(?path, "Failed to parse settings file ({e}); using defaults");
            T::default()
        }
    }
}

/// Split a legacy combined `settings.json` into the new local + cloud
/// pair. Operates on `serde_json::Value` rather than maintaining the
/// old monolithic struct so the legacy shape stops being a compile-time
/// dependency.
///
/// Unrecognised fields in the legacy file are dropped. `extras` on the
/// new cloud sections is reserved for *newer*-client unknown keys, not
/// retroactive preservation of old shapes.
fn split_legacy(path: &Path) -> Result<(LocalSettings, CloudSettingsCache)> {
    let bytes = std::fs::read_to_string(path)?;
    let legacy: Value = serde_json_lenient::from_str(&bytes)?;

    let mut local = LocalSettings::default();
    let mut cloud = CloudSettings::default();

    if let Some(general) = legacy.get("general").and_then(Value::as_object)
        && let Some(autostart) = general.get("autostart").and_then(Value::as_bool)
    {
        local.general.autostart = autostart;
    }

    if let Some(api) = legacy.get("api")
        && let Ok(parsed) = serde_json::from_value(api.clone())
    {
        local.api = parsed;
    }

    if let Some(telemetry) = legacy.get("telemetry").and_then(Value::as_object) {
        if let Some(distinct_id) = telemetry.get("distinctId").and_then(Value::as_str) {
            local.telemetry.distinct_id = Some(distinct_id.to_owned());
        }
        if let Some(v) = telemetry.get("consentVersion").and_then(Value::as_u64) {
            cloud.desktop.telemetry.consent_version = v.try_into().unwrap_or(u32::MAX);
        }
        if let Some(v) = telemetry.get("anonymousMetrics").and_then(Value::as_bool) {
            cloud.desktop.telemetry.anonymous_metrics = v;
        }
        if let Some(v) = telemetry.get("anonymousErrors").and_then(Value::as_bool) {
            cloud.desktop.telemetry.anonymous_errors = v;
        }
        if let Some(v) = telemetry.get("nonAnonymousMetrics").and_then(Value::as_bool) {
            cloud.desktop.telemetry.non_anonymous_metrics = v;
        }
    }

    if let Some(appearance) = legacy.get("appearance").and_then(Value::as_object) {
        if let Some(theme) = appearance.get("theme")
            && let Ok(parsed) = serde_json::from_value(theme.clone())
        {
            cloud.shared.theme = parsed;
        }
        if let Some(v) = appearance.get("dynamicAccent").and_then(Value::as_bool) {
            cloud.shared.dynamic_accent = v;
        }
        if let Some(v) = appearance.get("interfaceScale").and_then(Value::as_f64) {
            cloud.desktop.interface_scale = v as f32;
        }
        if let Some(v) = appearance.get("textScale").and_then(Value::as_f64) {
            cloud.desktop.text_scale = v as f32;
        }
    }

    cloud.schema_version = CURRENT_SCHEMA_VERSION;
    cloud.sanitize();

    Ok((
        local,
        CloudSettingsCache {
            // A legacy file lives on disk before any cloud round-trip
            // has happened, so the imported settings aren't tied to a
            // user id yet — the first authenticated pull will stamp it.
            last_user_id: None,
            settings: cloud,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ConnectionMode;
    use settings_core::{MAX_SCALE, MIN_SCALE, ThemePreference};

    fn write(dir: &Path, name: &str, contents: &str) {
        std::fs::write(dir.join(name), contents).expect("write fixture");
    }

    #[test]
    fn fresh_install_persists_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let state = SettingsState::load_or_migrate(tmp.path()).unwrap();
        assert_eq!(state, SettingsState::default());
        assert!(tmp.path().join(LOCAL_FILE).exists());
        assert!(tmp.path().join(CLOUD_FILE).exists());
    }

    #[test]
    fn existing_local_only_loads_with_default_cache() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            tmp.path(),
            LOCAL_FILE,
            r#"{"general": {"autostart": false}}"#,
        );
        let state = SettingsState::load_or_migrate(tmp.path()).unwrap();
        assert!(!state.local.general.autostart);
        assert_eq!(state.cache, CloudSettingsCache::default());
    }

    #[test]
    fn legacy_full_round_trip_splits_correctly() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            tmp.path(),
            LEGACY_FILE,
            r#"{
                "general": { "autostart": false },
                "api": { "mode": { "kind": "custom", "url": "https://example.test" } },
                "telemetry": {
                    "consentVersion": 1,
                    "anonymousMetrics": true,
                    "anonymousErrors": true,
                    "nonAnonymousMetrics": false,
                    "distinctId": "abc-123"
                },
                "appearance": {
                    "theme": "dark",
                    "dynamicAccent": false,
                    "interfaceScale": 1.25,
                    "textScale": 1.1
                }
            }"#,
        );

        let state = SettingsState::load_or_migrate(tmp.path()).unwrap();

        assert!(!state.local.general.autostart);
        assert!(matches!(
            state.local.api.mode,
            ConnectionMode::Custom { ref url } if url == "https://example.test"
        ));
        assert_eq!(state.local.telemetry.distinct_id.as_deref(), Some("abc-123"));

        let cloud = &state.cache.settings;
        assert_eq!(cloud.shared.theme, ThemePreference::Dark);
        assert!(!cloud.shared.dynamic_accent);
        assert_eq!(cloud.desktop.interface_scale, 1.25);
        assert_eq!(cloud.desktop.text_scale, 1.1);
        assert_eq!(cloud.desktop.telemetry.consent_version, 1);
        assert!(cloud.desktop.telemetry.anonymous_metrics);
        assert!(cloud.desktop.telemetry.anonymous_errors);
        assert!(!cloud.desktop.telemetry.non_anonymous_metrics);
        assert_eq!(cloud.schema_version, CURRENT_SCHEMA_VERSION);

        assert!(tmp.path().join(LOCAL_FILE).exists());
        assert!(tmp.path().join(CLOUD_FILE).exists());
        assert!(tmp.path().join(LEGACY_BACKUP_FILE).exists());
        assert!(!tmp.path().join(LEGACY_FILE).exists());
    }

    #[test]
    fn legacy_partial_only_general_fills_other_sections_with_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            tmp.path(),
            LEGACY_FILE,
            r#"{ "general": { "autostart": false } }"#,
        );

        let state = SettingsState::load_or_migrate(tmp.path()).unwrap();
        assert!(!state.local.general.autostart);
        assert!(state.local.telemetry.distinct_id.is_none());
        assert_eq!(state.cache.settings.shared.theme, ThemePreference::default());
    }

    #[test]
    fn legacy_malformed_falls_back_to_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), LEGACY_FILE, "not valid json {");

        let state = SettingsState::load_or_migrate(tmp.path()).unwrap();
        assert_eq!(state, SettingsState::default());
        // The malformed file is still renamed so the next launch skips
        // the migration branch.
        assert!(tmp.path().join(LEGACY_BACKUP_FILE).exists());
    }

    #[test]
    fn legacy_with_out_of_range_scales_is_sanitised() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            tmp.path(),
            LEGACY_FILE,
            r#"{ "appearance": { "interfaceScale": 9.0, "textScale": 0.1 } }"#,
        );

        let state = SettingsState::load_or_migrate(tmp.path()).unwrap();
        assert_eq!(state.cache.settings.desktop.interface_scale, MAX_SCALE);
        assert_eq!(state.cache.settings.desktop.text_scale, MIN_SCALE);
    }

    #[test]
    fn save_local_does_not_touch_cache_file() {
        let tmp = tempfile::tempdir().unwrap();
        let mut state = SettingsState::load_or_migrate(tmp.path()).unwrap();

        let cloud_before = std::fs::read_to_string(tmp.path().join(CLOUD_FILE)).unwrap();
        state.local.general.autostart = false;
        state.save_local(tmp.path()).unwrap();
        let cloud_after = std::fs::read_to_string(tmp.path().join(CLOUD_FILE)).unwrap();
        assert_eq!(cloud_before, cloud_after);

        let reloaded = SettingsState::load_or_migrate(tmp.path()).unwrap();
        assert!(!reloaded.local.general.autostart);
    }

    #[test]
    fn save_cache_does_not_touch_local_file() {
        let tmp = tempfile::tempdir().unwrap();
        let mut state = SettingsState::load_or_migrate(tmp.path()).unwrap();

        let local_before = std::fs::read_to_string(tmp.path().join(LOCAL_FILE)).unwrap();
        state.cache.settings.shared.theme = ThemePreference::Dark;
        state.save_cache(tmp.path()).unwrap();
        let local_after = std::fs::read_to_string(tmp.path().join(LOCAL_FILE)).unwrap();
        assert_eq!(local_before, local_after);

        let reloaded = SettingsState::load_or_migrate(tmp.path()).unwrap();
        assert_eq!(reloaded.cache.settings.shared.theme, ThemePreference::Dark);
    }

    #[test]
    fn migration_is_idempotent_when_local_already_exists() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            tmp.path(),
            LOCAL_FILE,
            r#"{"general": {"autostart": false}}"#,
        );
        // A stray legacy file is left untouched once the new layout is
        // in place — we don't re-import it and risk clobbering the
        // user's recent changes.
        write(tmp.path(), LEGACY_FILE, r#"{"general": {"autostart": true}}"#);

        let state = SettingsState::load_or_migrate(tmp.path()).unwrap();
        assert!(!state.local.general.autostart);
        assert!(tmp.path().join(LEGACY_FILE).exists());
        assert!(!tmp.path().join(LEGACY_BACKUP_FILE).exists());
    }

    #[test]
    fn unknown_cloud_extras_round_trip_through_save_and_load() {
        let tmp = tempfile::tempdir().unwrap();
        let raw = serde_json::json!({
            "settings": {
                "shared": { "theme": "light", "dynamicAccent": false, "futureKnob": "x" },
                "desktop": {
                    "interfaceScale": 1.0,
                    "textScale": 1.0,
                    "telemetry": {
                        "consentVersion": 1,
                        "anonymousMetrics": false,
                        "anonymousErrors": false,
                        "nonAnonymousMetrics": false,
                        "futureTelemetryKnob": true
                    }
                }
            }
        });
        write(tmp.path(), CLOUD_FILE, &raw.to_string());
        write(tmp.path(), LOCAL_FILE, "{}");

        let state = SettingsState::load_or_migrate(tmp.path()).unwrap();
        state.save_cache(tmp.path()).unwrap();

        let reread: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(tmp.path().join(CLOUD_FILE)).unwrap())
                .unwrap();
        assert_eq!(
            reread["settings"]["shared"]["futureKnob"],
            serde_json::json!("x")
        );
        assert_eq!(
            reread["settings"]["desktop"]["telemetry"]["futureTelemetryKnob"],
            serde_json::json!(true)
        );
    }

    #[test]
    fn read_or_default_recovers_from_malformed_file() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), CLOUD_FILE, "{not json");
        let cache = read_or_default::<CloudSettingsCache>(&tmp.path().join(CLOUD_FILE));
        assert_eq!(cache, CloudSettingsCache::default());
    }
}
