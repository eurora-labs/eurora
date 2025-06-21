use crate::{EurLLMService, LLMMessage};
use anyhow::Result;
use llm::{
    LLMProvider,
    builder::{LLMBackend, LLMBuilder},
    chat::ChatMessage,
};

#[derive(Debug, Default)]
pub struct PromptKitService {
    llm_backend: EurLLMService,
}

impl PromptKitService {
    pub fn new(llm_backend: EurLLMService) -> Self {
        Self { llm_backend }
    }

    pub fn default() -> Self {
        Self::new(EurLLMService::OpenAI)
    }

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

    pub async fn chat(&self, messages: Vec<LLMMessage>) -> Result<String> {
        let llm = LLMBuilder::new()
            .backend(LLMBackend::from(self.llm_backend))
            .model("gpt-4o")
            .temperature(0.7)
            .build()
            .expect("Failed to build LLM");
        let messages = messages
            .into_iter()
            .map(|message| message.into())
            .collect::<Vec<_>>();
        match llm.chat_stream(&messages).await {
            Ok(mut stream) => {
                let stdout = io::stdout();
                let mut handle = stdout.lock();

                while let Some(Ok(token)) = stream.next().await {
                    handle.write_all(token.as_bytes()).unwrap();
                    handle.flush().unwrap();
                }
                println!("\n\nStreaming completed.");
            }
            Err(e) => eprintln!("Chat error: {}", e),
        };
        let response = llm.chat(&messages).await;
        Ok(response)
    }
}
