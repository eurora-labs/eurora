use agent_chain_eurora::EuroraConfig;
use euro_prompt_kit::{OllamaConfig, OpenAIConfig};
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

impl From<EuroraConfig> for BackendSettings {
    fn from(config: EuroraConfig) -> Self {
        Self {
            backend_type: BackendType::Eurora,
            config: Some(serde_json::to_value(config).expect("Failed to serialize EuroraConfig")),
        }
    }
}

impl BackendSettings {
    pub async fn initialize(&self) -> Result<euro_prompt_kit::PromptKitService, String> {
        match self.backend_type {
            BackendType::None => Err("No backend selected".to_string()),
            BackendType::Ollama => {
                if let Some(config) = &self.config {
                    let config: OllamaConfig = serde_json::from_value(config.clone())
                        .map_err(|e| format!("Failed to deserialize OllamaConfig: {e}"))?;

                    Ok(euro_prompt_kit::PromptKitService::from(config))
                } else {
                    Err("No Ollama config provided".to_string())
                }
            }
            BackendType::OpenAI => {
                if let Some(config) = &self.config {
                    let mut config: OpenAIConfig = serde_json::from_value(config.clone())
                        .expect("Failed to deserialize OpenAIConfig");

                    if let Some(api_key) = euro_secret::secret::retrieve(
                        "OPENAI_API_KEY",
                        euro_secret::secret::Namespace::Global,
                    )
                    .map_err(|e| format!("Failed to retrieve OpenAI API key: {e}"))?
                    {
                        config.api_key = Some(api_key.0.clone());
                        config.base_url = None;
                    } else {
                        return Err("No OpenAI API key provided".to_string());
                    }

                    Ok(euro_prompt_kit::PromptKitService::from(config))
                } else {
                    Err("No OpenAI config provided".to_string())
                }
            }
            BackendType::Eurora => {
                if let Some(config) = &self.config {
                    // let config: EuroraConfig = serde_json::from_value(config.clone())
                    //     .expect("Failed to deserialize EuroraConfig");

                    // Ok(euro_prompt_kit::PromptKitService::async_try_from(config)
                    //     .await
                    //     .map_err(|e| e.to_string())?)
                    Ok(euro_chat_client::ChatEurora::new()
                        .await
                        .expect("Failed to initialize Eurora backend")
                        .into())
                } else {
                    Err("No Eurora config provided".to_string())
                }
            }
            _ => Err("Unsupported backend".to_string()),
        }
    }
}
