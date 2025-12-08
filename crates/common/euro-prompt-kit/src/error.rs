use euro_eurora_provider::EuroraError;
use ferrous_llm::{ollama::OllamaError, openai::OpenAIError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PromptKitError {
    #[error("{0}")]
    OpenAIError(OpenAIError),

    #[error("{0}")]
    OllamaError(OllamaError),

    #[error("{0}")]
    EuroraError(EuroraError),

    #[error("{service} not initialized")]
    ServiceNotInitialized { service: String },
}
