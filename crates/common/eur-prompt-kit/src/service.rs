use crate::{EurLLMService, LLMMessage};
use anyhow::Result;
use llm::{
    LLMProvider,
    builder::{LLMBackend, LLMBuilder},
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

    fn build_llm(&self) -> Result<Box<dyn LLMProvider>> {
        let llm = LLMBuilder::new()
            .backend(LLMBackend::from(self.llm_backend))
            .model("gpt-4o")
            .temperature(0.7)
            .stream(true)
            .build()
            .expect("Failed to build LLM");
        Ok(llm)
    }
}
