//! Error types for agent-chain.
//!
//! This module provides error types used across the crate for handling
//! various failure modes in chat model operations.

use thiserror::Error;

/// Result type alias for agent-chain operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for agent-chain operations.
#[derive(Debug, Error)]
pub enum Error {
    /// Error from HTTP requests.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Error parsing JSON.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// API returned an error response.
    #[error("API error ({status}): {message}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// Error message from the API.
        message: String,
    },

    /// Missing required configuration.
    #[error("Missing configuration: {0}")]
    MissingConfig(String),

    /// Unsupported provider.
    #[error("Unsupported provider: {0}")]
    UnsupportedProvider(String),

    /// Unable to infer provider from model name.
    #[error("Unable to infer provider for model '{0}'. Please specify model_provider explicitly.")]
    UnableToInferProvider(String),

    /// Invalid model configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Tool invocation error.
    #[error("Tool invocation error: {0}")]
    ToolInvocation(String),

    /// Feature or method not implemented.
    #[error("Not implemented: {0}")]
    NotImplemented(String),

    /// Generic error with message.
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Create a new API error.
    pub fn api(status: u16, message: impl Into<String>) -> Self {
        Self::Api {
            status,
            message: message.into(),
        }
    }

    /// Create a missing config error.
    pub fn missing_config(key: impl Into<String>) -> Self {
        Self::MissingConfig(key.into())
    }

    /// Create an unsupported provider error.
    pub fn unsupported_provider(provider: impl Into<String>) -> Self {
        Self::UnsupportedProvider(provider.into())
    }

    /// Create an unable to infer provider error.
    pub fn unable_to_infer_provider(model: impl Into<String>) -> Self {
        Self::UnableToInferProvider(model.into())
    }

    /// Create a generic error.
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }

    /// Whether this error is worth retrying.
    ///
    /// Returns `false` for client errors (4xx) except 429 (rate limit),
    /// and for parse/config errors that won't resolve on retry.
    /// Returns `true` for transient network/server errors.
    pub fn is_retryable(&self) -> bool {
        match self {
            // Network / transport errors are retryable
            Self::Http(_) => true,
            // Server errors (5xx) and 429 rate-limit are retryable
            Self::Api { status, .. } => *status == 429 || *status >= 500,
            // Everything else (parse, config, etc.) is not
            _ => false,
        }
    }
}
