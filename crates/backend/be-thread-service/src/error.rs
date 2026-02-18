//! Error types for the Thread Service.
//!
//! This module provides structured error handling with automatic
//! conversion to gRPC status codes.

use thiserror::Error;
use tonic::{Code, Status};

#[derive(Error, Debug)]
pub enum ThreadServiceError {
    #[error("Authentication failed: {0}")]
    Unauthenticated(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Database error: {0}")]
    Database(#[source] be_remote_db::DbError),

    #[error("Invalid UUID '{field}': {source}")]
    InvalidUuid {
        field: &'static str,
        #[source]
        source: uuid::Error,
    },

    #[error("Invalid timestamp for field '{0}'")]
    InvalidTimestamp(&'static str),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl ThreadServiceError {
    pub fn unauthenticated(msg: impl Into<String>) -> Self {
        Self::Unauthenticated(msg.into())
    }

    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        Self::InvalidArgument(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    pub fn invalid_uuid(field: &'static str, source: uuid::Error) -> Self {
        Self::InvalidUuid { field, source }
    }

    pub fn invalid_timestamp(field: &'static str) -> Self {
        Self::InvalidTimestamp(field)
    }

    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound(_))
    }

    pub fn is_unauthenticated(&self) -> bool {
        matches!(self, Self::Unauthenticated(_))
    }

    pub fn code(&self) -> Code {
        match self {
            Self::Unauthenticated(_) => Code::Unauthenticated,
            Self::InvalidArgument(_) | Self::InvalidUuid { .. } | Self::InvalidTimestamp(_) => {
                Code::InvalidArgument
            }
            Self::NotFound(_) => Code::NotFound,
            Self::Database(_) | Self::Internal(_) => Code::Internal,
        }
    }
}

impl From<be_remote_db::DbError> for ThreadServiceError {
    fn from(err: be_remote_db::DbError) -> Self {
        Self::Database(err)
    }
}

impl From<ThreadServiceError> for Status {
    fn from(err: ThreadServiceError) -> Self {
        let code = err.code();
        let message = err.to_string();

        match &err {
            ThreadServiceError::Unauthenticated(_) => {
                tracing::warn!("Authentication error: {}", message);
            }
            ThreadServiceError::InvalidArgument(_)
            | ThreadServiceError::InvalidUuid { .. }
            | ThreadServiceError::InvalidTimestamp(_) => {
                tracing::debug!("Client error: {}", message);
            }
            ThreadServiceError::NotFound(_) => {
                tracing::debug!("Resource not found: {}", message);
            }
            ThreadServiceError::Database(e) => {
                tracing::error!("Database error: {} (source: {:?})", message, e);
            }
            ThreadServiceError::Internal(_) => {
                tracing::error!("Internal error: {}", message);
            }
        }

        // For internal errors, don't expose implementation details to clients
        let client_message = match err {
            ThreadServiceError::Database(_) => "Database operation failed".to_string(),
            ThreadServiceError::Internal(_) => "Internal server error".to_string(),
            _ => message,
        };

        Status::new(code, client_message)
    }
}

pub type ThreadServiceResult<T> = std::result::Result<T, ThreadServiceError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let auth_err = ThreadServiceError::unauthenticated("Missing claims");
        assert!(auth_err.is_unauthenticated());
        assert_eq!(auth_err.code(), Code::Unauthenticated);
        assert_eq!(
            auth_err.to_string(),
            "Authentication failed: Missing claims"
        );

        let not_found = ThreadServiceError::not_found("Thread 123");
        assert!(not_found.is_not_found());
        assert_eq!(not_found.code(), Code::NotFound);

        let invalid_arg = ThreadServiceError::invalid_argument("title is required");
        assert_eq!(invalid_arg.code(), Code::InvalidArgument);
    }

    #[test]
    fn test_uuid_error() {
        let uuid_err = uuid::Uuid::parse_str("invalid").unwrap_err();
        let err = ThreadServiceError::invalid_uuid("thread_id", uuid_err);
        assert_eq!(err.code(), Code::InvalidArgument);
        assert!(err.to_string().contains("thread_id"));
    }

    #[test]
    fn test_timestamp_error() {
        let err = ThreadServiceError::invalid_timestamp("created_at");
        assert_eq!(err.code(), Code::InvalidArgument);
        assert!(err.to_string().contains("created_at"));
    }

    #[test]
    fn test_status_conversion() {
        let err = ThreadServiceError::not_found("Thread not found");
        let status: Status = err.into();
        assert_eq!(status.code(), Code::NotFound);

        let err = ThreadServiceError::unauthenticated("No token");
        let status: Status = err.into();
        assert_eq!(status.code(), Code::Unauthenticated);
    }
}
