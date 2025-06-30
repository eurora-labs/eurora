use crate::{
    LLMMessage, OllamaConfig, RemoteConfig,
    config::{Config, EuroraConfig},
};
use anyhow::Result;
use eur_util::redact_emails;
use futures::Stream;
use llm::{
    builder::{LLMBackend, LLMBuilder},
    chat::ChatMessage,
    error::LLMError,
};

#[derive(Debug, Clone)]
pub struct PromptKitService {
    config: Option<Config>,
}

impl Default for PromptKitService {
    fn default() -> Self {
        Self::new(None)
    }
}

impl PromptKitService {
    pub fn get_service_name(&self) -> Result<String> {
        if let Some(config) = &self.config {
            return Ok(config.get_display_name());
        }

        Err(anyhow::anyhow!("No LLM backend configured"))
    }
}

impl PromptKitService {
    pub fn new(config: Option<Config>) -> Self {
        Self { config }
    }

    pub async fn anonymize_text(text: String) -> Result<String> {
        let base_url = std::env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
        let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "deepseek-v2:16b".to_string());
        let original_text = text.clone();
        // Send messages to self-hosted LLM with instruction to remove personal data
        let llm = LLMBuilder::new()
            .backend(LLMBackend::Ollama)
            .base_url(base_url)
            .model(&model)
            .max_tokens(128)
            .temperature(0.1)
            .top_p(0.1)
            .stream(false)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build LLM (Ollama): {}", e))?;

        let messages = vec![
            ChatMessage::user()
                .content(format!("You are a redactor. 
Input: {}  
Rules:  
1. Extract every substring that can identify a natural person (name, address, phone, e-mail, numeric ID, date of birth, GPS coordinate, licence plate, face-recognisable description, biometric string).  
2. Preserve original casing and punctuation.  
3. Return one comma-separated line; no duplicates; no extra text; output “NONE” if nothing found.  ", text.to_lowercase()))
                .build(),
        ];

        // eprintln!("Messages: {:#?}", messages);

        let response = match llm.chat(&messages).await {
            Ok(response) => response,
            Err(e) => return Err(e.into()),
        };

        let response_text = response.text().unwrap_or_default();
        let sensitive_words: Vec<String> = response_text
            .split(',')
            .map(|word| word.trim().to_string())
            .filter(|word| !word.is_empty() && word.to_uppercase() != "NONE")
            .collect();

        let mut text = original_text;
        for word in sensitive_words {
            // Case-insensitive replacement using regex
            let pattern = regex::Regex::new(&regex::escape(word.trim()))
                .map_err(|e| anyhow::anyhow!("Invalid regex pattern: {}", e))?;
            text = pattern.replace_all(&text, " <REDACTED> ").to_string();
        }
        text = redact_emails(text);

        Ok(text)
    }

    pub async fn chat_stream(
        &self,
        messages: Vec<LLMMessage>,
    ) -> Result<std::pin::Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>, LLMError>
    {
        if self.config.is_none() {
            return Err(LLMError::Generic("No LLM config set".to_string()));
        }

        let config = self.config.as_ref().unwrap();
        match config {
            Config::Ollama(config) => self._ollama_chat_stream(messages, config).await,
            Config::Remote(config) => self._remote_chat_stream(messages, config).await,
            _ => Err(LLMError::Generic("Unsupported LLM backend".to_string())),
        }
    }

    async fn _remote_chat_stream(
        &self,
        messages: Vec<LLMMessage>,
        config: &RemoteConfig,
    ) -> Result<std::pin::Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>, LLMError>
    {
        let remote_config = config;

        let api_key = remote_config.api_key.clone();

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

    pub async fn switch_to_remote(&mut self, config: RemoteConfig) -> Result<(), String> {
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
