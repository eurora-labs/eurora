use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::caches::BaseCache;
use crate::callbacks::Callbacks;
use crate::error::Result;
use crate::messages::{AIMessage, AnyMessage};
use crate::outputs::LLMResult;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LangSmithParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_provider: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_model_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_model_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_temperature: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_max_tokens: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls_stop: Option<Vec<String>>,
}

#[bon::bon]
impl LangSmithParams {
    #[builder]
    pub fn new(
        #[builder(into)] provider: Option<String>,
        #[builder(into)] model_name: Option<String>,
        #[builder(into)] model_type: Option<String>,
        temperature: Option<f64>,
        max_tokens: Option<u32>,
        stop: Option<Vec<String>>,
    ) -> Self {
        Self {
            ls_provider: provider,
            ls_model_name: model_name,
            ls_model_type: model_type,
            ls_temperature: temperature,
            ls_max_tokens: max_tokens,
            ls_stop: stop,
        }
    }
}

pub type CustomGetTokenIds = fn(&str) -> Vec<u32>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LanguageModelConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbose: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,

    #[serde(skip)]
    pub custom_get_token_ids: Option<CustomGetTokenIds>,

    #[serde(skip)]
    pub callbacks: Option<Callbacks>,
}

#[bon::bon]
impl LanguageModelConfig {
    #[builder]
    pub fn new(
        cache: Option<bool>,
        verbose: Option<bool>,
        tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, Value>>,
        custom_get_token_ids: Option<CustomGetTokenIds>,
        callbacks: Option<Callbacks>,
    ) -> Self {
        Self {
            cache,
            verbose,
            tags,
            metadata,
            custom_get_token_ids,
            callbacks,
        }
    }
}

#[async_trait]
pub trait BaseLanguageModel: Send + Sync {
    fn llm_type(&self) -> &str;

    fn model_name(&self) -> &str;

    fn config(&self) -> &LanguageModelConfig;

    fn cache(&self) -> Option<&dyn BaseCache> {
        None
    }

    fn callbacks(&self) -> Option<&Callbacks> {
        None
    }

    fn verbose(&self) -> bool {
        self.config().verbose.unwrap_or(false)
    }

    async fn generate_prompt(
        &self,
        prompts: Vec<Vec<AnyMessage>>,
        stop: Option<Vec<String>>,
        callbacks: Option<Callbacks>,
    ) -> Result<LLMResult>;

    fn get_ls_params(&self, stop: Option<&[String]>) -> LangSmithParams {
        let llm_type = self.llm_type();
        let provider = if llm_type.starts_with("Chat") {
            llm_type
                .strip_prefix("Chat")
                .unwrap_or(llm_type)
                .to_lowercase()
        } else if llm_type.ends_with("Chat") {
            llm_type
                .strip_suffix("Chat")
                .unwrap_or(llm_type)
                .to_lowercase()
        } else {
            llm_type.to_lowercase()
        };

        LangSmithParams::builder()
            .provider(provider)
            .model_name(self.model_name().to_string())
            .maybe_stop(stop.map(|s| s.to_vec()))
            .build()
    }

    fn identifying_params(&self) -> HashMap<String, Value> {
        let mut params = HashMap::new();
        params.insert(
            "_type".to_string(),
            Value::String(self.llm_type().to_string()),
        );
        params.insert(
            "model".to_string(),
            Value::String(self.model_name().to_string()),
        );
        params
    }

    fn get_token_ids(&self, text: &str) -> Vec<u32> {
        text.split_whitespace()
            .enumerate()
            .map(|(i, _)| i as u32)
            .collect()
    }

    fn get_num_tokens(&self, text: &str) -> usize {
        self.get_token_ids(text).len()
    }

    fn get_num_tokens_from_messages(
        &self,
        messages: &[AnyMessage],
        _tools: Option<&[crate::tools::ToolDefinition]>,
    ) -> usize {
        messages
            .iter()
            .map(|m| {
                let role_tokens = 4; // Approximate overhead for role
                let content_tokens = self.get_num_tokens(&m.text());
                role_tokens + content_tokens
            })
            .sum()
    }
}

pub type LanguageModelLike =
    Arc<dyn crate::runnables::base::Runnable<Input = Vec<AnyMessage>, Output = AIMessage>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_langsmith_params_builder() {
        let params = LangSmithParams::builder()
            .provider("openai")
            .model_name("gpt-4")
            .model_type("chat")
            .temperature(0.7)
            .max_tokens(1000)
            .stop(vec!["STOP".to_string()])
            .build();

        assert_eq!(params.ls_provider, Some("openai".to_string()));
        assert_eq!(params.ls_model_name, Some("gpt-4".to_string()));
        assert_eq!(params.ls_model_type, Some("chat".to_string()));
        assert_eq!(params.ls_temperature, Some(0.7));
        assert_eq!(params.ls_max_tokens, Some(1000));
        assert_eq!(params.ls_stop, Some(vec!["STOP".to_string()]));
    }

    #[test]
    fn test_ai_message_text() {
        let output = AIMessage::builder().content("Hello").build();
        assert_eq!(output.text(), "Hello");
    }

    #[test]
    fn test_language_model_config_builder() {
        let config = LanguageModelConfig::builder()
            .cache(true)
            .tags(vec!["test".to_string()])
            .build();

        assert_eq!(config.cache, Some(true));
        assert_eq!(config.tags, Some(vec!["test".to_string()]));
    }
}
