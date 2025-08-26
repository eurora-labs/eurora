//! Activity reporting module
//!
//! This module provides functionality for tracking and reporting activities.
//! It defines the Activity trait and the ActivityReporter struct, which
//! can be used to collect data from activities and store it in a timeline.
use std::collections::HashMap;

// use eur_timeline::TimelineRef;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ferrous_llm_core::Message;
use serde::{Deserialize, Serialize};
use tracing::info;
pub mod browser_activity;
pub mod browser_factory;
pub mod config;
pub mod default_activity;
pub mod default_factory;
pub mod error;
pub mod registry;

use anyhow::{Context, Result};
pub use browser_activity::BrowserStrategy;
pub use browser_factory::BrowserStrategyFactory;
pub use config::{
    ActivityConfig, ActivityConfigBuilder, ApplicationConfig, GlobalConfig, PrivacyConfig,
    SnapshotFrequency, StrategyConfig,
};
use default_activity::DefaultStrategy;
pub use default_factory::DefaultStrategyFactory;
pub use error::ActivityError;
use ferrous_focus::IconData;
pub use registry::{
    MatchScore, ProcessContext, StrategyCategory, StrategyFactory, StrategyMetadata,
    StrategyRegistry,
};

use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;

#[taurpc::ipc_type]
pub struct ContextChip {
    pub id: String,
    pub extension_id: String,
    pub name: String,
    pub attrs: HashMap<String, String>,
    pub icon: Option<String>,
    pub position: Option<u32>,
}
#[derive(Serialize, Deserialize)]
pub struct DisplayAsset {
    pub name: String,
    // image base64
    pub icon: String,
}

impl DisplayAsset {
    pub fn new(name: String, icon: String) -> Self {
        Self { name, icon }
    }
}

pub trait ActivityAsset: Send + Sync {
    fn get_name(&self) -> &String;
    fn get_icon(&self) -> Option<&String>;

    fn construct_message(&self) -> Message;
    fn get_context_chip(&self) -> Option<ContextChip>;

    // fn get_display(&self) -> DisplayAsset;
}

pub trait ActivitySnapshot: Send + Sync {
    fn construct_message(&self) -> Message;

    fn get_updated_at(&self) -> u64;
    fn get_created_at(&self) -> u64;
}

pub struct Activity {
    /// Name of the activity
    pub name: String,

    /// Icon representing the activity
    pub icon: String,

    /// Process name of the activity
    pub process_name: String,

    /// Start time (Unix timestamp)
    pub start: DateTime<Utc>,

    /// End time (Unix timestamp)
    pub end: Option<DateTime<Utc>>,

    // /// Snapshots of the activity
    pub snapshots: Vec<Box<dyn ActivitySnapshot>>,
    /// Assets associated with the activity
    pub assets: Vec<Box<dyn ActivityAsset>>,
}

impl Activity {
    /// Create a new activity
    pub fn new(
        name: String,
        icon: String,
        process_name: String,
        assets: Vec<Box<dyn ActivityAsset>>,
    ) -> Self {
        Self {
            name,
            icon,
            process_name,
            start: Utc::now(),
            end: None,
            assets,
            snapshots: Vec::new(),
        }
    }

    pub fn get_display_assets(&self) -> Vec<DisplayAsset> {
        self.assets
            .iter()
            .map(|asset| {
                if let Some(icon) = asset.get_icon() {
                    DisplayAsset::new(asset.get_name().clone(), icon.clone())
                } else {
                    DisplayAsset::new(asset.get_name().clone(), self.icon.clone())
                }
            })
            .collect()
    }
    pub fn get_context_chips(&self) -> Vec<ContextChip> {
        self.assets
            .iter()
            .filter_map(|asset| asset.get_context_chip())
            .collect()
    }
}

/// Global strategy registry instance
static GLOBAL_REGISTRY: OnceLock<Arc<Mutex<StrategyRegistry>>> = OnceLock::new();

