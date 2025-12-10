//! Anthropic-specific error types.

use crate::ProviderError;
use std::time::Duration;
use thiserror::Error;

/// Anthropic-specific error types.
#[derive(Debug, Error)]
pub enum AnthropicError {
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

    /// Request too large
    #[error("Request too large: {message}")]
    RequestTooLarge { message: String },

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
        source: crate::ConfigError,
    },

    /// Generic error
    #[error("Anthropic error: {message}")]
    Other { message: String },
}

impl ProviderError for AnthropicError {
    fn error_code(&self) -> Option<&str> {
        match self {
            Self::Authentication { .. } => Some("authentication_failed"),
            Self::RateLimit { .. } => Some("rate_limit_exceeded"),
            Self::InvalidRequest { .. } => Some("invalid_request"),
            Self::ServiceUnavailable { .. } => Some("service_unavailable"),
            Self::ContentFiltered { .. } => Some("content_filtered"),
            Self::ModelNotFound { .. } => Some("model_not_found"),
            Self::InsufficientQuota { .. } => Some("insufficient_quota"),
            Self::RequestTooLarge { .. } => Some("request_too_large"),
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
            Self::InvalidRequest { .. } | Self::ModelNotFound { .. } | Self::RequestTooLarge { .. }
        )
    }

    fn is_service_unavailable(&self) -> bool {
        matches!(self, Self::ServiceUnavailable { .. })
    }

    fn is_content_filtered(&self) -> bool {
        matches!(self, Self::ContentFiltered { .. })
    }
}

impl AnthropicError {
    /// Create an error from an HTTP status code and response body.
    pub fn from_response(status: u16, body: &str) -> Self {
        // Try to parse the error response
        if let Ok(error_response) = serde_json::from_str::<AnthropicErrorResponse>(body) {
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
                413 => Self::RequestTooLarge {
                    message: "Request entity too large".to_string(),
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

    /// Create an error from a parsed Anthropic error response.
    pub fn from_error_response(status: u16, response: AnthropicErrorResponse) -> Self {
        let error = &response.error;

        match error.error_type.as_str() {
            "authentication_error" => Self::Authentication {
                message: error.message.clone(),
            },
            "permission_error" => Self::Authentication {
                message: error.message.clone(),
            },
            "not_found_error" => Self::ModelNotFound {
                model: error.message.clone(),
            },
            "rate_limit_error" => Self::RateLimit {
                retry_after: None, // Could parse from headers
            },
            "api_error" => Self::ServiceUnavailable {
                message: error.message.clone(),
            },
            "overloaded_error" => Self::ServiceUnavailable {
                message: error.message.clone(),
            },
            "invalid_request_error" => Self::InvalidRequest {
                message: error.message.clone(),
            },
            _ => match status {
                400 => Self::InvalidRequest {
                    message: error.message.clone(),
                },
                401 | 403 => Self::Authentication {
                    message: error.message.clone(),
                },
                404 => Self::ModelNotFound {
                    model: error.message.clone(),
                },
                413 => Self::RequestTooLarge {
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

/// Anthropic API error response structure.
#[derive(Debug, serde::Deserialize, Clone)]
pub struct AnthropicErrorResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    pub error: AnthropicErrorDetail,
}

/// Anthropic API error detail.
#[derive(Debug, serde::Deserialize, Clone)]
pub struct AnthropicErrorDetail {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}
