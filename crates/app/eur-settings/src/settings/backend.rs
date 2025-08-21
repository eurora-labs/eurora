use eur_prompt_kit::{OllamaConfig, OpenAIConfig};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type, Default)]
pub enum BackendType {
    #[default]
    None,
    Ollama,
    Eurora,
    OpenAI,
    Anthropic,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type, Default)]
#[serde(rename_all = "camelCase")]
pub struct BackendSettings {
    pub backend_type: BackendType,
    pub config: Option<String>,
}

impl From<OllamaConfig> for BackendSettings {
    fn from(config: OllamaConfig) -> Self {
        Self {
            backend_type: BackendType::Ollama,
            config: Some(serde_json::to_string(&config).unwrap()),
        }
    }
}

impl From<OpenAIConfig> for BackendSettings {
    fn from(config: OpenAIConfig) -> Self {
        Self {
            backend_type: BackendType::OpenAI,
            config: Some(serde_json::to_string(&config).unwrap()),
        }
    }
}
