//! Configuration system for LLM providers.
//!
//! This module defines the configuration traits and utilities that providers
//! use to manage their settings, validation, and initialization.

use crate::error::ConfigError;
use serde::{Deserialize, Serialize, Serializer};
use std::fmt::Debug;
use std::time::Duration;

/// Trait for provider configuration types.
///
/// All provider configurations must implement this trait to provide
/// consistent validation and construction patterns.
pub trait ProviderConfig: Clone + Debug + Send + Sync {
    /// The provider type that this configuration creates
    type Provider;

    /// Build a provider instance from this configuration.
    ///
    /// This method should validate the configuration and create a new
    /// provider instance. It should fail if the configuration is invalid.
    fn build(self) -> Result<Self::Provider, ConfigError>;

    /// Validate the configuration without building a provider.
    ///
    /// This method should check that all required fields are present
    /// and that all values are valid.
    fn validate(&self) -> Result<(), ConfigError>;
}

/// A secure string type for sensitive configuration values like API keys.
///
/// This type ensures that sensitive values are not accidentally logged
/// or displayed in debug output. It also redacts the value during serialization
/// to avoid accidentally exposing secrets in configuration files or logs.
#[derive(Clone, Deserialize)]
pub struct SecretString(String);

impl SecretString {
    /// Create a new secret string
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Get the secret value
    ///
    /// # Security Note
    /// Be careful when using this method - the returned string
    /// can be logged or displayed if not handled properly.
    pub fn expose_secret(&self) -> &str {
        &self.0
    }

    /// Check if the secret is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get the length of the secret
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl From<String> for SecretString {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for SecretString {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl Serialize for SecretString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str("[REDACTED]")
    }
}

/// Common HTTP client configuration options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    /// Request timeout duration
    pub timeout: Duration,

    /// Maximum number of retry attempts
    pub max_retries: u32,

    /// Base delay for exponential backoff
    pub retry_delay: Duration,

    /// Maximum delay for exponential backoff
    pub max_retry_delay: Duration,

    /// User agent string for requests
    pub user_agent: Option<String>,

    /// Additional HTTP headers to include in requests
    pub headers: std::collections::HashMap<String, String>,

    /// Whether to enable compression
    pub compression: bool,

    /// Connection pool settings
    pub pool: PoolConfig,
}

/// Connection pool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Maximum number of connections per host
    pub max_connections_per_host: usize,

    /// Maximum number of idle connections to keep alive
    pub max_idle_connections: usize,

    /// How long to keep idle connections alive
    pub idle_timeout: Duration,

    /// Connection timeout
    pub connect_timeout: Duration,
}

/// Rate limiting configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per second
    pub requests_per_second: f64,

    /// Burst capacity (maximum requests that can be made at once)
    pub burst_capacity: u32,

    /// Whether rate limiting is enabled
    pub enabled: bool,
}

/// Retry configuration for failed requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,

    /// Base delay between retries
    pub base_delay: Duration,

    /// Maximum delay between retries
    pub max_delay: Duration,

    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,

    /// Whether to add jitter to retry delays
    pub jitter: bool,

    /// Whether retries are enabled
    pub enabled: bool,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_delay: Duration::from_millis(100),
            max_retry_delay: Duration::from_secs(60),
            user_agent: Some("euro-llm-core/2.0".to_string()),
            headers: std::collections::HashMap::new(),
            compression: true,
            pool: PoolConfig::default(),
        }
    }
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections_per_host: 100,
            max_idle_connections: 10,
            idle_timeout: Duration::from_secs(90),
            connect_timeout: Duration::from_secs(10),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 10.0,
            burst_capacity: 20,
            enabled: true,
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.0,
            jitter: true,
            enabled: true,
        }
    }
}

/// Validation utilities for configuration values.
pub mod validation {
    use super::*;
    use url::Url;

    /// Validate that a string is not empty
    pub fn validate_non_empty(value: &str, field_name: &str) -> Result<(), ConfigError> {
        if value.trim().is_empty() {
            Err(ConfigError::missing_field(field_name))
        } else {
            Ok(())
        }
    }

    /// Validate that a secret string is not empty
    pub fn validate_secret_non_empty(
        value: &SecretString,
        field_name: &str,
    ) -> Result<(), ConfigError> {
        if value.is_empty() {
            Err(ConfigError::missing_field(field_name))
        } else {
            Ok(())
        }
    }

    /// Validate that a URL is well-formed
    pub fn validate_url(url: &str, field_name: &str) -> Result<Url, ConfigError> {
        Url::parse(url)
            .map_err(|_| ConfigError::invalid_value(field_name, format!("Invalid URL: {url}")))
    }

    /// Validate that a URL is HTTPS
    pub fn validate_https_url(url: &Url, field_name: &str) -> Result<(), ConfigError> {
        if url.scheme() != "https" {
            Err(ConfigError::invalid_value(
                field_name,
                "URL must use HTTPS scheme",
            ))
        } else {
            Ok(())
        }
    }

