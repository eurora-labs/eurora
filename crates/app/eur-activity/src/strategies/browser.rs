//! Browser strategy implementation for the refactored activity system

pub use crate::strategies::ActivityStrategyFunctionality;
pub use crate::strategies::processes::*;
pub use crate::strategies::{ActivityStrategy, StrategySupport};
use async_trait::async_trait;
use dashmap::DashMap;
use eur_native_messaging::proto::RequestFrame;
use eur_native_messaging::proto::ResponseFrame;
use eur_native_messaging::{
    NativeMessage, create_browser_bridge_client,
    server::{Frame, FrameKind},
};
use ferrous_focus::FocusedWindow;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::broadcast;
use tokio::sync::{Mutex, mpsc, oneshot};
use tracing::{debug, error, info, warn};
use url::Url;

use crate::strategies::{ActivityReport, StrategyMetadata};

/// Wrapper for pending request sender
struct PendingRequest {
    sender: oneshot::Sender<Frame>,
}

impl std::fmt::Debug for PendingRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingRequest")
            .field("sender", &"oneshot::Sender<Frame>")
            .finish()
    }
}

impl PendingRequest {
    fn new(sender: oneshot::Sender<Frame>) -> Self {
        Self { sender }
    }

    fn send(self, frame: Frame) -> Result<(), ()> {
        if self.sender.send(frame).is_err() {
            error!("Failed to send frame to waiting request");
        }
        Ok(())
    }
}

use crate::{
    Activity, ActivityError,
    error::ActivityResult,
    types::{ActivityAsset, ActivitySnapshot},
};

/// Browser strategy for collecting web browser activity data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BrowserStrategy {
    #[serde(skip)]
    tracking_handle: Option<Arc<tokio::task::JoinHandle<()>>>,
    #[serde(skip)]
    sender: Option<mpsc::UnboundedSender<ActivityReport>>,

    // Bidirectional stream components
    #[serde(skip)]
    stream_tx: Option<mpsc::UnboundedSender<Frame>>,
    #[serde(skip)]
    pending_requests: Option<Arc<DashMap<u32, PendingRequest>>>,
    #[serde(skip)]
    request_id_counter: Option<Arc<AtomicU32>>,
    #[serde(skip)]
    stream_task_handle: Option<Arc<tokio::task::JoinHandle<()>>>,

    #[serde(skip)]
    activity_event_tx: Option<broadcast::Sender<Frame>>,

    #[serde(skip)]
    snapshot_collection_handle: Option<Arc<tokio::task::JoinHandle<()>>>,

    #[serde(skip)]
    active_browser: Option<String>,
}

impl BrowserStrategy {
    /// Creates thin network layer to manage incoming and outgoing requests
    async fn initialize_browser_communication(&mut self) -> ActivityResult<()> {
        let mut client = create_browser_bridge_client().await.map_err(|e| {
            ActivityError::Network(format!("Failed to create browser bridge client: {}", e))
        })?;

        let activity_event_tx: broadcast::Sender<Frame> = broadcast::channel(100).0;

        let (tx, rx) = mpsc::unbounded_channel::<Frame>();
        let pending_requests = Arc::new(DashMap::<u32, PendingRequest>::new());
        let request_id_counter = Arc::new(AtomicU32::new(1));

        let pending_requests_clone = Arc::clone(&pending_requests);

        let response = client
            .open(tokio_stream::wrappers::UnboundedReceiverStream::new(rx))
            .await
            .map_err(|e| {
                error!("Failed to open bidirectional browser channel: {}", e);
                ActivityError::Network("Failed to open browser bridge client".to_string())
            })?;
        let mut inbound_stream = response.into_inner();

        let activity_event_tx_clone = activity_event_tx.clone();

        let stream_task = tokio::spawn(async move {
            debug!("Stream handler task started");
            while let Ok(Some(frame)) = inbound_stream.message().await {
                let kind = frame.kind.unwrap();
                match kind {
                    FrameKind::Request(frame) => {
                        debug!(
                            "Received request frame: id={}, action={}",
                            frame.id, frame.action
                        );
                        // For now, log unsupported requests from browser extension
                        // In the future, this could handle requests initiated by the extension
                        warn!(
                            "Received unsupported request from browser extension: action={}",
                            frame.action
                        );
                    }
                    FrameKind::Response(frame) => {
                        // Match response to pending request
                        if let Some((_, pending_request)) = pending_requests_clone.remove(&frame.id)
                        {
                            if let Err(err) = pending_request.send(frame.into()) {
                                warn!("Failed to send frame to waiting request: {:?}", err);
                            }
                        } else {
                            debug!(
                                "Received frame with no pending request: id={} action={}",
                                frame.id.clone(),
                                frame.action.clone(),
                            );
                            let _ = activity_event_tx_clone.send(frame.into());
                        }
                    }
                    FrameKind::Event(frame) => {
                        // Broadcast event frames to activity tracking
                        debug!("Received event frame: action={}", frame.action.clone());
                        let _ = activity_event_tx_clone.send(frame.into());
                    }
                    FrameKind::Error(frame) => {
                        error!(
                            "Received error frame: id={}, message={}",
                            frame.id, frame.message
                        );
                        // Match error to pending request if applicable
                        if let Some((_, pending_request)) = pending_requests_clone.remove(&frame.id)
                            && let Err(err) = pending_request.send(frame.into())
                        {
                            warn!("Failed to send error frame to waiting request: {:?}", err);
                        }
                    }
                    FrameKind::Cancel(frame) => {
                        debug!("Received cancel frame: id={}", frame.id);
                        // Remove pending request if it exists
                        if pending_requests_clone.remove(&frame.id).is_some() {
                            debug!("Cancelled pending request: id={}", frame.id);
                        }
                    }
                }
            }
            debug!("Stream handler task ended");
        });

        self.stream_tx = Some(tx);
        self.pending_requests = Some(pending_requests);
        self.request_id_counter = Some(request_id_counter);
        self.stream_task_handle = Some(Arc::new(stream_task));
        self.activity_event_tx = Some(activity_event_tx);

        Ok(())
    }

