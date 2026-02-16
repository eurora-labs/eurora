//! Storage backend implementation using OpenDAL.
//!
//! This module provides a unified storage interface that can use either
//! local filesystem or S3 as the backend, configurable via environment variables.
//!
//! ## Environment Variables
//!
//! - `ASSET_STORAGE_BACKEND`: Either "fs" (default) or "s3"
//!
//! ### For filesystem backend:
//! - `ASSET_STORAGE_FS_ROOT`: Root directory for file storage (default: "./assets")
//!
//! ### For S3 backend:
//! - `ASSET_STORAGE_S3_BUCKET`: S3 bucket name (required)
//! - `ASSET_STORAGE_S3_REGION`: S3 region (required)
//! - `ASSET_STORAGE_S3_ENDPOINT`: S3 endpoint URL (optional, for S3-compatible services)
//! - `ASSET_STORAGE_S3_ACCESS_KEY_ID`: AWS access key ID (optional, uses default credentials if not set)
//! - `ASSET_STORAGE_S3_SECRET_ACCESS_KEY`: AWS secret access key (optional)

mod error;

pub use error::{StorageError, StorageResult};

use std::sync::Arc;

use bon::bon;
use opendal::{Operator, services};
use sha2::{Digest, Sha256};
use tracing::{debug, info};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum StorageConfig {
    FS {
        root: String,
    },
    S3 {
        bucket: String,
        region: String,
        endpoint: Option<String>,
        access_key_id: Option<String>,
        secret_access_key: Option<String>,
    },
}

impl Default for StorageConfig {
    fn default() -> Self {
        StorageConfig::FS {
            root: "./assets".to_string(),
        }
    }
}

