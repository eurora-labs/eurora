//! Asset storage functionality for saving activity assets to disk

use crate::error::{ActivityError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// Configuration for asset storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Base directory for storing assets
    pub base_dir: PathBuf,
    /// Whether to organize assets by type (youtube/, article/, etc.)
    pub organize_by_type: bool,
    /// Whether to use content hashing for deduplication
    pub use_content_hash: bool,
    /// Maximum file size in bytes (None for no limit)
    pub max_file_size: Option<u64>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            base_dir: PathBuf::from("./assets"),
            organize_by_type: true,
            use_content_hash: true,
            max_file_size: Some(100 * 1024 * 1024), // 100MB default limit
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
    /// MIME type of the saved content
    pub mime_type: String,
    /// Timestamp when the asset was saved
    pub saved_at: chrono::DateTime<chrono::Utc>,
}

/// Trait for assets that can be saved to disk
#[async_trait]
// #[enum_dispatch(ActivityAsset)]
pub trait SaveableAsset {
    /// Get the asset type for organizing files
    fn get_asset_type(&self) -> &'static str;

    /// Get the preferred file extension
    fn get_file_extension(&self) -> &'static str;

    /// Get the MIME type of the content
    fn get_mime_type(&self) -> &'static str;

    /// Serialize the asset content for saving
    async fn serialize_content(&self) -> Result<Vec<u8>>;

    /// Get a unique identifier for the asset (used for filename)
    fn get_unique_id(&self) -> String;

    /// Get a human-readable name for the asset
    fn get_display_name(&self) -> String;
}

/// Asset storage manager
pub struct AssetStorage {
    config: StorageConfig,
}

impl AssetStorage {
    /// Create a new asset storage manager
    pub fn new(config: StorageConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    pub fn with_base_dir<P: Into<PathBuf>>(base_dir: P) -> Self {
        let mut config = StorageConfig::default();
        config.base_dir = base_dir.into();
        Self::new(config)
    }

    /// Save an asset to disk
    pub async fn save_asset<T: SaveableAsset>(&self, asset: &T) -> Result<SavedAssetInfo> {
        // Serialize the content
        let content = asset.serialize_content().await?;

        // Check file size limit
        if let Some(max_size) = self.config.max_file_size {
            if content.len() as u64 > max_size {
                return Err(ActivityError::invalid_data(format!(
                    "Asset content size ({} bytes) exceeds maximum allowed size ({} bytes)",
                    content.len(),
                    max_size
                )));
            }
        }

        // Generate content hash if enabled
        let content_hash = if self.config.use_content_hash {
            let mut hasher = Sha256::new();
            hasher.update(&content);
            Some(hex::encode(hasher.finalize()))
        } else {
            None
        };

        // Determine the file path
        let file_path = self.generate_file_path(asset, content_hash.as_deref())?;
        let absolute_path = self.config.base_dir.join(&file_path);

        // Create parent directories if they don't exist
        if let Some(parent) = absolute_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // // Check if file already exists (for deduplication)
        // if absolute_path.exists() && self.config.use_content_hash {
        //     // File already exists, return existing info
        //     let metadata = fs::metadata(&absolute_path).await?;
        //     return Ok(SavedAssetInfo {
        //         file_path,
        //         absolute_path,
        //         content_hash,
        //         file_size: metadata.len(),
        //         mime_type: asset.get_mime_type().to_string(),
        //         saved_at: chrono::Utc::now(),
        //     });
        // }

        // // Write the content to file
        // let mut file = fs::File::create(&absolute_path).await?;
        // file.write_all(&content).await?;
        // file.flush().await?;

        // Write the content to file (race-safe when hashing)
        let mut open_opts = fs::OpenOptions::new();
        if self.config.use_content_hash {
            // Avoid clobbering and make dedup robust under concurrency
            open_opts.create_new(true).write(true);
            match open_opts.open(&absolute_path).await {
                Ok(mut file) => {
                    file.write_all(&content).await?;
                    file.flush().await?;
                    file.sync_all().await?;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    // File already exists, no need to create a new one
                    let metadata = fs::metadata(&absolute_path).await?;
                    return Ok(SavedAssetInfo {
                        file_path,
                        absolute_path,
                        content_hash,
                        file_size: metadata.len(),
                        mime_type: asset.get_mime_type().to_string(),
                        saved_at: chrono::Utc::now(),
                    });
                }
                Err(e) => return Err(e.into()),
            }
        } else {
            // Non-hash mode: allow overwrite
            use std::ffi::OsStr;
            let parent = absolute_path.parent().unwrap_or_else(|| Path::new("."));
            let tmp_name = absolute_path
                .file_name()
                .and_then(OsStr::to_str)
                .map(|n| format!(".{}.tmp", n))
                .unwrap_or_else(|| ".tmpfile".to_string());
            let tmp_path = parent.join(tmp_name);
            open_opts.create(true).write(true).truncate(true);
            let mut file = open_opts.open(&tmp_path).await?;
            file.write_all(&content).await?;
            file.flush().await?;
            file.sync_all().await?;
            // Persist atomically
            fs::rename(&tmp_path, &absolute_path).await?;
        }

        Ok(SavedAssetInfo {
            file_path,
            absolute_path,
            content_hash,
            file_size: content.len() as u64,
            mime_type: asset.get_mime_type().to_string(),
            saved_at: chrono::Utc::now(),
        })
    }

    /// Generate a file path for an asset
    fn generate_file_path<T: SaveableAsset>(
        &self,
        asset: &T,
        content_hash: Option<&str>,
    ) -> Result<PathBuf> {
        let mut path = PathBuf::new();

        if self.config.organize_by_type {
            path.push(sanitize_filename(asset.get_asset_type()));
        }

        let mut ext = asset
            .get_file_extension()
            .trim_matches(|c| c == '.' || c == '/' || c == '\\')
            .to_ascii_lowercase();

        if !ext.chars().all(|c| c.is_ascii_alphanumeric()) {
            ext = "bin".to_string();
        }

        let filename = if let Some(hash) = content_hash {
            // Use content hash for deduplication
            format!("{}.{}", &hash[..16], ext)
        } else {
            // Use sanitized unique ID + sanitized display name
            let sanitized_name = sanitize_filename(&asset.get_display_name());
            let sanitized_id = sanitize_filename(&asset.get_unique_id());
            format!("{}_{}.{}", sanitized_id, sanitized_name, ext)
        };

        path.push(filename);
        Ok(path)
    }

    /// Get the storage configuration
    pub fn get_config(&self) -> &StorageConfig {
        &self.config
    }

    /// Check if an asset exists in storage
    pub async fn asset_exists(&self, file_path: &Path) -> bool {
        match fs::try_exists(self.config.base_dir.join(file_path)).await {
            Ok(v) => v,
            Err(_) => false,
        }
    }

    /// Get the absolute path for a relative asset path
    pub fn get_absolute_path(&self, file_path: &Path) -> PathBuf {
        self.config.base_dir.join(file_path)
    }
}

/// Sanitize a filename by removing/replacing invalid characters
fn sanitize_filename(name: &str) -> String {
    // // Replace invalid characters with underscores
    // let invalid_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    // let mut sanitized = name.to_string();

    // for invalid_char in invalid_chars {
    //     sanitized = sanitized.replace(invalid_char, "_");
    // }

    // // Limit length to avoid filesystem issues
    // if sanitized.len() > 100 {
    //     sanitized.truncate(100);
    // }

    // // Ensure it's not empty
    // if sanitized.trim().is_empty() {
    //     sanitized = "unnamed".to_string();
    // }

    // sanitized
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
    use tempfile::TempDir;

    // Mock asset for testing
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

        fn get_file_extension(&self) -> &'static str {
            "txt"
        }

