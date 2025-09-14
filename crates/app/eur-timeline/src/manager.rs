//! High-level timeline manager implementation

use crate::{
    Activity, ActivityStorage, ActivityStorageConfig, ActivityStrategy, AssetFunctionality,
    ContextChip, DisplayAsset, TimelineError,
};
use eur_activity::types::SnapshotFunctionality;
use eur_activity::{ActivityAsset, SavedAssetInfo};
use ferrous_llm_core::Message;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

use crate::collector::{CollectorService, CollectorStats};
use crate::config::TimelineConfig;
use crate::error::TimelineResult;
use crate::storage::{StorageStats, TimelineStorage};

/// Builder for creating TimelineManager instances
pub struct TimelineManagerBuilder {
    timeline_config: Option<TimelineConfig>,
    activity_storage_config: Option<ActivityStorageConfig>,
}

impl TimelineManagerBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self {
            timeline_config: None,
            activity_storage_config: None,
        }
    }

    /// Set the timeline configuration
    pub fn with_timeline_config(mut self, config: TimelineConfig) -> Self {
        self.timeline_config = Some(config);
        self
    }

    /// Set the activity storage configuration
    pub fn with_activity_storage_config(mut self, config: ActivityStorageConfig) -> Self {
        self.activity_storage_config = Some(config);
        self
    }

    /// Set the maximum number of activities to store
    pub fn with_max_activities(mut self, max_activities: usize) -> Self {
        let mut config = self.timeline_config.unwrap_or_default();
        config.storage.max_activities = max_activities;
        self.timeline_config = Some(config);
        self
    }

    /// Set the collection interval
    pub fn with_collection_interval(mut self, interval: std::time::Duration) -> Self {
        let mut config = self.timeline_config.unwrap_or_default();
        config.collector.collection_interval = interval;
        self.timeline_config = Some(config);
        self
    }

    /// Disable focus tracking
    pub fn disable_focus_tracking(mut self) -> Self {
        let mut config = self.timeline_config.unwrap_or_default();
        config.focus_tracking.enabled = false;
        self.timeline_config = Some(config);
        self
    }

    /// Enable focus tracking
    pub fn enable_focus_tracking(mut self) -> Self {
        let mut config = self.timeline_config.unwrap_or_default();
        config.focus_tracking.enabled = true;
        self.timeline_config = Some(config);
        self
    }

    /// Build the TimelineManager
    pub fn build(self) -> TimelineResult<TimelineManager> {
        let timeline_config = self.timeline_config.unwrap_or_default();
        let activity_storage_config = self.activity_storage_config.unwrap_or_default();

        // Validate configuration
        timeline_config.validate()?;

        info!(
            "Creating timeline manager with config: {:?}",
            timeline_config
        );

        let storage = Arc::new(Mutex::new(TimelineStorage::new(
            timeline_config.storage.clone(),
        )));

        let collector = CollectorService::new_with_timeline_config(
            Arc::clone(&storage),
            timeline_config.clone(),
        );

        let activity_storage = Arc::new(Mutex::new(ActivityStorage::new(activity_storage_config)));

        Ok(TimelineManager {
            storage,
            collector,
            config: timeline_config,
            activity_storage,
        })
    }
}

impl Default for TimelineManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// High-level timeline manager that provides a simple API for timeline operations
pub struct TimelineManager {
    /// Shared storage for timeline data
    pub storage: Arc<Mutex<TimelineStorage>>,
    /// Shared disk storage for saving activities
    pub activity_storage: Arc<Mutex<ActivityStorage>>,
    /// Collection service
    pub collector: CollectorService,
    /// Configuration
    pub config: TimelineConfig,
}

impl TimelineManager {
    /// Create a new builder for TimelineManager
    pub fn builder() -> TimelineManagerBuilder {
        TimelineManagerBuilder::new()
    }

    /// Create a new timeline manager with default configuration
    pub fn new() -> Self {
        TimelineManagerBuilder::new()
            .build()
            .expect("Failed to build timeline manager")
    }

    /// Create a new timeline manager with custom configuration
    pub fn with_config(timeline_config: TimelineConfig) -> TimelineResult<Self> {
        TimelineManagerBuilder::new()
            .with_timeline_config(timeline_config)
            .build()
    }

    /// Start the timeline manager (begins activity collection)
    pub async fn start(&mut self) -> TimelineResult<()> {
        info!("Starting timeline manager");
        self.collector.start().await
    }

