use url::Url;

use crate::proto;
use crate::provider::{OllamaConfig, OpenAIConfig, ProviderSettings};
use crate::redacted::Redacted;

impl TryFrom<proto::OllamaSettings> for OllamaConfig {
    type Error = url::ParseError;

    fn try_from(p: proto::OllamaSettings) -> Result<Self, Self::Error> {
        Ok(Self {
            base_url: Url::parse(&p.base_url)?,
            model: p.model,
        })
    }
}

impl TryFrom<proto::OpenAiSettings> for OpenAIConfig {
    type Error = url::ParseError;

    fn try_from(p: proto::OpenAiSettings) -> Result<Self, Self::Error> {
        let title_model = if p.title_model.is_empty() {
            None
        } else {
            Some(p.title_model)
        };
        Ok(Self {
            base_url: Url::parse(&p.base_url)?,
            api_key: Redacted::new(p.api_key),
            model: p.model,
            title_model,
        })
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
                api_key: String::new(), // never echo the key back
                model: c.model.clone(),
                title_model: c.title_model.clone().unwrap_or_default(),
            }),
        };
        proto::ProviderSettings {
            provider: Some(provider),
        }
    }
}
