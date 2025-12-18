mod error;
mod service;
pub use error::PromptKitError;
pub use euro_llm::ProviderConfig;
pub use euro_llm::openai::OpenAIConfig;
pub use service::{OllamaConfig, PromptKitService};

#[derive(Debug, Default, Copy, Clone)]
pub enum EurLLMService {
    #[default]
    OpenAI,
    Anthropic,
    Google,
    Eurora,
    Local,
    Ollama,
}

impl From<String> for EurLLMService {
    fn from(value: String) -> Self {
        match value.as_str() {
            "openai" => EurLLMService::OpenAI,
            "anthropic" => EurLLMService::Anthropic,
            "google" => EurLLMService::Google,
            "eurora" => EurLLMService::Eurora,
            "local" => EurLLMService::Local,
            "ollama" => EurLLMService::Ollama,
            _ => EurLLMService::OpenAI,
        }
    }
}
