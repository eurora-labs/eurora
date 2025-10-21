// Re-export main types for easy access
pub use collector::{CollectorService, CollectorStats, FocusChangeEvent};
pub use config::{CollectorConfig, FocusTrackingConfig, StorageConfig, TimelineConfig};
pub use error::{TimelineError, TimelineResult};
// Re-export activity types for convenience
pub use eur_activity::{
    Activity, ActivityAsset, ActivityError, ActivitySnapshot, ActivityStorage,
    ActivityStorageConfig, ActivityStrategy, AssetFunctionality, ContextChip, DisplayAsset,
    SaveableAsset, select_strategy_for_process,
};
pub use ferrous_llm_core::Message;
pub use manager::{TimelineManager, TimelineManagerBuilder, create_timeline};
pub use storage::{StorageStats, TimelineStorage};

// Internal modules
mod collector;
mod config;
mod db;
mod error;
mod manager;
mod storage;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_api() {
        let timeline = TimelineManager::new();
        assert!(!timeline.is_running());
        assert!(timeline.is_empty().await);
    }

    #[tokio::test]
    async fn test_config_builder() {
        let config = TimelineConfig::builder()
            .max_activities(100)
            .collection_interval(std::time::Duration::from_secs(5))
            .disable_focus_tracking()
            .build();

        assert!(config.validate().is_ok());

        let timeline =
            TimelineManager::with_config(config).expect("Failed to create timeline manager");
        assert_eq!(timeline.get_config().storage.max_activities, 100);
    }

    #[tokio::test]
    async fn test_convenience_functions() {
        let timeline1 = TimelineManager::new();
        assert!(!timeline1.is_running());

        let timeline2 = create_timeline(500, 5).expect("Failed to create timeline manager");
        assert_eq!(timeline2.get_config().storage.max_activities, 500);
    }
}
