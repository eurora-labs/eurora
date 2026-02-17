use url::Url;

#[derive(Debug, Clone, PartialEq)]
pub struct OllamaConfig {
    pub base_url: Url,
    pub model: String,
}
