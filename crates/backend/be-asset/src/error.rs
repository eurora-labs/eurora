use thiserror::Error;

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("invalid user ID in claims: {0}")]
    InvalidUserId(#[source] uuid::Error),

    #[error("content cannot be empty")]
    EmptyContent,

    #[error("MIME type is required")]
    MissingMimeType,

    #[error("invalid asset ID: {0}")]
    InvalidAssetId(#[source] uuid::Error),

    #[error("invalid activity ID: {0}")]
    InvalidActivityId(#[source] uuid::Error),

    #[error("invalid message ID: {0}")]
    InvalidMessageId(#[source] uuid::Error),

    #[error("invalid base64 SHA256 hash: {0}")]
    InvalidSha256(#[source] base64::DecodeError),

    #[error("invalid metadata JSON: {0}")]
    InvalidMetadata(#[source] serde_json::Error),

    #[error("asset not found")]
    AssetNotFound,

    #[error("asset not found or not owned by user")]
    AssetNotOwned,

    #[error("failed to upload asset to storage: {0}")]
    StorageUpload(#[source] be_storage::StorageError),

    #[error("failed to delete asset from storage: {0}")]
    StorageDelete(#[source] be_storage::StorageError),

    #[error("failed to create asset in database")]
    DatabaseCreate(#[source] be_remote_db::DbError),

    #[error("failed to retrieve asset from database")]
    DatabaseGet(#[source] be_remote_db::DbError),

    #[error("failed to update asset in database")]
    DatabaseUpdate(#[source] be_remote_db::DbError),

    #[error("failed to delete asset from database")]
    DatabaseDelete(#[source] be_remote_db::DbError),

    #[error("failed to list assets from database")]
    DatabaseList(#[source] be_remote_db::DbError),

    #[error("failed to find asset by SHA256")]
    DatabaseFindBySha256(#[source] be_remote_db::DbError),

    #[error("failed to get assets by message ID")]
    DatabaseGetByMessageId(#[source] be_remote_db::DbError),

    #[error("failed to get assets by activity ID")]
    DatabaseGetByActivityId(#[source] be_remote_db::DbError),

    #[error("failed to link asset to message")]
    DatabaseLinkMessage(#[source] be_remote_db::DbError),

    #[error("failed to unlink asset from message")]
    DatabaseUnlinkMessage(#[source] be_remote_db::DbError),

    #[error("failed to link asset to activity")]
    DatabaseLinkActivity(#[source] be_remote_db::DbError),

    #[error("failed to unlink asset from activity")]
    DatabaseUnlinkActivity(#[source] be_remote_db::DbError),

    #[error("failed to configure storage from environment: {0}")]
    StorageConfig(#[source] be_storage::StorageError),
}

pub type AssetResult<T> = std::result::Result<T, AssetError>;