    /// Validate that a numeric value is within a range
    pub fn validate_range<T>(value: T, min: T, max: T, field_name: &str) -> Result<(), ConfigError>
    where
        T: PartialOrd + std::fmt::Display,
    {
        if value < min || value > max {
            Err(ConfigError::invalid_value(
                field_name,
                format!("Value {value} must be between {min} and {max}"),
            ))
        } else {
            Ok(())
        }
    }

    /// Validate that a duration is positive
    pub fn validate_positive_duration(
        duration: Duration,
        field_name: &str,
    ) -> Result<(), ConfigError> {
        if duration.is_zero() {
            Err(ConfigError::invalid_value(
                field_name,
                "Duration must be positive",
            ))
        } else {
            Ok(())
        }
    }

    /// Validate an API key format (basic checks)
    pub fn validate_api_key(api_key: &SecretString, field_name: &str) -> Result<(), ConfigError> {
        let key = api_key.expose_secret();

        // Basic validation - not empty and reasonable length
        if key.is_empty() {
            return Err(ConfigError::missing_field(field_name));
        }

        if key.len() < 10 {
            return Err(ConfigError::invalid_value(
                field_name,
                "API key appears to be too short",
            ));
        }

        // Check for common patterns that indicate a placeholder
        let placeholder_patterns = ["your_api_key", "api_key_here", "replace_me", "xxx"];
        for pattern in &placeholder_patterns {
            if key.to_lowercase().contains(pattern) {
                return Err(ConfigError::invalid_value(
                    field_name,
                    "API key appears to be a placeholder",
                ));
            }
        }

        Ok(())
    }

    /// Validate a model name
    pub fn validate_model_name(model: &str, field_name: &str) -> Result<(), ConfigError> {
        validate_non_empty(model, field_name)?;

        // Basic validation - no whitespace, reasonable length
        if model.contains(char::is_whitespace) {
            return Err(ConfigError::invalid_value(
                field_name,
                "Model name cannot contain whitespace",
            ));
        }

        if model.len() > 100 {
            return Err(ConfigError::invalid_value(
                field_name,
                "Model name is too long",
            ));
        }

        Ok(())
    }
}

/// Builder pattern helper for configuration types.
///
/// This trait provides a consistent interface for building configurations
/// with method chaining.
pub trait ConfigBuilder<T> {
    /// Build the final configuration
    fn build(self) -> T;
}

/// Environment variable helper for loading configuration.
pub mod env {
    use super::*;
    use std::env;

    /// Load a required environment variable
    pub fn required(key: &str) -> Result<String, ConfigError> {
        env::var(key).map_err(|_| ConfigError::missing_field(key))
    }

    /// Load an optional environment variable
    pub fn optional(key: &str) -> Option<String> {
        env::var(key).ok()
    }

    /// Load a required environment variable as a SecretString
    pub fn required_secret(key: &str) -> Result<SecretString, ConfigError> {
        required(key).map(SecretString::new)
    }

    /// Load an optional environment variable as a SecretString
    pub fn optional_secret(key: &str) -> Option<SecretString> {
        optional(key).map(SecretString::new)
    }

    /// Load an environment variable with a default value
    pub fn with_default(key: &str, default: &str) -> String {
        optional(key).unwrap_or_else(|| default.to_string())
    }

    /// Parse an environment variable as a specific type
    pub fn parse<T>(key: &str) -> Result<T, ConfigError>
    where
        T: std::str::FromStr,
        T::Err: std::fmt::Display,
    {
        let value = required(key)?;
        value
            .parse()
            .map_err(|e| ConfigError::invalid_value(key, format!("Failed to parse: {e}")))
    }

    /// Parse an optional environment variable as a specific type
    pub fn parse_optional<T>(key: &str) -> Result<Option<T>, ConfigError>
    where
        T: std::str::FromStr,
        T::Err: std::fmt::Display,
    {
        match optional(key) {
            Some(value) => value
                .parse()
                .map(Some)
                .map_err(|e| ConfigError::invalid_value(key, format!("Failed to parse: {e}"))),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_string_debug() {
        let secret = SecretString::new("super_secret_key");
        let debug_output = format!("{:?}", secret);
        assert_eq!(debug_output, "[REDACTED]");
        assert!(!debug_output.contains("super_secret_key"));
    }

    #[test]
    fn test_secret_string_expose() {
        let secret = SecretString::new("my_secret");
        assert_eq!(secret.expose_secret(), "my_secret");
    }

    #[test]
    fn test_validation_non_empty() {
        use validation::*;

        assert!(validate_non_empty("valid", "test").is_ok());
        assert!(validate_non_empty("", "test").is_err());
        assert!(validate_non_empty("   ", "test").is_err());
    }

    #[test]
    fn test_validation_url() {
        use validation::*;

        assert!(validate_url("https://api.example.com", "url").is_ok());
        assert!(validate_url("not_a_url", "url").is_err());
    }

    #[test]
    fn test_validation_range() {
        use validation::*;

        assert!(validate_range(5, 1, 10, "value").is_ok());
        assert!(validate_range(0, 1, 10, "value").is_err());
        assert!(validate_range(15, 1, 10, "value").is_err());
    }
}
