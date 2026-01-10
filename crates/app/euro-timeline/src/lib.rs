// Re-export main types for easy access
pub use agent_chain_core::BaseMessage;
pub use collector::{ActivityEvent, CollectorService, CollectorStats};
pub use config::{CollectorConfig, FocusTrackingConfig, StorageConfig, TimelineConfig};
pub use error::{TimelineError, TimelineResult};
// Re-export activity types for convenience
pub use euro_activity::{
    Activity, ActivityAsset, ActivityError, ActivitySnapshot, ActivityStorage,
    ActivityStorageConfig, ActivityStrategy, AssetFunctionality, ContextChip, DisplayAsset,
    SaveableAsset,
};
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

    fn init_crypto_provider() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }

    #[tokio::test]
    async fn test_new_api() {
        init_crypto_provider();
        let timeline = TimelineManager::new().await;
        assert!(!timeline.is_running());
        assert!(timeline.is_empty().await);
    }

    #[tokio::test]
    async fn test_config_builder() {
        init_crypto_provider();
        let config = TimelineConfig::builder()
            .max_activities(100)
            .collection_interval(std::time::Duration::from_secs(5))
            .build();

        assert!(config.validate().is_ok());

        let timeline = TimelineManager::with_config(config)
            .await
            .expect("Failed to create timeline manager");
        assert_eq!(timeline.get_config().storage.max_activities, 100);
    }

    #[tokio::test]
    async fn test_convenience_functions() {
        init_crypto_provider();
        let timeline1 = TimelineManager::new().await;
        assert!(!timeline1.is_running());

        let timeline2 = create_timeline(500, 5)
            .await
            .expect("Failed to create timeline manager");
        assert_eq!(timeline2.get_config().storage.max_activities, 500);
    }
}
