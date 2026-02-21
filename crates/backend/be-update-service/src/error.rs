use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::Serialize;

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateServiceError {
    #[error("Invalid version format: {0}")]
    InvalidVersion(String),

    #[error("Invalid target architecture: {0}")]
    InvalidTargetArch(String),

    #[error("Invalid channel: {0}")]
    InvalidChannel(String),

    #[error("Invalid extension channel: {0}")]
    InvalidExtensionChannel(String),

    #[error("S3 operation failed: {0}")]
    S3Error(String),

    #[error("Signature file not found: {0}")]
    SignatureNotFound(String),

    #[error("Download file not found in: {0}")]
    DownloadFileNotFound(String),

    #[error("Failed to generate presigned URL: {0}")]
    PresignedUrlError(String),
}

impl UpdateServiceError {
    pub fn error_kind(&self) -> &'static str {
        match self {
            Self::InvalidVersion(_) => "invalid_version",
            Self::InvalidTargetArch(_) => "invalid_target_arch",
            Self::InvalidChannel(_) => "invalid_channel",
            Self::InvalidExtensionChannel(_) => "invalid_extension_channel",
            Self::S3Error(_) => "s3_error",
            Self::SignatureNotFound(_) => "signature_not_found",
            Self::DownloadFileNotFound(_) => "download_not_found",
            Self::PresignedUrlError(_) => "presigned_url_error",
        }
    }
}

impl IntoResponse for UpdateServiceError {
    fn into_response(self) -> Response {
        let (status, error_code, message, details) = match &self {
            UpdateServiceError::InvalidVersion(v) => {
                tracing::warn!("Invalid version provided: {}", v);
                (
                    StatusCode::BAD_REQUEST,
                    "invalid_version",
                    "Invalid version format",
                    Some(format!("Version '{}' is not a valid semantic version", v)),
                )
            }
            UpdateServiceError::InvalidTargetArch(t) => {
                tracing::warn!("Invalid target architecture: {}", t);
                (
                    StatusCode::BAD_REQUEST,
                    "invalid_target_arch",
                    "Invalid target architecture format",
                    Some(format!(
                        "Target architecture '{}' should be in format 'os-arch'",
                        t
                    )),
                )
            }
            UpdateServiceError::InvalidChannel(c) => {
                tracing::warn!("Invalid channel: {}", c);
                (
                    StatusCode::BAD_REQUEST,
                    "invalid_channel",
                    "Invalid channel",
                    Some(format!(
                        "Channel '{}' is not supported. Use 'nightly', 'release', or 'beta'",
                        c
                    )),
                )
            }
            UpdateServiceError::InvalidExtensionChannel(c) => {
                tracing::warn!("Invalid extension channel: {}", c);
                (
                    StatusCode::BAD_REQUEST,
                    "invalid_extension_channel",
                    "Invalid extension channel",
                    Some(format!(
                        "Channel '{}' is not supported. Use 'release' or 'nightly'",
                        c
                    )),
                )
            }
            UpdateServiceError::S3Error(e) => {
                tracing::error!("S3 operation failed: {}", e);
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "service_unavailable",
                    "Update service temporarily unavailable",
                    None,
                )
            }
            UpdateServiceError::SignatureNotFound(k) => {
                tracing::error!("Signature file not found: {}", k);
                (
                    StatusCode::NOT_FOUND,
                    "signature_not_found",
                    "Signature file not found",
                    None,
                )
            }
            UpdateServiceError::DownloadFileNotFound(dir) => {
                tracing::error!("Download file not found in directory: {}", dir);
                (
                    StatusCode::NOT_FOUND,
                    "download_not_found",
                    "Update package not found",
                    None,
                )
            }
            UpdateServiceError::PresignedUrlError(e) => {
                tracing::error!("Failed to generate presigned URL: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "url_generation_failed",
                    "Failed to generate download URL",
                    None,
                )
            }
        };

        (
            status,
            Json(ErrorResponse {
                error: error_code.to_owned(),
                message: message.to_owned(),
                details,
            }),
        )
            .into_response()
    }
}
