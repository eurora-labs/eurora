//! Browser strategy implementation for the refactored activity system

pub use crate::strategies::ActivityStrategyFunctionality;
pub use crate::strategies::processes::*;
pub use crate::strategies::{ActivityStrategy, StrategySupport};
use async_trait::async_trait;
use eur_native_messaging::{Channel, NativeMessage, TauriIpcClient, create_grpc_ipc_client};
use eur_proto::ipc::MessageRequest;
use serde::{Deserialize, Serialize};
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tonic::transport::Server;
use tracing::{debug, info, warn};

use crate::strategies::StrategyMetadata;
use eur_native_messaging::NativeIcon;

use crate::{
    ActivityError,
    error::ActivityResult,
    types::{ActivityAsset, ActivitySnapshot},
};

/// Global singleton for the gRPC server
static GRPC_SERVER: OnceCell<Arc<tokio::task::JoinHandle<()>>> = OnceCell::const_new();

/// Browser strategy for collecting web browser activity data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserStrategy {
    #[serde(skip)]
    client: Option<TauriIpcClient<Channel>>,
}

impl BrowserStrategy {
    /// Create a new browser strategy
    pub async fn new() -> ActivityResult<Self> {
        // Initialize the gRPC server exactly once across all strategy instances
        Self::ensure_grpc_server_running().await;

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

        Ok(Self { client })
    }

    /// Ensures the gRPC server is running, initializing it only once
    async fn ensure_grpc_server_running() {
        GRPC_SERVER
            .get_or_init(|| async {
                info!("Initializing persistent gRPC server on port {}", super::server::PORT);

                let handle = tokio::spawn(async {
                    // Create a server handler that doesn't depend on any specific strategy instance
                    let service = GrpcServiceHandler;

                    let addr = format!("[::1]:{}", super::server::PORT)
                        .to_socket_addrs()
                        .expect("Failed to parse gRPC server address")
                        .next()
                        .expect("Failed to resolve gRPC server address");

                    info!("Starting gRPC server at {}", addr);

                    if let Err(e) = Server::builder()
                        .add_service(
                            eur_proto::nm_ipc::native_messaging_ipc_server::NativeMessagingIpcServer::new(
                                service,
                            ),
                        )
                        .serve(addr)
                        .await
                    {
                        warn!("gRPC server terminated with error: {}", e);
                    }
                });

                Arc::new(handle)
            })
            .await;

        debug!("gRPC server is running");
    }
}

/// Stateless handler for gRPC requests
/// This decouples the server from individual BrowserStrategy instances
#[derive(Debug, Clone)]
struct GrpcServiceHandler;

#[tonic::async_trait]
impl eur_proto::nm_ipc::native_messaging_ipc_server::NativeMessagingIpc for GrpcServiceHandler {
    async fn switch_activity(
        &self,
        request: tonic::Request<eur_proto::nm_ipc::SwitchActivityRequest>,
    ) -> Result<tonic::Response<eur_proto::nm_ipc::SwitchActivityResponse>, tonic::Status> {
        use tonic::Status;

        info!("Received switch activity request via persistent gRPC server");
        let req = request.into_inner();

        // Validate the URL is not empty
        if req.url.is_empty() {
            return Err(Status::invalid_argument("URL cannot be empty"));
        }

        // TODO: Implement actual activity switching logic here
        // This is a placeholder implementation that just acknowledges the request
        // In a production system, you'd emit events to a channel or call a handler

        info!("Processing activity switch for URL: {}", req.url);

        // Return success response
        Ok(tonic::Response::new(
            eur_proto::nm_ipc::SwitchActivityResponse {},
        ))
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

    async fn close_strategy(&mut self) -> ActivityResult<()> {
        Ok(())
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
