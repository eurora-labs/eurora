//! Ollama provider configuration.

use euro_llm_core::{ConfigError, HttpConfig, ProviderConfig, validation};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use url::Url;

/// Configuration for the Ollama provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// Model to use (e.g., "llama2", "codellama", "mistral")
    pub model: String,

    /// Base URL for the Ollama API (defaults to http://localhost:11434)
    pub base_url: Option<Url>,

    /// HTTP client configuration
    pub http: HttpConfig,

    /// Embedding model to use (e.g., "nomic-embed-text")
    pub embedding_model: Option<String>,

    /// Keep alive duration for the model (in seconds)
    pub keep_alive: Option<u64>,

    /// Additional options for the model
    pub options: Option<serde_json::Value>,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            model: "llama2".to_string(),
            base_url: None,
            http: HttpConfig::default(),
            embedding_model: None,
            keep_alive: None,
            options: None,
        }
    }
}

impl ProviderConfig for OllamaConfig {
    type Provider = crate::provider::OllamaProvider;

    fn build(self) -> Result<Self::Provider, ConfigError> {
        self.validate()?;
        crate::provider::OllamaProvider::new(self).map_err(|e| match e {
            crate::error::OllamaError::Config { source } => source,
            _ => ConfigError::validation_failed("Failed to create provider"),
        })
    }

    fn validate(&self) -> Result<(), ConfigError> {
        // Validate model name
        validation::validate_model_name(&self.model, "model")?;

        // Validate base URL if provided
        if let Some(ref url) = self.base_url {
            // URL is already validated when parsed, just check it's not empty
            if url.as_str().is_empty() {
                return Err(ConfigError::invalid_value(
                    "base_url",
                    "Base URL cannot be empty",
                ));
            }
        }

        // Validate HTTP configuration
        validation::validate_positive_duration(self.http.timeout, "http.timeout")?;
        validation::validate_range(self.http.max_retries, 0, 10, "http.max_retries")?;

        // Validate keep_alive if provided
        if let Some(keep_alive) = self.keep_alive
            && keep_alive > 86400
        {
            // Max 24 hours
            return Err(ConfigError::invalid_value(
                "keep_alive",
                "Keep alive duration cannot exceed 24 hours (86400 seconds)",
            ));
        }

        Ok(())
    }
}

impl OllamaConfig {
    /// Create a new Ollama configuration with the given model.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Default::default()
        }
    }

    /// Create a configuration builder.
    pub fn builder() -> OllamaConfigBuilder {
        OllamaConfigBuilder::new()
    }

    /// Get the base URL for API requests.
    pub fn base_url(&self) -> &str {
        self.base_url
            .as_ref()
            .map(|u| u.as_str())
            .unwrap_or("http://localhost:11434")
    }

    /// Get the chat endpoint URL.
    pub fn chat_url(&self) -> String {
        let base = self.base_url().trim_end_matches('/');
        format!("{base}/api/chat")
    }

    /// Get the generate endpoint URL.
    pub fn generate_url(&self) -> String {
        let base = self.base_url().trim_end_matches('/');
        format!("{base}/api/generate")
    }

    /// Get the embeddings endpoint URL.
    pub fn embeddings_url(&self) -> String {
        let base = self.base_url().trim_end_matches('/');
        format!("{base}/api/embeddings")
    }

    /// Get the models endpoint URL.
    pub fn models_url(&self) -> String {
        let base = self.base_url().trim_end_matches('/');
        format!("{base}/api/tags")
    }

    /// Load configuration from environment variables.
    pub fn from_env() -> Result<Self, ConfigError> {
        use euro_llm_core::env;

        let model = env::with_default("OLLAMA_MODEL", "llama2");
        let embedding_model = env::optional("OLLAMA_EMBEDDING_MODEL");

        let base_url = if let Some(url_str) = env::optional("OLLAMA_BASE_URL") {
            Some(validation::validate_url(&url_str, "OLLAMA_BASE_URL")?)
        } else {
            None
        };

        let keep_alive = if let Some(keep_alive_str) = env::optional("OLLAMA_KEEP_ALIVE") {
            Some(keep_alive_str.parse().map_err(|_| {
                ConfigError::invalid_value("OLLAMA_KEEP_ALIVE", "Must be a valid number")
            })?)
        } else {
            None
        };

        Ok(Self {
            model,
            base_url,
            http: HttpConfig::default(),
            embedding_model,
            keep_alive,
            options: None,
        })
    }
}

/// Builder for Ollama configuration.
pub struct OllamaConfigBuilder {
    config: OllamaConfig,
}

impl OllamaConfigBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            config: OllamaConfig::default(),
        }
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

    /// Set the embedding model.
    pub fn embedding_model(mut self, embedding_model: impl Into<String>) -> Self {
        self.config.embedding_model = Some(embedding_model.into());
        self
    }

    /// Set the keep alive duration.
    pub fn keep_alive(mut self, keep_alive: u64) -> Self {
        self.config.keep_alive = Some(keep_alive);
        self
    }

    /// Set model options.
    pub fn options(mut self, options: serde_json::Value) -> Self {
        self.config.options = Some(options);
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
    pub fn build(self) -> OllamaConfig {
        self.config
    }
}

impl Default for OllamaConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let config = OllamaConfig::new("llama2");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_empty_model() {
        let config = OllamaConfig::new("");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_builder() {
        let config = OllamaConfig::builder()
            .model("codellama")
            .embedding_model("nomic-embed-text")
            .keep_alive(300)
            .timeout(Duration::from_secs(60))
            .build();

        assert_eq!(config.model, "codellama");
        assert_eq!(config.embedding_model, Some("nomic-embed-text".to_string()));
        assert_eq!(config.keep_alive, Some(300));
        assert_eq!(config.http.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_urls() {
        let config = OllamaConfig::new("llama2");
        assert_eq!(config.chat_url(), "http://localhost:11434/api/chat");
        assert_eq!(config.generate_url(), "http://localhost:11434/api/generate");
        assert_eq!(
            config.embeddings_url(),
            "http://localhost:11434/api/embeddings"
        );
    }

    #[test]
    fn test_custom_base_url() {
        let mut config = OllamaConfig::new("llama2");
        config.base_url = Some("http://custom-ollama:11434".parse().unwrap());
        assert_eq!(config.chat_url(), "http://custom-ollama:11434/api/chat");
    }

    #[test]
    fn test_keep_alive_validation() {
        let mut config = OllamaConfig::new("llama2");
        config.keep_alive = Some(100000); // Too large
        assert!(config.validate().is_err());
    }
}
