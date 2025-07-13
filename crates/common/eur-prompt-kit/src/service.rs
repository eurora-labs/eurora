use crate::PromptKitError;
use anyhow::Result;
use ferrous_llm::{
    ChatRequest, Message, ProviderConfig, StreamingProvider,
    ollama::{OllamaConfig, OllamaProvider},
    openai::{OpenAIConfig, OpenAIProvider},
};
use tokio_stream::{Stream, StreamExt};

#[derive(Debug, Clone)]
enum LLMProvider {
    OpenAI(OpenAIProvider),
    Ollama(OllamaProvider),
}

pub trait PromptKitServiceTrait {
    fn get_service_name(&self) -> Result<String>;
}

#[derive(Debug, Clone)]
pub struct PromptKitService<T: ProviderConfig> {
    config: T,
    provider: LLMProvider,
}

impl PromptKitService<OpenAIConfig> {
    pub fn new(config: OpenAIConfig) -> Self {
        let provider =
            OpenAIProvider::new(config.clone()).expect("Failed to create OpenAI provider");
        Self {
            config,
            provider: LLMProvider::OpenAI(provider),
        }
    }

    pub async fn chat_stream(
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
                .map_err(|e| PromptKitError::OpenAIError(e))?
                .map(|result| result.map_err(|e| PromptKitError::OpenAIError(e)));

            Ok(Box::pin(stream))
        } else {
            Err(PromptKitError::ServiceNotInitialized {
                service: "OpenAI".to_string(),
            })
        }
    }
}

impl PromptKitServiceTrait for PromptKitService<OpenAIConfig> {
    fn get_service_name(&self) -> Result<String> {
        Ok("OpenAI".to_string())
    }
}

impl PromptKitService<OllamaConfig> {
    pub fn new(config: OllamaConfig) -> Self {
        let provider =
            OllamaProvider::new(config.clone()).expect("Failed to create Ollama provider");
        Self {
            config,
            provider: LLMProvider::Ollama(provider),
        }
    }

    pub async fn chat_stream(
        &self,
        messages: Vec<Message>,
    ) -> Result<
        std::pin::Pin<Box<dyn Stream<Item = Result<String, PromptKitError>> + Send>>,
        PromptKitError,
    > {
        if let LLMProvider::Ollama(provider) = &self.provider {
            let request = ChatRequest {
                messages,
                parameters: Default::default(),
                metadata: Default::default(),
            };

            let stream = provider
                .chat_stream(request)
                .await
                .map_err(|e| PromptKitError::OllamaError(e))?
                .map(|result| result.map_err(|e| PromptKitError::OllamaError(e)));

            Ok(Box::pin(stream))
        } else {
            Err(PromptKitError::ServiceNotInitialized {
                service: "Ollama".to_string(),
            })
        }
    }
}

impl PromptKitServiceTrait for PromptKitService<OllamaConfig> {
    fn get_service_name(&self) -> Result<String> {
        Ok("Ollama".to_string())
    }
}
