//! Browser strategy implementation for the refactored activity system

use std::sync::Arc;

use eur_native_messaging::{Channel, NativeMessage, TauriIpcClient, create_grpc_ipc_client};
use eur_proto::ipc::MessageRequest;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, warn};

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

    /// Get list of supported browser processes
    pub fn get_supported_processes() -> Vec<&'static str> {
        #[cfg(target_os = "windows")]
        let processes = vec![
            "firefox.exe",
            "chrome.exe",
            "msedge.exe",
            "brave.exe",
            "opera.exe",
            "vivaldi.exe",
            "librewolf.exe",
        ];

        #[cfg(target_os = "macos")]
        let processes = vec![
            "Firefox",
            "Google Chrome",
            "Microsoft Edge",
            "Brave Browser",
            "Opera",
            "Vivaldi",
            "Safari",
            "LibreWolf",
        ];

        #[cfg(target_os = "linux")]
        let processes = vec![
            "firefox",
            "chrome",
            "chromium",
            "brave",
            "opera",
            "vivaldi",
            "librewolf",
        ];

        processes
    }

    /// Retrieve assets from the browser
    pub async fn retrieve_assets(&mut self) -> ActivityResult<Vec<ActivityAsset>> {
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
    pub async fn retrieve_snapshots(&mut self) -> ActivityResult<Vec<ActivitySnapshot>> {
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

    /// Gather current state as string
    pub fn gather_state(&self) -> String {
        format!("Browser: {} ({})", self.name, self.process_name)
    }
}

/// Browser strategy factory for creating browser strategy instances
pub struct BrowserStrategyFactory;

impl BrowserStrategyFactory {
    pub fn new() -> Self {
        Self
    }
}

use async_trait::async_trait;

use crate::{
    registry::{MatchScore, ProcessContext, StrategyCategory, StrategyFactory, StrategyMetadata},
    strategies::ActivityStrategy,
};

#[async_trait]
impl StrategyFactory for BrowserStrategyFactory {
    async fn create_strategy(&self, context: &ProcessContext) -> ActivityResult<ActivityStrategy> {
        let strategy = BrowserStrategy::new(
            context.display_name.clone(),
            "browser-icon".to_string(),
            context.process_name.clone(),
        )
        .await?;

        Ok(ActivityStrategy::Browser(strategy))
    }

    fn supports_process(&self, process_name: &str, _window_title: Option<&str>) -> MatchScore {
        let supported_processes = BrowserStrategy::get_supported_processes();

        for supported in &supported_processes {
            if process_name.eq_ignore_ascii_case(supported) {
                return MatchScore::PERFECT;
            }

            // Check for partial matches (e.g., "firefox" matches "firefox.exe")
            if supported
                .to_lowercase()
                .contains(&process_name.to_lowercase())
                || process_name
                    .to_lowercase()
                    .contains(&supported.to_lowercase())
            {
                return MatchScore::HIGH;
            }
        }

        // Check for common browser keywords
        let browser_keywords = [
            "firefox", "chrome", "edge", "brave", "opera", "vivaldi", "safari",
        ];
        for keyword in &browser_keywords {
            if process_name.to_lowercase().contains(keyword) {
                return MatchScore::MEDIUM;
            }
        }

        MatchScore::NO_MATCH
    }

    fn get_metadata(&self) -> StrategyMetadata {
        StrategyMetadata {
            id: "browser".to_string(),
            name: "Browser Strategy".to_string(),
            version: "2.0.0".to_string(),
            description: "Collects activity data from web browsers including YouTube, articles, and social media".to_string(),
            supported_processes: BrowserStrategy::get_supported_processes()
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            category: StrategyCategory::Browser,
        }
    }
}

#[cfg(test)]
mod tests {
    use ferrous_focus::IconData;

    use super::*;

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

    #[test]
    fn test_factory_process_matching() {
        let factory = BrowserStrategyFactory::new();

        // Test perfect matches
        assert_eq!(
            factory.supports_process("firefox", None),
            MatchScore::PERFECT
        );
        assert_eq!(
            factory.supports_process("chrome", None),
            MatchScore::PERFECT
        );

        // Test partial matches
        assert!(factory.supports_process("firefox-dev", None).is_match());
        assert!(factory.supports_process("google-chrome", None).is_match());

        // Test no match
        assert_eq!(
            factory.supports_process("notepad", None),
            MatchScore::NO_MATCH
        );
    }

    #[test]
    fn test_factory_metadata() {
        let factory = BrowserStrategyFactory::new();
        let metadata = factory.get_metadata();

        assert_eq!(metadata.id, "browser");
        assert_eq!(metadata.name, "Browser Strategy");
        assert_eq!(metadata.version, "2.0.0");
        assert_eq!(metadata.category, StrategyCategory::Browser);
        assert!(!metadata.supported_processes.is_empty());
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
    async fn test_factory_strategy_creation() {
        let factory = BrowserStrategyFactory::new();
        let context = ProcessContext::new(
            "firefox".to_string(),
            "Firefox Browser".to_string(),
            IconData::default(),
        );

        let result = factory.create_strategy(&context).await;
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
