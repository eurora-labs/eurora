use ferrous_llm::{ollama::OllamaError, openai::OpenAIError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PromptKitError {
    #[error("{0}")]
    OpenAIError(OpenAIError),

    #[error("{0}")]
    OllamaError(OllamaError),

    #[error("{service} not initialized")]
    ServiceNotInitialized { service: String },
}
