//! Browser strategy implementation for the refactored activity system
//!
//! This module uses the singleton gRPC server from `euro_browser` crate to accept
//! connections from multiple native messaging hosts (euro-native-messaging). Each
//! host registers with its browser PID, allowing the server to route requests to
//! the correct browser.
//!
//! The gRPC server is managed by the TimelineManager and runs as long as the manager
//! is alive. The BrowserStrategy only connects to the singleton service but does not
//! manage its lifecycle.

pub use crate::strategies::ActivityStrategyFunctionality;
pub use crate::strategies::processes::*;
use crate::strategies::{ActivityReport, StrategyMetadata};
pub use crate::strategies::{ActivityStrategy, StrategySupport};
use crate::{
    Activity, ActivityError,
    error::ActivityResult,
    types::{ActivityAsset, ActivitySnapshot},
};
use async_trait::async_trait;
use euro_native_messaging::NativeMessage;
use focus_tracker::FocusedWindow;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use url::Url;

pub use euro_browser::{
    BrowserBridgeServer, BrowserBridgeService, EventFrame, Frame, FrameKind, RequestFrame,
    ResponseFrame,
};

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct BrowserStrategy {
    #[serde(skip)]
    sender: Option<mpsc::UnboundedSender<ActivityReport>>,

    #[serde(skip)]
    bridge_service: Option<&'static BrowserBridgeService>,

    #[serde(skip)]
    event_subscription_handle: Option<Arc<tokio::task::JoinHandle<()>>>,

    #[serde(skip)]
    snapshot_collection_handle: Option<Arc<tokio::task::JoinHandle<()>>>,

    #[serde(skip)]
    active_browser: Option<String>,

    #[serde(skip)]
    active_browser_pid: Option<u32>,
}

impl BrowserStrategy {
    async fn initialize_service(&mut self) -> ActivityResult<()> {
        let service = BrowserBridgeService::get_or_init().await;

        self.bridge_service = Some(service);

        Ok(())
    }

    async fn init_collection(&mut self, focus_window: &FocusedWindow) -> ActivityResult<()> {
        let Some(sender) = self.sender.clone() else {
            return Err(ActivityError::Strategy(
                "Sender not initialized".to_string(),
            ));
        };

        let service = self
            .bridge_service
            .as_ref()
            .ok_or_else(|| ActivityError::Strategy("Bridge service not initialized".to_string()))?;

        let mut events_rx = service.subscribe_to_events();
        let _default_icon = focus_window.icon.clone();
        let mut strategy = self.clone();
        let last_url: Arc<tokio::sync::Mutex<Option<Url>>> =
            Arc::new(tokio::sync::Mutex::new(None));

        let handle = tokio::spawn(async move {
            let last_url = Arc::clone(&last_url);

            while let Ok((browser_pid, event_frame)) = events_rx.recv().await {
                debug!(
                    "Received event from browser PID {}: action={}",
                    browser_pid, event_frame.action
                );

                let Some(payload_str) = event_frame.payload else {
                    continue;
                };

                let native_message = match serde_json::from_str::<NativeMessage>(&payload_str) {
                    Ok(msg) => msg,
                    Err(e) => {
                        warn!("Failed to parse native message: {}", e);
                        continue;
                    }
                };

                let metadata = match native_message {
                    NativeMessage::NativeMetadata(data) => StrategyMetadata::from(data),
                    _ => {
                        debug!("Ignoring non-metadata event");
                        continue;
                    }
                };

                let mut prev = last_url.lock().await;
                let url = match Url::parse(&metadata.url.clone().unwrap_or_default()) {
                    Ok(u) => u,
                    Err(_) => continue,
                };

                if let Some(prev_url) = prev.take()
                    && prev_url.domain() == url.domain()
                {
                    *prev = Some(url);
                    continue;
                }
                *prev = Some(url);

                let icon = metadata.icon.clone();
                let url_str = metadata.url.clone().unwrap_or_default();

                let assets = strategy.retrieve_assets().await.map_err(|e| {
                    warn!("Failed to retrieve assets: {}", e);
                    e
                });

                let activity =
                    Activity::new(url_str, icon, "".to_string(), assets.unwrap_or_default());

                info!(
                    "Creating new activity from event: browser_pid={}, name={}",
                    browser_pid, activity.name
                );
                if sender
                    .send(ActivityReport::NewActivity(activity.clone()))
                    .is_err()
                {
                    warn!("Failed to send new activity report - receiver dropped");
                    break;
                }
            }

            debug!("Event subscription task ended");
        });

        self.event_subscription_handle = Some(Arc::new(handle));

        self.collect_assets_and_snapshots();
        Ok(())
    }

    pub async fn new() -> ActivityResult<Self> {
        let mut strategy = BrowserStrategy::default();
        strategy.initialize_service().await?;

        Ok(strategy)
    }
}

#[async_trait]
impl StrategySupport for BrowserStrategy {
    fn get_supported_processes() -> Vec<&'static str> {
        vec![Librewolf.get_name(), Firefox.get_name(), Chrome.get_name()]
    }
}

