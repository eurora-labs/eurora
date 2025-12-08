//! Error handling for LLM providers.
//!
//! This module defines common error patterns and traits that all providers
//! should implement, allowing for consistent error handling across the ecosystem.

use std::error::Error;
use std::time::Duration;
use thiserror::Error;

/// Common trait for all provider errors.
///
/// This trait provides a consistent interface for error handling across
/// different providers, allowing clients to handle errors generically
/// while still preserving provider-specific error information.
pub trait ProviderError: Error + Send + Sync + 'static {
    /// Get the provider-specific error code if available.
    fn error_code(&self) -> Option<&str>;

    /// Check if this error is retryable.
    ///
    /// Returns true if the operation that caused this error can be safely retried.
    fn is_retryable(&self) -> bool;

    /// Check if this error is due to rate limiting.
    ///
    /// Returns true if the error was caused by hitting rate limits.
    fn is_rate_limited(&self) -> bool;

    /// Check if this error is due to authentication issues.
    ///
    /// Returns true if the error was caused by invalid or missing credentials.
    fn is_auth_error(&self) -> bool;

    /// Get the suggested retry delay if this is a rate limit error.
    ///
    /// Returns the duration to wait before retrying, if specified by the provider.
    fn retry_after(&self) -> Option<Duration>;

    /// Check if this error is due to invalid input.
    ///
    /// Returns true if the error was caused by invalid request parameters.
    fn is_invalid_input(&self) -> bool {
        false
    }

    /// Check if this error is due to service unavailability.
    ///
    /// Returns true if the error was caused by the service being temporarily unavailable.
    fn is_service_unavailable(&self) -> bool {
        false
    }

    /// Check if this error is due to content filtering.
    ///
    /// Returns true if the error was caused by content being filtered or blocked.
    fn is_content_filtered(&self) -> bool {
        false
    }
}

/// Common configuration errors.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Missing required configuration field
    #[error("Missing required configuration: {field}")]
    MissingField { field: String },

    /// Invalid configuration value
    #[error("Invalid configuration value for {field}: {message}")]
    InvalidValue { field: String, message: String },

    /// Invalid URL format
    #[error("Invalid URL: {url}")]
    InvalidUrl { url: String },

    /// Invalid API key format
    #[error("Invalid API key format")]
    InvalidApiKey,

    /// Configuration validation failed
    #[error("Configuration validation failed: {message}")]
    ValidationFailed { message: String },
}

/// Common request errors.
#[derive(Debug, Error)]
pub enum RequestError {
    /// Invalid request parameters
    #[error("Invalid request: {message}")]
    InvalidRequest { message: String },

    /// Request too large
    #[error("Request too large: {size} bytes exceeds limit of {limit} bytes")]
    RequestTooLarge { size: usize, limit: usize },

    /// Unsupported feature
    #[error("Unsupported feature: {feature}")]
    UnsupportedFeature { feature: String },

    /// Invalid message format
    #[error("Invalid message format: {message}")]
    InvalidMessage { message: String },

    /// Invalid tool definition
    #[error("Invalid tool definition: {message}")]
    InvalidTool { message: String },
}

/// Common response errors.
#[derive(Debug, Error)]
pub enum ResponseError {
    /// Failed to parse response
    #[error("Failed to parse response: {message}")]
    ParseError { message: String },

    /// Unexpected response format
    #[error("Unexpected response format: expected {expected}, got {actual}")]
    UnexpectedFormat { expected: String, actual: String },

    /// Missing required response field
    #[error("Missing required response field: {field}")]
    MissingField { field: String },

    /// Invalid response data
    #[error("Invalid response data: {message}")]
    InvalidData { message: String },
}

/// Common network errors.
#[derive(Debug, Error)]
pub enum NetworkError {
    /// HTTP request failed
    #[error("HTTP request failed: {status}")]
    HttpError { status: u16, message: String },

    /// Connection timeout
    #[error("Connection timeout after {timeout:?}")]
    Timeout { timeout: Duration },

    /// Connection failed
    #[error("Connection failed: {message}")]
    ConnectionFailed { message: String },

    /// DNS resolution failed
    #[error("DNS resolution failed: {host}")]
    DnsError { host: String },

    /// TLS/SSL error
    #[error("TLS error: {message}")]
    TlsError { message: String },
}

/// A generic error type that can wrap any provider error.
#[derive(Debug, Error)]
pub enum LlmError<E: ProviderError> {
    /// Provider-specific error
    #[error("Provider error: {0}")]
    Provider(E),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Request error
    #[error("Request error: {0}")]
    Request(#[from] RequestError),

    /// Response error
    #[error("Response error: {0}")]
    Response(#[from] ResponseError),

    /// Network error
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),

    /// Memory error (for memory-enabled providers)
    #[error("Memory error: {message}")]
    Memory { message: String },

    /// Tool execution error
    #[error("Tool execution error: {message}")]
    ToolExecution { message: String },

    /// Generic error for cases not covered above
    #[error("Error: {message}")]
    Other { message: String },
}

impl<E: ProviderError> ProviderError for LlmError<E> {
    fn error_code(&self) -> Option<&str> {
        match self {
            Self::Provider(e) => e.error_code(),
            Self::Config(_) => Some("config_error"),
            Self::Request(_) => Some("request_error"),
            Self::Response(_) => Some("response_error"),
            Self::Network(_) => Some("network_error"),
            Self::Memory { .. } => Some("memory_error"),
            Self::ToolExecution { .. } => Some("tool_error"),
            Self::Other { .. } => Some("other_error"),
        }
    }

