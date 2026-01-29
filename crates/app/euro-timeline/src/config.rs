//! Configuration types for the timeline module

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Configuration for timeline storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Maximum number of activities to keep in memory
    pub max_activities: usize,
    /// Maximum age of activities before cleanup
    pub max_age: Duration,
    /// Whether to automatically cleanup old activities
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

/// Configuration for the collector service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorConfig {
    /// How often to collect activity snapshots
    pub collection_interval: Duration,
    /// Whether to automatically restart on errors
    pub auto_restart_on_error: bool,
    /// Maximum number of restart attempts
    pub max_restart_attempts: u32,
    /// Delay between restart attempts
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

/// Configuration for focus tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusTrackingConfig {
    /// Processes to ignore (e.g., the app itself)
    pub ignored_processes: Vec<String>,
}

impl Default for FocusTrackingConfig {
    fn default() -> Self {
        Self {
            ignored_processes: vec!["euro-tauri".to_string(), "euro-tauri.exe".to_string()],
        }
    }
}

/// Main timeline configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimelineConfig {
    /// Storage configuration
    pub storage: StorageConfig,
    /// Collector configuration
    pub collector: CollectorConfig,
    /// Focus tracking configuration
    pub focus_tracking: FocusTrackingConfig,
}

impl TimelineConfig {
    /// Validate the configuration
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
