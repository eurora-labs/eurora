//! Refactored Activity System - Enum-Based Type-Safe Design
//!
//! This crate provides a completely refactored activity system that eliminates
//! dynamic trait objects in favor of concrete enums, providing better performance,
//! type safety, and cloneable activities.

pub mod assets;
pub mod config;
pub mod error;
pub mod registry;
pub mod snapshots;
pub mod storage;
pub mod strategies;
pub mod types;

// Re-export core types
use std::sync::{Arc, OnceLock};

// Re-export asset sub-types
pub use assets::twitter::{TwitterContextType, TwitterTweet};
pub use assets::youtube::TranscriptLine;
// Re-export asset types
pub use assets::{ArticleAsset, DefaultAsset, TwitterAsset, YoutubeAsset};
// Re-export configuration types
pub use config::{
    ActivityConfig, ActivityConfigBuilder, ApplicationConfig, GlobalConfig, PrivacyConfig,
    SnapshotFrequency, StrategyConfig,
};
pub use error::{ActivityError, ActivityResult};
use ferrous_focus::IconData;
// Re-export registry types
pub use registry::{
    MatchScore, ProcessContext, StrategyCategory, StrategyFactory, StrategyMetadata,
    StrategyRegistry,
};
// Re-export snapshot types
pub use snapshots::{ArticleSnapshot, DefaultSnapshot, TwitterSnapshot, YoutubeSnapshot};
// Re-export storage types
pub use storage::{ActivityStorage, ActivityStorageConfig, SaveableAsset, SavedAssetInfo};
pub use strategies::ActivityStrategy;
// Re-export strategy types
pub use strategies::{BrowserStrategy, DefaultStrategy};
use tokio::sync::Mutex;
use tracing::debug;
pub use types::{
    Activity, ActivityAsset, ActivitySnapshot, AssetFunctionality, ContextChip, DisplayAsset,
};

/// Global strategy registry instance
static GLOBAL_REGISTRY: OnceLock<Arc<Mutex<StrategyRegistry>>> = OnceLock::new();

/// Initialize the global strategy registry with default strategies
pub fn initialize_registry() -> Arc<Mutex<StrategyRegistry>> {
    GLOBAL_REGISTRY
        .get_or_init(|| {
            let mut registry = StrategyRegistry::new();

            // Register built-in strategy factories
            registry.register_factory(Arc::new(
                crate::strategies::browser::BrowserStrategyFactory::new(),
            ));
            registry.register_factory(Arc::new(
                crate::strategies::default::DefaultStrategyFactory::new(),
            ));

            debug!(
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
/// A ActivityStrategy if a suitable strategy is found, or an error if no strategy supports the process
pub async fn select_strategy_for_process(
    process_name: &str,
    display_name: String,
    icon: IconData,
) -> ActivityResult<ActivityStrategy> {
    debug!("Selecting strategy for process: {}", process_name);

    let registry = get_registry();
    let mut registry_guard = registry.lock().await;

    let context = ProcessContext::new(process_name.to_string(), display_name, icon);

    registry_guard.select_strategy(&context).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SnapshotFunctionality;

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
    fn test_activity_clone() {
        let mut activity = Activity::new(
            "Test Activity".to_string(),
            "test_icon".to_string(),
            "test_process".to_string(),
            vec![ActivityAsset::DefaultAsset(DefaultAsset::simple(
                "Test Asset".to_string(),
            ))],
        );

        activity.add_snapshot(ActivitySnapshot::DefaultSnapshot(DefaultSnapshot::new(
            "Test state".to_string(),
        )));

        // This should compile and work - the main benefit of the refactor!
        let cloned_activity = activity.clone();

        assert_eq!(activity.name, cloned_activity.name);
        assert_eq!(activity.assets.len(), cloned_activity.assets.len());
        assert_eq!(activity.snapshots.len(), cloned_activity.snapshots.len());
    }

    #[test]
    fn test_activity_serialization() {
        let activity = Activity::new(
            "Test Activity".to_string(),
            "test_icon".to_string(),
            "test_process".to_string(),
            vec![ActivityAsset::DefaultAsset(DefaultAsset::simple(
                "Test Asset".to_string(),
            ))],
        );

        // Test serialization
        let serialized = serde_json::to_string(&activity).unwrap();
        assert!(!serialized.is_empty());

        // Test deserialization
        let deserialized: Activity = serde_json::from_str(&serialized).unwrap();
        assert_eq!(activity.name, deserialized.name);
        assert_eq!(activity.assets.len(), deserialized.assets.len());
    }

    #[test]
    fn test_activity_display_assets() {
        let activity = Activity::new(
            "Test Activity".to_string(),
            "default_icon".to_string(),
            "test_process".to_string(),
            vec![
                ActivityAsset::YoutubeAsset(YoutubeAsset::new(
                    "yt1".to_string(),
                    "https://youtube.com/watch?v=test".to_string(),
                    "Test Video".to_string(),
                    vec![],
                    0.0,
                )),
                ActivityAsset::DefaultAsset(DefaultAsset::simple("Test Asset".to_string())),
            ],
        );

        let display_assets = activity.get_display_assets();
        assert_eq!(display_assets.len(), 2);
        assert_eq!(display_assets[0].name, "Test Video");
        assert_eq!(display_assets[0].icon, "youtube");
        assert_eq!(display_assets[1].name, "Test Asset");
        assert_eq!(display_assets[1].icon, "default"); // Falls back to activity icon
    }

    #[test]
    fn test_activity_context_chips() {
        let activity = Activity::new(
            "Test Activity".to_string(),
            "default_icon".to_string(),
            "test_process".to_string(),
            vec![
                ActivityAsset::YoutubeAsset(YoutubeAsset::new(
                    "yt1".to_string(),
                    "https://youtube.com/watch?v=test".to_string(),
                    "Test Video".to_string(),
                    vec![],
                    0.0,
                )),
                ActivityAsset::DefaultAsset(DefaultAsset::simple("Test Asset".to_string())),
            ],
        );

        let context_chips = activity.get_context_chips();
        assert_eq!(context_chips.len(), 1); // Only YouTube asset provides a context chip
        assert_eq!(context_chips[0].name, "video");
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

    #[test]
    fn test_global_registry_singleton() {
        let registry1 = get_registry();
        let registry2 = get_registry();

        // Should be the same instance
        assert!(Arc::ptr_eq(&registry1, &registry2));
    }

    #[test]
    fn test_asset_enum_methods() {
        let youtube_asset = ActivityAsset::YoutubeAsset(YoutubeAsset::new(
            "yt1".to_string(),
            "https://youtube.com/watch?v=test".to_string(),
            "Test Video".to_string(),
            vec![],
            0.0,
        ));

        assert_eq!(youtube_asset.get_name(), "Test Video");
        assert_eq!(youtube_asset.get_icon(), Some("youtube"));
        assert!(youtube_asset.get_context_chip().is_some());

        let default_asset =
            ActivityAsset::DefaultAsset(DefaultAsset::simple("Test Asset".to_string()));
        assert_eq!(default_asset.get_name(), "Test Asset");
        assert_eq!(default_asset.get_icon(), Some("default"));
        assert!(default_asset.get_context_chip().is_none());
    }
}
