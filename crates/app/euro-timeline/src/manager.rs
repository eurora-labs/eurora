use agent_chain_core::messages::ContentBlocks;
use bon::bon;
use euro_activity::{SavedAssetInfo, types::SnapshotFunctionality};
use euro_auth::AuthManager;
use euro_endpoint::EndpointManager;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    ActivityStorage, AssetFunctionality, ContextChip, TimelineError, collector::CollectorService,
    config::TimelineConfig, error::TimelineResult, storage::TimelineStorage, types::ActivityEvent,
};

pub struct TimelineManager {
    pub storage: Arc<Mutex<TimelineStorage>>,
    pub activity_storage: Arc<ActivityStorage>,
    pub collector: CollectorService,
}

#[bon]
impl TimelineManager {
    #[builder]
    pub fn new(
        endpoint_manager: Arc<EndpointManager>,
        auth_manager: AuthManager,
    ) -> TimelineResult<Self> {
        let timeline_config = TimelineConfig::default();
        timeline_config.validate()?;
        let storage = Arc::new(Mutex::new(TimelineStorage::new(
            timeline_config.storage.clone(),
        )));

        let activity_storage = Arc::new(ActivityStorage::new(endpoint_manager, auth_manager));

        let collector = CollectorService::new_with_timeline_config(
            Arc::clone(&storage),
            Arc::clone(&activity_storage),
            timeline_config,
        );

        Ok(TimelineManager {
            storage,
            activity_storage,
            collector,
        })
    }

    pub async fn start(&mut self) -> TimelineResult<()> {
        tracing::debug!("Starting timeline manager");
        self.collector.start().await
    }

    pub async fn stop(&mut self) -> TimelineResult<()> {
        tracing::debug!("Stopping timeline manager");
        // Override the last heartbeat value with the precise local
        // `end` timestamp before the process exits. Best-effort and
        // bounded internally by a short timeout — a slow network must
        // not stall shutdown.
        self.collector.flush_current_end().await;
        tracing::info!("Timeline manager stopped");
        Ok(())
    }

    pub async fn get_context_chip(&self) -> Option<ContextChip> {
        let storage = self.storage.lock().await;
        storage
            .get_current_activity()
            .map(|activity| activity.get_context_chip())
    }

    pub async fn save_assets_to_service_by_ids(
        &self,
        ids: &[String],
    ) -> TimelineResult<Vec<SavedAssetInfo>> {
        let activity = {
            let storage = self.storage.lock().await;
            storage.get_current_activity().cloned()
        };

        match activity {
            Some(activity) => Ok(self
                .activity_storage
                .save_assets_to_service_by_ids(&activity, ids)
                .await?),
            None => Err(TimelineError::Storage(
                "No current activity found".to_string(),
            )),
        }
    }

    pub async fn refresh_current_activity(&self) -> TimelineResult<()> {
        self.collector.refresh_current_activity().await
    }

    pub async fn construct_messages_from_last_snapshot(&self) -> ContentBlocks {
        let storage = self.storage.lock().await;
        if let Some(activity) = storage.get_current_activity() {
            if let Some(snapshot) = activity.snapshots.last() {
                snapshot.construct_messages()
            } else {
                ContentBlocks::new()
            }
        } else {
            ContentBlocks::new()
        }
    }

    pub async fn construct_messages_from_last_asset(&self) -> ContentBlocks {
        let storage = self.storage.lock().await;
        if let Some(activity) = storage.get_current_activity() {
            if let Some(asset) = activity.assets.last() {
                asset.construct_messages()
            } else {
                ContentBlocks::new()
            }
        } else {
            ContentBlocks::new()
        }
    }

    pub async fn construct_asset_messages_by_ids(&self, ids: &[String]) -> ContentBlocks {
        let storage = self.storage.lock().await;
        if let Some(activity) = storage.get_current_activity() {
            activity
                .assets
                .iter()
                .filter(|asset| ids.contains(&asset.get_id().to_string()))
                .flat_map(|asset| asset.construct_messages().into_inner())
                .collect::<Vec<_>>()
                .into()
        } else {
            ContentBlocks::new()
        }
    }

    pub async fn construct_snapshot_messages_by_ids(&self, _ids: &[String]) -> ContentBlocks {
        let storage = self.storage.lock().await;
        if let Some(activity) = storage.get_current_activity() {
            if let Some(snapshot) = activity.snapshots.last() {
                snapshot.construct_messages()
            } else {
                ContentBlocks::new()
            }
        } else {
            ContentBlocks::new()
        }
    }

    pub fn subscribe_to_activity_events(&self) -> tokio::sync::broadcast::Receiver<ActivityEvent> {
        self.collector.subscribe_to_activity_events()
    }

    pub fn subscribe_to_assets_events(&self) -> tokio::sync::broadcast::Receiver<Vec<ContextChip>> {
        self.collector.subscribe_to_assets_events()
    }
}
