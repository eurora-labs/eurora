//! Error types for the storage service

use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Storage operation failed: {0}")]
    OpenDal(#[from] opendal::Error),

    #[error("Asset not found: {0}")]
    NotFound(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Upload failed: {0}")]
    UploadFailed(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Delete failed: {0}")]
    DeleteFailed(String),

    #[error("Encryption error: {0}")]
    Encryption(String),
}

impl StorageError {
    pub fn configuration(msg: impl Into<String>) -> Self {
        Self::Configuration(msg.into())
    }

    pub fn missing_env_var(var_name: impl Into<String>) -> Self {
        Self::MissingEnvVar(var_name.into())
    }

    pub fn not_found(path: impl Into<String>) -> Self {
        Self::NotFound(path.into())
    }

    pub fn invalid_path(msg: impl Into<String>) -> Self {
        Self::InvalidPath(msg.into())
    }

    pub fn upload_failed(msg: impl Into<String>) -> Self {
        Self::UploadFailed(msg.into())
    }

    pub fn download_failed(msg: impl Into<String>) -> Self {
        Self::DownloadFailed(msg.into())
    }

    pub fn delete_failed(msg: impl Into<String>) -> Self {
        Self::DeleteFailed(msg.into())
    }

    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound(_))
    }

    pub fn is_configuration(&self) -> bool {
        matches!(self, Self::Configuration(_) | Self::MissingEnvVar(_))
    }
}

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
