//! Asset storage functionality for saving activity assets to disk

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use euro_encrypt::{MainKey, encrypt_file_contents};
use euro_fs::create_dirs_then_write;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::debug;

use crate::{Activity, ActivityAsset, ActivityError, AssetFunctionality, error::ActivityResult};

/// Configuration for asset storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityStorageConfig {
    /// Base directory for storing assets
    pub base_dir: PathBuf,
    /// Whether to use content hashing for deduplication
    pub use_content_hash: bool,
    /// Maximum file size in bytes (None for no limit)
    pub max_file_size: Option<u64>,
    /// Master key
    pub main_key: MainKey,
}

impl Default for ActivityStorageConfig {
    fn default() -> Self {
        let main_key = MainKey::new().expect("Failed to generate main key");
        Self {
            base_dir: dirs::data_dir().unwrap_or_else(|| PathBuf::from("./assets")),
            use_content_hash: true,
            max_file_size: Some(100 * 1024 * 1024), // 100MB default limit
            main_key,
        }
    }
}

/// Information about a saved asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedAssetInfo {
    /// Relative path to the saved file
    pub file_path: PathBuf,
    /// Absolute path to the saved file
    pub absolute_path: PathBuf,
    /// Content hash (if enabled)
    pub content_hash: Option<String>,
    /// File size in bytes
    pub file_size: u64,
    /// Timestamp when the asset was saved
    pub saved_at: chrono::DateTime<chrono::Utc>,
}

/// Trait for assets that can be saved to disk
#[async_trait]
#[enum_dispatch]
pub trait SaveableAsset {
    // async fn load(bytes: &[u8]) -> ActivityResult<Self>
    // where
    //     Self: Sized,
    // {
    // }

    /// Get the asset type for organizing files
    fn get_asset_type(&self) -> &'static str;

    /// Serialize the asset content for saving
    async fn serialize_content(&self) -> ActivityResult<Vec<u8>>;

    /// Get a unique identifier for the asset (used for filename)
    fn get_unique_id(&self) -> String;

    /// Get a human-readable name for the asset
    fn get_display_name(&self) -> String;
}

/// Asset storage manager
pub struct ActivityStorage {
    config: ActivityStorageConfig,
}

impl ActivityStorage {
    /// Create a new asset storage manager
    pub fn new(config: ActivityStorageConfig) -> Self {
        Self { config }
    }

    // /// Create with default configuration
    // pub fn with_base_dir<P: Into<PathBuf>>(base_dir: P) -> Self {
    //     let mut config = ActivityStorageConfig::default();
    //     config.base_dir = base_dir.into();
    //     Self::new(config)
    // }

    /// Retrieve asset by path
    pub async fn load_asset_from_path(&self, path: &Path) -> ActivityResult<ActivityAsset> {
        let asset =
            euro_encrypt::load_encrypted_file::<ActivityAsset>(&self.config.main_key, path).await?;
        Ok(asset)
    }

    /// Save all assets of an activity to service by ids
    pub async fn save_assets_to_service_by_ids(
        &self,
        activity: &Activity,
        ids: &[String],
    ) -> ActivityResult<Vec<SavedAssetInfo>> {
        let mut saved_assets = Vec::new();

        for asset in &activity.assets {
            if ids.contains(&asset.get_id().to_string()) {
                let saved_info = self.save_asset_to_service(asset).await?;
                saved_assets.push(saved_info);
            }
        }

        Ok(saved_assets)
    }

    /// Save all assets of an activity to disk by ids
    pub async fn save_assets_to_disk_by_ids(
        &self,
        activity: &Activity,
        ids: &[String],
    ) -> ActivityResult<Vec<SavedAssetInfo>> {
        let mut saved_assets = Vec::new();

        for asset in &activity.assets {
            if ids.contains(&asset.get_id().to_string()) {
                let saved_info = self.save_asset(asset).await?;
                saved_assets.push(saved_info);
            }
        }

        Ok(saved_assets)
    }

    /// Save all assets of an activity to disk
    pub async fn save_assets_to_disk(
        &self,
        activity: &Activity,
    ) -> ActivityResult<Vec<SavedAssetInfo>> {
        let mut saved_assets = Vec::new();

        for asset in &activity.assets {
            let saved_info = self.save_asset(asset).await?;
            saved_assets.push(saved_info);
        }

        Ok(saved_assets)
    }

