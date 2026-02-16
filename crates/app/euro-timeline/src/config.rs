use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub max_activities: usize,
    pub max_age: Duration,
    pub auto_cleanup: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            max_activities: 5,
            max_age: Duration::from_secs(3600), // 1 hour
            auto_cleanup: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorConfig {
    pub collection_interval: Duration,
    pub auto_restart_on_error: bool,
    pub max_restart_attempts: u32,
    pub restart_delay: Duration,
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self {
            collection_interval: Duration::from_secs(3),
            auto_restart_on_error: true,
            max_restart_attempts: 5,
            restart_delay: Duration::from_secs(1),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusTrackingConfig {
    pub ignored_processes: Vec<String>,
}

impl Default for FocusTrackingConfig {
    fn default() -> Self {
        Self {
            ignored_processes: vec!["euro-tauri".to_string(), "euro-tauri.exe".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimelineConfig {
    pub storage: StorageConfig,
    pub collector: CollectorConfig,
    pub focus_tracking: FocusTrackingConfig,
}

impl TimelineConfig {
    pub fn validate(&self) -> crate::error::TimelineResult<()> {
        if self.storage.max_activities == 0 {
            return Err(crate::error::TimelineError::Configuration(
                "max_activities must be greater than 0".to_string(),
            ));
        }

        if self.collector.collection_interval.is_zero() {
            return Err(crate::error::TimelineError::Configuration(
                "collection_interval must be greater than 0".to_string(),
            ));
        }

        if self.storage.max_age.is_zero() {
            return Err(crate::error::TimelineError::Configuration(
                "max_age must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}
