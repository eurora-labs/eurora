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
    #[specta(skip)]
    pub config: Option<serde_json::Value>,
}

impl From<OllamaConfig> for BackendSettings {
    fn from(config: OllamaConfig) -> Self {
        Self {
            backend_type: BackendType::Ollama,
            config: Some(serde_json::to_value(config).expect("Failed to serialize OllamaConfig")),
        }
    }
}

impl From<OpenAIConfig> for BackendSettings {
    fn from(config: OpenAIConfig) -> Self {
        Self {
            backend_type: BackendType::OpenAI,
            config: Some(serde_json::to_value(config).expect("Failed to serialize OpenAIConfig")),
        }
    }
}

impl BackendSettings {
    pub fn initialize(&self) -> Result<eur_prompt_kit::PromptKitService, String> {
        match self.backend_type {
            BackendType::None => Err("No backend selected".to_string()),
            BackendType::Ollama => {
                if let Some(config) = &self.config {
                    let config: OllamaConfig = serde_json::from_value(config.clone())
                        .map_err(|e| format!("Failed to deserialize OllamaConfig: {e}"))?;

                    Ok(eur_prompt_kit::PromptKitService::from(config))
                } else {
                    Err("No Ollama config provided".to_string())
                }
            }
            BackendType::OpenAI => {
                if let Some(config) = &self.config {
                    let mut config: OpenAIConfig = serde_json::from_value(config.clone())
                        .expect("Failed to deserialize OpenAIConfig");

                    if let Some(api_key) = eur_secret::secret::retrieve(
                        "OPENAI_API_KEY",
                        eur_secret::secret::Namespace::Global,
                    )
                    .map_err(|e| format!("Failed to retrieve OpenAI API key: {e}"))?
                    {
                        config.api_key = api_key.0.into();
                    } else {
                        return Err("No OpenAI API key provided".to_string());
                    }

                    Ok(eur_prompt_kit::PromptKitService::from(config))
                } else {
                    Err("No OpenAI config provided".to_string())
                }
            }
            _ => Err("Unsupported backend".to_string()),
        }
    }
}