    async fn init_collection(&mut self, focus_window: &FocusedWindow) -> ActivityResult<()> {
        // Initialize tracking logic here
        let Some(sender) = self.sender.clone() else {
            return Err(ActivityError::Strategy(
                "Sender not initialized".to_string(),
            ));
        };
        let mut activity_receiver = self.activity_event_tx.clone().unwrap().subscribe();
        let _default_icon = focus_window.icon.clone();
        let mut strategy = self.clone();
        let last_url: Arc<Mutex<Option<Url>>> = Arc::new(Mutex::new(None));

        let handle = tokio::spawn(async move {
            let last_url = Arc::clone(&last_url);

            while let Ok(frame) = activity_receiver.recv().await {
                let kind = frame.kind.unwrap();
                let payload = match kind {
                    FrameKind::Response(frame) => frame.payload,
                    FrameKind::Event(frame) => frame.payload,
                    _ => None,
                };
                let native_asset = serde_json::from_str::<NativeMessage>(&payload.unwrap())
                    .map_err(|e| -> ActivityError { ActivityError::from(e) })
                    .unwrap();

                let event = match native_asset {
                    NativeMessage::NativeMetadata(data) => StrategyMetadata::from(data),
                    _ => {
                        panic!("Unexpected native asset type");
                    }
                };
                let mut prev = last_url.lock().await;
                let url = Url::parse(&event.url.clone().unwrap()).unwrap();
                if let Some(prev_url) = prev.take()
                    && prev_url.domain() == url.domain()
                {
                    *prev = Some(url);
                    continue;
                }
                *prev = Some(url);
                let icon = event.icon;

                let assets = strategy.retrieve_assets().await.map_err(|e| {
                    warn!("Failed to retrieve assets: {}", e);
                    e
                });
                let activity = Activity::new(
                    event.url.unwrap().clone(),
                    icon,
                    "".to_string(),
                    assets.unwrap_or_default(),
                );

                if sender
                    .send(ActivityReport::NewActivity(activity.clone()))
                    .is_err()
                {
                    warn!("Failed to send new activity report - receiver dropped");
                    break;
                }
            }
        });

        self.tracking_handle = Some(Arc::new(handle));

        // Start snapshot collection
        self.collect_snapshots();
        Ok(())
    }

