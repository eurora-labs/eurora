use crate::{EurLLMService, LLMMessage};
use anyhow::Result;
use eur_util::redact_emails;
use futures::Stream;
use llm::{
    builder::{LLMBackend, LLMBuilder},
    chat::ChatMessage,
    error::LLMError,
};

#[derive(Debug)]
pub struct PromptKitService {
    llm_backend: EurLLMService,
    model: String,
}

impl Default for PromptKitService {
    fn default() -> Self {
        Self::new(EurLLMService::OpenAI, "gpt-4o".to_string())
    }
}

impl PromptKitService {
    pub fn new(llm_backend: EurLLMService, model: String) -> Self {
        Self { llm_backend, model }
    }

    pub async fn anonymize_text(text: String) -> Result<String> {
        let base_url = "http://127.0.0.1:11434".to_string();
        let original_text = text.clone();

        // Send messages to self-hosted LLM with instruction to remove personal data
        let llm = LLMBuilder::new()
            .backend(LLMBackend::Ollama)
            .base_url(base_url)
            .model("deepseek-v2:16b")
            .max_tokens(128)
            .temperature(0.1)
            .top_p(0.1)
            .stream(false)
            .build()
            .expect("Failed to build LLM (Ollama)");

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

        let sensitive_words = response_text.split(',').collect::<Vec<&str>>();

        let mut text = original_text.to_lowercase();
        sensitive_words.iter().for_each(|word| {
            let word = word.to_lowercase().to_string();
            text = text.replace(&word, " <REDACTED> ");
        });
        text = redact_emails(text);

        Ok(text)
    }

    pub async fn chat_stream(
        &self,
        messages: Vec<LLMMessage>,
    ) -> Result<std::pin::Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>, LLMError>
    {
        let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not found");

        // Let's try with explicit configuration to ensure streaming works properly
        let llm = LLMBuilder::new()
            .backend(LLMBackend::from(self.llm_backend))
            .model(&self.model)
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
}
