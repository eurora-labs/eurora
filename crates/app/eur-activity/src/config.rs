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

impl ActivityConfig {
    /// Create a new configuration with default browser strategy settings
    pub fn with_browser_defaults() -> Self {
        let mut config = Self::default();

        // Configure browser strategy
        let mut browser_config = StrategyConfig::default();
        browser_config.priority = 80;
        browser_config.asset_types = vec![
            "youtube".to_string(),
            "article".to_string(),
            "twitter".to_string(),
            "pdf".to_string(),
        ];
        browser_config.snapshot_frequency = SnapshotFrequency::Interval(Duration::from_secs(5));

        config
            .strategies
            .insert("browser".to_string(), browser_config);

        // Configure default strategy
        let mut default_config = StrategyConfig::default();
        default_config.priority = 10;
        default_config.snapshot_frequency = SnapshotFrequency::Never;

        config
            .strategies
            .insert("default".to_string(), default_config);

        config
    }

    /// Get configuration for a specific strategy
    pub fn get_strategy_config(&self, strategy_id: &str) -> StrategyConfig {
        self.strategies
            .get(strategy_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get configuration for a specific application
    pub fn get_application_config(&self, app_name: &str) -> ApplicationConfig {
        self.applications.get(app_name).cloned().unwrap_or_default()
    }

    /// Check if an application should be ignored
    pub fn is_application_ignored(&self, app_name: &str) -> bool {
        // Check global ignore list
        if self
            .global
            .privacy
            .ignored_applications
            .contains(&app_name.to_string())
        {
            return true;
        }

        // Check application-specific config
        let app_config = self.get_application_config(app_name);
        !app_config.enabled
    }

    /// Check if content should be collected for an application
    pub fn should_collect_content(&self, app_name: &str) -> bool {
        let app_config = self.get_application_config(app_name);

        // Use application-specific privacy override if available
        if let Some(privacy_override) = &app_config.privacy_override {
            privacy_override.collect_content
        } else {
            self.global.privacy.collect_content
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.global.max_assets_per_activity == 0 {
            return Err("max_assets_per_activity must be greater than 0".to_string());
        }

        if self.global.max_snapshots_per_activity == 0 {
            return Err("max_snapshots_per_activity must be greater than 0".to_string());
        }

        if self.global.default_collection_interval.is_zero() {
            return Err("default_collection_interval must be greater than 0".to_string());
        }

        // Validate strategy priorities
        for (strategy_id, config) in &self.strategies {
            if config.collection_interval.is_zero() {
                return Err(format!(
                    "Strategy {} has invalid collection_interval",
                    strategy_id
                ));
            }
        }

        Ok(())
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

    pub fn anonymize_data(mut self, anonymize: bool) -> Self {
        self.config.global.privacy.anonymize_data = anonymize;
        self
    }

    pub fn ignore_application(mut self, app_name: String) -> Self {
        self.config
            .global
            .privacy
            .ignored_applications
            .push(app_name);
        self
    }

    pub fn configure_strategy(mut self, strategy_id: String, config: StrategyConfig) -> Self {
        self.config.strategies.insert(strategy_id, config);
        self
    }

    pub fn configure_application(mut self, app_name: String, config: ApplicationConfig) -> Self {
        self.config.applications.insert(app_name, config);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ActivityConfig::default();
        assert!(config.global.enabled);
        assert_eq!(
            config.global.default_collection_interval,
            Duration::from_secs(3)
        );
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_browser_defaults() {
        let config = ActivityConfig::with_browser_defaults();

        let browser_config = config.get_strategy_config("browser");
        assert_eq!(browser_config.priority, 80);
        assert!(browser_config.asset_types.contains(&"youtube".to_string()));

        let default_config = config.get_strategy_config("default");
        assert_eq!(default_config.priority, 10);
    }

    #[test]
    fn test_config_builder() {
        let config = ActivityConfigBuilder::new()
            .enable_collection(false)
            .default_collection_interval(Duration::from_secs(5))
            .max_assets_per_activity(20)
            .collect_content(false)
            .ignore_application("sensitive_app".to_string())
            .build();

        assert!(!config.global.enabled);
        assert_eq!(
            config.global.default_collection_interval,
            Duration::from_secs(5)
        );
        assert_eq!(config.global.max_assets_per_activity, 20);
        assert!(!config.global.privacy.collect_content);
        assert!(config.is_application_ignored("sensitive_app"));
    }

    #[test]
    fn test_config_validation() {
        let mut config = ActivityConfig::default();

        // Valid config should pass
        assert!(config.validate().is_ok());

        // Invalid max_assets_per_activity
        config.global.max_assets_per_activity = 0;
        assert!(config.validate().is_err());

        // Fix and test invalid collection interval
        config.global.max_assets_per_activity = 10;
        config.global.default_collection_interval = Duration::ZERO;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_application_config() {
        let mut config = ActivityConfig::default();

        // Add application-specific config
        let mut app_config = ApplicationConfig::default();
        app_config.enabled = false;
        config
            .applications
            .insert("test_app".to_string(), app_config);

        assert!(config.is_application_ignored("test_app"));
        assert!(!config.is_application_ignored("other_app"));
    }

    #[test]
    fn test_privacy_settings() {
        let mut config = ActivityConfig::default();

        // Test global privacy settings
        assert!(config.should_collect_content("any_app"));

        config.global.privacy.collect_content = false;
        assert!(!config.should_collect_content("any_app"));

        // Test application-specific privacy override
        let mut app_config = ApplicationConfig::default();
        app_config.privacy_override = Some(PrivacyConfig {
            collect_content: true,
            ..Default::default()
        });
        config
            .applications
            .insert("special_app".to_string(), app_config);

        assert!(config.should_collect_content("special_app"));
        assert!(!config.should_collect_content("other_app"));
    }
}
