//! High-level timeline manager implementation

use std::sync::Arc;

use agent_chain_core::BaseMessage;
use euro_activity::{SavedAssetInfo, types::SnapshotFunctionality};
use tokio::sync::Mutex;
use tracing::debug;

use crate::{
    ActivityStorage, ActivityStorageConfig, AssetFunctionality, ContextChip, TimelineError,
    collector::{ActivityEvent, CollectorService},
    config::TimelineConfig,
    error::TimelineResult,
    storage::TimelineStorage,
};

/// Builder for creating TimelineManager instances
pub struct TimelineManagerBuilder {
    activity_storage_config: Option<ActivityStorageConfig>,
}

impl TimelineManagerBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self {
            activity_storage_config: None,
        }
    }

    /// Set the activity storage configuration
    pub fn with_activity_storage_config(mut self, config: ActivityStorageConfig) -> Self {
        self.activity_storage_config = Some(config);
        self
    }

    /// Build the TimelineManager
    pub async fn build(self) -> TimelineResult<TimelineManager> {
        let timeline_config = TimelineConfig::default();
        let activity_storage_config = self.activity_storage_config.unwrap_or_default();

        // Validate configuration
        timeline_config.validate()?;

        debug!(
            "Creating timeline manager with config: {:?}",
            timeline_config
        );

        let storage = Arc::new(Mutex::new(TimelineStorage::new(
            timeline_config.storage.clone(),
        )));

        let collector =
            CollectorService::new_with_timeline_config(Arc::clone(&storage), timeline_config);

        let activity_storage = Arc::new(Mutex::new(
            ActivityStorage::new(activity_storage_config).await,
        ));

        Ok(TimelineManager {
            storage,
            collector,
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
}

impl TimelineManager {
    /// Create a new builder for TimelineManager
    pub fn builder() -> TimelineManagerBuilder {
        TimelineManagerBuilder::new()
    }

    /// Start the timeline manager (begins activity collection)
    pub async fn start(&mut self) -> TimelineResult<()> {
        debug!("Starting timeline manager");
        self.collector.start().await
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

    /// Save the assets via the be-asset-service
    pub async fn save_assets_to_service_by_ids(
        &self,
        ids: &[String],
    ) -> TimelineResult<Vec<SavedAssetInfo>> {
        let activity = {
            let storage = self.storage.lock().await;
            storage.get_current_activity().cloned()
        };

        match activity {
            Some(activity) => {
                let activity_storage = self.activity_storage.lock().await;
                return Ok(activity_storage
                    .save_assets_to_service_by_ids(&activity, ids)
                    .await?);
            }
            None => Err(TimelineError::Storage(
                "No current activity found".to_string(),
            )),
        }
    }

    /// Save current activity to disk
    pub async fn save_current_activity_to_service(&self) -> TimelineResult<()> {
        let activity = {
            let storage = self.storage.lock().await;
            storage.get_current_activity().cloned()
        };

        match activity {
            Some(activity) => {
                let activity_storage = self.activity_storage.lock().await;
                activity_storage.save_activity_to_service(&activity).await?;
                Ok(())
            }
            None => Err(TimelineError::Storage(
                "No current activity found".to_string(),
            )),
        }
    }

    /// Construct messages from current activity by ids
    pub async fn construct_asset_messages_by_ids(&self, ids: &[String]) -> Vec<BaseMessage> {
        let storage = self.storage.lock().await;
        if let Some(activity) = storage.get_current_activity() {
            activity
                .assets
                .iter()
                .filter(|asset| ids.contains(&asset.get_id().to_string()))
                .flat_map(|asset| asset.construct_messages())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Construct messages from current activity snapshots by ids
    pub async fn construct_snapshot_messages_by_ids(&self, _ids: &[String]) -> Vec<BaseMessage> {
        let storage = self.storage.lock().await;
        if let Some(activity) = storage.get_current_activity() {
            if let Some(snapshot) = activity.snapshots.last() {
                snapshot.construct_messages()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }

    /// Subscribe to activity events
    pub fn subscribe_to_activity_events(&self) -> tokio::sync::broadcast::Receiver<ActivityEvent> {
        self.collector.subscribe_to_activity_events()
    }

    /// Subscribe to new assets events
    pub fn subscribe_to_assets_events(&self) -> tokio::sync::broadcast::Receiver<Vec<ContextChip>> {
        self.collector.subscribe_to_assets_events()
    }
}
