use crate::{EurLLMService, LLMMessage};
use anyhow::Result;
use futures::Stream;
use llm::{
    builder::{LLMBackend, LLMBuilder},
    chat::ChatMessage,
    error::LLMError,
};

#[derive(Debug)]
pub struct PromptKitService {
    llm_backend: EurLLMService,
}

impl Default for PromptKitService {
    fn default() -> Self {
        Self::new(EurLLMService::OpenAI)
    }
}

impl PromptKitService {
    pub fn new(llm_backend: EurLLMService) -> Self {
        Self { llm_backend }
    }

    #[allow(dead_code)]
    async fn anonymize_text(text: String) -> Result<String> {
        // Send messages to self-hosted LLM with instruction to remove personal data
        let llm = LLMBuilder::new()
            .backend(LLMBackend::OpenAI)
            .model("gpt-4.5-turbo")
            .temperature(0.7)
            .stream(false)
            .build()
            .expect("Failed to build LLM (OpenAI)");
        let messages = vec![
            ChatMessage::user()
                .content("Anonymize the text and remove any personal data from the next message: ")
                .build(),
            ChatMessage::user().content(text).build(),
        ];

        let response = match llm.chat(&messages).await {
            Ok(response) => response,
            Err(e) => return Err(e.into()),
        };

        Ok(response.text().unwrap_or_default())
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
            .model("gpt-4o")
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
