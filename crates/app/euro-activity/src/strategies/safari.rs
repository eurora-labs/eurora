//! Browser strategy implementation for the activity system
//!
//! This module uses the singleton gRPC server from `euro_browser` crate to accept
//! connections from multiple native messaging hosts (euro-native-messaging). Each
//! host registers with its browser PID, allowing the server to route requests to
//! the correct browser.
//!
//! ## Hybrid push/pull collection model
//!
//! The browser extension proactively sends metadata, assets and snapshots as
//! Event frames whenever the browser window is focused.  The strategy subscribes
//! to these events and forwards them through the `ActivityReport` channel.
//!
//! When Safari is re-focused (same browser PID), the strategy also sends a
//! `GET_METADATA` request via the gRPC stream.  The Safari extension picks this
//! up through its 500ms polling loop (`safari-poller.ts`) and responds with the
//! current tab metadata.  This avoids the unreliable
//! `SFSafariApplication.dispatchMessage` path.

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
use url::Url;

pub use euro_browser::{
    BrowserBridgeServer, BrowserBridgeService, EventFrame, Frame, FrameKind, RequestFrame,
    ResponseFrame,
};

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct SafariStrategy {
    #[serde(skip)]
    sender: Option<mpsc::UnboundedSender<ActivityReport>>,

    #[serde(skip)]
    bridge_service: Option<&'static BrowserBridgeService>,

    #[serde(skip)]
    event_subscription_handle: Option<Arc<tokio::task::JoinHandle<()>>>,

    #[serde(skip)]
    active_browser: Option<String>,

    #[serde(skip)]
    active_browser_pid: Option<u32>,
}

impl SafariStrategy {
    async fn initialize_service(&mut self) -> ActivityResult<()> {
        let service = BrowserBridgeService::get_or_init().await;
        self.bridge_service = Some(service);
        Ok(())
    }

