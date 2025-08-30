//! Configuration types for the timeline module

use serde::{Deserialize, Serialize};
use std::time::Duration;

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
            max_activities: 1000,
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
    /// Whether to track focus changes
    pub enabled: bool,
    /// Processes to ignore (e.g., the app itself)
    pub ignored_processes: Vec<String>,
    /// Minimum time a window must be focused before tracking
    pub min_focus_duration: Duration,
}

impl Default for FocusTrackingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ignored_processes: vec!["eur-tauri".to_string(), "eur-tauri.exe".to_string()],
            min_focus_duration: Duration::from_millis(500),
        }
    }
}

/// Main timeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineConfig {
    /// Storage configuration
    pub storage: StorageConfig,
    /// Collector configuration
    pub collector: CollectorConfig,
    /// Focus tracking configuration
    pub focus_tracking: FocusTrackingConfig,
}

impl Default for TimelineConfig {
    fn default() -> Self {
        Self {
            storage: StorageConfig::default(),
            collector: CollectorConfig::default(),
            focus_tracking: FocusTrackingConfig::default(),
        }
    }
}

/// Builder for timeline configuration
pub struct TimelineConfigBuilder {
    config: TimelineConfig,
}

impl TimelineConfigBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: TimelineConfig::default(),
        }
    }

    /// Set maximum number of activities to keep
    pub fn max_activities(mut self, count: usize) -> Self {
        self.config.storage.max_activities = count;
        self
    }

    /// Set collection interval
    pub fn collection_interval(mut self, interval: Duration) -> Self {
        self.config.collector.collection_interval = interval;
        self
    }

    /// Set maximum age for activities
    pub fn max_age(mut self, age: Duration) -> Self {
        self.config.storage.max_age = age;
        self
    }

    /// Disable focus tracking
    pub fn disable_focus_tracking(mut self) -> Self {
        self.config.focus_tracking.enabled = false;
        self
    }

    /// Enable focus tracking
    pub fn enable_focus_tracking(mut self) -> Self {
        self.config.focus_tracking.enabled = true;
        self
    }

    /// Disable automatic cleanup
    pub fn disable_auto_cleanup(mut self) -> Self {
        self.config.storage.auto_cleanup = false;
        self
    }

    /// Disable automatic restart on errors
    pub fn disable_auto_restart(mut self) -> Self {
        self.config.collector.auto_restart_on_error = false;
        self
    }

    /// Add ignored process for focus tracking
    pub fn ignore_process(mut self, process_name: String) -> Self {
        self.config
            .focus_tracking
            .ignored_processes
            .push(process_name);
        self
    }

    /// Set minimum focus duration
    pub fn min_focus_duration(mut self, duration: Duration) -> Self {
        self.config.focus_tracking.min_focus_duration = duration;
        self
    }

    /// Build the configuration
    pub fn build(self) -> TimelineConfig {
        self.config
    }
}

impl TimelineConfig {
    /// Create a new builder
    pub fn builder() -> TimelineConfigBuilder {
        TimelineConfigBuilder::new()
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TimelineConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.storage.max_activities, 1000);
        assert_eq!(config.collector.collection_interval, Duration::from_secs(3));
        assert!(config.focus_tracking.enabled);
    }

    #[test]
    fn test_config_builder() {
        let config = TimelineConfig::builder()
            .max_activities(500)
            .collection_interval(Duration::from_secs(5))
            .disable_focus_tracking()
            .build();

        assert!(config.validate().is_ok());
        assert_eq!(config.storage.max_activities, 500);
        assert_eq!(config.collector.collection_interval, Duration::from_secs(5));
        assert!(!config.focus_tracking.enabled);
    }

    #[test]
    fn test_config_validation() {
        let mut config = TimelineConfig::default();
        config.storage.max_activities = 0;
        assert!(config.validate().is_err());

        config.storage.max_activities = 100;
        config.collector.collection_interval = Duration::ZERO;
        assert!(config.validate().is_err());
    }
}
