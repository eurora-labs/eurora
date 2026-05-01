use agent_chain_core::messages::ContentBlocks;
use bon::bon;
use euro_activity::{SavedAssetInfo, types::SnapshotFunctionality};
use euro_auth::AuthManager;
use std::sync::Arc;
use tokio::sync::{Mutex, watch};
use tonic::transport::Channel;

use crate::{
    ActivityStorage, AssetFunctionality, ContextChip, TimelineError, collector::CollectorService,
    config::TimelineConfig, error::TimelineResult, storage::TimelineStorage, types::ActivityEvent,
};

pub struct TimelineManager {
    pub storage: Arc<Mutex<TimelineStorage>>,
    pub activity_storage: Arc<Mutex<ActivityStorage>>,
    pub collector: CollectorService,
}

#[bon]
impl TimelineManager {
    #[builder]
    pub fn new(
        channel_rx: watch::Receiver<Channel>,
        auth_manager: AuthManager,
    ) -> TimelineResult<Self> {
        let timeline_config = TimelineConfig::default();
        timeline_config.validate()?;
        let storage = Arc::new(Mutex::new(TimelineStorage::new(
            timeline_config.storage.clone(),
        )));

        let collector =
            CollectorService::new_with_timeline_config(Arc::clone(&storage), timeline_config);
        let activity_storage = Arc::new(Mutex::new(ActivityStorage::new(channel_rx, auth_manager)));

        Ok(TimelineManager {
            storage,
            activity_storage,
            collector,
        })
    }

    pub async fn start(&mut self) -> TimelineResult<()> {
        tracing::debug!("Starting timeline manager");

        tracing::info!("Starting app bridge transports");
        euro_bridge::start_app_bridge().await;

        self.collector.start().await
    }

    pub async fn stop(&mut self) -> TimelineResult<()> {
        tracing::debug!("Stopping timeline manager");

        tracing::info!("Stopping app bridge transports");
        euro_bridge::stop_app_bridge().await;

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

    pub async fn refresh_current_activity(&self) -> TimelineResult<()> {
        self.collector.refresh_current_activity().await
    }

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