    /// Save an asset to service
    pub async fn save_asset_to_service(
        &self,
        asset: &ActivityAsset,
    ) -> ActivityResult<SavedAssetInfo> {
        // Serialize the entire ActivityAsset enum, not just the individual asset
        let mut bytes = serde_json::to_vec(asset)?;

        bytes = encrypt_file_contents(&self.config.main_key, &bytes, asset.get_asset_type())
            .await
            .map_err(ActivityError::Encryption)?;

        // Make a placeholder filepath
        let file_path = self.generate_asset_path(asset, None)?;
        let absolute_path = self.config.base_dir.join(&file_path);
        let final_path = self.config.base_dir.join(&absolute_path);
        debug!("Saving asset to {}", final_path.display());
        create_dirs_then_write(&final_path, &bytes)?;

        Ok(SavedAssetInfo {
            file_path,
            absolute_path,
            content_hash: None,
            file_size: 0,
            saved_at: chrono::Utc::now(),
        })
    }

    /// Save an asset to disk
    pub async fn save_asset(&self, asset: &ActivityAsset) -> ActivityResult<SavedAssetInfo> {
        // Serialize the entire ActivityAsset enum, not just the individual asset
        let mut bytes = serde_json::to_vec(asset)?;

        bytes = encrypt_file_contents(&self.config.main_key, &bytes, asset.get_asset_type())
            .await
            .map_err(ActivityError::Encryption)?;

        // Make a placeholder filepath
        let file_path = self.generate_asset_path(asset, None)?;
        let absolute_path = self.config.base_dir.join(&file_path);
        let final_path = self.config.base_dir.join(&absolute_path);
        debug!("Saving asset to {}", final_path.display());
        create_dirs_then_write(&final_path, &bytes)?;

        Ok(SavedAssetInfo {
            file_path,
            absolute_path,
            content_hash: None,
            file_size: 0,
            saved_at: chrono::Utc::now(),
        })
    }

    /// Generate a file path for an asset
    fn generate_asset_path(
        &self,
        asset: &ActivityAsset,
        content_hash: Option<&str>,
    ) -> ActivityResult<PathBuf> {
        let mut path = PathBuf::new();
        path.push("assets");
        path.push(sanitize_filename(asset.get_asset_type()));

        let filename = if let Some(hash) = content_hash {
            // Use content hash for deduplication
            hash[..16].to_string()
        } else {
            // Use sanitized unique ID + sanitized display name
            // let _sanitized_name = sanitize_filename(&asset.get_display_name());
            sanitize_filename(&asset.get_unique_id()).to_string()
        };

        path.push(filename);
        Ok(path)
    }

    /// Get the storage configuration
    pub fn get_config(&self) -> &ActivityStorageConfig {
        &self.config
    }

    /// Check if an asset exists in storage
    pub async fn asset_exists(&self, file_path: &Path) -> bool {
        (fs::try_exists(self.config.base_dir.join(file_path)).await).unwrap_or_default()
    }

    /// Get the absolute path for a relative asset path
    pub fn get_absolute_path(&self, file_path: &Path) -> PathBuf {
        self.config.base_dir.join(file_path)
    }
}

/// Sanitize a filename by removing/replacing invalid characters
fn sanitize_filename(name: &str) -> String {
    // Replace reserved and control characters
    let invalid_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    let mut sanitized: String = name
        .chars()
        .map(|c| if c.is_control() { '_' } else { c })
        .collect();
    for ch in invalid_chars {
        sanitized = sanitized.replace(ch, "_");
    }

    // Collapse whitespace to single spaces
    sanitized = sanitized.split_whitespace().collect::<Vec<_>>().join(" ");

    // Trim leading/trailing dots and spaces
    sanitized = sanitized
        .trim_matches(|c: char| c == '.' || c == ' ')
        .to_string();

    // Limit length to avoid filesystem issues
    if sanitized.chars().count() > 100 {
        sanitized = sanitized.chars().take(100).collect();
    }

    // Fallback to a default name if empty
    if sanitized.trim().is_empty() {
        "unnamed".to_string()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock asset for testing
    #[allow(dead_code)]
    struct MockAsset {
        id: String,
        name: String,
        content: String,
    }

    #[async_trait]
    impl SaveableAsset for MockAsset {
        fn get_asset_type(&self) -> &'static str {
            "mock"
        }

        async fn serialize_content(&self) -> ActivityResult<Vec<u8>> {
            Ok(self.content.as_bytes().to_vec())
        }

        fn get_unique_id(&self) -> String {
            self.id.clone()
        }

        fn get_display_name(&self) -> String {
            self.name.clone()
        }
    }

    #[tokio::test]
    async fn test_filename_sanitization() {
        let invalid_name = "Test/Asset\\With:Invalid*Characters?";
        let sanitized = sanitize_filename(invalid_name);
        assert_eq!(sanitized, "Test_Asset_With_Invalid_Characters_");
    }
}
