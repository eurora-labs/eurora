use std::borrow::Cow;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use be_asset::AssetError;
use be_auth_core::{InvalidUserId, MissingClaims};
use serde::Serialize;
use thiserror::Error;

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[derive(Debug, Error)]
pub enum AssetServiceError {
    #[error(transparent)]
    MissingClaims(#[from] MissingClaims),

    #[error(transparent)]
    InvalidUserId(#[from] InvalidUserId),

    #[error("invalid base64 content: {0}")]
    InvalidBase64(#[source] base64::DecodeError),

    #[error(transparent)]
    Asset(#[from] AssetError),
}

struct Rendered {
    status: StatusCode,
    kind: &'static str,
    message: Cow<'static, str>,
    details: Option<String>,
}

impl AssetServiceError {
    fn render(&self) -> Rendered {
        match self {
            Self::MissingClaims(_) => Rendered {
                status: StatusCode::UNAUTHORIZED,
                kind: "missing_claims",
                message: Cow::Borrowed("Missing authentication claims"),
                details: None,
            },
            Self::InvalidUserId(e) => Rendered {
                status: StatusCode::UNAUTHORIZED,
                kind: "invalid_user_id",
                message: Cow::Borrowed("Invalid user ID in claims"),
                details: Some(e.to_string()),
            },
            Self::InvalidBase64(e) => Rendered {
                status: StatusCode::BAD_REQUEST,
                kind: "invalid_base64",
                message: Cow::Borrowed("Asset content is not valid base64"),
                details: Some(e.to_string()),
            },
            Self::Asset(err) => render_asset(err),
        }
    }
}

fn render_asset(err: &AssetError) -> Rendered {
    match err {
        AssetError::NotFound => Rendered {
            status: StatusCode::NOT_FOUND,
            kind: "not_found",
            message: Cow::Borrowed("Asset not found"),
            details: None,
        },
        AssetError::EmptyContent => Rendered {
            status: StatusCode::BAD_REQUEST,
            kind: "empty_content",
            message: Cow::Borrowed("Asset content cannot be empty"),
            details: None,
        },
        AssetError::MissingMimeType => Rendered {
            status: StatusCode::BAD_REQUEST,
            kind: "missing_mime_type",
            message: Cow::Borrowed("MIME type is required"),
            details: None,
        },
        AssetError::UnsupportedMimeType(mime) => Rendered {
            status: StatusCode::BAD_REQUEST,
            kind: "unsupported_mime_type",
            message: Cow::Borrowed("Unsupported MIME type"),
            details: Some(mime.clone()),
        },
        AssetError::MimeTypeMismatch => Rendered {
            status: StatusCode::BAD_REQUEST,
            kind: "mime_type_mismatch",
            message: Cow::Borrowed("Content does not match declared MIME type"),
            details: None,
        },
        AssetError::StorageUpload(e) => {
            tracing::error!(error = %e, "storage upload failed");
            Rendered {
                status: StatusCode::BAD_GATEWAY,
                kind: "storage_upload",
                message: Cow::Borrowed("Failed to upload asset to storage"),
                details: None,
            }
        }
        AssetError::DatabaseCreate(e) => {
            tracing::error!(error = %e, "database create failed");
            Rendered {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                kind: "database_create",
                message: Cow::Borrowed("Failed to persist asset"),
                details: None,
            }
        }
        AssetError::DatabaseRead(e) => {
            tracing::error!(error = %e, "database read failed");
            Rendered {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                kind: "database_read",
                message: Cow::Borrowed("Failed to read asset from database"),
                details: None,
            }
        }
        AssetError::StorageDownload(e) => {
            tracing::error!(error = %e, "storage download failed");
            Rendered {
                status: StatusCode::BAD_GATEWAY,
                kind: "storage_download",
                message: Cow::Borrowed("Failed to download asset from storage"),
                details: None,
            }
        }
        AssetError::StorageConfig(e) => {
            tracing::error!(error = %e, "storage config error");
            Rendered {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                kind: "storage_config",
                message: Cow::Borrowed("Asset storage misconfigured"),
                details: None,
            }
        }
    }
}

impl IntoResponse for AssetServiceError {
    fn into_response(self) -> Response {
        let Rendered {
            status,
            kind,
            message,
            details,
        } = self.render();

        let body = ErrorResponse {
            error: kind,
            message: message.into_owned(),
            details,
        };

        (status, Json(body)).into_response()
    }
}
