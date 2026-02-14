use std::{collections::HashMap, time::Duration};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub enabled: bool,
    #[serde(with = "humantime_serde")]
    pub default_collection_interval: Duration,
    pub max_assets_per_activity: usize,
    pub max_snapshots_per_activity: usize,
    pub privacy: PrivacyConfig,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_collection_interval: Duration::from_secs(5),
            max_assets_per_activity: 10,
            max_snapshots_per_activity: 100,
            privacy: PrivacyConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    pub collect_content: bool,
    pub anonymize_data: bool,
    pub exclude_patterns: Vec<String>,
    pub ignored_applications: Vec<String>,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            collect_content: true,
            anonymize_data: false,
            exclude_patterns: vec![
                r"password".to_string(),
                r"secret".to_string(),
                r"token".to_string(),
                r"key".to_string(),
            ],
            ignored_applications: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub enabled: bool,
    pub priority: u32,
    #[serde(with = "humantime_serde")]
    pub collection_interval: Duration,
    pub asset_types: Vec<String>,
    pub snapshot_frequency: SnapshotFrequency,
    pub settings: HashMap<String, String>,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            priority: 50,
            collection_interval: Duration::from_secs(5),
            asset_types: vec![],
            snapshot_frequency: SnapshotFrequency::Interval(Duration::from_secs(10)),
            settings: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SnapshotFrequency {
    Never,
    Interval(#[serde(with = "humantime_serde")] Duration),
    OnEvent(Vec<String>),
    OnChange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationConfig {
    pub enabled: bool,
    pub force_strategy: Option<String>,
    pub privacy_override: Option<PrivacyConfig>,
    #[serde(with = "humantime_serde")]
    pub collection_interval_override: Option<Duration>,
    pub settings: HashMap<String, String>,
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            force_strategy: None,
            privacy_override: None,
            collection_interval_override: None,
            settings: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActivityConfig {
    pub global: GlobalConfig,
    pub strategies: HashMap<String, StrategyConfig>,
    pub applications: HashMap<String, ApplicationConfig>,
}

#[derive(Debug, Default)]
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

    pub fn max_snapshots_per_activity(mut self, max: usize) -> Self {
        self.config.global.max_snapshots_per_activity = max;
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

    pub fn add_exclusion_pattern(mut self, pattern: String) -> Self {
        self.config.global.privacy.exclude_patterns.push(pattern);
        self
    }

    pub fn ignore_application(mut self, app: String) -> Self {
        self.config.global.privacy.ignored_applications.push(app);
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

impl ActivityConfig {
    pub fn builder() -> ActivityConfigBuilder {
        ActivityConfigBuilder::new()
    }

    pub fn get_strategy_config(&self, strategy_id: &str) -> Option<&StrategyConfig> {
        self.strategies.get(strategy_id)
    }

    pub fn get_application_config(&self, app_name: &str) -> Option<&ApplicationConfig> {
        self.applications.get(app_name)
    }

    pub fn is_collection_enabled(&self) -> bool {
        self.global.enabled
    }

    pub fn is_application_enabled(&self, app_name: &str) -> bool {
        if !self.global.enabled {
            return false;
        }

        if self
            .global
            .privacy
            .ignored_applications
            .contains(&app_name.to_string())
        {
            return false;
        }

        self.get_application_config(app_name)
            .is_none_or(|config| config.enabled)
    }

    pub fn get_collection_interval(&self, app_name: &str, strategy_id: &str) -> Duration {
        // Check application-specific override first
        if let Some(app_config) = self.get_application_config(app_name)
            && let Some(interval) = app_config.collection_interval_override
        {
            return interval;
        }

        // Check strategy-specific configuration
        if let Some(strategy_config) = self.get_strategy_config(strategy_id) {
            return strategy_config.collection_interval;
        }

        // Fall back to global default
        self.global.default_collection_interval
    }

    pub fn get_privacy_config(&self, app_name: &str) -> &PrivacyConfig {
        self.get_application_config(app_name)
            .and_then(|config| config.privacy_override.as_ref())
            .unwrap_or(&self.global.privacy)
    }

    pub fn should_collect_content(&self, app_name: &str) -> bool {
        self.get_privacy_config(app_name).collect_content
    }

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

        for pattern in &self.global.privacy.exclude_patterns {
            if let Err(e) = regex::Regex::new(pattern) {
                return Err(format!("Invalid regex pattern '{}': {}", pattern, e));
            }
        }

        for (id, sc) in &self.strategies {
            if sc.collection_interval.is_zero() {
                return Err(format!(
                    "strategy '{}' collection_interval must be greater than 0",
                    id
                ));
            }
        }
        for (name, app) in &self.applications {
            if let Some(d) = app.collection_interval_override
                && d.is_zero()
            {
                return Err(format!(
                    "application '{}' collection_interval_override must be greater than 0",
                    name
                ));
            }
        }

        Ok(())
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
            Duration::from_secs(5)
        );
        assert_eq!(config.global.max_assets_per_activity, 10);
        assert_eq!(config.global.max_snapshots_per_activity, 100);
        assert!(config.global.privacy.collect_content);
        assert!(!config.global.privacy.anonymize_data);
    }

    #[test]
    fn test_config_builder() {
        let config = ActivityConfig::builder()
            .enable_collection(false)
            .default_collection_interval(Duration::from_secs(10))
            .max_assets_per_activity(5)
            .collect_content(false)
            .anonymize_data(true)
            .add_exclusion_pattern(r"sensitive".to_string())
            .ignore_application("private-app".to_string())
            .build();

        assert!(!config.global.enabled);
        assert_eq!(
            config.global.default_collection_interval,
            Duration::from_secs(10)
        );
        assert_eq!(config.global.max_assets_per_activity, 5);
        assert!(!config.global.privacy.collect_content);
        assert!(config.global.privacy.anonymize_data);
        assert!(
            config
                .global
                .privacy
                .exclude_patterns
                .contains(&"sensitive".to_string())
        );
        assert!(
            config
                .global
                .privacy
                .ignored_applications
                .contains(&"private-app".to_string())
        );
    }

    #[test]
    fn test_strategy_configuration() {
        let strategy_config = StrategyConfig {
            enabled: true,
            priority: 80,
            collection_interval: Duration::from_secs(3),
            asset_types: vec!["youtube".to_string(), "article".to_string()],
            snapshot_frequency: SnapshotFrequency::Interval(Duration::from_secs(15)),
            settings: HashMap::new(),
        };

        let config = ActivityConfig::builder()
            .configure_strategy("browser".to_string(), strategy_config.clone())
            .build();

        let retrieved = config.get_strategy_config("browser").unwrap();
        assert_eq!(retrieved.priority, 80);
        assert_eq!(retrieved.collection_interval, Duration::from_secs(3));
        assert_eq!(retrieved.asset_types.len(), 2);
    }

    #[test]
    fn test_application_configuration() {
        let app_config = ApplicationConfig {
            enabled: false,
            force_strategy: Some("custom".to_string()),
            privacy_override: Some(PrivacyConfig {
                collect_content: false,
                ..Default::default()
            }),
            collection_interval_override: Some(Duration::from_secs(30)),
            settings: HashMap::new(),
        };

        let config = ActivityConfig::builder()
            .configure_application("firefox".to_string(), app_config)
            .build();

        assert!(!config.is_application_enabled("firefox"));
        assert_eq!(
            config.get_collection_interval("firefox", "browser"),
            Duration::from_secs(30)
        );
        assert!(!config.should_collect_content("firefox"));
    }

    #[test]
    fn test_collection_interval_precedence() {
        let strategy_config = StrategyConfig {
            collection_interval: Duration::from_secs(7),
            ..Default::default()
        };

        let app_config = ApplicationConfig {
            collection_interval_override: Some(Duration::from_secs(15)),
            ..Default::default()
        };

        let config = ActivityConfig::builder()
            .default_collection_interval(Duration::from_secs(5))
            .configure_strategy("browser".to_string(), strategy_config)
            .configure_application("firefox".to_string(), app_config)
            .build();

        // Application override should take precedence
        assert_eq!(
            config.get_collection_interval("firefox", "browser"),
            Duration::from_secs(15)
        );

        // Strategy config should be used for other apps
        assert_eq!(
            config.get_collection_interval("chrome", "browser"),
            Duration::from_secs(7)
        );

        // Global default for unknown strategy
        assert_eq!(
            config.get_collection_interval("notepad", "unknown"),
            Duration::from_secs(5)
        );
    }

    #[test]
    fn test_ignored_applications() {
        let config = ActivityConfig::builder()
            .ignore_application("private-app".to_string())
            .build();

        assert!(!config.is_application_enabled("private-app"));
        assert!(config.is_application_enabled("public-app"));
    }

    #[test]
    fn test_config_validation() {
        let valid_config = ActivityConfig::default();
        assert!(valid_config.validate().is_ok());

        let invalid_config = ActivityConfig {
            global: GlobalConfig {
                max_assets_per_activity: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_snapshot_frequency() {
        let interval = SnapshotFrequency::Interval(Duration::from_secs(10));

        let serialized = serde_json::to_string(&interval).unwrap();
        assert!(!serialized.is_empty());
    }
}
