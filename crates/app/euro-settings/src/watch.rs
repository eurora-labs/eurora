use anyhow::Result;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher, event::ModifyKind};
use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard, mpsc},
    time::Duration,
};
use tracing::{debug, error};

use crate::AppSettings;

#[derive(Clone)]
pub struct SettingsWithDiskSync {
    config_path: PathBuf,
    snapshot: Arc<RwLock<AppSettings>>,
}

/// Wrapper that asserts mutations are saved to disk before being dropped.
pub(crate) struct SettingsEnforceSaveToDisk<'a> {
    config_path: &'a Path,
    snapshot: RwLockWriteGuard<'a, AppSettings>,
    saved: bool,
}

impl SettingsEnforceSaveToDisk<'_> {
    #[allow(dead_code)]
    pub fn save(&mut self) -> Result<()> {
        // Mark before save so a save failure doesn't trigger the Drop assertion
        self.saved = true;
        self.snapshot.save(self.config_path)?;
        Ok(())
    }
}

impl Deref for SettingsEnforceSaveToDisk<'_> {
    type Target = AppSettings;

    fn deref(&self) -> &Self::Target {
        &self.snapshot
    }
}

impl DerefMut for SettingsEnforceSaveToDisk<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.snapshot
    }
}

impl Drop for SettingsEnforceSaveToDisk<'_> {
    fn drop(&mut self) {
        assert!(
            self.saved,
            "BUG: every change must immediately be saved to disk."
        );
    }
}

pub(crate) const SETTINGS_FILE: &str = "settings.json";

impl SettingsWithDiskSync {
    #[allow(dead_code)]
    pub fn new(config_dir: impl AsRef<Path>) -> Result<Self> {
        let config_path = config_dir.as_ref().join(SETTINGS_FILE);
        let app_settings = AppSettings::load(&config_path)?;
        let app_settings = Arc::new(RwLock::new(app_settings));

        Ok(Self {
            config_path,
            snapshot: app_settings,
        })
    }

    #[allow(dead_code)]
    pub fn get(&self) -> Result<RwLockReadGuard<'_, AppSettings>> {
        self.snapshot
            .read()
            .map_err(|e| anyhow::anyhow!("Could not read settings: {:?}", e))
    }

    #[allow(dead_code)]
    pub(crate) fn get_mut_enforce_save(&self) -> Result<SettingsEnforceSaveToDisk<'_>> {
        self.snapshot
            .write()
            .map(|snapshot| SettingsEnforceSaveToDisk {
                snapshot,
                config_path: &self.config_path,
                saved: false,
            })
            .map_err(|e| anyhow::anyhow!("Could not write settings: {:?}", e))
    }

    #[allow(dead_code)]
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    #[allow(dead_code)]
    pub fn watch_in_background(
        &mut self,
        send_event: impl Fn(AppSettings) -> Result<()> + Send + Sync + 'static,
    ) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        let snapshot = self.snapshot.clone();
        let config_path = self.config_path.to_owned();
        let watcher_config = Config::default()
            .with_compare_contents(true)
            .with_poll_interval(Duration::from_secs(2));
        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut watcher: RecommendedWatcher = Watcher::new(tx, watcher_config)?;
            watcher.watch(&config_path, RecursiveMode::NonRecursive)?;
            loop {
                match rx.recv() {
                    Ok(Ok(Event {
                        kind: notify::event::EventKind::Modify(ModifyKind::Data(_)),
                        ..
                    })) => {
                        let Ok(mut last_seen_settings) = snapshot.write() else {
                            continue;
                        };
                        if let Ok(update) = AppSettings::load(&config_path) {
                            debug!("settings.json modified; refreshing settings");
                            *last_seen_settings = update.clone();
                            send_event(update)?;
                        }
                    }

                    Err(_) => {
                        error!(
                            "Error watching config file {:?} - watcher terminated",
                            config_path
                        );
                        break;
                    }

                    _ => {
                        // Noop
                    }
                }
            }
            Ok(())
        });
        Ok(())
    }
}
