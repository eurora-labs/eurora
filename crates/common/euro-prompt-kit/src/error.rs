use euro_llm_eurora::EuroraError;
use euro_llm_ollama::OllamaError;
use euro_llm_openai::OpenAIError;
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
