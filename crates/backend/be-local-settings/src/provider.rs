mod ollama;
mod openai;

use crate::{
    error::{Error, Result},
    proto,
};
pub use ollama::OllamaConfig;
pub use openai::OpenAIConfig;

#[derive(Debug, Clone, PartialEq)]
pub enum ProviderSettings {
    Ollama(OllamaConfig),
    OpenAI(OpenAIConfig),
}

impl From<OllamaConfig> for ProviderSettings {
    fn from(config: OllamaConfig) -> Self {
        Self::Ollama(config)
    }
}

impl From<OpenAIConfig> for ProviderSettings {
    fn from(config: OpenAIConfig) -> Self {
        Self::OpenAI(config)
    }
}

impl TryFrom<proto::ProviderSettings> for ProviderSettings {
    type Error = Error;

    fn try_from(settings: proto::ProviderSettings) -> Result<Self> {
        use proto::provider_settings::Provider;

        let provider = settings.provider.ok_or(Error::EmptyField("provider"))?;

        match provider {
            Provider::Ollama(p) => Ok(Self::Ollama(p.try_into()?)),
            Provider::Openai(p) => Ok(Self::OpenAI(p.try_into()?)),
        }
    }
}

impl From<&ProviderSettings> for proto::ProviderSettings {
    fn from(settings: &ProviderSettings) -> Self {
        use proto::provider_settings::Provider;

        let provider = match settings {
            ProviderSettings::Ollama(c) => Provider::Ollama(proto::OllamaSettings {
                base_url: c.base_url.to_string(),
                model: c.model.clone(),
            }),
            ProviderSettings::OpenAI(c) => Provider::Openai(proto::OpenAiSettings {
                base_url: c.base_url.to_string(),
                api_key: c.api_key.masked(),
                model: c.model.clone(),
                title_model: c.title_model.clone().unwrap_or_default(),
            }),
        };
        proto::ProviderSettings {
            provider: Some(provider),
        }
    }
}
