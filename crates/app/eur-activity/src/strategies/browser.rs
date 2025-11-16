//! Browser strategy implementation for the refactored activity system

pub use crate::strategies::ActivityStrategyFunctionality;
pub use crate::strategies::processes::*;
pub use crate::strategies::{ActivityStrategy, StrategySupport};
use async_trait::async_trait;
use dashmap::DashMap;
use eur_native_messaging::server::Frame;
use eur_native_messaging::{NativeMessage, create_browser_bridge_client};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::broadcast;
use tokio::sync::{Mutex, mpsc, oneshot};
use tracing::{debug, error, warn};
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserStrategy {
    #[serde(skip)]
    tracking_handle: Option<Arc<tokio::task::JoinHandle<()>>>,
    #[serde(skip)]
    sender: Option<mpsc::UnboundedSender<ActivityReport>>,

    // Bidirectional stream components
    #[serde(skip)]
    stream_tx: Option<mpsc::UnboundedSender<Frame>>,
    #[serde(skip)]
    pending_requests: Option<Arc<DashMap<u64, PendingRequest>>>,
    #[serde(skip)]
    request_id_counter: Option<Arc<AtomicU64>>,
    #[serde(skip)]
    stream_task_handle: Option<Arc<tokio::task::JoinHandle<()>>>,

    #[serde(skip)]
    activity_event_tx: Option<broadcast::Sender<Frame>>,
}

