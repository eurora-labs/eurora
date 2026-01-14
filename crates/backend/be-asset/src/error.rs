//! Error types for the Asset Service.

use thiserror::Error;

/// The main error type for the Asset Service.
#[derive(Debug, Error)]
pub enum AssetError {
    /// The user ID in the claims is not a valid UUID.
    #[error("invalid user ID in claims: {0}")]
    InvalidUserId(#[source] uuid::Error),

    // === Validation Errors ===
    /// The asset content is empty.
    #[error("content cannot be empty")]
    EmptyContent,

    /// The MIME type is required but was not provided.
    #[error("MIME type is required")]
    MissingMimeType,

    /// The provided asset ID is not a valid UUID.
    #[error("invalid asset ID: {0}")]
    InvalidAssetId(#[source] uuid::Error),

    /// The provided activity ID is not a valid UUID.
    #[error("invalid activity ID: {0}")]
    InvalidActivityId(#[source] uuid::Error),

    /// The provided message ID is not a valid UUID.
    #[error("invalid message ID: {0}")]
    InvalidMessageId(#[source] uuid::Error),

    /// The provided base64-encoded SHA256 hash is invalid.
    #[error("invalid base64 SHA256 hash: {0}")]
    InvalidSha256(#[source] base64::DecodeError),

    /// The provided metadata is not valid JSON.
    #[error("invalid metadata JSON: {0}")]
    InvalidMetadata(#[source] serde_json::Error),

    // === Not Found Errors ===
    /// The requested asset was not found.
    #[error("asset not found")]
    AssetNotFound,

    /// The asset exists but is not owned by the requesting user.
    #[error("asset not found or not owned by user")]
    AssetNotOwned,

    // === Storage Errors ===
    /// Failed to upload the asset to storage.
    #[error("failed to upload asset to storage: {0}")]
    StorageUpload(#[source] be_storage::StorageError),

    /// Failed to delete the asset from storage.
    #[error("failed to delete asset from storage: {0}")]
    StorageDelete(#[source] be_storage::StorageError),

    // === Database Errors ===
    /// Failed to create the asset in the database.
    #[error("failed to create asset in database")]
    DatabaseCreate(#[source] be_remote_db::DbError),

    /// Failed to retrieve the asset from the database.
    #[error("failed to retrieve asset from database")]
    DatabaseGet(#[source] be_remote_db::DbError),

    /// Failed to update the asset in the database.
    #[error("failed to update asset in database")]
    DatabaseUpdate(#[source] be_remote_db::DbError),

    /// Failed to delete the asset from the database.
    #[error("failed to delete asset from database")]
    DatabaseDelete(#[source] be_remote_db::DbError),

    /// Failed to list assets from the database.
    #[error("failed to list assets from database")]
    DatabaseList(#[source] be_remote_db::DbError),

    /// Failed to find asset by SHA256 hash.
    #[error("failed to find asset by SHA256")]
    DatabaseFindBySha256(#[source] be_remote_db::DbError),

    /// Failed to get assets by message ID.
    #[error("failed to get assets by message ID")]
    DatabaseGetByMessageId(#[source] be_remote_db::DbError),

    /// Failed to get assets by activity ID.
    #[error("failed to get assets by activity ID")]
    DatabaseGetByActivityId(#[source] be_remote_db::DbError),

    /// Failed to link asset to message.
    #[error("failed to link asset to message")]
    DatabaseLinkMessage(#[source] be_remote_db::DbError),

    /// Failed to unlink asset from message.
    #[error("failed to unlink asset from message")]
    DatabaseUnlinkMessage(#[source] be_remote_db::DbError),

    /// Failed to link asset to activity.
    #[error("failed to link asset to activity")]
    DatabaseLinkActivity(#[source] be_remote_db::DbError),

    /// Failed to unlink asset from activity.
    #[error("failed to unlink asset from activity")]
    DatabaseUnlinkActivity(#[source] be_remote_db::DbError),

    // === Configuration Errors ===
    /// Failed to configure storage from environment.
    #[error("failed to configure storage from environment: {0}")]
    StorageConfig(#[source] be_storage::StorageError),
}

/// A specialized Result type for Asset Service operations.
pub type AssetResult<T> = std::result::Result<T, AssetError>;
