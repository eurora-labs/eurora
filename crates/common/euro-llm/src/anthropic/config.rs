//! Anthropic provider configuration.

use crate::{ConfigError, HttpConfig, ProviderConfig, SecretString, validation};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use url::Url;

/// Configuration for the Anthropic provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    /// Anthropic API key
    pub api_key: SecretString,

    /// Model to use (e.g., "claude-3-5-sonnet-20241022", "claude-3-haiku-20240307")
    pub model: String,

    /// Base URL for the Anthropic API (defaults to https://api.anthropic.com)
    pub base_url: Option<Url>,

    /// Anthropic version header (defaults to "2023-06-01")
    pub version: String,

    /// HTTP client configuration
    pub http: HttpConfig,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: SecretString::new(""),
            model: "claude-3-5-sonnet-20241022".to_string(),
            base_url: None,
            version: "2023-06-01".to_string(),
            http: HttpConfig::default(),
        }
    }
}

impl ProviderConfig for AnthropicConfig {
    type Provider = super::provider::AnthropicProvider;

    fn build(self) -> Result<Self::Provider, ConfigError> {
        self.validate()?;
        super::provider::AnthropicProvider::new(self).map_err(|e| match e {
            super::error::AnthropicError::Config { source } => source,
            _ => ConfigError::validation_failed("Failed to create provider"),
        })
    }

    fn validate(&self) -> Result<(), ConfigError> {
        // Validate API key
        validation::validate_api_key(&self.api_key, "api_key")?;

        // Validate model name
        validation::validate_model_name(&self.model, "model")?;

        // Validate base URL if provided
        if let Some(ref url) = self.base_url {
            validation::validate_https_url(url, "base_url")?;
        }

        // Validate version format (should be YYYY-MM-DD)
        if !self.version.is_empty() && self.version.matches('-').count() != 2 {
            return Err(ConfigError::invalid_value(
                "version",
                "Version must be in YYYY-MM-DD format",
            ));
        }

        // Validate HTTP configuration
        validation::validate_positive_duration(self.http.timeout, "http.timeout")?;
        validation::validate_range(self.http.max_retries, 0, 10, "http.max_retries")?;

        Ok(())
    }
}

impl AnthropicConfig {
    /// Create a new Anthropic configuration with the given API key and model.
    pub fn new(api_key: impl Into<SecretString>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            ..Default::default()
        }
    }

    /// Create a configuration builder.
    pub fn builder() -> AnthropicConfigBuilder {
        AnthropicConfigBuilder::new()
    }

    /// Get the base URL for API requests.
    pub fn base_url(&self) -> &str {
        self.base_url
            .as_ref()
            .map(|u| u.as_str())
            .unwrap_or("https://api.anthropic.com")
    }

    /// Get the messages endpoint URL.
    pub fn messages_url(&self) -> String {
        let base = self.base_url();
        if base.ends_with('/') {
            format!("{base}v1/messages")
        } else {
            format!("{base}/v1/messages")
        }
    }

    /// Load configuration from environment variables.
    pub fn from_env() -> Result<Self, ConfigError> {
        use crate::env;

        let api_key = env::required_secret("ANTHROPIC_API_KEY")?;
        let model = env::with_default("ANTHROPIC_MODEL", "claude-3-5-sonnet-20241022");
        let version = env::with_default("ANTHROPIC_VERSION", "2023-06-01");

        let base_url = if let Some(url_str) = env::optional("ANTHROPIC_BASE_URL") {
            Some(validation::validate_url(&url_str, "ANTHROPIC_BASE_URL")?)
        } else {
            None
        };

        Ok(Self {
            api_key,
            model,
            base_url,
            version,
            http: HttpConfig::default(),
        })
    }
}

/// Builder for Anthropic configuration.
pub struct AnthropicConfigBuilder {
    config: AnthropicConfig,
}

impl AnthropicConfigBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            config: AnthropicConfig::default(),
        }
    }

    /// Set the API key.
    pub fn api_key(mut self, api_key: impl Into<SecretString>) -> Self {
        self.config.api_key = api_key.into();
        self
    }

    /// Set the model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.config.model = model.into();
        self
    }

    /// Set the base URL.
    pub fn base_url(mut self, base_url: impl Into<String>) -> Result<Self, ConfigError> {
        let url = validation::validate_url(&base_url.into(), "base_url")?;
        self.config.base_url = Some(url);
        Ok(self)
    }

    /// Set the API version.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.config.version = version.into();
        self
    }

    /// Set the request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.http.timeout = timeout;
        self
    }

    /// Set the maximum number of retries.
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.config.http.max_retries = max_retries;
        self
    }

    /// Set a custom HTTP header.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.http.headers.insert(key.into(), value.into());
        self
    }

    /// Build the configuration.
    pub fn build(self) -> AnthropicConfig {
        self.config
    }
}

impl Default for AnthropicConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let config = AnthropicConfig::new("sk-ant-test123456789", "claude-3-5-sonnet-20241022");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_empty_api_key() {
        let config = AnthropicConfig::new("", "claude-3-5-sonnet-20241022");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_builder() {
        let config = AnthropicConfig::builder()
            .api_key("sk-ant-test123456789")
            .model("claude-3-haiku-20240307")
            .version("2023-06-01")
            .timeout(Duration::from_secs(60))
            .build();

        assert_eq!(config.model, "claude-3-haiku-20240307");
        assert_eq!(config.version, "2023-06-01");
        assert_eq!(config.http.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_urls() {
        let config = AnthropicConfig::new("sk-ant-test", "claude-3-5-sonnet-20241022");
        assert_eq!(
            config.messages_url(),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn test_custom_base_url() {
        let mut config = AnthropicConfig::new("sk-ant-test", "claude-3-5-sonnet-20241022");
        config.base_url = Some("https://custom.anthropic.com".parse().unwrap());
        assert_eq!(
            config.messages_url(),
            "https://custom.anthropic.com/v1/messages"
        );
    }
}
