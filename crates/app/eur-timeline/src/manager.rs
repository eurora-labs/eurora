//! High-level timeline manager implementation

use eur_activity::{Activity, ActivityStrategy, ContextChip, DisplayAsset};
use ferrous_llm_core::Message;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::collector::{CollectorService, CollectorStats};
use crate::config::TimelineConfig;
use crate::error::Result;
use crate::storage::{StorageStats, TimelineStorage};

/// High-level timeline manager that provides a simple API for timeline operations
pub struct TimelineManager {
    /// Shared storage for timeline data
    storage: Arc<Mutex<TimelineStorage>>,
    /// Collection service
    collector: CollectorService,
    /// Configuration
    config: TimelineConfig,
}

impl TimelineManager {
    /// Create a new timeline manager with default configuration
    pub fn new() -> Self {
        let config = TimelineConfig::default();
        Self::with_config(config)
    }

    /// Create a new timeline manager with custom configuration
    pub fn with_config(config: TimelineConfig) -> Self {
        info!("Creating timeline manager with config: {:?}", config);

        // Validate configuration
        if let Err(e) = config.validate() {
            warn!("Invalid configuration provided: {}", e);
        }

        let storage = Arc::new(Mutex::new(TimelineStorage::new(config.storage.clone())));
        let collector =
            CollectorService::new_with_timeline_config(Arc::clone(&storage), config.clone());

        Self {
            storage,
            collector,
            config,
        }
    }

    /// Start the timeline manager (begins activity collection)
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting timeline manager");
        self.collector.start().await
    }

    /// Stop the timeline manager (stops activity collection)
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping timeline manager");
        self.collector.stop().await
    }

    /// Restart the timeline manager
    pub async fn restart(&mut self) -> Result<()> {
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
        storage.get_current_activity().map(|activity| {
            // Create a new activity with the same data since Activity doesn't implement Clone
            Activity::new(
                activity.name.clone(),
                activity.icon.clone(),
                activity.process_name.clone(),
                vec![], // We can't clone the assets easily, so return empty for now
            )
        })
    }

    /// Get recent activities (most recent first)
    pub async fn get_recent_activities(&self, count: usize) -> Vec<Activity> {
        let storage = self.storage.lock().await;
        storage
            .get_recent_activities(count)
            .iter()
            .map(|activity| {
                Activity::new(
                    activity.name.clone(),
                    activity.icon.clone(),
                    activity.process_name.clone(),
                    vec![], // We can't clone the assets easily, so return empty for now
                )
            })
            .collect()
    }

    /// Get activities since a specific time
    pub async fn get_activities_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Vec<Activity> {
        let storage = self.storage.lock().await;
        storage
            .get_activities_since(since)
            .iter()
            .map(|activity| {
                Activity::new(
                    activity.name.clone(),
                    activity.icon.clone(),
                    activity.process_name.clone(),
                    vec![], // We can't clone the assets easily, so return empty for now
                )
            })
            .collect()
    }

    /// Get all activities
    pub async fn get_all_activities(&self) -> Vec<Activity> {
        let storage = self.storage.lock().await;
        storage
            .get_all_activities()
            .iter()
            .map(|activity| {
                Activity::new(
                    activity.name.clone(),
                    activity.icon.clone(),
                    activity.process_name.clone(),
                    vec![], // We can't clone the assets easily, so return empty for now
                )
            })
            .collect()
    }

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
    pub async fn collect_activity(&self, strategy: Box<dyn ActivityStrategy>) -> Result<()> {
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
    pub async fn configure_storage(&mut self, config: crate::config::StorageConfig) -> Result<()> {
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
    pub async fn update_config(&mut self, config: TimelineConfig) -> Result<()> {
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

/// Create a timeline manager with default settings (convenience function)
pub fn create_default_timeline() -> TimelineManager {
    TimelineManager::new()
}

/// Create a timeline manager with custom capacity and interval (convenience function)
pub fn create_timeline(capacity: usize, interval_seconds: u64) -> TimelineManager {
    let config = TimelineConfig::builder()
        .max_activities(capacity)
        .collection_interval(std::time::Duration::from_secs(interval_seconds))
        .build();

    TimelineManager::with_config(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TimelineConfig;
    use std::time::Duration;

    fn create_test_activity(name: &str) -> Activity {
        eur_activity::Activity::new(
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

        let manager = TimelineManager::with_config(config);
        assert!(!manager.is_running());
        assert_eq!(manager.get_config().storage.max_activities, 100);
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

    #[tokio::test]
    async fn test_get_recent_activities() {
        let manager = TimelineManager::new();

        for i in 1..=5 {
            let activity = create_test_activity(&format!("Activity {}", i));
            manager.add_activity(activity).await;
        }

        let recent = manager.get_recent_activities(3).await;
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].name, "Activity 5"); // Most recent first
        assert_eq!(recent[1].name, "Activity 4");
        assert_eq!(recent[2].name, "Activity 3");
    }

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

        let mut manager = TimelineManager::with_config(config);

        // Start manager
        assert!(manager.start().await.is_ok());
        assert!(manager.is_running());

        // Stop manager
        assert!(manager.stop().await.is_ok());
        assert!(!manager.is_running());
    }

    #[tokio::test]
    async fn test_convenience_functions() {
        let manager1 = create_default_timeline();
        assert!(!manager1.is_running());

        let manager2 = create_timeline(500, 5);
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
