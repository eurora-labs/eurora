use agent_chain::{ollama::ChatOllama, openai::ChatOpenAI};
use agent_chain_core::chat_models::ChatModel;
use agent_chain_core::messages::BaseMessage;
use agent_chain_eurora::{ChatEurora, EuroraConfig};
use anyhow::Result;
use async_from::{AsyncTryFrom, async_trait};
use serde::{Deserialize, Serialize};
use tokio_stream::{Stream, StreamExt};
use tracing::info;

use crate::PromptKitError;

/// Configuration for OpenAI provider using agent-chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    /// Model name (e.g., "gpt-4o", "gpt-4-turbo")
    pub model: String,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Base URL for OpenAI API (default: https://api.openai.com/v1)
    pub base_url: Option<String>,
    /// Temperature for generation
    pub temperature: Option<f64>,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            api_key: None,
            base_url: None,
            temperature: None,
            max_tokens: None,
        }
    }
}

impl OpenAIConfig {
    /// Create a new OpenAI config with the given API key and model
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            api_key: Some(api_key.into()),
            base_url: None,
            temperature: None,
            max_tokens: None,
        }
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, std::env::VarError> {
        let api_key = std::env::var("OPENAI_API_KEY")?;
        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());
        Ok(Self {
            model,
            api_key: Some(api_key),
            base_url: std::env::var("OPENAI_BASE_URL").ok(),
            temperature: None,
            max_tokens: None,
        })
    }
}

/// Configuration for Ollama provider using agent-chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// Model name (e.g., "llama3.2", "mistral")
    pub model: String,
    /// Base URL for Ollama API (default: http://localhost:11434)
    pub base_url: Option<String>,
    /// Temperature for generation
    pub temperature: Option<f64>,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            model: "llama3.2".to_string(),
            base_url: None,
            temperature: None,
        }
    }
}

#[derive(Debug, Clone)]
enum LLMProvider {
    OpenAI(ChatOpenAI),
    Ollama(ChatOllama),
    Eurora(ChatEurora),
}

#[derive(Debug, Clone)]
pub struct PromptKitService {
    provider: LLMProvider,
}

impl Default for PromptKitService {
    fn default() -> Self {
        Self {
            provider: LLMProvider::OpenAI(ChatOpenAI::new("gpt-4o")),
        }
    }
}

impl PromptKitService {
    pub fn get_service_name(&self) -> Result<String> {
        match &self.provider {
            LLMProvider::OpenAI(_) => Ok("OpenAI".to_string()),
            LLMProvider::Ollama(_) => Ok("Ollama".to_string()),
            LLMProvider::Eurora(_) => Ok("Eurora".to_string()),
        }
    }

    pub async fn chat_stream(
        &self,
        messages: Vec<BaseMessage>,
    ) -> Result<
        std::pin::Pin<Box<dyn Stream<Item = Result<String, PromptKitError>> + Send>>,
        PromptKitError,
    > {
        match &self.provider {
            LLMProvider::OpenAI(_) => self._chat_stream_openai(messages).await,
            LLMProvider::Ollama(_) => self._chat_stream_ollama(messages).await,
            LLMProvider::Eurora(_) => self._chat_stream_eurora(messages).await,
        }
    }

    async fn _chat_stream_eurora(
        &self,
        messages: Vec<BaseMessage>,
    ) -> Result<
        std::pin::Pin<Box<dyn Stream<Item = Result<String, PromptKitError>> + Send>>,
        PromptKitError,
    > {
        if let LLMProvider::Eurora(llm) = &self.provider {
            info!("Starting Eurora chat stream with agent-chain");

            let stream = llm
                .stream(messages, None)
                .await
                .map_err(PromptKitError::AgentChainError)?
                .map(|result| {
                    result
                        .map(|chunk| chunk.content)
                        .map_err(PromptKitError::AgentChainError)
                });

            Ok(Box::pin(stream))
        } else {
            Err(PromptKitError::ServiceNotInitialized {
                service: "Eurora".to_string(),
            })
        }
    }

    async fn _chat_stream_openai(
        &self,
        messages: Vec<BaseMessage>,
    ) -> Result<
        std::pin::Pin<Box<dyn Stream<Item = Result<String, PromptKitError>> + Send>>,
        PromptKitError,
    > {
        if let LLMProvider::OpenAI(llm) = &self.provider {
            info!("Starting OpenAI chat stream with agent-chain");

            let stream = llm
                .stream(messages, None)
                .await
                .map_err(PromptKitError::AgentChainError)?
                .map(|result| {
                    result
                        .map(|chunk| chunk.content)
                        .map_err(PromptKitError::AgentChainError)
                });

            Ok(Box::pin(stream))
        } else {
            Err(PromptKitError::ServiceNotInitialized {
                service: "OpenAI".to_string(),
            })
        }
    }

    async fn _chat_stream_ollama(
        &self,
        messages: Vec<BaseMessage>,
    ) -> Result<
        std::pin::Pin<Box<dyn Stream<Item = Result<String, PromptKitError>> + Send>>,
        PromptKitError,
    > {
        if let LLMProvider::Ollama(llm) = &self.provider {
            info!("Starting Ollama chat stream with agent-chain");

            let stream = llm
                .stream(messages, None)
                .await
                .map_err(PromptKitError::AgentChainError)?
                .map(|result| {
                    result
                        .map(|chunk| chunk.content)
                        .map_err(PromptKitError::AgentChainError)
                });

            Ok(Box::pin(stream))
        } else {
            Err(PromptKitError::ServiceNotInitialized {
                service: "Ollama".to_string(),
            })
        }
    }
}

impl From<OpenAIConfig> for PromptKitService {
    fn from(config: OpenAIConfig) -> Self {
        let mut llm = ChatOpenAI::new(&config.model);

        if let Some(api_key) = config.api_key {
            llm = llm.api_key(api_key);
        }

        if let Some(base_url) = config.base_url {
            llm = llm.api_base(base_url);
        }

        if let Some(temp) = config.temperature {
            llm = llm.temperature(temp);
        }

        if let Some(max_tokens) = config.max_tokens {
            llm = llm.max_tokens(max_tokens);
        }

        Self {
            provider: LLMProvider::OpenAI(llm),
        }
    }
}

impl From<OllamaConfig> for PromptKitService {
    fn from(config: OllamaConfig) -> Self {
        let mut llm = ChatOllama::new(&config.model);

        if let Some(base_url) = config.base_url {
            llm = llm.base_url(base_url);
        }

        if let Some(temp) = config.temperature {
            llm = llm.temperature(temp);
        }

        Self {
            provider: LLMProvider::Ollama(llm),
        }
    }
}

#[async_trait]
impl AsyncTryFrom<EuroraConfig> for PromptKitService {
    type Error = PromptKitError;
    async fn async_try_from(config: EuroraConfig) -> Result<Self, Self::Error> {
        let chat_eurora = ChatEurora::new(config)
            .await
            .map_err(PromptKitError::EuroraError)?;
        Ok(Self {
            provider: LLMProvider::Eurora(chat_eurora),
        })
    }
}