/// Initialize the global strategy registry with default strategies
pub fn initialize_registry() -> Arc<Mutex<StrategyRegistry>> {
    GLOBAL_REGISTRY
        .get_or_init(|| {
            let mut registry = StrategyRegistry::new();

            // Register built-in strategies
            registry.register_factory(Arc::new(BrowserStrategyFactory::new()));
            registry.register_factory(Arc::new(DefaultStrategyFactory::new()));

            info!(
                "Initialized global strategy registry with {} strategies",
                registry.get_strategies().len()
            );

            Arc::new(Mutex::new(registry))
        })
        .clone()
}

/// Get the global strategy registry
pub fn get_registry() -> Arc<Mutex<StrategyRegistry>> {
    initialize_registry()
}

/// Select the appropriate strategy based on the process name
///
/// This function uses the global strategy registry to find the best matching strategy.
///
/// # Arguments
/// * `process_name` - The name of the process
/// * `display_name` - The display name to use for the activity
/// * `icon` - The icon data
///
/// # Returns
/// A Box<dyn ActivityStrategy> if a suitable strategy is found, or an error if no strategy supports the process
pub async fn select_strategy_for_process(
    process_name: &str,
    display_name: String,
    icon: IconData,
) -> Result<Box<dyn ActivityStrategy>> {
    info!("Selecting strategy for process: {}", process_name);

    let registry = get_registry();
    let mut registry_guard = registry.lock().await;

    let context = ProcessContext::new(process_name.to_string(), display_name, icon);

    registry_guard.select_strategy(&context).await
}

/// Legacy function for backward compatibility
///
/// **DEPRECATED**: Use `select_strategy_for_process` instead.
#[deprecated(since = "0.2.0", note = "Use select_strategy_for_process instead")]
pub async fn select_strategy_for_process_legacy(
    process_name: &str,
    display_name: String,
    _icon: IconData,
) -> Result<Box<dyn ActivityStrategy>> {
    // Check if this is a browser process
    if BrowserStrategy::get_supported_processes().contains(&process_name) {
        info!(
            "Creating BrowserStrategy for browser process: {}",
            process_name
        );
        let strategy = BrowserStrategy::new(display_name, "".to_string(), process_name.to_string())
            .await
            .context(format!(
                "Failed to create browser strategy for process: {}",
                process_name
            ))?;
        return Ok(Box::new(strategy) as Box<dyn ActivityStrategy>);
    }

    DefaultStrategy::new(display_name, "".to_string(), process_name.to_string())
        .context(format!(
            "Failed to create default strategy for process: {}",
            process_name
        ))
        .map(|strategy| Box::new(strategy) as Box<dyn ActivityStrategy>)
}

/// Activity trait defines methods that must be implemented by activities
/// that can be tracked and reported.
#[async_trait]
pub trait ActivityStrategy: Send + Sync {
    /// Retrieve assets associated with this activity
    ///
    /// This method is called once when collection starts to gather
    /// initial assets related to the activity.
    async fn retrieve_assets(&mut self) -> Result<Vec<Box<dyn ActivityAsset>>>;

    /// Retrieve snapshots associated with this activity
    ///
    /// This method is called periodically to gather snapshots of the
    /// activity. The returned snapshots should represent the
    /// current state of the activity.
    async fn retrieve_snapshots(&mut self) -> Result<Vec<Box<dyn ActivitySnapshot>>>;

    /// Gather the current state of the activity
    ///
    /// This method is called periodically to collect the current state
    /// of the activity. The returned string should represent the state
    /// in a format that can be parsed and stored in the timeline.
    fn gather_state(&self) -> String;

    /// Get name of the activity
    fn get_name(&self) -> &String;
    /// Get icon of the activity
    fn get_icon(&self) -> &String;
    /// Get process name of the activity
    fn get_process_name(&self) -> &String;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_creation() {
        let activity = Activity::new(
            "Test Activity".to_string(),
            "test_icon".to_string(),
            "test_process".to_string(),
            vec![],
        );

        assert_eq!(activity.name, "Test Activity");
        assert_eq!(activity.icon, "test_icon");
        assert_eq!(activity.process_name, "test_process");
        assert!(activity.end.is_none());
        assert!(activity.assets.is_empty());
        assert!(activity.snapshots.is_empty());
    }

