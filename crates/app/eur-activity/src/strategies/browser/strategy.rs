//! Browser strategy implementation for the refactored activity system

use super::server::{ensure_grpc_server_running, get_server};
pub use crate::strategies::ActivityStrategyFunctionality;
pub use crate::strategies::processes::*;
pub use crate::strategies::{ActivityStrategy, StrategySupport};
use async_trait::async_trait;
use eur_native_messaging::{Channel, NativeMessage, TauriIpcClient, create_grpc_ipc_client};
use eur_proto::ipc::MessageRequest;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::strategies::{ActivityReport, StrategyMetadata};
use eur_native_messaging::NativeIcon;

use crate::{
    Activity, ActivityError,
    error::ActivityResult,
    types::{ActivityAsset, ActivitySnapshot},
};

/// Browser strategy for collecting web browser activity data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserStrategy {
    #[serde(skip)]
    client: Option<TauriIpcClient<Channel>>,
    #[serde(skip)]
    tracking_handle: Option<Arc<tokio::task::JoinHandle<()>>>,
}

impl BrowserStrategy {
    /// Create a new browser strategy
    pub async fn new() -> ActivityResult<Self> {
        // Try to create the IPC client
        let client = match create_grpc_ipc_client().await {
            Ok(client) => {
                debug!("Successfully created IPC client for browser strategy");
                Some(client)
            }
            Err(e) => {
                warn!(
                    "Failed to create IPC client: {}. Browser strategy will work with limited functionality.",
                    e
                );
                None
            }
        };

        Ok(Self {
            client,
            tracking_handle: None,
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
        let process_name = focus_window.process_name.clone();
        let window_title = focus_window.window_title.clone();

        debug!("Browser strategy starting tracking for: {:?}", process_name);

        // Clone process_name before using it to avoid borrow issues
        // Ensure the gRPC server is running
        ensure_grpc_server_running().await;

        // Get the server and subscribe to activity events
        let server = get_server();
        let mut activity_receiver = server.activity_event_tx.subscribe();
        let default_icon = focus_window.icon.clone();

        let handle = tokio::spawn(async move {
            while let Ok(event) = activity_receiver.recv().await {
                info!("Received activity event: {:?}", event.url);
                let icon = match event.icon {
                    Some(icon) => {
                        image::RgbaImage::from_vec(icon.width as u32, icon.height as u32, icon.data)
                    }
                    None => default_icon.clone(),
                };
                let activity = Activity::new(event.url.clone(), icon, "".to_string(), vec![]);
                if sender.send(ActivityReport::NewActivity(activity)).is_err() {
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
            Ok(true)
        } else {
            debug!(
                "Browser strategy cannot handle: {}, need to switch",
                process_name
            );
            self.tracking_handle.take();
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

        Ok(())
    }

    /// Retrieve assets from the browser
    async fn retrieve_assets(&mut self) -> ActivityResult<Vec<ActivityAsset>> {
        debug!("Retrieving assets for browser strategy");

        let Some(ref client) = self.client else {
            warn!("No IPC client available for browser strategy");
            return Ok(vec![]);
        };

        let request = MessageRequest {};
        let mut client = client.clone();

        match client.get_assets(request).await {
            Ok(response) => {
                debug!("Received assets response from browser extension");
                let mut assets: Vec<ActivityAsset> = Vec::new();

                let resp = response.into_inner();

                let native_asset = serde_json::from_slice::<NativeMessage>(&resp.content)
                    .map_err(|e| -> ActivityError { ActivityError::from(e) })?;

                let asset =
                    ActivityAsset::try_from(native_asset).map_err(|e| -> ActivityError {
                        ActivityError::InvalidAssetType(e.to_string())
                    })?;

                assets.push(asset);

                debug!("Retrieved {} assets from browser", assets.len());
                Ok(assets)
            }
            Err(e) => {
                warn!("Failed to retrieve assets from browser: {}", e);
                Err(ActivityError::invalid_data(e.to_string()))
            }
        }
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

        let Some(ref client) = self.client else {
            warn!("No IPC client available for browser strategy trying to retrieve metadata");
            return Ok(StrategyMetadata::default());
        };

        let request = MessageRequest {};
        let mut client = client.clone();

        match client.get_metadata(request).await {
            Ok(response) => {
                debug!("Received metadata response from browser extension");

                let resp = response.into_inner();

                let native_metadata = serde_json::from_slice::<NativeMessage>(&resp.content)
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
            Err(e) => {
                warn!("Failed to retrieve metadata from browser: {}", e);

                Ok(StrategyMetadata::default())
            }
        }
    }
}

impl BrowserStrategy {
    async fn _get_icon(&mut self) -> ActivityResult<NativeIcon> {
        debug!("Retrieving metadata for browser strategy");

        let Some(ref client) = self.client else {
            warn!("No IPC client available for browser strategy trying to retrieve metadata");
            return Ok(NativeIcon::default());
        };

        let request = MessageRequest {};
        let mut client = client.clone();

        match client.get_icon(request).await {
            Ok(response) => {
                debug!("Received metadata response from browser extension");

                let resp = response.into_inner();

                let native_metadata = serde_json::from_slice::<NativeMessage>(&resp.content)
                    .map_err(|e| -> ActivityError { ActivityError::from(e) })?;

                let metadata = match native_metadata {
                    NativeMessage::NativeIcon(metadata) => metadata,
                    _ => NativeIcon::default(),
                };
                Ok(metadata)
            }
            Err(e) => {
                warn!("Failed to retrieve metadata from browser: {}", e);
                Ok(NativeIcon::default())
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
