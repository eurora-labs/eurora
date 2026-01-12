//! Error types for the Asset Service.
//!
//! This module provides structured error handling using `thiserror` for
//! deriving error implementations and proper conversion to gRPC `Status`.

use thiserror::Error;
use tonic::Status;

/// The main error type for the Asset Service.
///
/// This enum categorizes all possible errors that can occur in the service,
/// enabling type-safe error handling and consistent conversion to gRPC status codes.
#[derive(Debug, Error)]
pub enum AssetServiceError {
    // === Authentication Errors ===
    /// Missing authentication claims in the request.
    #[error("missing authentication claims")]
    MissingClaims,

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
    DatabaseCreate(#[source] euro_remote_db::DbError),

    /// Failed to retrieve the asset from the database.
    #[error("failed to retrieve asset from database")]
    DatabaseGet(#[source] euro_remote_db::DbError),

    /// Failed to update the asset in the database.
    #[error("failed to update asset in database")]
    DatabaseUpdate(#[source] euro_remote_db::DbError),

    /// Failed to delete the asset from the database.
    #[error("failed to delete asset from database")]
    DatabaseDelete(#[source] euro_remote_db::DbError),

    /// Failed to list assets from the database.
    #[error("failed to list assets from database")]
    DatabaseList(#[source] euro_remote_db::DbError),

    /// Failed to find asset by SHA256 hash.
    #[error("failed to find asset by SHA256")]
    DatabaseFindBySha256(#[source] euro_remote_db::DbError),

    /// Failed to get assets by message ID.
    #[error("failed to get assets by message ID")]
    DatabaseGetByMessageId(#[source] euro_remote_db::DbError),

    /// Failed to get assets by activity ID.
    #[error("failed to get assets by activity ID")]
    DatabaseGetByActivityId(#[source] euro_remote_db::DbError),

    /// Failed to link asset to message.
    #[error("failed to link asset to message")]
    DatabaseLinkMessage(#[source] euro_remote_db::DbError),

    /// Failed to unlink asset from message.
    #[error("failed to unlink asset from message")]
    DatabaseUnlinkMessage(#[source] euro_remote_db::DbError),

    /// Failed to link asset to activity.
    #[error("failed to link asset to activity")]
    DatabaseLinkActivity(#[source] euro_remote_db::DbError),

    /// Failed to unlink asset from activity.
    #[error("failed to unlink asset from activity")]
    DatabaseUnlinkActivity(#[source] euro_remote_db::DbError),

    // === Configuration Errors ===
    /// Failed to configure storage from environment.
    #[error("failed to configure storage from environment: {0}")]
    StorageConfig(#[source] be_storage::StorageError),
}

impl From<AssetServiceError> for Status {
    fn from(err: AssetServiceError) -> Self {
        use AssetServiceError::*;

        match &err {
            // Authentication errors -> UNAUTHENTICATED
            MissingClaims => Status::unauthenticated(err.to_string()),

            // Invalid user ID is internal because it means the auth layer provided bad data
            InvalidUserId(_) => Status::internal("invalid user ID in authentication token"),

            // Validation errors -> INVALID_ARGUMENT
            EmptyContent | MissingMimeType => Status::invalid_argument(err.to_string()),
            InvalidAssetId(_) | InvalidActivityId(_) | InvalidMessageId(_) => {
                Status::invalid_argument(err.to_string())
            }
            InvalidSha256(_) | InvalidMetadata(_) => Status::invalid_argument(err.to_string()),

            // Not found errors -> NOT_FOUND
            AssetNotFound | AssetNotOwned => Status::not_found(err.to_string()),

            // Storage errors -> INTERNAL (don't expose internal details)
            StorageUpload(_) => Status::internal("failed to upload asset to storage"),
            StorageDelete(_) => Status::internal("failed to delete asset from storage"),
            StorageConfig(_) => Status::internal("storage configuration error"),

            // Database errors -> INTERNAL (don't expose internal details)
            DatabaseCreate(_) => Status::internal("failed to create asset"),
            DatabaseGet(_) => Status::internal("failed to retrieve asset"),
            DatabaseUpdate(_) => Status::internal("failed to update asset"),
            DatabaseDelete(_) => Status::internal("failed to delete asset"),
            DatabaseList(_) => Status::internal("failed to list assets"),
            DatabaseFindBySha256(_) => Status::internal("failed to find asset by SHA256"),
            DatabaseGetByMessageId(_) => Status::internal("failed to get assets by message ID"),
            DatabaseGetByActivityId(_) => Status::internal("failed to get assets by activity ID"),
            DatabaseLinkMessage(_) => Status::internal("failed to link asset to message"),
            DatabaseUnlinkMessage(_) => Status::internal("failed to unlink asset from message"),
            DatabaseLinkActivity(_) => Status::internal("failed to link asset to activity"),
            DatabaseUnlinkActivity(_) => Status::internal("failed to unlink asset from activity"),
        }
    }
}

/// A specialized Result type for Asset Service operations.
pub type Result<T> = std::result::Result<T, AssetServiceError>;
