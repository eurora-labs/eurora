use serde::{Deserialize, Serialize};
use specta::Type;

mod ollama;
mod openai;

pub use openai::OpenAISettings;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
pub enum ProviderSettings {
    OllamaSettings,
    OpenAISettings,
}
