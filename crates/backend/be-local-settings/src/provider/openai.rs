use url::Url;

use crate::Redacted;

#[derive(Debug, Clone, PartialEq)]
pub struct OpenAIConfig {
    pub base_url: Url,
    pub api_key: Redacted<String>,
    pub model: String,
    pub title_model: Option<String>,
}