#[async_trait]
impl ActivityStrategyFunctionality for BrowserStrategy {
    fn can_handle_process(&self, focus_window: &FocusedWindow) -> bool {
        BrowserStrategy::get_supported_processes().contains(&focus_window.process_name.as_str())
    }

    async fn start_tracking(
        &mut self,
        focus_window: &FocusedWindow,
        sender: mpsc::UnboundedSender<ActivityReport>,
    ) -> ActivityResult<()> {
        self.sender = Some(sender.clone());
        let process_name = focus_window.process_name.clone();
        self.active_browser = Some(process_name.clone());
        self.active_browser_pid = Some(focus_window.process_id);

        match self.get_metadata().await {
            Ok(metadata) => {
                let assets = self.retrieve_assets().await.unwrap_or(vec![]);
                let activity = Activity::new(
                    metadata.url.unwrap_or_default(),
                    metadata.icon,
                    process_name.clone(),
                    assets,
                );
                if sender.send(ActivityReport::NewActivity(activity)).is_err() {
                    warn!("Failed to send new activity report - receiver dropped");
                }
            }
            Err(err) => {
                let activity = Activity::new(
                    focus_window.process_name.clone(),
                    focus_window.icon.clone(),
                    focus_window.process_name.clone(),
                    vec![],
                );
                if sender.send(ActivityReport::NewActivity(activity)).is_err() {
                    warn!("Failed to send new activity report - receiver dropped");
                }

                warn!("Failed to get metadata: {}", err);
            }
        }

        self.init_collection(focus_window).await?;

        debug!("Browser strategy starting tracking for: {:?}", process_name);

        Ok(())
    }

    async fn handle_process_change(
        &mut self,
        focus_window: &FocusedWindow,
    ) -> ActivityResult<bool> {
        debug!(
            "Browser strategy handling process change to: {}",
            focus_window.process_name
        );

        if self.can_handle_process(focus_window) {
            debug!(
                "Browser strategy can continue handling: {}",
                focus_window.process_name
            );
            if self.active_browser_pid == Some(focus_window.process_id) {
                info!("Detected the same browser. Ignoring...",);
            } else {
                self.active_browser_pid = Some(focus_window.process_id);
                self.active_browser = Some(focus_window.process_name.to_string());
            }

            let has_registered_client = if let Some(service) = self.bridge_service.as_ref() {
                service.is_registered(focus_window.process_id).await
            } else {
                false
            };

            if !has_registered_client {
                if let Some(sender) = self.sender.clone() {
                    match self.get_metadata().await {
                        Ok(metadata) => {
                            let activity = Activity::new(
                                metadata.url.unwrap_or_default(),
                                metadata.icon,
                                focus_window.process_name.to_string(),
                                vec![],
                            );
                            if sender.send(ActivityReport::NewActivity(activity)).is_err() {
                                warn!("Failed to send new activity report - receiver dropped");
                            }
                        }
                        Err(err) => {
                            let activity = Activity::new(
                                focus_window.process_name.clone(),
                                focus_window.icon.clone(),
                                focus_window.process_name.clone(),
                                vec![],
                            );
                            if sender.send(ActivityReport::NewActivity(activity)).is_err() {
                                warn!("Failed to send new activity report - receiver dropped");
                            }

                            warn!("Failed to get metadata: {}", err);
                        }
                    }
                }
            } else {
                debug!(
                    "Browser PID {} has registered gRPC client, skipping activity report (will be handled by event subscription)",
                    focus_window.process_id
                );
            }

            Ok(true)
        } else {
            debug!(
                "Browser strategy cannot handle: {}, stopping tracking",
                focus_window.process_name
            );
            self.stop_tracking().await?;
            Ok(false)
        }
    }

    async fn stop_tracking(&mut self) -> ActivityResult<()> {
        debug!("Browser strategy stopping tracking");
        self.active_browser = None;
        self.active_browser_pid = None;

        if let Some(handle) = self.event_subscription_handle.take()
            && let Ok(handle) = Arc::try_unwrap(handle)
        {
            handle.abort();
        }

        if let Some(handle) = self.snapshot_collection_handle.take()
            && let Ok(handle) = Arc::try_unwrap(handle)
        {
            handle.abort();
        }

        Ok(())
    }

    async fn retrieve_assets(&mut self) -> ActivityResult<Vec<ActivityAsset>> {
        debug!("Retrieving assets for browser strategy");

        let service = self
            .bridge_service
            .as_ref()
            .ok_or_else(|| ActivityError::invalid_data("Bridge service not available"))?;

        let browser_pid = self
            .active_browser_pid
            .ok_or_else(|| ActivityError::invalid_data("No active browser PID set"))?;

        let response_frame = service.generate_assets(browser_pid).await.map_err(|e| {
            ActivityError::invalid_data(format!("Failed to generate assets: {}", e))
        })?;

        let Some(payload) = response_frame.payload else {
            warn!("No payload in assets response");
            return Ok(vec![]);
        };

        let native_asset = serde_json::from_str::<NativeMessage>(&payload)
            .map_err(|e| -> ActivityError { ActivityError::from(e) })?;

        let asset = ActivityAsset::try_from(native_asset)
            .map_err(|e| -> ActivityError { ActivityError::InvalidAssetType(e.to_string()) })?;

        debug!("Retrieved 1 asset from browser");
        Ok(vec![asset])
    }

