//! Browser strategy implementation for the refactored activity system

use std::sync::Arc;

pub use super::ActivityStrategyFunctionality;
pub use super::processes::*;
pub use super::{ActivityStrategy, StrategySupport};
use crate::utils::convert_svg_to_rgba;
use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use eur_native_messaging::{Channel, NativeMessage, TauriIpcClient, create_grpc_ipc_client};
use eur_proto::ipc::MessageRequest;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, warn};

use crate::strategies::StrategyMetadata;
use eur_native_messaging::NativeIcon;

use crate::{
    ActivityError,
    error::ActivityResult,
    types::{ActivityAsset, ActivitySnapshot},
};

/// Browser strategy for collecting web browser activity data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserStrategy {
    #[serde(skip)]
    client: Option<Arc<Mutex<TauriIpcClient<Channel>>>>,
}

impl BrowserStrategy {
    /// Create a new browser strategy
    pub async fn new() -> ActivityResult<Self> {
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

        Ok(Self { client })
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
        debug!("Retrieving metadata for browser strategy");

        let Some(client) = &self.client else {
            warn!("No IPC client available for browser strategy trying to retrieve metadata");
            return Ok(StrategyMetadata::default());
        };

        let mut client_guard = client.lock().await;
        let request = MessageRequest {};

        match client_guard.get_metadata(request).await {
            Ok(response) => {
                debug!("Received metadata response from browser extension");

                let resp = response.into_inner();

                let native_metadata = serde_json::from_slice::<NativeMessage>(&resp.content)
                    .map_err(|e| -> ActivityError { ActivityError::from(e) })?;

                let metadata = match native_metadata {
                    NativeMessage::NativeMetadata(metadata) => {
                        let metadata = StrategyMetadata::from(metadata);
                        metadata
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

    async fn get_icon(&mut self) -> Option<image::RgbaImage> {
        match self._get_icon().await {
            Ok(icon) => {
                let icon_url = icon.base64;
                if let Some(icon) = icon_url {
                    match icon.starts_with("data:image/svg+xml;base64") {
                        true => convert_svg_to_rgba(&icon).ok(),
                        false => {
                            let icon = icon.split(',').nth(1).unwrap_or(&icon);
                            let icon_data = BASE64_STANDARD.decode(icon.trim()).ok();

                            let icon_image =
                                image::load_from_memory(&icon_data.unwrap_or_default()).unwrap();

                            Some(icon_image.to_rgba8())
                        }
                    }

                    // let b64 = icon
                    //     .strip_prefix("data:image/svg+xml;base64,")
                    //     .unwrap_or(&icon);
                    // let svg_bytes = BASE64_STANDARD.decode(b64).ok();
                    // if svg_bytes.is_none() {
                    //     return None;
                    // }
                    // let svg_bytes = svg_bytes.unwrap();
                    // let mut opt = Options::default();
                    // opt.fontdb_mut().load_system_fonts();

                    // let tree = Tree::from_data(&svg_bytes, &opt).unwrap();
                    // let mut pixmap = Pixmap::new(
                    //     opt.default_size.width() as u32,
                    //     opt.default_size.height() as u32,
                    // )
                    // .unwrap();
                    // render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
                    // let img =
                    //     ImageBuffer::<Rgba<u8>, _>::from_raw(100, 100, pixmap.data().to_vec())
                    //         .ok_or("Failed to create image buffer")
                    //         .unwrap();
                    // Some(img)

                    // let icon_data = BASE64_STANDARD.decode(icon.trim()).ok();

                    // let icon_image =
                    //     image::load_from_memory(&icon_data.unwrap_or_default()).unwrap();

                    // Some(icon_image.to_rgba8())
                } else {
                    None
                }
            }
            Err(e) => {
                warn!("Failed to retrieve metadata from browser: {}", e);

                None
            }
        }
    }
}

impl BrowserStrategy {
    async fn _get_icon(&mut self) -> ActivityResult<NativeIcon> {
        debug!("Retrieving metadata for browser strategy");

        let Some(client) = &self.client else {
            warn!("No IPC client available for browser strategy trying to retrieve metadata");
            return Ok(NativeIcon::default());
        };

        let mut client_guard = client.lock().await;
        let request = MessageRequest {};

        match client_guard.get_icon(request).await {
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
}
