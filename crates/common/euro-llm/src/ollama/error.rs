//! Ollama-specific error types.

use crate::core::ProviderError;
use std::time::Duration;
use thiserror::Error;

/// Ollama-specific error types.
#[derive(Debug, Error)]
pub enum OllamaError {
    /// Model not found or not loaded
    #[error("Model not found: {model}")]
    ModelNotFound { model: String },

    /// Model not loaded (needs to be pulled first)
    #[error("Model not loaded: {model}")]
    ModelNotLoaded { model: String },

    /// Invalid request
    #[error("Invalid request: {message}")]
    InvalidRequest { message: String },

    /// Service unavailable (Ollama server not running)
    #[error("Service unavailable: {message}")]
    ServiceUnavailable { message: String },

    /// Resource exhausted (out of memory, etc.)
    #[error("Resource exhausted: {message}")]
    ResourceExhausted { message: String },

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
        source: crate::core::ConfigError,
    },

    /// Generic error
    #[error("Ollama error: {message}")]
    Other { message: String },
}

impl ProviderError for OllamaError {
    fn error_code(&self) -> Option<&str> {
        match self {
            Self::ModelNotFound { .. } => Some("model_not_found"),
            Self::ModelNotLoaded { .. } => Some("model_not_loaded"),
            Self::InvalidRequest { .. } => Some("invalid_request"),
            Self::ServiceUnavailable { .. } => Some("service_unavailable"),
            Self::ResourceExhausted { .. } => Some("resource_exhausted"),
            Self::Network { .. } => Some("network_error"),
            Self::Json { .. } => Some("json_error"),
            Self::Config { .. } => Some("config_error"),
            Self::Other { .. } => Some("other_error"),
        }
    }

    fn is_retryable(&self) -> bool {
        match self {
            Self::ServiceUnavailable { .. } => true,
            Self::ResourceExhausted { .. } => true,
            Self::Network { source } => {
                // Retry on timeout and connection errors
                source.is_timeout() || source.is_connect()
            }
            _ => false,
        }
    }

    fn is_rate_limited(&self) -> bool {
        // Ollama doesn't typically have rate limiting, but resource exhaustion
        // can be treated similarly
        matches!(self, Self::ResourceExhausted { .. })
    }

    fn is_auth_error(&self) -> bool {
        // Ollama doesn't have authentication by default
        false
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::ResourceExhausted { .. } => Some(Duration::from_secs(5)),
            Self::ServiceUnavailable { .. } => Some(Duration::from_secs(2)),
            _ => None,
        }
    }

    fn is_invalid_input(&self) -> bool {
        matches!(
            self,
            Self::InvalidRequest { .. } | Self::ModelNotFound { .. } | Self::ModelNotLoaded { .. }
        )
    }

    fn is_service_unavailable(&self) -> bool {
        matches!(self, Self::ServiceUnavailable { .. })
    }

    fn is_content_filtered(&self) -> bool {
        // Ollama doesn't typically filter content
        false
    }
}

impl OllamaError {
    /// Create an error from an HTTP status code and response body.
    pub fn from_response(status: u16, body: &str) -> Self {
        // Try to parse the error response
        if let Ok(error_response) = serde_json::from_str::<OllamaErrorResponse>(body) {
            Self::from_error_response(status, error_response)
        } else {
            // Fallback to generic error based on status code
            match status {
                400 => Self::InvalidRequest {
                    message: body.to_string(),
                },
                404 => {
                    // Check if it's a model not found error
                    if body.contains("model") && body.contains("not found") {
                        Self::ModelNotFound {
                            model: "unknown".to_string(),
                        }
                    } else {
                        Self::InvalidRequest {
                            message: "Not found".to_string(),
                        }
                    }
                }
                500..=599 => Self::ServiceUnavailable {
                    message: format!("Server error: {status}"),
                },
                _ => Self::Other {
                    message: format!("HTTP {status}: {body}"),
                },
            }
        }
    }

    /// Create an error from a parsed Ollama error response.
    pub fn from_error_response(status: u16, response: OllamaErrorResponse) -> Self {
        let message = response.error;

        // Check for specific error patterns
        if message.contains("model") && message.contains("not found") {
            Self::ModelNotFound {
                model: extract_model_name(&message).unwrap_or_else(|| "unknown".to_string()),
            }
        } else if message.contains("model") && message.contains("not loaded") {
            Self::ModelNotLoaded {
                model: extract_model_name(&message).unwrap_or_else(|| "unknown".to_string()),
            }
        } else if message.contains("out of memory") || message.contains("resource") {
            Self::ResourceExhausted { message }
        } else {
            match status {
                400 => Self::InvalidRequest { message },
                404 => Self::ModelNotFound {
                    model: "unknown".to_string(),
                },
                500..=599 => Self::ServiceUnavailable { message },
                _ => Self::Other { message },
            }
        }
    }

    /// Create a model not found error.
    pub fn model_not_found(model: impl Into<String>) -> Self {
        Self::ModelNotFound {
            model: model.into(),
        }
    }

    /// Create a model not loaded error.
    pub fn model_not_loaded(model: impl Into<String>) -> Self {
        Self::ModelNotLoaded {
            model: model.into(),
        }
    }

    /// Create a service unavailable error.
    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::ServiceUnavailable {
            message: message.into(),
        }
    }
}

/// Ollama API error response structure.
#[derive(Debug, serde::Deserialize)]
pub struct OllamaErrorResponse {
    pub error: String,
}

/// Extract model name from error message.
fn extract_model_name(message: &str) -> Option<String> {
    // Try to extract model name from common error patterns
    if let Some(start) = message.find("model '") {
        let start = start + 7; // Length of "model '"
        if let Some(end) = message[start..].find('\'') {
            return Some(message[start..start + end].to_string());
        }
    }

    if let Some(start) = message.find("model \"") {
        let start = start + 7; // Length of "model \""
        if let Some(end) = message[start..].find('"') {
            return Some(message[start..start + end].to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(
            OllamaError::ModelNotFound {
                model: "test".to_string()
            }
            .error_code(),
            Some("model_not_found")
        );
        assert_eq!(
            OllamaError::ServiceUnavailable {
                message: "test".to_string()
            }
            .error_code(),
            Some("service_unavailable")
        );
    }

    #[test]
    fn test_retryable_errors() {
        assert!(
            OllamaError::ServiceUnavailable {
                message: "test".to_string()
            }
            .is_retryable()
        );
        assert!(
            OllamaError::ResourceExhausted {
                message: "test".to_string()
            }
            .is_retryable()
        );
        assert!(
            !OllamaError::ModelNotFound {
                model: "test".to_string()
            }
            .is_retryable()
        );
    }

    #[test]
    fn test_extract_model_name() {
        assert_eq!(
            extract_model_name("model 'llama2' not found"),
            Some("llama2".to_string())
        );
        assert_eq!(
            extract_model_name("model \"codellama\" not loaded"),
            Some("codellama".to_string())
        );
        assert_eq!(extract_model_name("generic error"), None);
    }

    #[test]
    fn test_from_response() {
        let error = OllamaError::from_response(404, "model not found");
        assert!(matches!(error, OllamaError::ModelNotFound { .. }));

        let error = OllamaError::from_response(500, "internal server error");
        assert!(matches!(error, OllamaError::ServiceUnavailable { .. }));
    }
}