    /// Subscribe to the event stream coming from browser extensions.
    ///
    /// The extension now pushes three kinds of events:
    ///
    /// | `action`         | Payload type       | What we do                          |
    /// |------------------|--------------------|-------------------------------------|
    /// | `TAB_UPDATED`    | `NativeMetadata`   | Create a new `Activity`             |
    /// | `TAB_ACTIVATED`  | `NativeMetadata`   | Create a new `Activity`             |
    /// | `ASSETS`         | Any asset variant  | Forward as `ActivityReport::Assets` |
    /// | `SNAPSHOT`       | Any snapshot variant | Forward as `ActivityReport::Snapshots` |
    async fn init_collection(&mut self, _focus_window: &FocusedWindow) -> ActivityResult<()> {
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
        let last_url: Arc<tokio::sync::Mutex<Option<Url>>> =
            Arc::new(tokio::sync::Mutex::new(None));

        let handle = tokio::spawn(async move {
            let last_url = Arc::clone(&last_url);

            while let Ok((browser_pid, event_frame)) = events_rx.recv().await {
                tracing::debug!(
                    "Received event from browser PID {}: action={}",
                    browser_pid,
                    event_frame.action
                );

                let Some(payload_str) = event_frame.payload else {
                    continue;
                };

                match event_frame.action.as_str() {
                    "TAB_UPDATED" | "TAB_ACTIVATED" => {
                        let native_message =
                            match serde_json::from_str::<NativeMessage>(&payload_str) {
                                Ok(msg) => msg,
                                Err(e) => {
                                    tracing::warn!("Failed to parse metadata payload: {}", e);
                                    continue;
                                }
                            };

                        let metadata = match native_message {
                            NativeMessage::NativeMetadata(data) => StrategyMetadata::from(data),
                            _ => {
                                tracing::debug!("Ignoring non-metadata event");
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

                        let activity = Activity::new(url_str, icon, "".to_string(), vec![]);

                        tracing::info!(
                            "Creating new activity from event: browser_pid={}, name={}",
                            browser_pid,
                            activity.name
                        );
                        if sender.send(ActivityReport::NewActivity(activity)).is_err() {
                            tracing::warn!("Failed to send new activity report - receiver dropped");
                            break;
                        }
                    }

                    "ASSETS" => {
                        let native_message =
                            match serde_json::from_str::<NativeMessage>(&payload_str) {
                                Ok(msg) => msg,
                                Err(e) => {
                                    tracing::warn!("Failed to parse asset payload: {}", e);
                                    continue;
                                }
                            };

                        match ActivityAsset::try_from(native_message) {
                            Ok(asset) => {
                                tracing::debug!("Received asset from browser PID {}", browser_pid);
                                if sender.send(ActivityReport::Assets(vec![asset])).is_err() {
                                    tracing::warn!("Failed to send assets - receiver dropped");
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to convert native message to asset: {}", e);
                            }
                        }
                    }

                    "SNAPSHOT" => {
                        let native_message =
                            match serde_json::from_str::<NativeMessage>(&payload_str) {
                                Ok(msg) => msg,
                                Err(e) => {
                                    tracing::warn!("Failed to parse snapshot payload: {}", e);
                                    continue;
                                }
                            };

                        match ActivitySnapshot::try_from(native_message) {
                            Ok(snapshot) => {
                                tracing::debug!(
                                    "Received snapshot from browser PID {}",
                                    browser_pid
                                );
                                if sender
                                    .send(ActivityReport::Snapshots(vec![snapshot]))
                                    .is_err()
                                {
                                    tracing::warn!("Failed to send snapshots - receiver dropped");
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to convert native message to snapshot: {}",
                                    e
                                );
                            }
                        }
                    }

                    other => {
                        tracing::debug!("Ignoring unknown event action: {}", other);
                    }
                }
            }

            tracing::debug!("Event subscription task ended");
        });

        self.event_subscription_handle = Some(Arc::new(handle));
        Ok(())
    }

    async fn request_assets_and_snapshots(&mut self) {
        let Some(sender) = self.sender.clone() else {
            return;
        };

        match self.retrieve_assets().await {
            Ok(assets) if !assets.is_empty() => {
                if sender.send(ActivityReport::Assets(assets)).is_err() {
                    tracing::warn!("Failed to send assets report - receiver dropped");
                }
            }
            Err(e) => tracing::debug!("Failed to retrieve assets: {}", e),
            _ => {}
        }

        match self.retrieve_snapshots().await {
            Ok(snapshots) if !snapshots.is_empty() => {
                if sender.send(ActivityReport::Snapshots(snapshots)).is_err() {
                    tracing::warn!("Failed to send snapshots report - receiver dropped");
                }
            }
            Err(e) => tracing::debug!("Failed to retrieve snapshots: {}", e),
            _ => {}
        }
    }

    pub async fn new() -> ActivityResult<Self> {
        let mut strategy = SafariStrategy::default();
        strategy.initialize_service().await?;
        Ok(strategy)
    }
}

#[async_trait]
impl StrategySupport for SafariStrategy {
    fn get_supported_processes() -> Vec<&'static str> {
        vec![Safari.get_name()]
    }

    async fn create() -> ActivityResult<ActivityStrategy> {
        Ok(ActivityStrategy::SafariStrategy(
            SafariStrategy::new().await?,
        ))
    }
}

#[async_trait]
impl ActivityStrategyFunctionality for SafariStrategy {
    fn can_handle_process(&self, focus_window: &FocusedWindow) -> bool {
        SafariStrategy::get_supported_processes().contains(&focus_window.process_name.as_str())
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

        self.init_collection(focus_window).await?;

        match self.get_metadata().await {
            Ok(metadata) => {
                let activity = Activity::new(
                    metadata.url.unwrap_or_default(),
                    metadata.icon,
                    "".to_string(),
                    vec![],
                );
                tracing::info!(
                    "Safari start_tracking: initial metadata activity: {}",
                    activity.name
                );
                if sender.send(ActivityReport::NewActivity(activity)).is_err() {
                    tracing::warn!("Failed to send initial activity report - receiver dropped");
                }
            }
            Err(e) => {
                tracing::warn!("Failed to get initial metadata on start_tracking: {}", e);
            }
        }

        self.request_assets_and_snapshots().await;

        tracing::debug!("Browser strategy starting tracking for: {:?}", process_name);
        Ok(())
    }

    async fn handle_process_change(
        &mut self,
        focus_window: &FocusedWindow,
    ) -> ActivityResult<bool> {
        tracing::debug!(
            "Browser strategy handling process change to: {}",
            focus_window.process_name
        );

        if self.can_handle_process(focus_window) {
            tracing::debug!(
                "Browser strategy can continue handling: {}",
                focus_window.process_name
            );
            if self.active_browser_pid != Some(focus_window.process_id) {
                self.active_browser_pid = Some(focus_window.process_id);
                self.active_browser = Some(focus_window.process_name.to_string());
            }

            match self.get_metadata().await {
                Ok(metadata) => {
                    if let Some(sender) = &self.sender {
                        let activity = Activity::new(
                            metadata.url.unwrap_or_default(),
                            metadata.icon,
                            "".to_string(),
                            vec![],
                        );
                        tracing::info!(
                            "Safari refocus: created activity from metadata: {}",
                            activity.name
                        );
                        if sender.send(ActivityReport::NewActivity(activity)).is_err() {
                            tracing::warn!("Failed to send new activity report - receiver dropped");
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to get metadata on Safari refocus: {}", e);
                    if let Some(sender) = &self.sender {
                        let activity = Activity::new(
                            focus_window.process_name.clone(),
                            focus_window.icon.clone(),
                            focus_window.process_name.clone(),
                            vec![],
                        );
                        if sender.send(ActivityReport::NewActivity(activity)).is_err() {
                            tracing::warn!("Failed to send new activity report - receiver dropped");
                        }
                    }
                }
            }

            self.request_assets_and_snapshots().await;

            Ok(true)
        } else {
            tracing::debug!(
                "Browser strategy cannot handle: {}, stopping tracking",
                focus_window.process_name
            );
            self.stop_tracking().await?;
            Ok(false)
        }
    }

    async fn stop_tracking(&mut self) -> ActivityResult<()> {
        tracing::debug!("Browser strategy stopping tracking");
        self.active_browser = None;
        self.active_browser_pid = None;

        if let Some(handle) = self.event_subscription_handle.take()
            && let Ok(handle) = Arc::try_unwrap(handle)
        {
            handle.abort();
        }

        Ok(())
    }

    async fn retrieve_assets(&mut self) -> ActivityResult<Vec<ActivityAsset>> {
        let service = self
            .bridge_service
            .as_ref()
            .ok_or_else(|| ActivityError::invalid_data("Bridge service not available"))?;

        let browser_pid = self
            .active_browser_pid
            .ok_or_else(|| ActivityError::invalid_data("No active browser PID set"))?;

        let response_frame = service
            .send_request(browser_pid, "GET_ASSETS", None)
            .await
            .map_err(|e| ActivityError::invalid_data(format!("Failed to get assets: {}", e)))?;

        let Some(payload) = response_frame.payload else {
            return Ok(vec![]);
        };

        let native_message = serde_json::from_str::<NativeMessage>(&payload)
            .map_err(|e| -> ActivityError { ActivityError::from(e) })?;

        match ActivityAsset::try_from(native_message) {
            Ok(asset) => Ok(vec![asset]),
            Err(e) => {
                tracing::warn!("Failed to convert asset response: {}", e);
                Ok(vec![])
            }
        }
    }

    async fn retrieve_snapshots(&mut self) -> ActivityResult<Vec<ActivitySnapshot>> {
        let service = self
            .bridge_service
            .as_ref()
            .ok_or_else(|| ActivityError::invalid_data("Bridge service not available"))?;

        let browser_pid = self
            .active_browser_pid
            .ok_or_else(|| ActivityError::invalid_data("No active browser PID set"))?;

        let response_frame = service
            .send_request(browser_pid, "GET_SNAPSHOT", None)
            .await
            .map_err(|e| ActivityError::invalid_data(format!("Failed to get snapshot: {}", e)))?;

        let Some(payload) = response_frame.payload else {
            return Ok(vec![]);
        };

        let native_message = serde_json::from_str::<NativeMessage>(&payload)
            .map_err(|e| -> ActivityError { ActivityError::from(e) })?;

        match ActivitySnapshot::try_from(native_message) {
            Ok(snapshot) => Ok(vec![snapshot]),
            Err(e) => {
                tracing::warn!("Failed to convert snapshot response: {}", e);
                Ok(vec![])
            }
        }
    }

    async fn get_metadata(&mut self) -> ActivityResult<StrategyMetadata> {
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
            return Ok(StrategyMetadata::default());
        };

        let native_metadata = serde_json::from_str::<NativeMessage>(&payload)
            .map_err(|e| -> ActivityError { ActivityError::from(e) })?;

        match native_metadata {
            NativeMessage::NativeMetadata(metadata) => Ok(StrategyMetadata::from(metadata)),
            _ => Ok(StrategyMetadata::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::strategies::*;

    #[test]
    fn test_supported_processes() {
        let processes = SafariStrategy::get_supported_processes();
        assert!(!processes.is_empty());

        #[cfg(target_os = "windows")]
        assert!(processes.contains(&"safari.exe"));

        #[cfg(target_os = "linux")]
        assert!(processes.contains(&"safari"));

        #[cfg(target_os = "macos")]
        assert!(processes.contains(&"Safari"));
    }
}