impl StorageConfig {
    /// Create configuration from environment variables
    ///
    /// # Errors
    ///
    /// Returns `StorageError::MissingEnvVar` if required environment variables
    /// are not set when using S3 backend.
    pub fn from_env() -> StorageResult<Self> {
        let backend = std::env::var("ASSET_STORAGE_BACKEND")
            .unwrap_or_else(|_| "fs".to_string())
            .to_lowercase();

        match backend.as_str() {
            "s3" => {
                let bucket = std::env::var("ASSET_STORAGE_S3_BUCKET")
                    .map_err(|_| StorageError::missing_env_var("ASSET_STORAGE_S3_BUCKET"))?;
                let region = std::env::var("ASSET_STORAGE_S3_REGION")
                    .map_err(|_| StorageError::missing_env_var("ASSET_STORAGE_S3_REGION"))?;
                let endpoint = std::env::var("ASSET_STORAGE_S3_ENDPOINT").ok();
                let access_key_id = std::env::var("ASSET_STORAGE_S3_ACCESS_KEY_ID").ok();
                let secret_access_key = std::env::var("ASSET_STORAGE_S3_SECRET_ACCESS_KEY").ok();

                Ok(StorageConfig::S3 {
                    bucket,
                    region,
                    endpoint,
                    access_key_id,
                    secret_access_key,
                })
            }
            _ => {
                let root = std::env::var("ASSET_STORAGE_FS_ROOT")
                    .unwrap_or_else(|_| "./assets".to_string());
                Ok(StorageConfig::FS { root })
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct StorageService {
    operator: Operator,
    config: StorageConfig,
    #[cfg(feature = "encryption")]
    encryption_key: Arc<std::sync::RwLock<Option<be_encrypt::MainKey>>>,
}

#[bon]
impl StorageService {
    /// Create a new storage service with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the storage operator cannot be created.
    #[builder]
    pub fn new(
        config: StorageConfig,
        #[cfg(feature = "encryption")] encryption_key: Option<be_encrypt::MainKey>,
    ) -> StorageResult<Self> {
        let operator = Self::create_operator(&config)?;
        Ok(Self {
            operator,
            config,
            #[cfg(feature = "encryption")]
            encryption_key: Arc::new(std::sync::RwLock::new(encryption_key)),
        })
    }

    /// Set the encryption key at runtime. Subsequent uploads will be encrypted
    /// and downloads of encrypted assets will be decrypted.
    #[cfg(feature = "encryption")]
    pub fn set_encryption_key(&self, key: be_encrypt::MainKey) {
        *self
            .encryption_key
            .write()
            .expect("encryption key lock poisoned") = Some(key);
    }

    /// Create a new storage service using environment variables for configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration is invalid or the operator cannot be created.
    pub fn from_env() -> StorageResult<Self> {
        let config = StorageConfig::from_env()?;
        info!("Initializing storage service with config: {:?}", config);
        Self::builder().config(config).build()
    }

    fn create_operator(config: &StorageConfig) -> StorageResult<Operator> {
        match config {
            StorageConfig::FS { root } => {
                debug!("Creating filesystem storage operator with root: {}", root);

                std::fs::create_dir_all(root)?;

                let builder = services::Fs::default().root(root);

                Ok(Operator::new(builder)?.finish())
            }
            StorageConfig::S3 {
                bucket,
                region,
                endpoint,
                access_key_id,
                secret_access_key,
            } => {
                debug!("Creating S3 storage operator for bucket: {}", bucket);

                let mut builder = services::S3::default().bucket(bucket).region(region);

                if let Some(ep) = endpoint {
                    builder = builder.endpoint(ep);
                }

                if let Some(key_id) = access_key_id {
                    builder = builder.access_key_id(key_id);
                }

                if let Some(secret) = secret_access_key {
                    builder = builder.secret_access_key(secret);
                }

                Ok(Operator::new(builder)?.finish())
            }
        }
    }

    pub fn calculate_sha256(content: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(content);
        hasher.finalize().to_vec()
    }

    pub fn generate_path(user_id: &Uuid, asset_id: &Uuid, extension: Option<&str>) -> String {
        let ext = extension.unwrap_or("bin");
        format!("{}/{}.{}", user_id, asset_id, ext)
    }

    pub fn extension_from_mime(mime_type: &str) -> &'static str {
        match mime_type {
            "image/png" => "png",
            "image/jpeg" | "image/jpg" => "jpg",
            "image/gif" => "gif",
            "image/webp" => "webp",
            "image/svg+xml" => "svg",
            "image/bmp" => "bmp",
            "image/tiff" => "tiff",
            "application/pdf" => "pdf",
            "application/json" => "json",
            "text/plain" => "txt",
            "text/html" => "html",
            "text/css" => "css",
            "text/javascript" | "application/javascript" => "js",
            "application/xml" | "text/xml" => "xml",
            "video/mp4" => "mp4",
            "video/webm" => "webm",
            "audio/mpeg" => "mp3",
            "audio/wav" => "wav",
            "audio/ogg" => "ogg",
            "application/zip" => "zip",
            "application/gzip" => "gz",
            "application/x-tar" => "tar",
            _ => "bin",
        }
    }

    /// Upload content to storage and return the path
    ///
    /// # Errors
    ///
    /// Returns an error if the upload operation fails.
    pub async fn upload(
        &self,
        user_id: &Uuid,
        asset_id: &Uuid,
        content: &[u8],
        mime_type: &str,
    ) -> StorageResult<String> {
        let extension = Self::extension_from_mime(mime_type);
        let path = Self::generate_path(user_id, asset_id, Some(extension));

        debug!(
            "Uploading asset {} for user {} to path: {} ({} bytes)",
            asset_id,
            user_id,
            path,
            content.len()
        );

        #[cfg(feature = "encryption")]
        let content = {
            let key_guard = self
                .encryption_key
                .read()
                .expect("encryption key lock poisoned");
            match key_guard.as_ref() {
                Some(key) => {
                    let encrypted = be_encrypt::encrypt(key, content, "asset").map_err(|e| {
                        StorageError::Encryption(format!("Failed to encrypt asset: {}", e))
                    })?;
                    debug!(
                        "Encrypted asset {} ({} -> {} bytes)",
                        asset_id,
                        content.len(),
                        encrypted.len()
                    );
                    encrypted
                }
                None => content.to_vec(),
            }
        };

        #[cfg(not(feature = "encryption"))]
        let content = content.to_vec();

        self.operator.write(&path, content).await?;

        info!("Successfully uploaded asset {}", asset_id);

        Ok(path)
    }

    /// Download content from storage
    ///
    /// # Errors
    ///
    /// Returns an error if the download operation fails or the asset is not found.
    pub async fn download(&self, path: &str) -> StorageResult<Vec<u8>> {
        debug!("Downloading asset from path: {}", path);

        let content = self.operator.read(path).await.map_err(|e| {
            if e.kind() == opendal::ErrorKind::NotFound {
                StorageError::not_found(path)
            } else {
                StorageError::from(e)
            }
        })?;

        let bytes = content.to_vec();

        #[cfg(feature = "encryption")]
        let bytes = {
            let key_guard = self
                .encryption_key
                .read()
                .expect("encryption key lock poisoned");
            match key_guard.as_ref() {
                Some(key) if be_encrypt::is_encrypted(&bytes) => {
                    let decrypted = be_encrypt::decrypt(key, &bytes).map_err(|e| {
                        StorageError::Encryption(format!("Failed to decrypt asset: {}", e))
                    })?;
                    debug!(
                        "Decrypted asset from {} ({} -> {} bytes)",
                        path,
                        bytes.len(),
                        decrypted.len()
                    );
                    decrypted
                }
                None if be_encrypt::is_encrypted(&bytes) => {
                    return Err(StorageError::Encryption(
                        "asset is encrypted but no encryption key is configured".into(),
                    ));
                }
                _ => bytes,
            }
        };

        debug!(
            "Successfully downloaded {} bytes from {}",
            bytes.len(),
            path
        );

        Ok(bytes)
    }

    /// Delete content from storage
    ///
    /// # Errors
    ///
    /// Returns an error if the delete operation fails.
    pub async fn delete(&self, path: &str) -> StorageResult<()> {
        debug!("Deleting asset at path: {}", path);

        self.operator.delete(path).await?;

        info!("Successfully deleted asset at {}", path);

        Ok(())
    }

    /// Check if content exists at path
    ///
    /// # Errors
    ///
    /// Returns an error if the stat operation fails (other than NotFound).
    pub async fn exists(&self, path: &str) -> StorageResult<bool> {
        match self.operator.stat(path).await {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == opendal::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    pub fn config(&self) -> &StorageConfig {
        &self.config
    }

    pub fn operator(&self) -> &Operator {
        &self.operator
    }

    pub fn get_backend_name(&self) -> &str {
        match self.config {
            StorageConfig::S3 { .. } => "s3",
            StorageConfig::FS { .. } => "fs",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_sha256() {
        let content = b"Hello, World!";
        let hash = StorageService::calculate_sha256(content);

        let expected =
            hex::decode("dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f")
                .unwrap();
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_generate_path() {
        let user_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let asset_id = Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap();

        let path = StorageService::generate_path(&user_id, &asset_id, Some("png"));
        assert_eq!(
            path,
            "550e8400-e29b-41d4-a716-446655440000/6ba7b810-9dad-11d1-80b4-00c04fd430c8.png"
        );
    }

    #[test]
    fn test_extension_from_mime() {
        assert_eq!(StorageService::extension_from_mime("image/png"), "png");
        assert_eq!(StorageService::extension_from_mime("image/jpeg"), "jpg");
        assert_eq!(
            StorageService::extension_from_mime("application/pdf"),
            "pdf"
        );
        assert_eq!(
            StorageService::extension_from_mime("application/octet-stream"),
            "bin"
        );
    }

    #[test]
    fn test_storage_config_default() {
        let config = StorageConfig::default();
        match config {
            StorageConfig::FS { root } => {
                assert_eq!(root, "./assets");
            }
            _ => panic!("Expected Filesystem config"),
        }
    }
}
