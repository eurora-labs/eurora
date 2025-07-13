use anyhow::Result;
use ferrous_llm::{ollama::{OllamaConfig, OllamaProvider}, openai::{OpenAIConfig, OpenAIProvider}, ChatRequest, Message, ProviderConfig, StreamingProvider};
use tokio_stream::{Stream, StreamExt};
use crate::PromptKitError;

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
        let provider = OpenAIProvider::new(config.clone()).expect("Failed to create OpenAI provider");
        Self { config, provider: LLMProvider::OpenAI(provider) }
    }

    pub async fn chat_stream(&self, messages: Vec<Message>) -> Result<std::pin::Pin<Box<dyn Stream<Item = Result<String, PromptKitError>> + Send>>, PromptKitError> {
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
            Err(PromptKitError::ServiceNotInitialized { service: "OpenAI".to_string() })
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
        let provider = OllamaProvider::new(config.clone()).expect("Failed to create Ollama provider");
        Self { config, provider: LLMProvider::Ollama(provider) }
    }

    pub async fn chat_stream(&self, messages: Vec<Message>) -> Result<std::pin::Pin<Box<dyn Stream<Item = Result<String, PromptKitError>> + Send>>, PromptKitError> {
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
            Err(PromptKitError::ServiceNotInitialized { service: "Ollama".to_string() })
        }
    }
}

impl PromptKitServiceTrait for PromptKitService<OllamaConfig> {
    fn get_service_name(&self) -> Result<String> {
        Ok("Ollama".to_string())
    }
}

impl<T: ProviderConfig> PromptKitService<T> {
    // pub fn new(config: Option<T>) -> Self {
    //     Self { config }
    // }

    //     pub async fn anonymize_text(text: String) -> Result<String> {
    //         let base_url = std::env::var("OLLAMA_BASE_URL")
    //             .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
    //         let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "deepseek-v2:16b".to_string());
    //         let original_text = text.clone();
    //         // Send messages to self-hosted LLM with instruction to remove personal data
    //         let llm = LLMBuilder::new()
    //             .backend(LLMBackend::Ollama)
    //             .base_url(base_url)
    //             .model(&model)
    //             .max_tokens(128)
    //             .temperature(0.1)
    //             .top_p(0.1)
    //             .stream(false)
    //             .build()
    //             .map_err(|e| anyhow::anyhow!("Failed to build LLM (Ollama): {}", e))?;

    //         let messages = vec![
    //             ChatMessage::user()
    //                 .content(format!("You are a redactor.
    // Input: {}
    // Rules:
    // 1. Extract every substring that can identify a natural person (name, address, phone, e-mail, numeric ID, date of birth, GPS coordinate, licence plate, face-recognisable description, biometric string).
    // 2. Preserve original casing and punctuation.
    // 3. Return one comma-separated line; no duplicates; no extra text; output “NONE” if nothing found.  ", text.to_lowercase()))
    //                 .build(),
    //         ];

    //         // eprintln!("Messages: {:#?}", messages);

    //         let response = match llm.chat(&messages).await {
    //             Ok(response) => response,
    //             Err(e) => return Err(e.into()),
    //         };

    //         let response_text = response.text().unwrap_or_default();
    //         let sensitive_words: Vec<String> = response_text
    //             .split(',')
    //             .map(|word| word.trim().to_string())
    //             .filter(|word| !word.is_empty() && word.to_uppercase() != "NONE")
    //             .collect();

    //         let mut text = original_text;
    //         for word in sensitive_words {
    //             // Case-insensitive replacement using regex
    //             let pattern = regex::Regex::new(&regex::escape(word.trim()))
    //                 .map_err(|e| anyhow::anyhow!("Invalid regex pattern: {}", e))?;
    //             text = pattern.replace_all(&text, " <REDACTED> ").to_string();
    //         }
    //         text = redact_emails(text);

    //         Ok(text)
    //     }

    // pub async fn chat_stream(
    //     &self,
    //     messages: Vec<LLMMessage>,
    // ) -> Result<std::pin::Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>, LLMError>
    // {
    //     if self.config.is_none() {
    //         return Err(LLMError::Generic("No LLM config set".to_string()));
    //     }

