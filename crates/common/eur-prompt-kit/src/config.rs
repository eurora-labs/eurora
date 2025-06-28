use crate::EurLLMService;
use core::fmt::Debug;
#[derive(Debug, Clone)]
pub enum Config {
    #[allow(dead_code)]
    Eurora(EuroraConfig),
    Ollama(OllamaConfig),
    Remote(RemoteConfig),
}

impl Config {
    pub fn get_display_name(&self) -> String {
        match self {
            Config::Eurora(config) => config.get_display_name(),
            Config::Ollama(config) => config.get_display_name(),
            Config::Remote(config) => config.get_display_name(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OllamaConfig {
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct RemoteConfig {
    pub provider: EurLLMService,
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct EuroraConfig {
    pub model: String,
}

impl EuroraConfig {
    pub fn get_display_name(&self) -> String {
        format!("Eurora ({})", self.model)
    }
    // pub fn get_llm_backend(&self) -> EurLLMService {
    //     EurLLMService::Eurora
    // }
}

impl OllamaConfig {
    pub fn get_display_name(&self) -> String {
        format!("Ollama ({})", self.model)
    }

    pub fn get_llm_backend(&self) -> EurLLMService {
        EurLLMService::Ollama
    }
}

impl RemoteConfig {
    pub fn get_display_name(&self) -> String {
        format!("Remote ({})", self.model)
    }

    pub fn get_llm_backend(&self) -> EurLLMService {
        self.provider
    }
}
