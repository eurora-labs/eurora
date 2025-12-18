use agent_chain::chat_models::ChatModel;
use agent_chain::messages::{AIMessage, BaseMessage, HumanMessage, SystemMessage};
use agent_chain::ollama::ChatOllama;
use anyhow::Result;
use async_from::{AsyncTryFrom, async_trait};
use euro_llm::openai::{OpenAIConfig, OpenAIProvider};
use euro_llm::{ChatRequest, Message, Role};
use euro_llm_eurora::{EuroraConfig, EuroraStreamingProvider, StreamingProvider};
use serde::{Deserialize, Serialize};
use tokio_stream::{Stream, StreamExt};
use tracing::info;

use crate::PromptKitError;

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
    OpenAI(OpenAIProvider),
    Ollama(ChatOllama),
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
        if let LLMProvider::Ollama(llm) = &self.provider {
            info!("Starting Ollama chat stream with agent-chain");

            // Convert euro_llm::Message to agent_chain::BaseMessage
            let base_messages: Vec<BaseMessage> = messages
                .into_iter()
                .map(|msg| convert_message_to_base_message(msg))
                .collect();

            let stream = llm
                .stream(base_messages, None)
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

/// Convert euro_llm::Message to agent_chain::BaseMessage
fn convert_message_to_base_message(msg: Message) -> BaseMessage {
    let content = match msg.content {
        euro_llm::MessageContent::Text(text) => text,
        euro_llm::MessageContent::Multimodal(parts) => {
            // Extract text content from multimodal parts
            parts
                .into_iter()
                .filter_map(|part| match part {
                    euro_llm::ContentPart::Text { text } => Some(text),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        euro_llm::MessageContent::Tool(tool_content) => tool_content.text.unwrap_or_default(),
    };

    match msg.role {
        Role::User => HumanMessage::new(content).into(),
        Role::Assistant => AIMessage::new(content).into(),
        Role::System => SystemMessage::new(content).into(),
        Role::Tool => {
            // For tool messages, we'll convert to AI message with the content
            // In a more complete implementation, you'd use ToolMessage
            AIMessage::new(content).into()
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
        let provider = EuroraStreamingProvider::new(config.clone())
            .await
            .map_err(PromptKitError::EuroraError)?;
        Ok(Self {
            provider: LLMProvider::Eurora(provider),
        })
    }
}