    /// Create a new browser strategy
    pub async fn new() -> ActivityResult<Self> {
        let mut strategy = BrowserStrategy::default();
        strategy.initialize_browser_communication().await?;

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
    fn can_handle_process(&self, process_name: &str) -> bool {
        BrowserStrategy::get_supported_processes().contains(&process_name)
    }

    async fn start_tracking(
        &mut self,
        focus_window: &FocusedWindow,
        sender: mpsc::UnboundedSender<ActivityReport>,
    ) -> ActivityResult<()> {
        self.sender = Some(sender.clone());
        let process_name = focus_window.process_name.clone();
        self.active_browser = process_name.clone();

        match self.get_metadata().await {
            Ok(metadata) => {
                let assets = self.retrieve_assets().await.unwrap_or(vec![]);
                let activity = Activity::new(
                    metadata.url.unwrap_or_default(),
                    metadata.icon,
                    process_name.clone().unwrap_or_default(),
                    assets,
                );
                if sender.send(ActivityReport::NewActivity(activity)).is_err() {
                    warn!("Failed to send new activity report - receiver dropped");
                }
            }
            Err(err) => {
                let activity = Activity::new(
                    focus_window.process_name.clone().unwrap_or_default(),
                    focus_window.icon.clone(),
                    focus_window.process_name.clone().unwrap_or_default(),
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

    async fn handle_process_change(&mut self, process_name: &str) -> ActivityResult<bool> {
        debug!(
            "Browser strategy handling process change to: {}",
            process_name
        );

        // Check if this strategy can handle the new process
        if self.can_handle_process(process_name) {
            debug!("Browser strategy can continue handling: {}", process_name);
            if self.active_browser.as_deref() != Some(process_name) {
                info!(
                    "Detected new browser {} that is not being tracked. Ignoring.",
                    process_name
                );
                return Ok(true);
            }

            if let Some(sender) = self.sender.clone() {
                match self.get_metadata().await {
                    Ok(metadata) => {
                        let activity = Activity::new(
                            metadata.url.unwrap_or_default(),
                            metadata.icon,
                            process_name.to_string(),
                            vec![],
                        );
                        if sender.send(ActivityReport::NewActivity(activity)).is_err() {
                            warn!("Failed to send new activity report - receiver dropped");
                        }
                    }
                    Err(err) => {
                        warn!("Failed to get metadata: {}", err);
                    }
                }
            }

            Ok(true)
        } else {
            debug!(
                "Browser strategy cannot handle: {}, stopping tracking",
                process_name
            );
            // Properly stop tracking to abort the listener task
            self.stop_tracking().await?;
            Ok(false)
        }
    }

    async fn stop_tracking(&mut self) -> ActivityResult<()> {
        debug!("Browser strategy stopping tracking");
        self.active_browser = None;

        if let Some(handle) = self.tracking_handle.take() {
            // Try to unwrap Arc, if we're the only owner, abort the task
            if let Ok(handle) = Arc::try_unwrap(handle) {
                handle.abort();
            }
        }

        // Clean up stream task
        if let Some(handle) = self.stream_task_handle.take()
            && let Ok(handle) = Arc::try_unwrap(handle)
        {
            handle.abort();
        }

        // Clean up snapshot collection task
        if let Some(handle) = self.snapshot_collection_handle.take()
            && let Ok(handle) = Arc::try_unwrap(handle)
        {
            handle.abort();
        }

        Ok(())
    }

    /// Retrieve assets from the browser
    async fn retrieve_assets(&mut self) -> ActivityResult<Vec<ActivityAsset>> {
        debug!("Retrieving assets for browser strategy");

        let response_frame = self.send_request("GENERATE_ASSETS").await?;

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

    /// Retrieve snapshots from the browser
    async fn retrieve_snapshots(&mut self) -> ActivityResult<Vec<ActivitySnapshot>> {
        let response_frame = self.send_request("GENERATE_SNAPSHOT").await?;

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

        let response_frame = self.send_request("GET_METADATA").await?;

        let Some(payload) = response_frame.payload else {
            warn!("No payload in metadata response");
            return Ok(StrategyMetadata::default());
        };

        let native_metadata = serde_json::from_str::<NativeMessage>(&payload)
            .map_err(|e| -> ActivityError { ActivityError::from(e) })?;

        let metadata = match native_metadata {
            NativeMessage::NativeMetadata(metadata) => {
                // Validate URL if present
                if let Some(ref url) = metadata.url
                    && !url.starts_with("http")
                    // TODO: Add the actual extension ID after we're accepted to chrome
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
    /// Helper method to send a request frame and wait for response
    async fn send_request(&self, action: &str) -> ActivityResult<ResponseFrame> {
        let stream_tx = self
            .stream_tx
            .as_ref()
            .ok_or_else(|| ActivityError::invalid_data("No stream sender available"))?;

        let pending_requests = self
            .pending_requests
            .as_ref()
            .ok_or_else(|| ActivityError::invalid_data("No pending requests map available"))?;

        let request_id_counter = self
            .request_id_counter
            .as_ref()
            .ok_or_else(|| ActivityError::invalid_data("No request ID counter available"))?;

        // Generate unique request ID
        let request_id = request_id_counter.fetch_add(1, Ordering::SeqCst);

        // Create oneshot channel for response
        let (tx, rx) = oneshot::channel();

        // Register pending request
        pending_requests.insert(request_id, PendingRequest::new(tx));

        // Create and send request frame
        let request_frame = RequestFrame {
            id: request_id,
            action: action.to_string(),
            payload: None,
        };

        debug!(
            "Sending request frame: id={}, command={}",
            request_id, action
        );

        stream_tx
            .send(request_frame.into())
            .map_err(|_| ActivityError::invalid_data("Failed to send request frame"))?;

        // Wait for response with timeout
        match tokio::time::timeout(std::time::Duration::from_secs(5), rx).await {
            Ok(Ok(frame)) => match frame.kind.unwrap() {
                FrameKind::Response(frame) => {
                    debug!("Received response for request {}", request_id);
                    Ok(frame)
                }
                FrameKind::Error(frame) => Err(ActivityError::invalid_data(format!(
                    "Browser error: {}",
                    frame.message
                ))),
                _ => Err(ActivityError::invalid_data("Unexpected frame kind")),
            },
            Ok(Err(_)) => {
                error!("Response channel closed for request {}", request_id);
                Err(ActivityError::invalid_data("Response channel closed"))
            }
            Err(_) => {
                error!("Timeout waiting for response to request {}", request_id);
                // Clean up pending request on timeout
                pending_requests.remove(&request_id);
                Err(ActivityError::invalid_data("Request timeout"))
            }
        }
    }

    fn collect_snapshots(&mut self) {
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
