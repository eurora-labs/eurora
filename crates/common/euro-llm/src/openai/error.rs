//! OpenAI-specific error types.

use crate::core::{self, ProviderError};
use std::time::Duration;
use thiserror::Error;

/// OpenAI-specific error types.
#[derive(Debug, Error)]
pub enum OpenAIError {
    /// Authentication failed
    #[error("Authentication failed: {message}")]
    Authentication { message: String },

    /// Rate limited
    #[error("Rate limited: retry after {retry_after:?}")]
    RateLimit { retry_after: Option<Duration> },

    /// Invalid request
    #[error("Invalid request: {message}")]
    InvalidRequest { message: String },

    /// Service unavailable
    #[error("Service unavailable: {message}")]
    ServiceUnavailable { message: String },

    /// Content filtered
    #[error("Content filtered: {message}")]
    ContentFiltered { message: String },

    /// Model not found
    #[error("Model not found: {model}")]
    ModelNotFound { model: String },

    /// Insufficient quota
    #[error("Insufficient quota: {message}")]
    InsufficientQuota { message: String },

    /// Network error
    #[error("Network error: {source}")]
    Network {
        #[from]
        source: reqwest::Error,
    },

    /// JSON parsing error
    #[error("JSON parsing error: {source}")]
    Json {
        #[from]
        source: serde_json::Error,
    },

    /// Configuration error
    #[error("Configuration error: {source}")]
    Config {
        #[from]
        source: core::ConfigError,
    },

    /// Generic error
    #[error("OpenAI error: {message}")]
    Other { message: String },
}

impl ProviderError for OpenAIError {
    fn error_code(&self) -> Option<&str> {
        match self {
            Self::Authentication { .. } => Some("authentication_failed"),
            Self::RateLimit { .. } => Some("rate_limit_exceeded"),
            Self::InvalidRequest { .. } => Some("invalid_request"),
            Self::ServiceUnavailable { .. } => Some("service_unavailable"),
            Self::ContentFiltered { .. } => Some("content_filtered"),
            Self::ModelNotFound { .. } => Some("model_not_found"),
            Self::InsufficientQuota { .. } => Some("insufficient_quota"),
            Self::Network { .. } => Some("network_error"),
            Self::Json { .. } => Some("json_error"),
            Self::Config { .. } => Some("config_error"),
            Self::Other { .. } => Some("other_error"),
        }
    }

    fn is_retryable(&self) -> bool {
        match self {
            Self::RateLimit { .. } => true,
            Self::ServiceUnavailable { .. } => true,
            Self::Network { source } => {
                // Retry on timeout and connection errors
                source.is_timeout() || source.is_connect()
            }
            _ => false,
        }
    }

    fn is_rate_limited(&self) -> bool {
        matches!(self, Self::RateLimit { .. })
    }

    fn is_auth_error(&self) -> bool {
        matches!(
            self,
            Self::Authentication { .. } | Self::InsufficientQuota { .. }
        )
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RateLimit { retry_after } => *retry_after,
            _ => None,
        }
    }

    fn is_invalid_input(&self) -> bool {
        matches!(
            self,
            Self::InvalidRequest { .. } | Self::ModelNotFound { .. }
        )
    }

    fn is_service_unavailable(&self) -> bool {
        matches!(self, Self::ServiceUnavailable { .. })
    }

    fn is_content_filtered(&self) -> bool {
        matches!(self, Self::ContentFiltered { .. })
    }
}

impl OpenAIError {
    /// Create an error from an HTTP status code and response body.
    pub fn from_response(status: u16, body: &str) -> Self {
        // Try to parse the error response
        if let Ok(error_response) = serde_json::from_str::<OpenAIErrorResponse>(body) {
            Self::from_error_response(status, error_response)
        } else {
            // Fallback to generic error based on status code
            match status {
                401 => Self::Authentication {
                    message: "Invalid API key".to_string(),
                },
                403 => Self::Authentication {
                    message: "Forbidden".to_string(),
                },
                429 => Self::RateLimit { retry_after: None },
                400 => Self::InvalidRequest {
                    message: body.to_string(),
                },
                404 => Self::InvalidRequest {
                    message: "Not found".to_string(),
                },
                500..=599 => Self::ServiceUnavailable {
                    message: format!("Server error: {status}"),
                },
                _ => Self::Other {
                    message: format!("HTTP {status}: {body}"),
                },
            }
        }
    }

    /// Create an error from a parsed OpenAI error response.
    pub fn from_error_response(status: u16, response: OpenAIErrorResponse) -> Self {
        let error = &response.error;

        match error.error_type.as_deref() {
            Some("invalid_api_key") => Self::Authentication {
                message: error.message.clone(),
            },
            Some("insufficient_quota") => Self::InsufficientQuota {
                message: error.message.clone(),
            },
            Some("model_not_found") => Self::ModelNotFound {
                model: error.message.clone(),
            },
            Some("rate_limit_exceeded") => Self::RateLimit {
                retry_after: None, // Could parse from headers
            },
            Some("content_filter") => Self::ContentFiltered {
                message: error.message.clone(),
            },
            _ => match status {
                400 => Self::InvalidRequest {
                    message: error.message.clone(),
                },
                401 | 403 => Self::Authentication {
                    message: error.message.clone(),
                },
                429 => Self::RateLimit { retry_after: None },
                500..=599 => Self::ServiceUnavailable {
                    message: error.message.clone(),
                },
                _ => Self::Other {
                    message: error.message.clone(),
                },
            },
        }
    }
}

/// OpenAI API error response structure.
#[derive(Debug, serde::Deserialize)]
pub struct OpenAIErrorResponse {
    pub error: OpenAIErrorDetail,
}

/// OpenAI API error detail.
#[derive(Debug, serde::Deserialize)]
pub struct OpenAIErrorDetail {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub param: Option<String>,
    pub code: Option<String>,
}