    fn is_retryable(&self) -> bool {
        match self {
            Self::Provider(e) => e.is_retryable(),
            Self::Network(NetworkError::Timeout { .. }) => true,
            Self::Network(NetworkError::ConnectionFailed { .. }) => true,
            Self::Network(NetworkError::HttpError { status, .. }) => {
                // Retry on 5xx errors and some 4xx errors
                *status >= 500 || *status == 429 || *status == 408
            }
            _ => false,
        }
    }

    fn is_rate_limited(&self) -> bool {
        match self {
            Self::Provider(e) => e.is_rate_limited(),
            Self::Network(NetworkError::HttpError { status, .. }) => *status == 429,
            _ => false,
        }
    }

    fn is_auth_error(&self) -> bool {
        match self {
            Self::Provider(e) => e.is_auth_error(),
            Self::Config(ConfigError::InvalidApiKey) => true,
            Self::Network(NetworkError::HttpError { status, .. }) => {
                *status == 401 || *status == 403
            }
            _ => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::Provider(e) => e.retry_after(),
            Self::Network(NetworkError::HttpError { status, .. }) if *status == 429 => {
                // Default retry after for rate limits
                Some(Duration::from_secs(60))
            }
            _ => None,
        }
    }

    fn is_invalid_input(&self) -> bool {
        match self {
            Self::Provider(e) => e.is_invalid_input(),
            Self::Request(_) => true,
            Self::Config(_) => true,
            Self::Network(NetworkError::HttpError { status, .. }) => *status == 400,
            _ => false,
        }
    }

    fn is_service_unavailable(&self) -> bool {
        match self {
            Self::Provider(e) => e.is_service_unavailable(),
            Self::Network(NetworkError::HttpError { status, .. }) => {
                *status == 503 || *status == 502 || *status == 504
            }
            Self::Network(NetworkError::ConnectionFailed { .. }) => true,
            _ => false,
        }
    }

    fn is_content_filtered(&self) -> bool {
        match self {
            Self::Provider(e) => e.is_content_filtered(),
            _ => false,
        }
    }
}

/// Result type alias for provider operations.
pub type ProviderResult<T, E> = Result<T, E>;

/// Result type alias for LLM operations with generic error.
pub type LlmResult<T, E> = Result<T, LlmError<E>>;

// Utility functions for creating common errors
impl ConfigError {
    /// Create a missing field error
    pub fn missing_field(field: impl Into<String>) -> Self {
        Self::MissingField {
            field: field.into(),
        }
    }

    /// Create an invalid value error
    pub fn invalid_value(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidValue {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create an invalid URL error
    pub fn invalid_url(url: impl Into<String>) -> Self {
        Self::InvalidUrl { url: url.into() }
    }

    /// Create a validation failed error
    pub fn validation_failed(message: impl Into<String>) -> Self {
        Self::ValidationFailed {
            message: message.into(),
        }
    }
}

impl RequestError {
    /// Create an invalid request error
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest {
            message: message.into(),
        }
    }

    /// Create a request too large error
    pub fn request_too_large(size: usize, limit: usize) -> Self {
        Self::RequestTooLarge { size, limit }
    }

    /// Create an unsupported feature error
    pub fn unsupported_feature(feature: impl Into<String>) -> Self {
        Self::UnsupportedFeature {
            feature: feature.into(),
        }
    }

    /// Create an invalid message error
    pub fn invalid_message(message: impl Into<String>) -> Self {
        Self::InvalidMessage {
            message: message.into(),
        }
    }

    /// Create an invalid tool error
    pub fn invalid_tool(message: impl Into<String>) -> Self {
        Self::InvalidTool {
            message: message.into(),
        }
    }
}

impl ResponseError {
    /// Create a parse error
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::ParseError {
            message: message.into(),
        }
    }

    /// Create an unexpected format error
    pub fn unexpected_format(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::UnexpectedFormat {
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Create a missing field error
    pub fn missing_field(field: impl Into<String>) -> Self {
        Self::MissingField {
            field: field.into(),
        }
    }

    /// Create an invalid data error
    pub fn invalid_data(message: impl Into<String>) -> Self {
        Self::InvalidData {
            message: message.into(),
        }
    }
}

impl NetworkError {
    /// Create an HTTP error
    pub fn http_error(status: u16, message: impl Into<String>) -> Self {
        Self::HttpError {
            status,
            message: message.into(),
        }
    }

    /// Create a timeout error
    pub fn timeout(timeout: Duration) -> Self {
        Self::Timeout { timeout }
    }

    /// Create a connection failed error
    pub fn connection_failed(message: impl Into<String>) -> Self {
        Self::ConnectionFailed {
            message: message.into(),
        }
    }

    /// Create a DNS error
    pub fn dns_error(host: impl Into<String>) -> Self {
        Self::DnsError { host: host.into() }
    }

    /// Create a TLS error
    pub fn tls_error(message: impl Into<String>) -> Self {
        Self::TlsError {
            message: message.into(),
        }
    }
}
