pub mod assets;
pub mod config;
pub mod error;
pub mod snapshots;
pub mod storage;
pub mod strategies;
pub mod types;
mod utils;

pub use strategies::processes;

pub use assets::twitter::{TwitterContextType, TwitterTweet};
pub use assets::youtube::TranscriptLine;
pub use assets::{ArticleAsset, DefaultAsset, TwitterAsset, YoutubeAsset};
pub use config::{
    ActivityConfig, ActivityConfigBuilder, ApplicationConfig, GlobalConfig, PrivacyConfig,
    SnapshotFrequency, StrategyConfig,
};
pub use error::{ActivityError, ActivityResult};
pub use snapshots::{ArticleSnapshot, DefaultSnapshot, TwitterSnapshot, YoutubeSnapshot};
pub use storage::{ActivityStorage, SaveableAsset, SavedAssetInfo};
pub use strategies::ActivityStrategy;
pub use strategies::{ActivityReport, BrowserStrategy, DefaultStrategy, NoStrategy};
pub use types::{
    Activity, ActivityAsset, ActivitySnapshot, AssetFunctionality, ContextChip, DisplayAsset,
};

#[cfg(test)]
mod tests {
    use super::*;

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
