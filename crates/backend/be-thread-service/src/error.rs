use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;
use thread_core::ThreadErrorResponse;

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

    #[error("Invalid base64 for field '{field}': {source}")]
    InvalidBase64 {
        field: &'static str,
        #[source]
        source: base64::DecodeError,
    },

    #[error("Token limit reached: {0}")]
    TokenLimitReached(String),

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

    pub fn invalid_base64(field: &'static str, source: base64::DecodeError) -> Self {
        Self::InvalidBase64 { field, source }
    }

    pub fn token_limit_reached(msg: impl Into<String>) -> Self {
        Self::TokenLimitReached(msg.into())
    }

    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound(_))
    }

    pub fn is_unauthenticated(&self) -> bool {
        matches!(self, Self::Unauthenticated(_))
    }

    /// Stable string identifier used in the JSON error body (and analytics).
    pub fn error_kind(&self) -> &'static str {
        match self {
            Self::Unauthenticated(_) => "unauthenticated",
            Self::InvalidArgument(_) => "invalid_argument",
            Self::NotFound(_) => "not_found",
            Self::Database(_) => "database_error",
            Self::Storage(_) => "storage_error",
            Self::Asset(_) => "asset_error",
            Self::InvalidUuid { .. } => "invalid_uuid",
            Self::InvalidBase64 { .. } => "invalid_base64",
            Self::TokenLimitReached(_) => "token_limit_reached",
            Self::Internal(_) => "internal_error",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::Unauthenticated(_) => StatusCode::UNAUTHORIZED,
            Self::InvalidArgument(_) | Self::InvalidUuid { .. } | Self::InvalidBase64 { .. } => {
                StatusCode::BAD_REQUEST
            }
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::TokenLimitReached(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::Database(_) | Self::Storage(_) | Self::Asset(_) | Self::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

impl From<be_remote_db::DbError> for ThreadServiceError {
    fn from(err: be_remote_db::DbError) -> Self {
        Self::Database(err)
    }
}

impl From<be_storage::StorageError> for ThreadServiceError {
    fn from(err: be_storage::StorageError) -> Self {
        Self::Storage(err)
    }
}

impl From<be_asset::AssetError> for ThreadServiceError {
    fn from(err: be_asset::AssetError) -> Self {
        Self::Asset(err)
    }
}

impl IntoResponse for ThreadServiceError {
    fn into_response(self) -> Response {
        let status = self.status();
        let kind = self.error_kind();
        let detail = self.to_string();

        match &self {
            Self::Unauthenticated(_) => {
                tracing::warn!(error = %detail, "Thread service authentication error");
            }
            Self::InvalidArgument(_) | Self::InvalidUuid { .. } | Self::InvalidBase64 { .. } => {
                tracing::debug!(error = %detail, "Thread service client error");
            }
            Self::NotFound(_) => {
                tracing::debug!(error = %detail, "Thread service resource not found");
            }
            Self::TokenLimitReached(_) => {
                tracing::info!(error = %detail, "Thread service token limit reached");
            }
            Self::Database(_) | Self::Storage(_) | Self::Asset(_) | Self::Internal(_) => {
                tracing::error!(error = %detail, "Thread service internal error");
            }
        }

        let client_message = match self {
            Self::Database(_) => "Database operation failed".to_string(),
            Self::Storage(_) => "Storage operation failed".to_string(),
            Self::Asset(_) => "Asset operation failed".to_string(),
            Self::Internal(_) => "Internal server error".to_string(),
            other => other.to_string(),
        };

        (
            status,
            Json(ThreadErrorResponse {
                error: kind.to_owned(),
                message: client_message,
                details: None,
            }),
        )
            .into_response()
    }
}

pub type ThreadServiceResult<T> = std::result::Result<T, ThreadServiceError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unauthenticated_maps_to_401() {
        let err = ThreadServiceError::unauthenticated("Missing claims");
        assert!(err.is_unauthenticated());
        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn not_found_maps_to_404() {
        let err = ThreadServiceError::not_found("Thread 123");
        assert!(err.is_not_found());
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn invalid_argument_maps_to_400() {
        let err = ThreadServiceError::invalid_argument("title is required");
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn token_limit_maps_to_429() {
        let err = ThreadServiceError::token_limit_reached("monthly cap");
        assert_eq!(err.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(err.error_kind(), "token_limit_reached");
    }

    #[test]
    fn invalid_uuid_maps_to_400() {
        let uuid_err = uuid::Uuid::parse_str("invalid").unwrap_err();
        let err = ThreadServiceError::invalid_uuid("thread_id", uuid_err);
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
        assert_eq!(err.error_kind(), "invalid_uuid");
    }
}
