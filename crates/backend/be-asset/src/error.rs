use thiserror::Error;

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("content cannot be empty")]
    EmptyContent,

    #[error("MIME type is required")]
    MissingMimeType,

    #[error("unsupported MIME type: {0}")]
    UnsupportedMimeType(String),

    #[error("file content does not match declared MIME type")]
    MimeTypeMismatch,

    #[error("failed to upload asset to storage: {0}")]
    StorageUpload(#[source] be_storage::StorageError),

    #[error("failed to download asset from storage: {0}")]
    StorageDownload(#[source] be_storage::StorageError),

    #[error("failed to create asset in database")]
    DatabaseCreate(#[source] be_remote_db::DbError),

    #[error("failed to link asset to activity")]
    DatabaseLinkActivity(#[source] be_remote_db::DbError),

    #[error("failed to read asset from database")]
    DatabaseRead(#[source] be_remote_db::DbError),

    #[error("asset not found")]
    NotFound,

    #[error("failed to configure storage from environment: {0}")]
    StorageConfig(#[source] be_storage::StorageError),
}

pub type AssetResult<T> = std::result::Result<T, AssetError>;
