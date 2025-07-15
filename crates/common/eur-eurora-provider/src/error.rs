//! Error types for gRPC providers.

use ferrous_llm_core::error::ProviderError;
use thiserror::Error;
use tonic::Status;

/// Errors that can occur when using gRPC providers.
#[derive(Debug, Error)]
pub enum GrpcError {
    /// gRPC transport error
    #[error("gRPC transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    /// gRPC status error
    #[error("gRPC status error: {0}")]
    Status(#[from] tonic::Status),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// Request timeout
    #[error("Request timeout")]
    Timeout,

    /// Invalid response format
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),

    /// Stream error
    #[error("Stream error: {0}")]
    Stream(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded")]
    RateLimit,

    /// Service unavailable
    #[error("Service unavailable")]
    ServiceUnavailable,

    /// Generic error with message
    #[error("{0}")]
    Other(String),
}

impl ProviderError for GrpcError {
    fn error_code(&self) -> Option<&str> {
        match self {
            GrpcError::Transport(_) => Some("transport_error"),
            GrpcError::Status(status) => Some(match status.code() {
                tonic::Code::Ok => "ok",
                tonic::Code::Cancelled => "cancelled",
                tonic::Code::Unknown => "unknown",
                tonic::Code::InvalidArgument => "invalid_argument",
                tonic::Code::DeadlineExceeded => "deadline_exceeded",
                tonic::Code::NotFound => "not_found",
                tonic::Code::AlreadyExists => "already_exists",
                tonic::Code::PermissionDenied => "permission_denied",
                tonic::Code::ResourceExhausted => "resource_exhausted",
                tonic::Code::FailedPrecondition => "failed_precondition",
                tonic::Code::Aborted => "aborted",
                tonic::Code::OutOfRange => "out_of_range",
                tonic::Code::Unimplemented => "unimplemented",
                tonic::Code::Internal => "internal",
                tonic::Code::Unavailable => "unavailable",
                tonic::Code::DataLoss => "data_loss",
                tonic::Code::Unauthenticated => "unauthenticated",
            }),
            GrpcError::Serialization(_) => Some("serialization_error"),
            GrpcError::InvalidConfig(_) => Some("invalid_config"),
            GrpcError::Connection(_) => Some("connection_error"),
            GrpcError::Timeout => Some("timeout"),
            GrpcError::InvalidResponse(_) => Some("invalid_response"),
            GrpcError::Stream(_) => Some("stream_error"),
            GrpcError::Authentication(_) => Some("authentication_error"),
            GrpcError::RateLimit => Some("rate_limit"),
            GrpcError::ServiceUnavailable => Some("service_unavailable"),
            GrpcError::Other(_) => Some("other"),
        }
    }

    fn is_retryable(&self) -> bool {
        match self {
            GrpcError::Transport(_) => true,
            GrpcError::Status(status) => {
                matches!(
                    status.code(),
                    tonic::Code::Unavailable
                        | tonic::Code::DeadlineExceeded
                        | tonic::Code::ResourceExhausted
                        | tonic::Code::Internal
                )
            }
            GrpcError::Connection(_) => true,
            GrpcError::Timeout => true,
            GrpcError::RateLimit => true,
            GrpcError::ServiceUnavailable => true,
            _ => false,
        }
    }

    fn is_rate_limited(&self) -> bool {
        match self {
            GrpcError::RateLimit => true,
            GrpcError::Status(status) => status.code() == tonic::Code::ResourceExhausted,
            _ => false,
        }
    }

    fn is_auth_error(&self) -> bool {
        match self {
            GrpcError::Authentication(_) => true,
            GrpcError::Status(status) => {
                matches!(
                    status.code(),
                    tonic::Code::Unauthenticated | tonic::Code::PermissionDenied
                )
            }
            _ => false,
        }
    }

    fn retry_after(&self) -> Option<std::time::Duration> {
        match self {
            GrpcError::RateLimit => Some(std::time::Duration::from_secs(60)),
            GrpcError::Status(status) if status.code() == tonic::Code::ResourceExhausted => {
                Some(std::time::Duration::from_secs(30))
            }
            _ => None,
        }
    }

    fn is_invalid_input(&self) -> bool {
        match self {
            GrpcError::InvalidConfig(_) => true,
            GrpcError::Serialization(_) => true,
            GrpcError::Status(status) => {
                matches!(
                    status.code(),
                    tonic::Code::InvalidArgument | tonic::Code::OutOfRange
                )
            }
            _ => false,
        }
    }

    fn is_service_unavailable(&self) -> bool {
        match self {
            GrpcError::ServiceUnavailable => true,
            GrpcError::Connection(_) => true,
            GrpcError::Status(status) => status.code() == tonic::Code::Unavailable,
            _ => false,
        }
    }

    fn is_content_filtered(&self) -> bool {
        // gRPC doesn't have a standard content filtering error code
        false
    }
}

impl From<GrpcError> for Status {
    fn from(error: GrpcError) -> Self {
        match error {
            GrpcError::Transport(_) => Status::unavailable(error.to_string()),
            GrpcError::Status(status) => status,
            GrpcError::Serialization(_) => Status::invalid_argument(error.to_string()),
            GrpcError::InvalidConfig(_) => Status::invalid_argument(error.to_string()),
            GrpcError::Connection(_) => Status::unavailable(error.to_string()),
            GrpcError::Timeout => Status::deadline_exceeded(error.to_string()),
            GrpcError::InvalidResponse(_) => Status::internal(error.to_string()),
            GrpcError::Stream(_) => Status::internal(error.to_string()),
            GrpcError::Authentication(_) => Status::unauthenticated(error.to_string()),
            GrpcError::RateLimit => Status::resource_exhausted(error.to_string()),
            GrpcError::ServiceUnavailable => Status::unavailable(error.to_string()),
            GrpcError::Other(_) => Status::internal(error.to_string()),
        }
    }
}
