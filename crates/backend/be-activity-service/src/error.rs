//! Error types for the Activity Service.
//!
//! This module provides structured error handling with automatic
//! conversion to gRPC status codes.

use thiserror::Error;
use tonic::{Code, Status};

/// Errors that can occur in the activity service.
#[derive(Error, Debug)]
pub enum ActivityServiceError {
    /// Authentication error - missing or invalid claims.
    #[error("Authentication failed: {0}")]
    Unauthenticated(String),

    /// Invalid argument provided in the request.
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// Requested resource was not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Database operation failed.
    #[error("Database error: {0}")]
    Database(#[source] be_remote_db::DbError),

    /// Storage operation failed.
    #[error("Storage error: {0}")]
    Storage(#[source] be_storage::StorageError),

    /// Asset operation failed.
    #[error("Asset error: {0}")]
    Asset(#[source] be_asset::AssetError),

    /// UUID parsing failed.
    #[error("Invalid UUID '{field}': {source}")]
    InvalidUuid {
        field: &'static str,
        #[source]
        source: uuid::Error,
    },

    /// Timestamp conversion failed.
    #[error("Invalid timestamp for field '{0}'")]
    InvalidTimestamp(&'static str),

    /// Internal server error.
    #[error("Internal error: {0}")]
    Internal(String),
}

impl ActivityServiceError {
    /// Create an unauthenticated error.
    pub fn unauthenticated(msg: impl Into<String>) -> Self {
        Self::Unauthenticated(msg.into())
    }

    /// Create an invalid argument error.
    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        Self::InvalidArgument(msg.into())
    }

    /// Create a not found error.
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    /// Create an internal error.
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Create an invalid UUID error.
    pub fn invalid_uuid(field: &'static str, source: uuid::Error) -> Self {
        Self::InvalidUuid { field, source }
    }

    /// Create an invalid timestamp error.
    pub fn invalid_timestamp(field: &'static str) -> Self {
        Self::InvalidTimestamp(field)
    }

    /// Check if this is a not found error.
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound(_))
    }

    /// Check if this is an authentication error.
    pub fn is_unauthenticated(&self) -> bool {
        matches!(self, Self::Unauthenticated(_))
    }

    /// Get the gRPC status code for this error.
    pub fn code(&self) -> Code {
        match self {
            Self::Unauthenticated(_) => Code::Unauthenticated,
            Self::InvalidArgument(_) | Self::InvalidUuid { .. } | Self::InvalidTimestamp(_) => {
                Code::InvalidArgument
            }
            Self::NotFound(_) => Code::NotFound,
            Self::Database(_) | Self::Storage(_) | Self::Internal(_) => Code::Internal,
            Self::Asset(_) => Code::Internal,
        }
    }
}

impl From<be_remote_db::DbError> for ActivityServiceError {
    fn from(err: be_remote_db::DbError) -> Self {
        Self::Database(err)
    }
}

impl From<be_storage::StorageError> for ActivityServiceError {
    fn from(err: be_storage::StorageError) -> Self {
        Self::Storage(err)
    }
}

impl From<ActivityServiceError> for Status {
    fn from(err: ActivityServiceError) -> Self {
        let code = err.code();
        let message = err.to_string();

        // Log the error with appropriate level based on severity
        match &err {
            ActivityServiceError::Unauthenticated(_) => {
                tracing::warn!("Authentication error: {}", message);
            }
            ActivityServiceError::InvalidArgument(_)
            | ActivityServiceError::InvalidUuid { .. }
            | ActivityServiceError::InvalidTimestamp(_) => {
                tracing::debug!("Client error: {}", message);
            }
            ActivityServiceError::NotFound(_) => {
                tracing::debug!("Resource not found: {}", message);
            }
            ActivityServiceError::Database(e) => {
                tracing::error!("Database error: {} (source: {:?})", message, e);
            }
            ActivityServiceError::Storage(e) => {
                tracing::error!("Storage error: {} (source: {:?})", message, e);
            }
            ActivityServiceError::Internal(_) => {
                tracing::error!("Internal error: {}", message);
            }
            ActivityServiceError::Asset(_) => {
                tracing::error!("Asset error: {}", message);
            }
        }

        // For internal errors, don't expose implementation details to clients
        let client_message = match err {
            ActivityServiceError::Database(_) => "Database operation failed".to_string(),
            ActivityServiceError::Storage(_) => "Storage operation failed".to_string(),
            ActivityServiceError::Internal(_) => "Internal server error".to_string(),
            _ => message,
        };

        Status::new(code, client_message)
    }
}

/// Result type alias for activity service operations.
pub type ActivityResult<T> = std::result::Result<T, ActivityServiceError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let auth_err = ActivityServiceError::unauthenticated("Missing claims");
        assert!(auth_err.is_unauthenticated());
        assert_eq!(auth_err.code(), Code::Unauthenticated);
        assert_eq!(
            auth_err.to_string(),
            "Authentication failed: Missing claims"
        );

        let not_found = ActivityServiceError::not_found("Activity 123");
        assert!(not_found.is_not_found());
        assert_eq!(not_found.code(), Code::NotFound);

        let invalid_arg = ActivityServiceError::invalid_argument("name is required");
        assert_eq!(invalid_arg.code(), Code::InvalidArgument);
    }

    #[test]
    fn test_uuid_error() {
        let uuid_err = uuid::Uuid::parse_str("invalid").unwrap_err();
        let err = ActivityServiceError::invalid_uuid("activity_id", uuid_err);
        assert_eq!(err.code(), Code::InvalidArgument);
        assert!(err.to_string().contains("activity_id"));
    }

    #[test]
    fn test_timestamp_error() {
        let err = ActivityServiceError::invalid_timestamp("started_at");
        assert_eq!(err.code(), Code::InvalidArgument);
        assert!(err.to_string().contains("started_at"));
    }

    #[test]
    fn test_status_conversion() {
        let err = ActivityServiceError::not_found("Activity not found");
        let status: Status = err.into();
        assert_eq!(status.code(), Code::NotFound);

        let err = ActivityServiceError::unauthenticated("No token");
        let status: Status = err.into();
        assert_eq!(status.code(), Code::Unauthenticated);
    }
}