impl BrowserStrategy {
    /// Create a new browser strategy
    pub async fn new() -> ActivityResult<Self> {
        let activity_event_tx = broadcast::channel(100).0;

        // Try to create the IPC client and initialize bidirectional stream
        let (_client, stream_tx, pending_requests, request_id_counter, stream_task_handle) =
            match create_browser_bridge_client().await {
                Ok(mut client) => {
                    debug!("Successfully created IPC client for browser strategy");

                    // Initialize bidirectional stream
                    let (tx, rx) = mpsc::unbounded_channel::<Frame>();
                    let pending_requests = Arc::new(DashMap::<u64, PendingRequest>::new());
                    let request_id_counter = Arc::new(AtomicU64::new(1));

                    let pending_requests_clone = Arc::clone(&pending_requests);

                    // Open the bidirectional stream
                    match client
                        .open(tokio_stream::wrappers::UnboundedReceiverStream::new(rx))
                        .await
                    {
                        Ok(response) => {
                            let mut inbound_stream = response.into_inner();
                            let activity_event_tx = activity_event_tx.clone();

                            // Spawn task to handle incoming frames
                            let stream_task = tokio::spawn(async move {
                                debug!("Stream handler task started");
                                while let Ok(Some(frame)) = inbound_stream.message().await {
                                    debug!(
                                        "Received frame: kind={}, id={}, action={}",
                                        frame.kind, frame.id, frame.action
                                    );

                                    // Match response to pending request
                                    if let Some((_, pending_request)) =
                                        pending_requests_clone.remove(&frame.id)
                                    {
                                        if let Err(err) = pending_request.send(frame) {
                                            warn!(
                                                "Failed to send frame to waiting request: {:?}",
                                                err
                                            );
                                        }
                                    } else {
                                        debug!(
                                            "Received frame with no pending request: id={} kind={}",
                                            frame.id.clone(),
                                            frame.kind.clone(),
                                        );
                                        let _ = activity_event_tx.send(frame);
                                    }
                                }
                                debug!("Stream handler task ended");
                            });

                            (
                                Some(client),
                                Some(tx),
                                Some(pending_requests),
                                Some(request_id_counter),
                                Some(Arc::new(stream_task)),
                            )
                        }
                        Err(e) => {
                            warn!(
                                "Failed to open bidirectional stream: {}. Browser strategy will work with limited functionality.",
                                e
                            );
                            (Some(client), None, None, None, None)
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to create IPC client: {}. Browser strategy will work with limited functionality.",
                        e
                    );
                    (None, None, None, None, None)
                }
            };

        Ok(Self {
            tracking_handle: None,
            sender: None,
            stream_tx,
            pending_requests,
            request_id_counter,
            stream_task_handle,
            activity_event_tx: Some(activity_event_tx),
        })
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
        focus_window: &ferrous_focus::FocusedWindow,
        sender: mpsc::UnboundedSender<ActivityReport>,
    ) -> ActivityResult<()> {
        self.sender = Some(sender.clone());
        let process_name = focus_window.process_name.clone();

        match self.get_metadata().await {
            Ok(metadata) => {
                let activity = Activity::new(
                    metadata.url.unwrap_or_default(),
                    metadata.icon,
                    process_name.clone().unwrap_or_default(),
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

        debug!("Browser strategy starting tracking for: {:?}", process_name);

        let mut activity_receiver = self.activity_event_tx.clone().unwrap().subscribe();
        let _default_icon = focus_window.icon.clone();
        let mut strategy = self.clone();
        let last_url: Arc<Mutex<Option<Url>>> = Arc::new(Mutex::new(None));

        let handle = tokio::spawn(async move {
            let last_url = Arc::clone(&last_url);

            while let Ok(event) = activity_receiver.recv().await {
                let native_asset =
                    serde_json::from_str::<NativeMessage>(&event.payload.unwrap().content)
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

                // let icon = match event.icon {
                //     Some(icon) => {
                //         debug!("Received icon data");
                //         image::RgbaImage::from_vec(icon.width as u32, icon.height as u32, icon.data)
                //     }
                //     None => default_icon.clone(),
                // };

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

        Ok(())
    }

    /// Retrieve assets from the browser
    async fn retrieve_assets(&mut self) -> ActivityResult<Vec<ActivityAsset>> {
        debug!("Retrieving assets for browser strategy");

        let response_frame = self.send_request("request", "GENERATE_ASSETS").await?;

        if !response_frame.ok {
            warn!("Failed to retrieve assets: request failed");
            return Ok(vec![]);
        }

        let Some(payload) = response_frame.payload else {
            warn!("No payload in assets response");
            return Ok(vec![]);
        };

        let native_asset = serde_json::from_str::<NativeMessage>(&payload.content)
            .map_err(|e| -> ActivityError { ActivityError::from(e) })?;

        let asset = ActivityAsset::try_from(native_asset)
            .map_err(|e| -> ActivityError { ActivityError::InvalidAssetType(e.to_string()) })?;

        debug!("Retrieved 1 asset from browser");
        Ok(vec![asset])
    }

    /// Retrieve snapshots from the browser
    async fn retrieve_snapshots(&mut self) -> ActivityResult<Vec<ActivitySnapshot>> {
        Ok(vec![])
        // debug!("Retrieving snapshots for browser strategy");

        // let Some(client) = &self.client else {
        //     warn!("No IPC client available for browser strategy");
        //     return Ok(vec![]);
        // };

        // let mut client_guard = client.lock().await;
        // let request = StateRequest {};

        // match client_guard.get_snapshots(request).await {
        //     Ok(response) => {
        //         debug!("Received snapshot response from browser extension");
        //         let mut snapshots = Vec::new();

        //         if let Some(snapshot) = response.into_inner().snapshot {
        //             match snapshot {
        //                 ipc::snapshot_response::Snapshot::Youtube(youtube_snapshot) => {
        //                     match YoutubeSnapshot::try_from(youtube_snapshot) {
        //                         Ok(snapshot) => {
        //                             snapshots.push(ActivitySnapshot::YoutubeSnapshot(snapshot))
        //                         }
        //                         Err(e) => warn!("Failed to create YouTube snapshot: {}", e),
        //                     }
        //                 }
        //                 ipc::snapshot_response::Snapshot::Article(article_snapshot) => {
        //                     let snapshot = ArticleSnapshot::from(article_snapshot);
        //                     snapshots.push(ActivitySnapshot::ArticleSnapshot(snapshot));
        //                 }
        //                 ipc::snapshot_response::Snapshot::Twitter(twitter_snapshot) => {
        //                     let snapshot = TwitterSnapshot::from(twitter_snapshot);
        //                     snapshots.push(ActivitySnapshot::TwitterSnapshot(snapshot));
        //                 }
        //             }
        //         }

        //         debug!("Retrieved {} snapshots from browser", snapshots.len());
        //         Ok(snapshots)
        //     }
        //     Err(e) => {
        //         warn!("Failed to retrieve browser snapshots: {}", e);
        //         Ok(vec![])
        //     }
        // }
    }

    async fn get_metadata(&mut self) -> ActivityResult<StrategyMetadata> {
        debug!("Retrieving metadata for browser strategy");

        let response_frame = self.send_request("request", "GET_METADATA").await?;

        if !response_frame.ok {
            warn!("Failed to retrieve metadata: request failed");
            return Ok(StrategyMetadata::default());
        }

        let Some(payload) = response_frame.payload else {
            warn!("No payload in metadata response");
            return Ok(StrategyMetadata::default());
        };

        let native_metadata = serde_json::from_str::<NativeMessage>(&payload.content)
            .map_err(|e| -> ActivityError { ActivityError::from(e) })?;

        let metadata = match native_metadata {
            NativeMessage::NativeMetadata(metadata) => {
                // Validate URL if present
                if let Some(ref url) = metadata.url
                    && !url.starts_with("http")
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
    async fn send_request(&self, kind: &str, action: &str) -> ActivityResult<Frame> {
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
        let request_frame = Frame {
            kind: kind.to_string(),
            id: request_id,
            action: action.to_string(),
            event: String::new(),
            payload: None,
            ok: true,
        };

        debug!(
            "Sending request frame: kind={}, id={}, action={}",
            kind, request_id, action
        );

        stream_tx
            .send(request_frame)
            .map_err(|_| ActivityError::invalid_data("Failed to send request frame"))?;

        // Wait for response with timeout
        match tokio::time::timeout(std::time::Duration::from_secs(5), rx).await {
            Ok(Ok(response)) => {
                debug!("Received response for request {}", request_id);
                Ok(response)
            }
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