    /// Stop the timeline manager (stops activity collection)
    pub async fn stop(&mut self) -> TimelineResult<()> {
        info!("Stopping timeline manager");
        self.collector.stop().await
    }

    /// Restart the timeline manager
    pub async fn restart(&mut self) -> TimelineResult<()> {
        info!("Restarting timeline manager");
        self.collector.restart().await
    }

    /// Check if the timeline manager is running
    pub fn is_running(&self) -> bool {
        self.collector.is_running()
    }

    /// Get the current (most recent) activity
    pub async fn get_current_activity(&self) -> Option<Activity> {
        let storage = self.storage.lock().await;
        storage.get_current_activity().cloned()
    }

    // /// Get recent activities (most recent first)
    // pub async fn get_recent_activities(&self, count: usize) -> Vec<Activity> {
    //     let storage = self.storage.lock().await;
    //     storage
    //         .get_recent_activities(count)
    //         .iter()
    //         .cloned()
    //         .collect()
    // }

    // /// Get activities since a specific time
    // pub async fn get_activities_since(
    //     &self,
    //     since: chrono::DateTime<chrono::Utc>,
    // ) -> Vec<Activity> {
    //     let storage = self.storage.lock().await;
    //     storage
    //         .get_activities_since(since)
    //         .iter()
    //         .map(|activity| activity.clone())
    //         .collect()
    // }

    // /// Get all activities
    // pub async fn get_all_activities(&self) -> Vec<Activity> {
    //     let storage = self.storage.lock().await;
    //     storage.get_all_activities().iter().cloned().collect()
    // }

    /// Get context chips from the current activity
    pub async fn get_context_chips(&self) -> Vec<ContextChip> {
        let storage = self.storage.lock().await;
        if let Some(activity) = storage.get_current_activity() {
            activity.get_context_chips()
        } else {
            Vec::new()
        }
    }

    /// Get display assets from the current activity
    pub async fn get_display_assets(&self) -> Vec<DisplayAsset> {
        let storage = self.storage.lock().await;
        if let Some(activity) = storage.get_current_activity() {
            activity.get_display_assets()
        } else {
            Vec::new()
        }
    }

