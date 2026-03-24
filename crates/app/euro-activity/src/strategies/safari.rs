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
                        let title = metadata.title.clone();
                        let url_str = metadata.url.clone().unwrap_or_default();

                        let activity = Activity::new(url_str, title, icon, "".to_string(), vec![]);

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

                    _ => {}
                }
            }

            tracing::debug!("Event subscription task ended");
        });

        self.event_subscription_handle = Some(Arc::new(handle));
        Ok(())
    }

    async fn fetch_asset(
        service: &BrowserBridgeService,
        browser_pid: u32,
    ) -> Option<ActivityAsset> {
        let response = service
            .send_request(browser_pid, "GET_ASSETS", None)
            .await
            .ok()?;
        let payload = response.payload?;
        let native_message = serde_json::from_str::<NativeMessage>(&payload).ok()?;
        ActivityAsset::try_from(native_message).ok()
    }

    async fn fetch_snapshot(
        service: &BrowserBridgeService,
        browser_pid: u32,
    ) -> Option<ActivitySnapshot> {
        let response = service
            .send_request(browser_pid, "GET_SNAPSHOT", None)
            .await
            .ok()?;
        let payload = response.payload?;
        let native_message = serde_json::from_str::<NativeMessage>(&payload).ok()?;
        ActivitySnapshot::try_from(native_message).ok()
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
                    metadata.title,
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
                            metadata.title,
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
                            None,
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
        let Some(service) = self.bridge_service else {
            return Ok(vec![]);
        };
        let Some(browser_pid) = self.active_browser_pid else {
            return Ok(vec![]);
        };
        Ok(Self::fetch_asset(service, browser_pid)
            .await
            .into_iter()
            .collect())
    }

    async fn retrieve_snapshots(&mut self) -> ActivityResult<Vec<ActivitySnapshot>> {
        let Some(service) = self.bridge_service else {
            return Ok(vec![]);
        };
        let Some(browser_pid) = self.active_browser_pid else {
            return Ok(vec![]);
        };
        Ok(Self::fetch_snapshot(service, browser_pid)
            .await
            .into_iter()
            .collect())
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
