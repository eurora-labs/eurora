use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use be_asset::AssetError;
use serde::Serialize;
use thiserror::Error;

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[derive(Debug, Error)]
pub enum AssetServiceError {
    #[error("missing authentication claims")]
    MissingClaims,

    #[error("invalid user ID in claims: {0}")]
    InvalidUserId(#[source] uuid::Error),

    #[error("invalid base64 content: {0}")]
    InvalidBase64(#[source] base64::DecodeError),

    #[error(transparent)]
    Asset(#[from] AssetError),
}

impl AssetServiceError {
    pub fn error_kind(&self) -> &'static str {
        match self {
            Self::MissingClaims => "missing_claims",
            Self::InvalidUserId(_) => "invalid_user_id",
            Self::InvalidBase64(_) => "invalid_base64",
            Self::Asset(err) => match err {
                AssetError::EmptyContent => "empty_content",
                AssetError::MissingMimeType => "missing_mime_type",
                AssetError::UnsupportedMimeType(_) => "unsupported_mime_type",
                AssetError::MimeTypeMismatch => "mime_type_mismatch",
                AssetError::StorageUpload(_) => "storage_upload",
                AssetError::DatabaseCreate(_) => "database_create",
                AssetError::DatabaseLinkActivity(_) => "database_link_activity",
                AssetError::StorageConfig(_) => "storage_config",
            },
        }
    }
}

impl IntoResponse for AssetServiceError {
    fn into_response(self) -> Response {
        let (status, message, details) = match &self {
            AssetServiceError::MissingClaims => (
                StatusCode::UNAUTHORIZED,
                "Missing authentication claims".to_owned(),
                None,
            ),
            AssetServiceError::InvalidUserId(e) => (
                StatusCode::UNAUTHORIZED,
                "Invalid user ID in claims".to_owned(),
                Some(e.to_string()),
            ),
            AssetServiceError::InvalidBase64(e) => (
                StatusCode::BAD_REQUEST,
                "Asset content is not valid base64".to_owned(),
                Some(e.to_string()),
            ),
            AssetServiceError::Asset(err) => match err {
                AssetError::EmptyContent => (
                    StatusCode::BAD_REQUEST,
                    "Asset content cannot be empty".to_owned(),
                    None,
                ),
                AssetError::MissingMimeType => (
                    StatusCode::BAD_REQUEST,
                    "MIME type is required".to_owned(),
                    None,
                ),
                AssetError::UnsupportedMimeType(mime) => (
                    StatusCode::BAD_REQUEST,
                    "Unsupported MIME type".to_owned(),
                    Some(mime.clone()),
                ),
                AssetError::MimeTypeMismatch => (
                    StatusCode::BAD_REQUEST,
                    "Content does not match declared MIME type".to_owned(),
                    None,
                ),
                AssetError::StorageUpload(e) => {
                    tracing::error!("storage upload failed: {}", e);
                    (
                        StatusCode::BAD_GATEWAY,
                        "Failed to upload asset to storage".to_owned(),
                        None,
                    )
                }
                AssetError::DatabaseCreate(e) => {
                    tracing::error!("database create failed: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to persist asset".to_owned(),
                        None,
                    )
                }
                AssetError::DatabaseLinkActivity(e) => {
                    tracing::error!("database link-activity failed: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to link asset to activity".to_owned(),
                        None,
                    )
                }
                AssetError::StorageConfig(e) => {
                    tracing::error!("storage config error: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Asset storage misconfigured".to_owned(),
                        None,
                    )
                }
            },
        };

        let body = ErrorResponse {
            error: self.error_kind().to_owned(),
            message,
            details,
        };

        (status, Json(body)).into_response()
    }
}
