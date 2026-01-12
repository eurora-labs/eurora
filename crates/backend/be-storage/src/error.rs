//! Error types for the storage service

use thiserror::Error;

/// Errors that can occur in the storage service
#[derive(Error, Debug)]
pub enum StorageError {
    /// Configuration error - missing or invalid configuration
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Missing required environment variable
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),

    /// IO error when accessing filesystem
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// OpenDAL storage operation error
    #[error("Storage operation failed: {0}")]
    OpenDal(#[from] opendal::Error),

    /// Asset not found at the specified path
    #[error("Asset not found: {0}")]
    NotFound(String),

    /// Invalid path format
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Upload operation failed
    #[error("Upload failed: {0}")]
    UploadFailed(String),

    /// Download operation failed
    #[error("Download failed: {0}")]
    DownloadFailed(String),

    /// Delete operation failed
    #[error("Delete failed: {0}")]
    DeleteFailed(String),
}

impl StorageError {
    /// Create a new configuration error
    pub fn configuration(msg: impl Into<String>) -> Self {
        Self::Configuration(msg.into())
    }

    /// Create a missing environment variable error
    pub fn missing_env_var(var_name: impl Into<String>) -> Self {
        Self::MissingEnvVar(var_name.into())
    }

    /// Create a not found error
    pub fn not_found(path: impl Into<String>) -> Self {
        Self::NotFound(path.into())
    }

    /// Create an invalid path error
    pub fn invalid_path(msg: impl Into<String>) -> Self {
        Self::InvalidPath(msg.into())
    }

    /// Create an upload failed error
    pub fn upload_failed(msg: impl Into<String>) -> Self {
        Self::UploadFailed(msg.into())
    }

    /// Create a download failed error
    pub fn download_failed(msg: impl Into<String>) -> Self {
        Self::DownloadFailed(msg.into())
    }

    /// Create a delete failed error
    pub fn delete_failed(msg: impl Into<String>) -> Self {
        Self::DeleteFailed(msg.into())
    }

    /// Check if this error represents a "not found" condition
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound(_))
    }

    /// Check if this error is a configuration error
    pub fn is_configuration(&self) -> bool {
        matches!(self, Self::Configuration(_) | Self::MissingEnvVar(_))
    }
}

/// Result type alias for storage operations
pub type StorageResult<T> = std::result::Result<T, StorageError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let config_error = StorageError::configuration("Invalid bucket name");
        assert!(matches!(config_error, StorageError::Configuration(_)));
        assert_eq!(
            config_error.to_string(),
            "Configuration error: Invalid bucket name"
        );

        let missing_var = StorageError::missing_env_var("ASSET_STORAGE_S3_BUCKET");
        assert!(matches!(missing_var, StorageError::MissingEnvVar(_)));
        assert_eq!(
            missing_var.to_string(),
            "Missing required environment variable: ASSET_STORAGE_S3_BUCKET"
        );

        let not_found = StorageError::not_found("/user/asset.png");
        assert!(matches!(not_found, StorageError::NotFound(_)));
        assert!(not_found.is_not_found());
    }

    #[test]
    fn test_error_from_conversions() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let storage_error: StorageError = io_error.into();
        assert!(matches!(storage_error, StorageError::Io(_)));
    }

    #[test]
    fn test_is_configuration() {
        assert!(StorageError::configuration("test").is_configuration());
        assert!(StorageError::missing_env_var("VAR").is_configuration());
        assert!(!StorageError::not_found("path").is_configuration());
    }
}
