use activity_core::ActivityErrorResponse;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use be_auth_core::{InvalidUserId, MissingClaims};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ActivityServiceError {
    #[error("Authentication failed: {0}")]
    Unauthenticated(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

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

    pub fn invalid_base64(field: &'static str, source: base64::DecodeError) -> Self {
        Self::InvalidBase64 { field, source }
    }

    /// Stable string identifier surfaced to the client and used for
    /// analytics counters.
    pub fn error_kind(&self) -> &'static str {
        match self {
            Self::Unauthenticated(_) => "unauthenticated",
            Self::InvalidArgument(_) => "invalid_argument",
            Self::NotFound(_) => "not_found",
            Self::Conflict(_) => "conflict",
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
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Database(_) | Self::Storage(_) | Self::Asset(_) | Self::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

impl From<be_remote_db::DbError> for ActivityServiceError {
    fn from(err: be_remote_db::DbError) -> Self {
        use be_remote_db::DbError;
        match err {
            DbError::NotFound { entity, id } => Self::NotFound(match id {
                Some(id) => format!("{entity} {id}"),
                None => entity.to_string(),
            }),
            DbError::UniqueViolation { constraint } => {
                Self::Conflict(format!("Unique constraint violated: {constraint}"))
            }
            DbError::ForeignKeyViolation { entity } => {
                Self::InvalidArgument(format!("Referenced {entity} does not exist"))
            }
            DbError::InvalidInput(msg) => Self::InvalidArgument(msg),
            other => Self::Database(other),
        }
    }
}

impl From<be_storage::StorageError> for ActivityServiceError {
    fn from(err: be_storage::StorageError) -> Self {
        Self::Storage(err)
    }
}

impl From<be_asset::AssetError> for ActivityServiceError {
    fn from(err: be_asset::AssetError) -> Self {
        use be_asset::AssetError;
        match err {
            AssetError::EmptyContent
            | AssetError::MissingMimeType
            | AssetError::UnsupportedMimeType(_)
            | AssetError::MimeTypeMismatch => Self::InvalidArgument(err.to_string()),
            other => Self::Asset(other),
        }
    }
}

impl From<MissingClaims> for ActivityServiceError {
    fn from(_: MissingClaims) -> Self {
        Self::unauthenticated("Missing authenticated claims")
    }
}

impl From<InvalidUserId> for ActivityServiceError {
    fn from(err: InvalidUserId) -> Self {
        Self::unauthenticated(err.to_string())
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
            Self::Conflict(_) => {
                tracing::info!(error = %detail, "Activity service conflict");
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
    use be_remote_db::DbError;

    #[test]
    fn unauthenticated_maps_to_401() {
        let err = ActivityServiceError::unauthenticated("Missing claims");
        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
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

    #[test]
    fn db_not_found_maps_to_404() {
        let err: ActivityServiceError = DbError::not_found_with_id("activity", "abc").into();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
        assert_eq!(err.error_kind(), "not_found");
    }

    #[test]
    fn db_unique_violation_maps_to_409() {
        let err: ActivityServiceError = DbError::unique_violation("activities_pkey").into();
        assert_eq!(err.status(), StatusCode::CONFLICT);
        assert_eq!(err.error_kind(), "conflict");
    }

    #[test]
    fn db_foreign_key_maps_to_400() {
        let err: ActivityServiceError = DbError::foreign_key("asset").into();
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
        assert_eq!(err.error_kind(), "invalid_argument");
    }

    #[test]
    fn db_invalid_input_maps_to_400() {
        let err: ActivityServiceError = DbError::invalid_input("nope").into();
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn db_pool_error_maps_to_500() {
        let err: ActivityServiceError = DbError::pool("timed out").into();
        assert_eq!(err.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.error_kind(), "database_error");
    }

    #[test]
    fn missing_claims_maps_to_401() {
        let err: ActivityServiceError = MissingClaims.into();
        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn asset_validation_error_maps_to_400() {
        let err: ActivityServiceError = be_asset::AssetError::EmptyContent.into();
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
        assert_eq!(err.error_kind(), "invalid_argument");
    }

    #[test]
    fn asset_unexpected_error_maps_to_500() {
        let err: ActivityServiceError =
            be_asset::AssetError::DatabaseCreate(be_remote_db::DbError::Internal("boom".into()))
                .into();
        assert_eq!(err.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.error_kind(), "asset_error");
    }
}
