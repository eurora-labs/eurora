//! Error types and handling for the update service

use axum::{http::StatusCode, response::Json};
use serde::Serialize;
use tracing::error;

/// Detailed error response with error type
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub details: Option<String>,
}

/// Error response for when no update is available
#[derive(Serialize)]
pub struct NoUpdateResponse {
    pub message: String,
}

/// Custom error types for better error handling
#[derive(Debug)]
pub enum UpdateServiceError {
    InvalidVersion(String),
    InvalidTargetArch(String),
    InvalidChannel(String),
    S3Error(String),
    SignatureNotFound(String),
    DownloadFileNotFound(String),
    PresignedUrlError(String),
    ListObjectsError(String),
}

impl std::fmt::Display for UpdateServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateServiceError::InvalidVersion(v) => write!(f, "Invalid version format: {}", v),
            UpdateServiceError::InvalidTargetArch(t) => {
                write!(f, "Invalid target architecture: {}", t)
            }
            UpdateServiceError::InvalidChannel(c) => write!(f, "Invalid channel: {}", c),
            UpdateServiceError::S3Error(e) => write!(f, "S3 operation failed: {}", e),
            UpdateServiceError::SignatureNotFound(k) => {
                write!(f, "Signature file not found: {}", k)
            }
            UpdateServiceError::DownloadFileNotFound(d) => {
                write!(f, "Download file not found in: {}", d)
            }
            UpdateServiceError::PresignedUrlError(e) => {
                write!(f, "Failed to generate presigned URL: {}", e)
            }
            UpdateServiceError::ListObjectsError(e) => {
                write!(f, "Failed to list S3 objects: {}", e)
            }
        }
    }
}

impl std::error::Error for UpdateServiceError {}

/// Convert UpdateServiceError to HTTP error response
pub fn error_to_http_response(e: &anyhow::Error) -> (StatusCode, Json<ErrorResponse>) {
    match e.downcast_ref::<UpdateServiceError>() {
        Some(UpdateServiceError::InvalidVersion(v)) => {
            error!("Invalid version provided: {}", v);
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_version".to_string(),
                    message: "Invalid version format".to_string(),
                    details: Some(format!("Version '{}' is not a valid semantic version", v)),
                }),
            )
        }
        Some(UpdateServiceError::InvalidTargetArch(t)) => {
            error!("Invalid target architecture: {}", t);
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_target_arch".to_string(),
                    message: "Invalid target architecture format".to_string(),
                    details: Some(format!(
                        "Target architecture '{}' should be in format 'os-arch'",
                        t
                    )),
                }),
            )
        }
        Some(UpdateServiceError::InvalidChannel(c)) => {
            error!("Invalid channel: {}", c);
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_channel".to_string(),
                    message: "Invalid channel".to_string(),
                    details: Some(format!(
                        "Channel '{}' is not supported. Use 'nightly', 'release', or 'beta'",
                        c
                    )),
                }),
            )
        }
        Some(UpdateServiceError::S3Error(s3_err)) => {
            error!("S3 operation failed: {}", s3_err);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: "service_unavailable".to_string(),
                    message: "Update service temporarily unavailable".to_string(),
                    details: None, // Don't expose internal S3 errors
                }),
            )
        }
        Some(UpdateServiceError::DownloadFileNotFound(dir)) => {
            error!("Download file not found in directory: {}", dir);
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "download_not_found".to_string(),
                    message: "Update package not found".to_string(),
                    details: None,
                }),
            )
        }
        Some(UpdateServiceError::PresignedUrlError(url_err)) => {
            error!("Failed to generate presigned URL: {}", url_err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "url_generation_failed".to_string(),
                    message: "Failed to generate download URL".to_string(),
                    details: None,
                }),
            )
        }
        _ => {
            error!("Unexpected error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "internal_error".to_string(),
                    message: "Internal server error".to_string(),
                    details: None,
                }),
            )
        }
    }
}
