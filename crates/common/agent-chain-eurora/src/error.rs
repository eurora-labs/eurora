//! Error types for agent-chain-eurora.

use thiserror::Error;
use tonic::Status;

/// Errors that can occur when using the Eurora gRPC provider.
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

impl EuroraError {
    /// Returns the gRPC error code if available
    pub fn error_code(&self) -> Option<&str> {
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

    /// Returns true if this error is retryable
    pub fn is_retryable(&self) -> bool {
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

    /// Returns true if this is a rate limiting error
    pub fn is_rate_limited(&self) -> bool {
        match self {
            EuroraError::RateLimit => true,
            EuroraError::Status(status) => status.code() == tonic::Code::ResourceExhausted,
            _ => false,
        }
    }

    /// Returns true if this is an authentication error
    pub fn is_auth_error(&self) -> bool {
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

impl From<EuroraError> for agent_chain_core::Error {
    fn from(error: EuroraError) -> Self {
        match error {
            EuroraError::Transport(e) => agent_chain_core::Error::Other(e.to_string()),
            EuroraError::Status(s) => {
                let code = s.code();
                let message = s.message().to_string();
                match code {
                    tonic::Code::Unauthenticated | tonic::Code::PermissionDenied => {
                        agent_chain_core::Error::MissingConfig(format!(
                            "Authentication error: {}",
                            message
                        ))
                    }
                    tonic::Code::InvalidArgument => agent_chain_core::Error::InvalidConfig(message),
                    _ => agent_chain_core::Error::Other(format!(
                        "gRPC error ({}): {}",
                        code, message
                    )),
                }
            }
            EuroraError::Serialization(e) => agent_chain_core::Error::Json(e),
            EuroraError::InvalidConfig(msg) => agent_chain_core::Error::InvalidConfig(msg),
            EuroraError::Connection(msg) => {
                agent_chain_core::Error::Other(format!("Connection error: {}", msg))
            }
            EuroraError::Timeout => agent_chain_core::Error::Other("Request timeout".to_string()),
            EuroraError::InvalidResponse(msg) => {
                agent_chain_core::Error::Other(format!("Invalid response: {}", msg))
            }
            EuroraError::Stream(msg) => {
                agent_chain_core::Error::Other(format!("Stream error: {}", msg))
            }
            EuroraError::Authentication(msg) => {
                agent_chain_core::Error::MissingConfig(format!("Authentication error: {}", msg))
            }
            EuroraError::RateLimit => agent_chain_core::Error::Api {
                status: 429,
                message: "Rate limit exceeded".to_string(),
            },
            EuroraError::ServiceUnavailable => agent_chain_core::Error::Api {
                status: 503,
                message: "Service unavailable".to_string(),
            },
            EuroraError::Other(msg) => agent_chain_core::Error::Other(msg),
        }
    }
}
