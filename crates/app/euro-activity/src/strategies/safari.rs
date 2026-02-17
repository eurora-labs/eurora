//! Browser strategy implementation for the activity system
//!
//! This module uses the singleton gRPC server from `euro_browser` crate to accept
//! connections from multiple native messaging hosts (euro-native-messaging). Each
//! host registers with its browser PID, allowing the server to route requests to
//! the correct browser.
//!
//! ## Push-based collection model
//!
//! The browser extension proactively sends metadata, assets and snapshots as
//! Event frames whenever the browser window is focused.  The strategy simply
//! subscribes to these events and forwards them through the `ActivityReport`
//! channel.  This avoids the previous pull-based model where the server sent
//! Request frames (GENERATE_ASSETS, GENERATE_SNAPSHOT, GET_METADATA) that had
//! to be routed back to the extension – a path that was unreliable on Safari
//! due to `SFSafariApplication.dispatchMessage` limitations.

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
                debug!(
                    "Received event from browser PID {}: action={}",
                    browser_pid, event_frame.action
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
                                    warn!("Failed to parse metadata payload: {}", e);
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

                        let activity = Activity::new(url_str, icon, "".to_string(), vec![]);

                        info!(
                            "Creating new activity from event: browser_pid={}, name={}",
                            browser_pid, activity.name
                        );
                        if sender.send(ActivityReport::NewActivity(activity)).is_err() {
                            warn!("Failed to send new activity report - receiver dropped");
                            break;
                        }
                    }

                    "ASSETS" => {
                        let native_message =
                            match serde_json::from_str::<NativeMessage>(&payload_str) {
                                Ok(msg) => msg,
                                Err(e) => {
                                    warn!("Failed to parse asset payload: {}", e);
                                    continue;
                                }
                            };

                        match ActivityAsset::try_from(native_message) {
                            Ok(asset) => {
                                debug!("Received asset from browser PID {}", browser_pid);
                                if sender.send(ActivityReport::Assets(vec![asset])).is_err() {
                                    warn!("Failed to send assets - receiver dropped");
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Failed to convert native message to asset: {}", e);
                            }
                        }
                    }

                    "SNAPSHOT" => {
                        let native_message =
                            match serde_json::from_str::<NativeMessage>(&payload_str) {
                                Ok(msg) => msg,
                                Err(e) => {
                                    warn!("Failed to parse snapshot payload: {}", e);
                                    continue;
                                }
                            };

                        match ActivitySnapshot::try_from(native_message) {
                            Ok(snapshot) => {
                                debug!("Received snapshot from browser PID {}", browser_pid);
                                if sender
                                    .send(ActivityReport::Snapshots(vec![snapshot]))
                                    .is_err()
                                {
                                    warn!("Failed to send snapshots - receiver dropped");
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Failed to convert native message to snapshot: {}", e);
                            }
                        }
                    }

                    other => {
                        debug!("Ignoring unknown event action: {}", other);
                    }
                }
            }

            debug!("Event subscription task ended");
        });

        self.event_subscription_handle = Some(Arc::new(handle));
        Ok(())
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
                info!("Detected the same browser. Ignoring...");
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
                    let activity = Activity::new(
                        focus_window.process_name.clone(),
                        focus_window.icon.clone(),
                        focus_window.process_name.clone(),
                        vec![],
                    );
                    if sender.send(ActivityReport::NewActivity(activity)).is_err() {
                        warn!("Failed to send new activity report - receiver dropped");
                    }
                }
            } else {
                debug!(
                    "Browser PID {} has registered gRPC client, skipping activity report \
                     (will be handled by event subscription)",
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

        Ok(())
    }

    async fn retrieve_assets(&mut self) -> ActivityResult<Vec<ActivityAsset>> {
        debug!("retrieve_assets called (no-op in push model)");
        Ok(vec![])
    }

    async fn retrieve_snapshots(&mut self) -> ActivityResult<Vec<ActivitySnapshot>> {
        debug!("retrieve_snapshots called (no-op in push model)");
        Ok(vec![])
    }

    async fn get_metadata(&mut self) -> ActivityResult<StrategyMetadata> {
        debug!("get_metadata called (no-op in push model)");
        Ok(StrategyMetadata::default())
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
