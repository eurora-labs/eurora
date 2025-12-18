use agent_chain::chat_models::ChatModel;
use agent_chain::messages::{AIMessage, BaseMessage, HumanMessage, SystemMessage};
use agent_chain::ollama::ChatOllama;
use agent_chain::{ChatOpenAI, ContentPart, ImageDetail, ImageSource};
use anyhow::Result;
use async_from::{AsyncTryFrom, async_trait};
use euro_llm::{ChatRequest, Message, Role};
use euro_llm_eurora::{EuroraConfig, EuroraStreamingProvider, StreamingProvider};
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
    Eurora(EuroraStreamingProvider),
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
        if let LLMProvider::OpenAI(llm) = &self.provider {
            info!("Starting OpenAI chat stream with agent-chain");

            // Convert euro_llm::Message to agent_chain::BaseMessage
            let base_messages: Vec<BaseMessage> = messages
                .into_iter()
                .map(convert_message_to_base_message)
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
                .map(convert_message_to_base_message)
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
    // Helper function to extract text content from MessageContent
    fn extract_text_content(content: euro_llm::MessageContent) -> String {
        match content {
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
        }
    }

    match msg.role {
        Role::User => match msg.content {
            euro_llm::MessageContent::Text(text) => HumanMessage::new(text).into(),
            euro_llm::MessageContent::Multimodal(parts) => {
                let content_parts = parts
                    .into_iter()
                    .map(|part| match part {
                        euro_llm::ContentPart::Text { text } => ContentPart::Text { text },
                        euro_llm::ContentPart::Image {
                            image_source,
                            detail: _,
                        } => convert_image_source_to_content_part(image_source),
                        euro_llm::ContentPart::Audio { .. } => {
                            // Audio not directly supported, skip or convert to placeholder
                            ContentPart::Text {
                                text: "[Audio content]".to_string(),
                            }
                        }
                    })
                    .collect();
                HumanMessage::with_content(content_parts).into()
            }
            euro_llm::MessageContent::Tool(tool_content) => {
                HumanMessage::new(tool_content.text.unwrap_or_default()).into()
            }
        },
        Role::Assistant => {
            let content = extract_text_content(msg.content);
            AIMessage::new(content).into()
        }
        Role::System => {
            let content = extract_text_content(msg.content);
            SystemMessage::new(content).into()
        }
        Role::Tool => {
            // For tool messages, we'll convert to AI message with the content
            // In a more complete implementation, you'd use ToolMessage
            let content = extract_text_content(msg.content);
            AIMessage::new(content).into()
        }
    }
}

/// Convert euro_llm::ImageSource to agent_chain::ContentPart
fn convert_image_source_to_content_part(image_source: euro_llm::ImageSource) -> ContentPart {
    match image_source {
        euro_llm::ImageSource::Url(url) => {
            // Check if it's a data URL (base64 encoded)
            if url.starts_with("data:") {
                // Parse data URL: data:[<mediatype>][;base64],<data>
                if let Some((header, data)) =
                    url.strip_prefix("data:").and_then(|s| s.split_once(','))
                {
                    let media_type = header.split(';').next().unwrap_or("image/jpeg").to_string();
                    ContentPart::Image {
                        source: ImageSource::Base64 {
                            media_type,
                            data: data.to_string(),
                        },
                        detail: Some(ImageDetail::default()),
                    }
                } else {
                    // Invalid data URL, treat as regular URL
                    ContentPart::Image {
                        source: ImageSource::Url { url },
                        detail: Some(ImageDetail::default()),
                    }
                }
            } else {
                // Regular URL
                ContentPart::Image {
                    source: ImageSource::Url { url },
                    detail: Some(ImageDetail::default()),
                }
            }
        }
        _ => panic!("Unsupported image source: only URL is supported"),
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
        let provider = EuroraStreamingProvider::new(config.clone())
            .await
            .map_err(PromptKitError::EuroraError)?;
        Ok(Self {
            provider: LLMProvider::Eurora(provider),
        })
    }
}