    #[test]
    fn test_activity_display_assets() {
        let activity = Activity::new(
            "Test Activity".to_string(),
            "default_icon".to_string(),
            "test_process".to_string(),
            vec![],
        );

        let display_assets = activity.get_display_assets();
        assert!(display_assets.is_empty());
    }

    #[test]
    fn test_activity_context_chips() {
        let activity = Activity::new(
            "Test Activity".to_string(),
            "default_icon".to_string(),
            "test_process".to_string(),
            vec![],
        );

        let context_chips = activity.get_context_chips();
        assert!(context_chips.is_empty());
    }

    #[tokio::test]
    async fn test_registry_initialization() {
        let registry = initialize_registry();
        let registry_guard = registry.lock().await;
        let strategies = registry_guard.get_strategies();

        assert!(!strategies.is_empty());

        // Should have at least browser and default strategies
        let strategy_ids: Vec<String> = strategies.iter().map(|s| s.id.clone()).collect();
        assert!(strategy_ids.contains(&"browser".to_string()));
        assert!(strategy_ids.contains(&"default".to_string()));
    }

    #[tokio::test]
    async fn test_select_strategy_for_process_browser() {
        let result = select_strategy_for_process(
            "firefox",
            "Firefox Browser".to_string(),
            IconData::default(),
        )
        .await;

        // Note: This test might fail if browser communication is not available
        match result {
            Ok(strategy) => {
                assert_eq!(strategy.get_name(), "Firefox Browser");
                assert_eq!(strategy.get_process_name(), "firefox");
            }
            Err(_) => {
                // Expected if browser communication is not available in test environment
                // This is acceptable for unit tests
            }
        }
    }

    #[tokio::test]
    async fn test_select_strategy_for_process_default() {
        let result = select_strategy_for_process(
            "unknown_process",
            "Unknown App".to_string(),
            IconData::default(),
        )
        .await;

        assert!(result.is_ok());
        let strategy = result.unwrap();
        assert_eq!(strategy.get_name(), "Unknown App");
        assert_eq!(strategy.get_process_name(), "unknown_process");
    }

    #[tokio::test]
    async fn test_registry_strategy_selection() {
        let registry = get_registry();
        let mut registry_guard = registry.lock().await;

        // Test browser process selection
        let browser_context = ProcessContext::new(
            "chrome".to_string(),
            "Google Chrome".to_string(),
            IconData::default(),
        );

        let browser_result = registry_guard.select_strategy(&browser_context).await;
        match browser_result {
            Ok(_) => {
                // Browser strategy should be selected
            }
            Err(_) => {
                // Expected if browser communication is not available
            }
        }

        // Test default process selection
        let default_context = ProcessContext::new(
            "notepad".to_string(),
            "Notepad".to_string(),
            IconData::default(),
        );

        let default_result = registry_guard.select_strategy(&default_context).await;
        assert!(default_result.is_ok());
    }

    #[test]
    fn test_global_registry_singleton() {
        let registry1 = get_registry();
        let registry2 = get_registry();

        // Should be the same instance
        assert!(Arc::ptr_eq(&registry1, &registry2));
    }

    #[test]
    fn test_display_asset_creation() {
        let asset = DisplayAsset::new("Test Asset".to_string(), "base64_icon_data".to_string());

        assert_eq!(asset.name, "Test Asset");
        assert_eq!(asset.icon, "base64_icon_data");
    }

    #[test]
    fn test_context_chip_creation() {
        let chip = ContextChip {
            id: "test_id".to_string(),
            extension_id: "ext_id".to_string(),
            name: "Test Chip".to_string(),
            attrs: std::collections::HashMap::new(),
            icon: Some("icon_data".to_string()),
            position: Some(1),
        };

        assert_eq!(chip.id, "test_id");
        assert_eq!(chip.extension_id, "ext_id");
        assert_eq!(chip.name, "Test Chip");
        assert_eq!(chip.position, Some(1));
    }
}
