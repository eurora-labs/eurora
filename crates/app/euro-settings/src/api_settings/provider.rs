use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use specta::Type;

mod ollama;
mod openai;

use crate::error::Result;
pub use ollama::OllamaSettings;
pub use openai::OpenAISettings;

#[enum_dispatch(ProviderSettingsTrait)]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
pub enum ProviderSettings {
    OllamaSettings,
    OpenAISettings,
}

#[async_trait]
#[enum_dispatch]
pub trait ProviderSettingsTrait {
    async fn sync(&self, endpoint: &str) -> Result<()>;
}
