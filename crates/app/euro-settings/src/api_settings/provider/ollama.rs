use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Type)]
pub struct OllamaSettings {
    pub base_url: String,
    pub model: String,
}
