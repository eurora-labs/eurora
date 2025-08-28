//! Configuration system for activity strategies

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Configuration for the activity system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityConfig {
    /// Global configuration settings
    pub global: GlobalConfig,
    /// Strategy-specific configurations
    pub strategies: HashMap<String, StrategyConfig>,
    /// Application-specific configurations
    pub applications: HashMap<String, ApplicationConfig>,
}

/// Global configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Whether activity collection is enabled
    pub enabled: bool,
    /// Default collection interval for strategies
    pub default_collection_interval: Duration,
    /// Maximum number of assets per activity
    pub max_assets_per_activity: usize,
    /// Maximum number of snapshots per activity
    pub max_snapshots_per_activity: usize,
    /// Privacy settings
    pub privacy: PrivacyConfig,
}

/// Privacy configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    /// Whether to collect content data (vs metadata only)
    pub collect_content: bool,
    /// Whether to anonymize collected data
    pub anonymize_data: bool,
    /// Patterns to exclude from collection (regex)
    pub exclude_patterns: Vec<String>,
    /// Applications to completely ignore
    pub ignored_applications: Vec<String>,
}

/// Configuration for a specific strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    /// Whether this strategy is enabled
    pub enabled: bool,
    /// Priority for strategy selection (higher = more preferred)
    pub priority: u8,
    /// Collection interval for this strategy
    pub collection_interval: Duration,
    /// Asset types to collect
    pub asset_types: Vec<String>,
    /// Snapshot frequency
    pub snapshot_frequency: SnapshotFrequency,
    /// Strategy-specific settings
    pub settings: HashMap<String, serde_json::Value>,
}

/// Configuration for a specific application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationConfig {
    /// Whether to collect data from this application
    pub enabled: bool,
    /// Override strategy to use for this application
    pub override_strategy: Option<String>,
    /// Application-specific privacy settings
    pub privacy_override: Option<PrivacyConfig>,
    /// Custom settings for this application
    pub settings: HashMap<String, serde_json::Value>,
}

/// Frequency settings for snapshot collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SnapshotFrequency {
    /// Never collect snapshots
    Never,
    /// Collect on activity change only
    OnChange,
    /// Collect at regular intervals
    Interval(Duration),
    /// Collect based on user interaction
    OnInteraction,
}

impl Default for ActivityConfig {
    fn default() -> Self {
        Self {
            global: GlobalConfig::default(),
            strategies: HashMap::new(),
            applications: HashMap::new(),
        }
    }
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_collection_interval: Duration::from_secs(3),
            max_assets_per_activity: 10,
            max_snapshots_per_activity: 100,
            privacy: PrivacyConfig::default(),
        }
    }
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            collect_content: true,
            anonymize_data: false,
            exclude_patterns: vec![
                r"password".to_string(),
                r"token".to_string(),
                r"secret".to_string(),
                r"key".to_string(),
            ],
            ignored_applications: vec![],
        }
    }
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            priority: 50,
            collection_interval: Duration::from_secs(3),
            asset_types: vec!["*".to_string()],
            snapshot_frequency: SnapshotFrequency::Interval(Duration::from_secs(10)),
            settings: HashMap::new(),
        }
    }
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            override_strategy: None,
            privacy_override: None,
            settings: HashMap::new(),
        }
    }
}

/// Builder for activity configuration
pub struct ActivityConfigBuilder {
    config: ActivityConfig,
}

impl ActivityConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: ActivityConfig::default(),
        }
    }

    pub fn enable_collection(mut self, enabled: bool) -> Self {
        self.config.global.enabled = enabled;
        self
    }

    pub fn default_collection_interval(mut self, interval: Duration) -> Self {
        self.config.global.default_collection_interval = interval;
        self
    }

    pub fn max_assets_per_activity(mut self, max: usize) -> Self {
        self.config.global.max_assets_per_activity = max;
        self
    }

    pub fn collect_content(mut self, collect: bool) -> Self {
        self.config.global.privacy.collect_content = collect;
        self
    }

    pub fn configure_strategy(mut self, strategy_id: String, config: StrategyConfig) -> Self {
        self.config.strategies.insert(strategy_id, config);
        self
    }

    pub fn build(self) -> ActivityConfig {
        self.config
    }
}

impl Default for ActivityConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
