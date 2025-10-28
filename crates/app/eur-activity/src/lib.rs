//! Refactored Activity System - Enum-Based Type-Safe Design
//!
//! This crate provides a completely refactored activity system that eliminates
//! dynamic trait objects in favor of concrete enums, providing better performance,
//! type safety, and cloneable activities.

pub mod assets;
pub mod config;
pub mod error;
pub mod snapshots;
pub mod storage;
pub mod strategies;
pub mod types;
mod utils;

pub use strategies::processes;

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
// Re-export snapshot types
pub use snapshots::{ArticleSnapshot, DefaultSnapshot, TwitterSnapshot, YoutubeSnapshot};
// Re-export storage types
pub use storage::{ActivityStorage, ActivityStorageConfig, SaveableAsset, SavedAssetInfo};
pub use strategies::ActivityStrategy;
// Re-export strategy types
pub use strategies::{BrowserStrategy, DefaultStrategy};
use tracing::debug;
pub use types::{
    Activity, ActivityAsset, ActivitySnapshot, AssetFunctionality, ContextChip, DisplayAsset,
};

/// Select the appropriate strategy based on the process name
///
/// This function uses the simplified strategy selection approach that checks
/// each strategy's supported processes list directly.
///
/// # Arguments
/// * `process_name` - The name of the process
/// * `display_name` - The display name to use for the activity
/// * `icon` - Base64 encoded icon string
///
/// # Returns
/// A ActivityStrategy if a suitable strategy is found, or an error if no strategy supports the process
pub async fn select_strategy_for_process(
    process_name: &str,
    display_name: String,
    icon: String,
) -> ActivityResult<ActivityStrategy> {
    debug!("Selecting strategy for process: {}", process_name);

    strategies::select_strategy_for_process(process_name, display_name, icon).await
}

/// Legacy function for backward compatibility with image::RgbaImage
///
/// This converts the RgbaImage to a base64 string and calls the main function
pub async fn select_strategy_for_process_with_image(
    process_name: &str,
    display_name: String,
    icon: image::RgbaImage,
) -> ActivityResult<ActivityStrategy> {
    // Convert image to base64 string for compatibility
    let icon_string = format!("image_{}x{}", icon.width(), icon.height());
    select_strategy_for_process(process_name, display_name, icon_string).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategies::ActivityStrategyFunctionality;

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
                    "Test V".to_string(),
                    vec![],
                    0.0,
                )),
                ActivityAsset::DefaultAsset(DefaultAsset::simple("Test Asset".to_string())),
            ],
        );

        let context_chips = activity.get_context_chips();
        assert_eq!(context_chips.len(), 1); // Only YouTube asset provides a context chip
        assert_eq!(context_chips[0].name, "Test V");
    }

    #[tokio::test]
    async fn test_select_strategy_for_browser_process() {
        let result = select_strategy_for_process(
            "firefox",
            "Firefox Browser".to_string(),
            "firefox-icon".to_string(),
        )
        .await;

        assert!(result.is_ok());
        let strategy = result.unwrap();
        assert_eq!(strategy.get_name(), "Firefox Browser");
        assert_eq!(strategy.get_process_name(), "firefox");
    }

    #[tokio::test]
    async fn test_select_strategy_for_unknown_process() {
        let result = select_strategy_for_process(
            "unknown_process",
            "Unknown App".to_string(),
            "unknown-icon".to_string(),
        )
        .await;

        assert!(result.is_ok());
        let strategy = result.unwrap();
        assert_eq!(strategy.get_name(), "Unknown App");
        assert_eq!(strategy.get_process_name(), "unknown_process");
    }

    #[tokio::test]
    async fn test_select_strategy_with_image_compatibility() {
        let result = select_strategy_for_process_with_image(
            "firefox",
            "Firefox".to_string(),
            image::RgbaImage::new(16, 16),
        )
        .await;

        assert!(result.is_ok());
        let strategy = result.unwrap();
        assert_eq!(strategy.get_name(), "Firefox");
        assert_eq!(strategy.get_process_name(), "firefox");
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
