//! Browser strategy implementation for the refactored activity system

use std::sync::Arc;

pub use super::ActivityStrategyFunctionality;
pub use super::processes::*;
pub use super::{ActivityStrategy, StrategySupport};
use async_trait::async_trait;
use eur_native_messaging::{Channel, NativeMessage, TauriIpcClient, create_grpc_ipc_client};
use eur_proto::ipc::MessageRequest;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, warn};

use crate::strategies::StrategyMetadata;
use crate::{
    ActivityError,
    error::ActivityResult,
    types::{ActivityAsset, ActivitySnapshot},
};

/// Browser strategy for collecting web browser activity data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserStrategy {
    pub name: String,
    pub icon: String,
    pub process_name: String,
    #[serde(skip)]
    client: Option<Arc<Mutex<TauriIpcClient<Channel>>>>,
}

impl BrowserStrategy {
    /// Create a new browser strategy
    pub async fn new(name: String, icon: String, process_name: String) -> ActivityResult<Self> {
        debug!("Creating BrowserStrategy for process: {}", process_name);

        // Try to create the IPC client
        let client = match create_grpc_ipc_client().await {
            Ok(client) => {
                debug!("Successfully created IPC client for browser strategy");
                Some(Arc::new(Mutex::new(client)))
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
            name,
            icon,
            process_name,
            client,
        })
    }
}

#[async_trait]
impl StrategySupport for BrowserStrategy {
    fn get_supported_processes() -> Vec<&'static str> {
        vec![Librewolf.get_name(), Firefox.get_name(), Chrome.get_name()]
    }

    async fn create_strategy(
        process_name: String,
        display_name: String,
        icon: String,
    ) -> ActivityResult<ActivityStrategy> {
        let strategy = Self::new(display_name, icon, process_name).await?;
        Ok(ActivityStrategy::BrowserStrategy(strategy))
    }
}

#[async_trait]
impl ActivityStrategyFunctionality for BrowserStrategy {
    /// Retrieve assets from the browser
    async fn retrieve_assets(&mut self) -> ActivityResult<Vec<ActivityAsset>> {
        debug!("Retrieving assets for browser strategy");

        let Some(client) = &self.client else {
            warn!("No IPC client available for browser strategy");
            return Ok(vec![]);
        };

        let mut client_guard = client.lock().await;
        let request = MessageRequest {};

        match client_guard.get_assets(request).await {
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
                Ok(vec![])
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
        Ok(StrategyMetadata::default())
    }

    /// Gather current state as string
    fn gather_state(&self) -> String {
        format!("Browser: {} ({})", self.name, self.process_name)
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_icon(&self) -> &str {
        &self.icon
    }

    fn get_process_name(&self) -> &str {
        &self.process_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategies::ActivityStrategyFunctionality;

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

    #[tokio::test]
    async fn test_browser_strategy_creation() {
        let strategy = BrowserStrategy::new(
            "Firefox".to_string(),
            "firefox-icon".to_string(),
            "firefox".to_string(),
        )
        .await;

        // Should succeed even if IPC client creation fails
        assert!(strategy.is_ok());

        let strategy = strategy.unwrap();
        assert_eq!(strategy.name, "Firefox");
        assert_eq!(strategy.icon, "firefox-icon");
        assert_eq!(strategy.process_name, "firefox");
    }

    #[tokio::test]
    async fn test_strategy_support_creation() {
        let result = BrowserStrategy::create_strategy(
            "firefox".to_string(),
            "Firefox Browser".to_string(),
            "firefox-icon".to_string(),
        )
        .await;

        assert!(result.is_ok());
        let strategy = result.unwrap();
        assert_eq!(strategy.get_name(), "Firefox Browser");
        assert_eq!(strategy.get_process_name(), "firefox");
    }

    #[test]
    fn test_gather_state() {
        let strategy = BrowserStrategy {
            name: "Firefox".to_string(),
            icon: "firefox-icon".to_string(),
            process_name: "firefox".to_string(),
            client: None,
        };

        let state = strategy.gather_state();
        assert_eq!(state, "Browser: Firefox (firefox)");
    }
}
