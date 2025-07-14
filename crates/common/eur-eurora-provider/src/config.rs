use ferrous_llm_core::{ConfigError, HttpConfig, ProviderConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EuroraConfig {
    pub model: String,

    pub base_url: String,

    pub http: HttpConfig,

    pub embedding_model: Option<String>,

    pub keep_alive: Option<u64>,

    pub options: Option<serde_json::Value>,
}

impl Default for EuroraConfig {
    fn default() -> Self {
        Self {
            model: "default".to_string(),
            base_url: "http://localhost:50051".to_string(),
            http: HttpConfig::default(),
            embedding_model: None,
            keep_alive: None,
            options: None,
        }
    }
}

impl ProviderConfig for EuroraConfig {
    type Provider = crate::provider::EuroraProvider;

    fn build(self) -> Result<Self::Provider, ConfigError> {
        self.validate()?;
        crate::provider::EuroraProvider::new(self).map_err(|e| match e {
            crate::error::EuroraError::Config(source) => source,
            _ => ConfigError::validation_failed("Failed to create provider"),
        })
    }

    fn validate(&self) -> Result<(), ConfigError> {
        todo!()
    }
}

impl EuroraConfig {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Default::default()
        }
    }

    pub fn builder() -> EuroraConfigBuilder {
        EuroraConfigBuilder::new()
    }
}

pub struct EuroraConfigBuilder {
    config: EuroraConfig,
}

impl EuroraConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: EuroraConfig::default(),
        }
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.config.model = model.into();
        self
    }

    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.config.base_url = base_url.into();
        self
    }

    pub fn keep_alive(mut self, keep_alive: u64) -> Self {
        self.config.keep_alive = Some(keep_alive);
        self
    }

    pub fn build(self) -> EuroraConfig {
        self.config
    }
}

impl Default for EuroraConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
