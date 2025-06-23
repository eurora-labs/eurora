#[derive(Debug)]
pub struct OllamaConfig {
    pub base_url: String,
    pub model: String,
}

#[derive(Debug)]
pub struct RemoteConfig {
    pub api_key: String,
    pub model: String,
}
