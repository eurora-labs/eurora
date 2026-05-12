use crate::{
    cloud_cache::CloudSettingsCache, effective::EffectiveSettings, local::LocalSettings,
};

/// Single owner of the on-disk settings split. Stored in `tauri::Manager`
/// state under a `tokio::sync::Mutex`; every settings IPC handler locks
/// it, mutates whichever section it owns, and persists *only* the file
/// that changed via [`crate::persistence`].
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SettingsState {
    pub local: LocalSettings,
    pub cache: CloudSettingsCache,
}

impl SettingsState {
    pub fn new(local: LocalSettings, cache: CloudSettingsCache) -> Self {
        Self { local, cache }
    }

    /// Borrowed composite view, for handlers that only read.
    pub fn effective(&self) -> EffectiveSettings<'_> {
        EffectiveSettings::new(&self.local, &self.cache)
    }
}