    //     let config = self.config.as_ref().unwrap();
    //     match config {
    //         Config::Ollama(config) => self._ollama_chat_stream(messages, config).await,
    //         Config::Remote(config) => self._remote_chat_stream(messages, config).await,
    //         Config::Eurora(config) => self._eurora_chat_stream(messages, config).await,
    //     }
    // }

    // async fn _eurora_chat_stream(
    //     &self,
    //     messages: Vec<LLMMessage>,
    //     _config: &EuroraConfig,
    // ) -> Result<std::pin::Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>, LLMError>
    // {
    //     let client = PromptClient::new(None)
    //         .await
    //         .map_err(|e| LLMError::Generic(e.to_string()))?;

    //     let messages = messages.into_iter().map(|message| message.into()).collect();
    //     let stream = client
    //         .send_prompt(SendPromptRequest { messages })
    //         .await
    //         .map_err(|e| LLMError::Generic(e.to_string()))?;

    //     // Direct stream mapping without intermediate channels - much simpler!
    //     let mapped_stream = stream.map(|result| {
    //         result
    //             .map(|response| response.response)
    //             .map_err(|e| LLMError::Generic(e.to_string()))
    //     });

    //     Ok(Box::pin(mapped_stream))
    // }

    async fn _remote_chat_stream(
        &self,
        messages: Vec<Message>,
        config: &OpenAIConfig
    ) -> Result<std::pin::Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>, LLMError>
    {

        let provider = OpenAIProvider::new(config.clone());




        let remote_config = config;



        let api_key = remote_config.api_key.clone();


        let provider = OpenAIProvider::new

        // Let's try with explicit configuration to ensure streaming works properly
        let llm = LLMBuilder::new()
            .backend(LLMBackend::from(remote_config.provider))
            .model(&remote_config.model)
            .api_key(api_key)
            .temperature(0.7)
            .stream(true)
            .build()?;

        let chat_messages = messages
            .into_iter()
            .map(|message| message.into())
            .collect::<Vec<ChatMessage>>();

        llm.chat_stream(&chat_messages).await
    }

    async fn _ollama_chat_stream(
        &self,
        messages: Vec<LLMMessage>,
        config: &OllamaConfig,
    ) -> Result<std::pin::Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>, LLMError>
    {
        let llm = LLMBuilder::new()
            .backend(LLMBackend::from(config.get_llm_backend()))
            .model(&config.model)
            .base_url(&config.base_url)
            .temperature(0.7)
            .stream(true)
            .build()
            .map_err(|e| LLMError::Generic(format!("Failed to build LLM (Ollama): {}", e)))?;

        let chat_messages = messages
            .into_iter()
            .map(|message| message.into())
            .collect::<Vec<ChatMessage>>();

        llm.chat_stream(&chat_messages).await
    }

    pub async fn switch_to_ollama(&mut self, config: OllamaConfig) -> Result<(), String> {
        // Validate the configuration
        if config.base_url.is_empty() {
            return Err("Base URL cannot be empty".to_string());
        }

        if config.model.is_empty() {
            return Err("Model name cannot be empty".to_string());
        }

        // Optionally validate URL format
        if !config.base_url.starts_with("http://") && !config.base_url.starts_with("https://") {
            return Err("Base URL must start with http:// or https://".to_string());
        }

        let llm = LLMBuilder::new()
            .backend(LLMBackend::Ollama)
            .base_url(&config.base_url)
            .model(&config.model)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build LLM (Ollama): {}", e))
            .map_err(|e| e.to_string())?;

        let is_healthy = llm.health_check().await;

        if is_healthy.is_err() {
            return Err("Ollama is not healthy".to_string());
        }

        self.config = Some(Config::Ollama(config));

        Ok(())
    }

    pub fn switch_to_remote(&mut self, config: RemoteConfig) -> Result<(), String> {
        // Validate the configuration
        if config.model.is_empty() {
            return Err("Model name cannot be empty".to_string());
        }

        if config.api_key.is_empty() {
            return Err("API key cannot be empty".to_string());
        }

        self.config = Some(Config::Remote(config));

        Ok(())
    }

    pub async fn switch_to_eurora(&mut self, config: EuroraConfig) -> Result<(), String> {
        self.config = Some(Config::Eurora(config));

        Ok(())
    }
}
