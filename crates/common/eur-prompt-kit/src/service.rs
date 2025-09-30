use anyhow::Result;
use async_from::{AsyncTryFrom, async_trait};
use eur_eurora_provider::{EuroraConfig, EuroraStreamingProvider, StreamingProvider};
use ferrous_llm::{
    ChatRequest, Message,
    ollama::{OllamaConfig, OllamaProvider},
    openai::{OpenAIConfig, OpenAIProvider},
};
use tokio_stream::{Stream, StreamExt};
use tracing::info;

use crate::PromptKitError;

#[derive(Debug, Clone)]
enum LLMProvider {
    OpenAI(OpenAIProvider),
    Ollama(OllamaProvider),
    Eurora(EuroraStreamingProvider),
}

#[derive(Debug, Clone)]
pub struct PromptKitService {
    provider: LLMProvider,
}

impl Default for PromptKitService {
    fn default() -> Self {
        Self {
            provider: LLMProvider::OpenAI(OpenAIProvider::new(OpenAIConfig::default()).unwrap()),
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
        messages: Vec<Message>,
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
        messages: Vec<Message>,
    ) -> Result<
        std::pin::Pin<Box<dyn Stream<Item = Result<String, PromptKitError>> + Send>>,
        PromptKitError,
    > {
        if let LLMProvider::Eurora(provider) = &self.provider {
            let request = ChatRequest {
                messages,
                parameters: Default::default(),
                metadata: Default::default(),
            };

            let stream = provider
                .chat_stream(request)
                .await
                .map_err(PromptKitError::EuroraError)?
                .map(|result| result.map_err(PromptKitError::EuroraError))
                .map(|result| result.map(|message| message.content));

            Ok(Box::pin(stream))
        } else {
            Err(PromptKitError::ServiceNotInitialized {
                service: "Eurora".to_string(),
            })
        }
    }

    async fn _chat_stream_openai(
        &self,
        messages: Vec<Message>,
    ) -> Result<
        std::pin::Pin<Box<dyn Stream<Item = Result<String, PromptKitError>> + Send>>,
        PromptKitError,
    > {
        if let LLMProvider::OpenAI(provider) = &self.provider {
            let request = ChatRequest {
                messages,
                parameters: Default::default(),
                metadata: Default::default(),
            };

            let stream = provider
                .chat_stream(request)
                .await
                .map_err(PromptKitError::OpenAIError)?
                .map(|result| result.map_err(PromptKitError::OpenAIError));

            Ok(Box::pin(stream))
        } else {
            Err(PromptKitError::ServiceNotInitialized {
                service: "OpenAI".to_string(),
            })
        }
    }

    async fn _chat_stream_ollama(
        &self,
        messages: Vec<Message>,
    ) -> Result<
        std::pin::Pin<Box<dyn Stream<Item = Result<String, PromptKitError>> + Send>>,
        PromptKitError,
    > {
        if let LLMProvider::Ollama(provider) = &self.provider {
            info!("Starting Ollama chat stream");
            let request = ChatRequest {
                messages,
                parameters: Default::default(),
                metadata: Default::default(),
            };

            let stream = provider
                .chat_stream(request)
                .await
                .map_err(PromptKitError::OllamaError)?
                .map(|result| result.map_err(PromptKitError::OllamaError));

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
        let provider =
            OpenAIProvider::new(config.clone()).expect("Failed to create OpenAI provider");
        Self {
            provider: LLMProvider::OpenAI(provider),
        }
    }
}

impl From<OllamaConfig> for PromptKitService {
    fn from(config: OllamaConfig) -> Self {
        let provider =
            OllamaProvider::new(config.clone()).expect("Failed to create Ollama provider");
        Self {
            provider: LLMProvider::Ollama(provider),
        }
    }
}

#[async_trait]
impl AsyncTryFrom<EuroraConfig> for PromptKitService {
    type Error = PromptKitError;
    async fn async_try_from(config: EuroraConfig) -> Result<Self, Self::Error> {
        let provider = EuroraStreamingProvider::new(config.clone())
            .await
            .map_err(PromptKitError::EuroraError)?;
        Ok(Self {
            provider: LLMProvider::Eurora(provider),
        })
    }
}