        fn get_mime_type(&self) -> &'static str {
            "text/plain"
        }

        async fn serialize_content(&self) -> Result<Vec<u8>> {
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
    async fn test_asset_storage_basic() {
        let temp_dir = TempDir::new().unwrap();
        let storage = AssetStorage::with_base_dir(temp_dir.path());

        let asset = MockAsset {
            id: "test-123".to_string(),
            name: "Test Asset".to_string(),
            content: "Hello, World!".to_string(),
        };

        let saved_info = storage.save_asset(&asset).await.unwrap();

        assert!(saved_info.absolute_path.exists());
        assert_eq!(saved_info.file_size, 13); // "Hello, World!" length
        assert_eq!(saved_info.mime_type, "text/plain");
        assert!(saved_info.content_hash.is_some());
    }

    #[tokio::test]
    async fn test_filename_sanitization() {
        let invalid_name = "Test/Asset\\With:Invalid*Characters?";
        let sanitized = sanitize_filename(invalid_name);
        assert_eq!(sanitized, "Test_Asset_With_Invalid_Characters_");
    }

    #[tokio::test]
    async fn test_content_deduplication() {
        let temp_dir = TempDir::new().unwrap();
        let storage = AssetStorage::with_base_dir(temp_dir.path());

        let asset1 = MockAsset {
            id: "test-1".to_string(),
            name: "Asset 1".to_string(),
            content: "Same content".to_string(),
        };

        let asset2 = MockAsset {
            id: "test-2".to_string(),
            name: "Asset 2".to_string(),
            content: "Same content".to_string(),
        };

        let saved_info1 = storage.save_asset(&asset1).await.unwrap();
        let saved_info2 = storage.save_asset(&asset2).await.unwrap();

        // Should have the same file path due to content deduplication
        assert_eq!(saved_info1.file_path, saved_info2.file_path);
        assert_eq!(saved_info1.content_hash, saved_info2.content_hash);
    }

    #[test]
    fn test_storage_config_default() {
        let config = StorageConfig::default();
        assert_eq!(config.base_dir, PathBuf::from("./assets"));
        assert!(config.organize_by_type);
        assert!(config.use_content_hash);
        assert_eq!(config.max_file_size, Some(100 * 1024 * 1024));
    }

    #[tokio::test]
    async fn test_write_non_hash_mode() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cfg = StorageConfig {
            base_dir: tmp.path().to_path_buf(),
            use_content_hash: false,
            ..Default::default()
        };
        let storage = AssetStorage::new(cfg);
        let asset = MockAsset {
            id: "x".into(),
            name: "n".into(),
            content: "abc".into(),
        };
        let info = storage.save_asset(&asset).await.unwrap();
        assert!(info.absolute_path.exists());
    }
}