    /// Load assets from disk
    pub async fn load_assets_from_disk(
        &self,
        paths: &Vec<eur_personal_db::Asset>,
    ) -> TimelineResult<Vec<ActivityAsset>> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }
        let mut out = Vec::with_capacity(paths.len());
        for a in paths {
            let rel = PathBuf::from(&a.relative_path);
            let abs = {
                let storage = self.activity_storage.lock().await;
                storage.get_absolute_path(&rel)
            };
            // NOTE: consider not holding the mutex across .await inside load (see next comment).
            let asset = {
                let storage = self.activity_storage.lock().await;
                storage.load_asset_from_path(&abs).await?
            };
            out.push(asset);
        }
        Ok(out)
        // let activity_storage = self.activity_storage.lock().await;
        // let test_path = paths[0].clone().relative_path;
        // let path_instance = PathBuf::from(&test_path);
        // let path = activity_storage.get_absolute_path(&path_instance);

        // let asset = activity_storage.load_asset_from_path(&path).await?;

        // Ok(vec![asset])
    }

    /// Save the assets to disk by ids
    pub async fn save_assets_to_disk_by_ids(
        &self,
        ids: &Vec<String>,
    ) -> TimelineResult<Vec<SavedAssetInfo>> {
        let activity = {
            let storage = self.storage.lock().await;
            storage.get_current_activity().cloned()
        };

        match activity {
            Some(activity) => {
                let activity_storage = self.activity_storage.lock().await;
                return Ok(activity_storage
                    .save_assets_to_disk_by_ids(&activity, ids)
                    .await?);
            }
            None => Err(TimelineError::Storage(
                "No current activity found".to_string(),
            )),
        }
    }

    /// Save the assets to disk
    pub async fn save_assets_to_disk(&self) -> TimelineResult<Vec<SavedAssetInfo>> {
        let activity = {
            let storage = self.storage.lock().await;
            storage.get_current_activity().cloned()
        };

        match activity {
            Some(activity) => {
                let activity_storage = self.activity_storage.lock().await;
                return Ok(activity_storage.save_assets_to_disk(&activity).await?);
            }
            None => Err(TimelineError::Storage(
                "No current activity found".to_string(),
            )),
        }
    }

    /// Construct messages from current activity by ids
    pub async fn construct_asset_messages_by_ids(&self, ids: &Vec<String>) -> Vec<Message> {
        let storage = self.storage.lock().await;
        if let Some(activity) = storage.get_current_activity() {
            activity
                .assets
                .iter()
                .filter(|asset| ids.contains(&asset.get_id().to_string()))
                .map(|asset| asset.construct_message())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Construct messages from current activity assets
    pub async fn construct_asset_messages(&self) -> Vec<Message> {
        let storage = self.storage.lock().await;
        if let Some(activity) = storage.get_current_activity() {
            activity
                .assets
                .iter()
                .map(|asset| asset.construct_message())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Construct messages from current activity snapshots
    pub async fn construct_snapshot_messages(&self) -> Vec<Message> {
        let storage = self.storage.lock().await;
        if let Some(activity) = storage.get_current_activity() {
            if let Some(snapshot) = activity.snapshots.last() {
                vec![snapshot.construct_message()]
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }

    /// Manually collect an activity using the provided strategy
    pub async fn collect_activity(&self, strategy: ActivityStrategy) -> TimelineResult<()> {
        self.collector.collect_once(strategy).await
    }

    /// Add an activity directly to the timeline
    pub async fn add_activity(&self, activity: Activity) {
        let mut storage = self.storage.lock().await;
        storage.add_activity(activity);
    }

    /// Clear all activities from the timeline
    pub async fn clear_activities(&self) {
        let mut storage = self.storage.lock().await;
        storage.clear();
    }

    /// Force cleanup of old activities
    pub async fn cleanup_old_activities(&self) {
        let mut storage = self.storage.lock().await;
        storage.force_cleanup();
    }

    /// Get the number of activities stored
    pub async fn activity_count(&self) -> usize {
        let storage = self.storage.lock().await;
        storage.len()
    }

    /// Check if the timeline is empty
    pub async fn is_empty(&self) -> bool {
        let storage = self.storage.lock().await;
        storage.is_empty()
    }

    /// Get storage statistics
    pub async fn get_storage_stats(&self) -> StorageStats {
        let storage = self.storage.lock().await;
        storage.get_stats()
    }

    /// Get collector statistics
    pub fn get_collector_stats(&self) -> CollectorStats {
        self.collector.get_stats()
    }

    /// Update storage configuration
    pub async fn configure_storage(
        &mut self,
        config: crate::config::StorageConfig,
    ) -> TimelineResult<()> {
        let mut storage = self.storage.lock().await;
        storage.update_config(config.clone())?;
        self.config.storage = config;
        Ok(())
    }

    /// Update collector configuration
    pub fn configure_collector(&mut self, config: crate::config::CollectorConfig) {
        self.collector.update_config(config.clone());
        self.config.collector = config;
    }

    /// Update the entire configuration
    pub async fn update_config(&mut self, config: TimelineConfig) -> TimelineResult<()> {
        config.validate()?;

        // Update storage config
        {
            let mut storage = self.storage.lock().await;
            storage.update_config(config.storage.clone())?;
        }

        // Update collector config
        self.collector.update_config(config.collector.clone());

        self.config = config;
        Ok(())
    }

    /// Get the current configuration
    pub fn get_config(&self) -> &TimelineConfig {
        &self.config
    }
}

impl Default for TimelineManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a timeline manager with custom capacity and interval (convenience function)
pub fn create_timeline(capacity: usize, interval_seconds: u64) -> TimelineResult<TimelineManager> {
    TimelineManager::builder()
        .with_max_activities(capacity)
        .with_collection_interval(std::time::Duration::from_secs(interval_seconds))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TimelineConfig;
    use std::time::Duration;

    fn create_test_activity(name: &str) -> Activity {
        crate::Activity::new(
            name.to_string(),
            "test_icon".to_string(),
            "test_process".to_string(),
            vec![],
        )
    }

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = TimelineManager::new();
        assert!(!manager.is_running());
        assert!(manager.is_empty().await);
    }

    #[tokio::test]
    async fn test_manager_with_config() {
        let config = TimelineConfig::builder()
            .max_activities(100)
            .collection_interval(Duration::from_secs(5))
            .disable_focus_tracking()
            .build();

        let manager =
            TimelineManager::with_config(config).expect("Failed to create timeline manager");
        assert!(!manager.is_running());
        assert_eq!(manager.get_config().storage.max_activities, 100);
    }

    #[tokio::test]
    async fn test_builder_pattern() {
        let manager = TimelineManager::builder()
            .with_max_activities(200)
            .with_collection_interval(Duration::from_secs(10))
            .disable_focus_tracking()
            .build()
            .expect("Failed to build timeline manager");

        assert!(!manager.is_running());
        assert_eq!(manager.get_config().storage.max_activities, 200);
        assert_eq!(
            manager.get_config().collector.collection_interval,
            Duration::from_secs(10)
        );
        assert!(!manager.get_config().focus_tracking.enabled);
    }

    #[tokio::test]
    async fn test_builder_with_timeline_config() {
        let timeline_config = TimelineConfig::builder()
            .max_activities(150)
            .collection_interval(Duration::from_secs(3))
            .build();

        let activity_storage_config = ActivityStorageConfig::default();

        let manager = TimelineManager::builder()
            .with_timeline_config(timeline_config)
            .with_activity_storage_config(activity_storage_config)
            .build()
            .expect("Failed to build timeline manager");

        assert_eq!(manager.get_config().storage.max_activities, 150);
        assert_eq!(
            manager.get_config().collector.collection_interval,
            Duration::from_secs(3)
        );
    }

    #[tokio::test]
    async fn test_builder_enable_focus_tracking() {
        let manager = TimelineManager::builder()
            .enable_focus_tracking()
            .build()
            .expect("Failed to build timeline manager");

        assert!(manager.get_config().focus_tracking.enabled);
    }

    #[tokio::test]
    async fn test_builder_default() {
        let manager1 = TimelineManager::builder()
            .build()
            .expect("Failed to build timeline manager");
        let manager2 = TimelineManager::new();

        assert_eq!(
            manager1.get_config().storage.max_activities,
            manager2.get_config().storage.max_activities
        );
        assert_eq!(
            manager1.get_config().collector.collection_interval,
            manager2.get_config().collector.collection_interval
        );
    }

    #[tokio::test]
    async fn test_add_activity() {
        let manager = TimelineManager::new();
        let activity = create_test_activity("Test Activity");

        manager.add_activity(activity).await;

        assert_eq!(manager.activity_count().await, 1);
        assert!(!manager.is_empty().await);

        let current = manager.get_current_activity().await.unwrap();
        assert_eq!(current.name, "Test Activity");
    }

    // #[tokio::test]
    // async fn test_get_recent_activities() {
    //     let manager = TimelineManager::new();

    //     for i in 1..=5 {
    //         let activity = create_test_activity(&format!("Activity {}", i));
    //         manager.add_activity(activity).await;
    //     }

    //     let recent = manager.get_recent_activities(3).await;
    //     assert_eq!(recent.len(), 3);
    //     assert_eq!(recent[0].name, "Activity 5"); // Most recent first
    //     assert_eq!(recent[1].name, "Activity 4");
    //     assert_eq!(recent[2].name, "Activity 3");
    // }

    #[tokio::test]
    async fn test_clear_activities() {
        let manager = TimelineManager::new();

        manager
            .add_activity(create_test_activity("Activity 1"))
            .await;
        manager
            .add_activity(create_test_activity("Activity 2"))
            .await;

        assert_eq!(manager.activity_count().await, 2);

        manager.clear_activities().await;

        assert_eq!(manager.activity_count().await, 0);
        assert!(manager.is_empty().await);
    }

    #[tokio::test]
    async fn test_manager_lifecycle() {
        let config = TimelineConfig::builder()
            .disable_focus_tracking()
            .collection_interval(Duration::from_millis(100))
            .build();

        let mut manager =
            TimelineManager::with_config(config).expect("Failed to create timeline manager");

        // Start manager
        assert!(manager.start().await.is_ok());
        assert!(manager.is_running());

        // Stop manager
        assert!(manager.stop().await.is_ok());
        assert!(!manager.is_running());
    }

    #[tokio::test]
    async fn test_convenience_functions() {
        let manager1 = TimelineManager::new();
        assert!(!manager1.is_running());

        let manager2 = create_timeline(500, 5).expect("Failed to create timeline");
        assert_eq!(manager2.get_config().storage.max_activities, 500);
        assert_eq!(
            manager2.get_config().collector.collection_interval,
            Duration::from_secs(5)
        );
    }

    #[tokio::test]
    async fn test_get_stats() {
        let manager = TimelineManager::new();
        manager.add_activity(create_test_activity("Test")).await;

        let storage_stats = manager.get_storage_stats().await;
        assert_eq!(storage_stats.total_activities, 1);

        let collector_stats = manager.get_collector_stats();
        assert!(!collector_stats.is_running);
    }
}
