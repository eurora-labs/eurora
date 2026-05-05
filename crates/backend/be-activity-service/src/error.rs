use activity_core::ActivityErrorResponse;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

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

    #[error("Invalid base64 for field '{field}': {source}")]
    InvalidBase64 {
        field: &'static str,
        #[source]
        source: base64::DecodeError,
    },

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

    pub fn invalid_base64(field: &'static str, source: base64::DecodeError) -> Self {
        Self::InvalidBase64 { field, source }
    }

    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound(_))
    }

    pub fn is_unauthenticated(&self) -> bool {
        matches!(self, Self::Unauthenticated(_))
    }

    /// Stable string identifier used for analytics counters.
    pub fn error_kind(&self) -> &'static str {
        match self {
            Self::Unauthenticated(_) => "unauthenticated",
            Self::InvalidArgument(_) => "invalid_argument",
            Self::NotFound(_) => "not_found",
            Self::Database(_) => "database_error",
            Self::Storage(_) => "storage_error",
            Self::Asset(_) => "asset_error",
            Self::InvalidBase64 { .. } => "invalid_base64",
            Self::Internal(_) => "internal_error",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::Unauthenticated(_) => StatusCode::UNAUTHORIZED,
            Self::InvalidArgument(_) | Self::InvalidBase64 { .. } => StatusCode::BAD_REQUEST,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Database(_) | Self::Storage(_) | Self::Asset(_) | Self::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
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

impl From<be_asset::AssetError> for ActivityServiceError {
    fn from(err: be_asset::AssetError) -> Self {
        Self::Asset(err)
    }
}

impl IntoResponse for ActivityServiceError {
    fn into_response(self) -> Response {
        let status = self.status();
        let kind = self.error_kind();
        let detail = self.to_string();

        match &self {
            Self::Unauthenticated(_) => {
                tracing::warn!(error = %detail, "Activity service authentication error");
            }
            Self::InvalidArgument(_) | Self::InvalidBase64 { .. } => {
                tracing::debug!(error = %detail, "Activity service client error");
            }
            Self::NotFound(_) => {
                tracing::debug!(error = %detail, "Activity service resource not found");
            }
            Self::Database(_) | Self::Storage(_) | Self::Asset(_) | Self::Internal(_) => {
                tracing::error!(error = %detail, "Activity service internal error");
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
            axum::Json(ActivityErrorResponse {
                error: kind.to_owned(),
                message: client_message,
                details: None,
            }),
        )
            .into_response()
    }
}

pub type ActivityResult<T> = std::result::Result<T, ActivityServiceError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unauthenticated_maps_to_401() {
        let err = ActivityServiceError::unauthenticated("Missing claims");
        assert!(err.is_unauthenticated());
        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn not_found_maps_to_404() {
        let err = ActivityServiceError::not_found("Activity 123");
        assert!(err.is_not_found());
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn invalid_argument_maps_to_400() {
        let err = ActivityServiceError::invalid_argument("name is required");
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn invalid_base64_maps_to_400() {
        use base64::Engine;
        let decode_err = base64::engine::general_purpose::STANDARD
            .decode("not_valid_b64!!")
            .unwrap_err();
        let err = ActivityServiceError::invalid_base64("icon_png_base64", decode_err);
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
        assert_eq!(err.error_kind(), "invalid_base64");
    }
}
