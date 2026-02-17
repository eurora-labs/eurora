mod ollama;
mod openai;

pub use ollama::OllamaConfig;
pub use openai::OpenAIConfig;

#[derive(Debug, Clone, PartialEq)]
pub enum ProviderSettings {
    Ollama(OllamaConfig),
    OpenAI(OpenAIConfig),
}

impl From<OllamaConfig> for ProviderSettings {
    fn from(config: OllamaConfig) -> Self {
        Self::Ollama(config)
    }
}

impl From<OpenAIConfig> for ProviderSettings {
    fn from(config: OpenAIConfig) -> Self {
        Self::OpenAI(config)
    }
}
