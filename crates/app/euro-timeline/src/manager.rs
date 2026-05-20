use bon::bon;
use euro_auth::AuthManager;
use euro_endpoint::EndpointManager;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    ActivityStorage, ContextChip,
    collector::CollectorService,
    config::TimelineConfig,
    error::TimelineResult,
    storage::TimelineStorage,
    types::{ActivityEvent, SavedActivityEndedEvent, SavedActivityEvent},
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

    pub fn subscribe_to_activity_events(&self) -> tokio::sync::broadcast::Receiver<ActivityEvent> {
        self.collector.subscribe_to_activity_events()
    }

    pub fn subscribe_to_assets_events(&self) -> tokio::sync::broadcast::Receiver<Vec<ContextChip>> {
        self.collector.subscribe_to_assets_events()
    }

    pub fn subscribe_to_saved_activity_events(
        &self,
    ) -> tokio::sync::broadcast::Receiver<SavedActivityEvent> {
        self.collector.subscribe_to_saved_activity_events()
    }

    pub fn subscribe_to_saved_activity_ended_events(
        &self,
    ) -> tokio::sync::broadcast::Receiver<SavedActivityEndedEvent> {
        self.collector.subscribe_to_saved_activity_ended_events()
    }
}
