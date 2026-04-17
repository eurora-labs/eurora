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
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use tokio::sync::mpsc;
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
    active_browser: Option<String>,

    #[serde(skip)]
    active_browser_pid: Arc<AtomicU32>,

    #[serde(skip)]
    last_url: Arc<tokio::sync::Mutex<Option<Url>>>,
}

impl BrowserStrategy {
    async fn initialize_service(&mut self) -> ActivityResult<()> {
        let service = BrowserBridgeService::get_or_init().await;
        self.bridge_service = Some(service);
        Ok(())
    }

    async fn init_collection(&mut self) -> ActivityResult<()> {
        if self.event_subscription_handle.is_some() {
            return Ok(());
        }

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
        let last_url = Arc::clone(&self.last_url);
        let active_pid = Arc::clone(&self.active_browser_pid);

        let handle = tokio::spawn(async move {
            let last_url = Arc::clone(&last_url);

            loop {
                let (browser_pid, event_frame) = match events_rx.recv().await {
                    Ok(val) => val,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Event subscription lagged by {} events, resuming", n);
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                };

                let expected_pid = active_pid.load(Ordering::Relaxed);
                if expected_pid == 0 || browser_pid != expected_pid {
                    continue;
                }

                let Some(payload_str) = event_frame.payload else {
                    continue;
                };

                let native_message = match serde_json::from_str::<NativeMessage>(&payload_str) {
                    Ok(msg) => msg,
                    Err(e) => {
                        tracing::warn!("Failed to parse native message: {}", e);
                        continue;
                    }
                };

                if event_frame.action.as_str() == "TAB_ACTIVATED" {
                    let NativeMessage::NativeMetadata(data) = native_message else {
                        continue;
                    };
                    let metadata = StrategyMetadata::from(data);

                    let mut prev = last_url.lock().await;
                    let url = match Url::parse(&metadata.url.clone().unwrap_or_default()) {
                        Ok(u) => u,
                        Err(_) => continue,
                    };

                    if let Some(prev_url) = prev.take()
                        && prev_url.domain() == url.domain()
                    {
                        let title = metadata.title.unwrap_or_else(|| url.to_string());
                        let url_str = url.to_string();
                        *prev = Some(url);
                        let _ = sender.send(ActivityReport::TitleUpdated {
                            title,
                            url: url_str,
                        });
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

    async fn resolve_messenger_pid(&self, process_name: &str, fallback_pid: u32) -> u32 {
        if let Some(service) = &self.bridge_service
            && let Some(pid) = service.find_pid_by_browser_name(process_name).await
        {
            return pid;
        }
        fallback_pid
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
        vec![
            Librewolf.get_name(),
            Firefox.get_name(),
            Chrome.get_name(),
            Edge.get_name(),
            Brave.get_name(),
            Opera.get_name(),
        ]
    }

    async fn create() -> ActivityResult<ActivityStrategy> {
        Ok(ActivityStrategy::BrowserStrategy(
            BrowserStrategy::new().await?,
        ))
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

        let messenger_pid = self
            .resolve_messenger_pid(&process_name, focus_window.process_id)
            .await;
        self.active_browser_pid
            .store(messenger_pid, Ordering::Relaxed);

        self.init_collection().await?;

        match self.get_metadata().await {
            Ok(metadata) => {
                if let Some(ref url_str) = metadata.url
                    && let Ok(url) = Url::parse(url_str)
                {
                    *self.last_url.lock().await = Some(url);
                }
                let activity = Activity::new(
                    metadata.url.unwrap_or_default(),
                    metadata.title,
                    metadata.icon,
                    process_name.clone(),
                    vec![],
                );
                if sender.send(ActivityReport::NewActivity(activity)).is_err() {
                    tracing::warn!("Failed to send new activity report - receiver dropped");
                }
            }
            Err(err) => {
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
                tracing::warn!("Failed to get metadata: {}", err);
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
            let messenger_pid = self
                .resolve_messenger_pid(&focus_window.process_name, focus_window.process_id)
                .await;
            self.active_browser_pid
                .store(messenger_pid, Ordering::Relaxed);
            self.active_browser = Some(focus_window.process_name.to_string());

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
        self.active_browser_pid.store(0, Ordering::Relaxed);

        Ok(())
    }

    async fn retrieve_assets(&mut self) -> ActivityResult<Vec<ActivityAsset>> {
        let Some(service) = self.bridge_service else {
            return Ok(vec![]);
        };
        let browser_pid = self.active_browser_pid.load(Ordering::Relaxed);
        if browser_pid == 0 {
            return Ok(vec![]);
        }
        Ok(Self::fetch_asset(service, browser_pid)
            .await
            .into_iter()
            .collect())
    }

    async fn retrieve_snapshots(&mut self) -> ActivityResult<Vec<ActivitySnapshot>> {
        let Some(service) = self.bridge_service else {
            return Ok(vec![]);
        };
        let browser_pid = self.active_browser_pid.load(Ordering::Relaxed);
        if browser_pid == 0 {
            return Ok(vec![]);
        }
        Ok(Self::fetch_snapshot(service, browser_pid)
            .await
            .into_iter()
            .collect())
    }

    async fn get_metadata(&mut self) -> ActivityResult<StrategyMetadata> {
        tracing::debug!("Retrieving metadata for browser strategy");

        let service = self
            .bridge_service
            .as_ref()
            .ok_or_else(|| ActivityError::invalid_data("Bridge service not available"))?;

        let browser_pid = self.active_browser_pid.load(Ordering::Relaxed);
        if browser_pid == 0 {
            return Err(ActivityError::invalid_data("No active browser PID set"));
        }

        let response_frame = service
            .get_metadata(browser_pid)
            .await
            .map_err(|e| ActivityError::invalid_data(format!("Failed to get metadata: {}", e)))?;

        let Some(payload) = response_frame.payload else {
            tracing::warn!("No payload in metadata response");
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
