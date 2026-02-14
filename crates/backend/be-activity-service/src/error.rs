use thiserror::Error;
use tonic::{Code, Status};

#[derive(Error, Debug)]
pub enum ActivityServiceError {
    #[error("Authentication failed: {0}")]
    Unauthenticated(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Database error: {0}")]
    Database(#[source] be_remote_db::DbError),

    #[error("Storage error: {0}")]
    Storage(#[source] be_storage::StorageError),

    #[error("Asset error: {0}")]
    Asset(#[source] be_asset::AssetError),

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

impl ActivityServiceError {
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

        let client_message = match err {
            ActivityServiceError::Database(_) => "Database operation failed".to_string(),
            ActivityServiceError::Storage(_) => "Storage operation failed".to_string(),
            ActivityServiceError::Internal(_) => "Internal server error".to_string(),
            _ => message,
        };

        Status::new(code, client_message)
    }
}

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
