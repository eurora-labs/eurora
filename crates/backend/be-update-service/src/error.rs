use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::Serialize;
use tracing::{error, warn};

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

    #[error("Invalid browser type: {0}")]
    InvalidBrowserType(String),

    #[error("Invalid extension channel: {0}")]
    InvalidExtensionChannel(String),

    #[error("Extension not found for browser '{browser}' channel '{channel}'")]
    ExtensionNotFound { browser: String, channel: String },

    #[error("S3 operation failed: {0}")]
    S3Error(String),

    #[error("Signature file not found: {0}")]
    SignatureNotFound(String),

    #[error("Download file not found in: {0}")]
    DownloadFileNotFound(String),

    #[error("Failed to generate presigned URL: {0}")]
    PresignedUrlError(String),
}

impl IntoResponse for UpdateServiceError {
    fn into_response(self) -> Response {
        let (status, error_code, message, details) = match &self {
            UpdateServiceError::InvalidVersion(v) => {
                warn!("Invalid version provided: {}", v);
                (
                    StatusCode::BAD_REQUEST,
                    "invalid_version",
                    "Invalid version format",
                    Some(format!("Version '{}' is not a valid semantic version", v)),
                )
            }
            UpdateServiceError::InvalidTargetArch(t) => {
                warn!("Invalid target architecture: {}", t);
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
                warn!("Invalid channel: {}", c);
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
            UpdateServiceError::InvalidBrowserType(b) => {
                warn!("Invalid browser type: {}", b);
                (
                    StatusCode::BAD_REQUEST,
                    "invalid_browser_type",
                    "Invalid browser type",
                    Some(format!(
                        "Browser '{}' is not supported. Use 'firefox', 'chrome', or 'safari'",
                        b
                    )),
                )
            }
            UpdateServiceError::InvalidExtensionChannel(c) => {
                warn!("Invalid extension channel: {}", c);
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
            UpdateServiceError::ExtensionNotFound { browser, channel } => {
                warn!(
                    "Extension not found for browser '{}' channel '{}'",
                    browser, channel
                );
                (
                    StatusCode::NOT_FOUND,
                    "extension_not_found",
                    "Extension version not found",
                    Some(format!(
                        "No extension found for browser '{}' in channel '{}'",
                        browser, channel
                    )),
                )
            }
            UpdateServiceError::S3Error(e) => {
                error!("S3 operation failed: {}", e);
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "service_unavailable",
                    "Update service temporarily unavailable",
                    None,
                )
            }
            UpdateServiceError::SignatureNotFound(k) => {
                error!("Signature file not found: {}", k);
                (
                    StatusCode::NOT_FOUND,
                    "signature_not_found",
                    "Signature file not found",
                    None,
                )
            }
            UpdateServiceError::DownloadFileNotFound(dir) => {
                error!("Download file not found in directory: {}", dir);
                (
                    StatusCode::NOT_FOUND,
                    "download_not_found",
                    "Update package not found",
                    None,
                )
            }
            UpdateServiceError::PresignedUrlError(e) => {
                error!("Failed to generate presigned URL: {}", e);
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
