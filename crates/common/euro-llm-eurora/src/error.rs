//! Error types for gRPC providers.

use euro_llm::ProviderError;
use thiserror::Error;
use tonic::Status;

/// Errors that can occur when using gRPC providers.
#[derive(Debug, Error)]
pub enum EuroraError {
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

impl ProviderError for EuroraError {
    fn error_code(&self) -> Option<&str> {
        match self {
            EuroraError::Transport(_) => Some("transport_error"),
            EuroraError::Status(status) => Some(match status.code() {
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
            EuroraError::Serialization(_) => Some("serialization_error"),
            EuroraError::InvalidConfig(_) => Some("invalid_config"),
            EuroraError::Connection(_) => Some("connection_error"),
            EuroraError::Timeout => Some("timeout"),
            EuroraError::InvalidResponse(_) => Some("invalid_response"),
            EuroraError::Stream(_) => Some("stream_error"),
            EuroraError::Authentication(_) => Some("authentication_error"),
            EuroraError::RateLimit => Some("rate_limit"),
            EuroraError::ServiceUnavailable => Some("service_unavailable"),
            EuroraError::Other(_) => Some("other"),
        }
    }

    fn is_retryable(&self) -> bool {
        match self {
            EuroraError::Transport(_) => true,
            EuroraError::Status(status) => {
                matches!(
                    status.code(),
                    tonic::Code::Unavailable
                        | tonic::Code::DeadlineExceeded
                        | tonic::Code::ResourceExhausted
                        | tonic::Code::Internal
                )
            }
            EuroraError::Connection(_) => true,
            EuroraError::Timeout => true,
            EuroraError::RateLimit => true,
            EuroraError::ServiceUnavailable => true,
            _ => false,
        }
    }

    fn is_rate_limited(&self) -> bool {
        match self {
            EuroraError::RateLimit => true,
            EuroraError::Status(status) => status.code() == tonic::Code::ResourceExhausted,
            _ => false,
        }
    }

    fn is_auth_error(&self) -> bool {
        match self {
            EuroraError::Authentication(_) => true,
            EuroraError::Status(status) => {
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
            EuroraError::RateLimit => Some(std::time::Duration::from_secs(60)),
            EuroraError::Status(status) if status.code() == tonic::Code::ResourceExhausted => {
                Some(std::time::Duration::from_secs(30))
            }
            _ => None,
        }
    }

    fn is_invalid_input(&self) -> bool {
        match self {
            EuroraError::InvalidConfig(_) => true,
            EuroraError::Serialization(_) => true,
            EuroraError::Status(status) => {
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
            EuroraError::ServiceUnavailable => true,
            EuroraError::Connection(_) => true,
            EuroraError::Status(status) => status.code() == tonic::Code::Unavailable,
            _ => false,
        }
    }

    fn is_content_filtered(&self) -> bool {
        // gRPC doesn't have a standard content filtering error code
        false
    }
}

impl From<EuroraError> for Status {
    fn from(error: EuroraError) -> Self {
        match error {
            EuroraError::Transport(_) => Status::unavailable(error.to_string()),
            EuroraError::Status(status) => status,
            EuroraError::Serialization(_) => Status::invalid_argument(error.to_string()),
            EuroraError::InvalidConfig(_) => Status::invalid_argument(error.to_string()),
            EuroraError::Connection(_) => Status::unavailable(error.to_string()),
            EuroraError::Timeout => Status::deadline_exceeded(error.to_string()),
            EuroraError::InvalidResponse(_) => Status::internal(error.to_string()),
            EuroraError::Stream(_) => Status::internal(error.to_string()),
            EuroraError::Authentication(_) => Status::unauthenticated(error.to_string()),
            EuroraError::RateLimit => Status::resource_exhausted(error.to_string()),
            EuroraError::ServiceUnavailable => Status::unavailable(error.to_string()),
            EuroraError::Other(_) => Status::internal(error.to_string()),
        }
    }
}