    async fn retrieve_snapshots(&mut self) -> ActivityResult<Vec<ActivitySnapshot>> {
        let service = self
            .bridge_service
            .as_ref()
            .ok_or_else(|| ActivityError::invalid_data("Bridge service not available"))?;

        let browser_pid = self
            .active_browser_pid
            .ok_or_else(|| ActivityError::invalid_data("No active browser PID set"))?;

        let response_frame = service.generate_snapshot(browser_pid).await.map_err(|e| {
            ActivityError::invalid_data(format!("Failed to generate snapshot: {}", e))
        })?;

        let Some(payload) = response_frame.payload else {
            warn!("No payload in snapshot response");
            return Ok(vec![]);
        };

        let native_message = serde_json::from_str::<NativeMessage>(&payload)
            .map_err(|e| -> ActivityError { ActivityError::from(e) })?;

        let snapshot = ActivitySnapshot::try_from(native_message)
            .map_err(|e| -> ActivityError { ActivityError::InvalidSnapshotType(e.to_string()) })?;

        Ok(vec![snapshot])
    }

    async fn get_metadata(&mut self) -> ActivityResult<StrategyMetadata> {
        debug!("Retrieving metadata for browser strategy");

        let service = self
            .bridge_service
            .as_ref()
            .ok_or_else(|| ActivityError::invalid_data("Bridge service not available"))?;

        let browser_pid = self
            .active_browser_pid
            .ok_or_else(|| ActivityError::invalid_data("No active browser PID set"))?;

        let response_frame = service
            .get_metadata(browser_pid)
            .await
            .map_err(|e| ActivityError::invalid_data(format!("Failed to get metadata: {}", e)))?;

        let Some(payload) = response_frame.payload else {
            warn!("No payload in metadata response");
            return Ok(StrategyMetadata::default());
        };

        let native_metadata = serde_json::from_str::<NativeMessage>(&payload)
            .map_err(|e| -> ActivityError { ActivityError::from(e) })?;

        let metadata = match native_metadata {
            NativeMessage::NativeMetadata(metadata) => {
                if let Some(ref url) = metadata.url
                    && !url.starts_with("http")
                    && !url.starts_with("chrome-extension:")
                {
                    return Err(ActivityError::invalid_data(format!(
                        "Invalid metadata URL: must start with 'http', got: {}",
                        url
                    )));
                }
                StrategyMetadata::from(metadata)
            }
            _ => StrategyMetadata::default(),
        };
        Ok(metadata)
    }
}

impl BrowserStrategy {
    fn collect_assets_and_snapshots(&mut self) {
        info!("Starting active collection task");
        let sender = match self.sender.clone() {
            Some(sender) => sender,
            None => {
                warn!("No sender available for snapshot collection");
                return;
            }
        };

        let mut strategy_clone = self.clone();

        let handle = tokio::spawn(async move {
            debug!("Starting snapshot collection task");
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3));

            loop {
                interval.tick().await;

                match strategy_clone.retrieve_assets().await {
                    Ok(assets) if !assets.is_empty() => {
                        debug!("Collected {} asset(s)", assets.len());
                        if sender.send(ActivityReport::Assets(assets)).is_err() {
                            warn!("Failed to send assets - receiver dropped");
                            break;
                        }
                    }
                    Ok(_) => {
                        debug!("No assets collected");
                    }
                    Err(e) => {
                        warn!("Failed to retrieve assets: {}", e);
                    }
                }

                match strategy_clone.retrieve_snapshots().await {
                    Ok(snapshots) if !snapshots.is_empty() => {
                        debug!("Collected {} snapshot(s)", snapshots.len());
                        if sender.send(ActivityReport::Snapshots(snapshots)).is_err() {
                            warn!("Failed to send snapshots - receiver dropped");
                            break;
                        }
                    }
                    Ok(_) => {
                        debug!("No snapshots collected");
                    }
                    Err(e) => {
                        warn!("Failed to retrieve snapshots: {}", e);
                    }
                }
            }

            debug!("Snapshot collection task ended");
        });

        self.snapshot_collection_handle = Some(Arc::new(handle));
    }
}

#[cfg(test)]
mod tests {
    use crate::strategies::*;

    #[test]
    fn test_supported_processes() {
        let processes = BrowserStrategy::get_supported_processes();
        assert!(!processes.is_empty());

        #[cfg(target_os = "windows")]
        assert!(processes.contains(&"firefox.exe"));

        #[cfg(target_os = "linux")]
        assert!(processes.contains(&"firefox"));

        #[cfg(target_os = "macos")]
        assert!(processes.contains(&"Firefox"));
    }
}
