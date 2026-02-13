use agent_chain_core::BaseMessage;
use bon::bon;
use euro_activity::{SavedAssetInfo, types::SnapshotFunctionality};
use std::sync::Arc;
use tokio::sync::{Mutex, watch};
use tonic::transport::Channel;
use tracing::{debug, info};

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
    pub fn new(channel_rx: watch::Receiver<Channel>) -> TimelineResult<Self> {
        let timeline_config = TimelineConfig::default();
        timeline_config.validate()?;
        let storage = Arc::new(Mutex::new(TimelineStorage::new(
            timeline_config.storage.clone(),
        )));

        let collector =
            CollectorService::new_with_timeline_config(Arc::clone(&storage), timeline_config);
        let activity_storage = Arc::new(Mutex::new(ActivityStorage::new(channel_rx)));

        Ok(TimelineManager {
            storage,
            activity_storage,
            collector,
        })
    }

    pub async fn start(&mut self) -> TimelineResult<()> {
        debug!("Starting timeline manager");

        info!("Starting browser bridge gRPC server");
        euro_browser::start_browser_bridge_server().await;

        self.collector.start().await
    }

    pub async fn stop(&mut self) -> TimelineResult<()> {
        debug!("Stopping timeline manager");

        info!("Stopping browser bridge gRPC server");
        euro_browser::stop_browser_bridge_server().await;

        info!("Timeline manager stopped");

        Ok(())
    }

    pub async fn get_context_chips(&self) -> Vec<ContextChip> {
        let storage = self.storage.lock().await;
        if let Some(activity) = storage.get_current_activity() {
            activity.get_context_chips()
        } else {
            Vec::new()
        }
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

    pub async fn construct_messages_from_last_snapshot(&self) -> Vec<BaseMessage> {
        let storage = self.storage.lock().await;
        if let Some(activity) = storage.get_current_activity() {
            if let Some(snapshot) = activity.snapshots.last() {
                snapshot.construct_messages()
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }

    pub async fn construct_messages_from_last_asset(&self) -> Vec<BaseMessage> {
        let storage = self.storage.lock().await;
        if let Some(activity) = storage.get_current_activity() {
            if let Some(asset) = activity.assets.last() {
                asset.construct_messages()
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }

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

    pub fn subscribe_to_activity_events(&self) -> tokio::sync::broadcast::Receiver<ActivityEvent> {
        self.collector.subscribe_to_activity_events()
    }

    pub fn subscribe_to_assets_events(&self) -> tokio::sync::broadcast::Receiver<Vec<ContextChip>> {
        self.collector.subscribe_to_assets_events()
    }
}
