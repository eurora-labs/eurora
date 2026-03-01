use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::caches::BaseCache;
use crate::callbacks::Callbacks;
use crate::error::Result;
use crate::messages::{AIMessage, BaseMessage};
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

use crate::prompt_values::{ChatPromptValue, ImagePromptValue, StringPromptValue};

#[derive(Debug, Clone)]
pub enum LanguageModelInput {
    Text(String),
    StringPrompt(StringPromptValue),
    ChatPrompt(ChatPromptValue),
    ImagePrompt(ImagePromptValue),
    Messages(Vec<BaseMessage>),
}

impl From<String> for LanguageModelInput {
    fn from(s: String) -> Self {
        LanguageModelInput::Text(s)
    }
}

impl From<&str> for LanguageModelInput {
    fn from(s: &str) -> Self {
        LanguageModelInput::Text(s.to_string())
    }
}

impl From<StringPromptValue> for LanguageModelInput {
    fn from(p: StringPromptValue) -> Self {
        LanguageModelInput::StringPrompt(p)
    }
}

impl From<ChatPromptValue> for LanguageModelInput {
    fn from(p: ChatPromptValue) -> Self {
        LanguageModelInput::ChatPrompt(p)
    }
}

impl From<ImagePromptValue> for LanguageModelInput {
    fn from(p: ImagePromptValue) -> Self {
        LanguageModelInput::ImagePrompt(p)
    }
}

impl From<Vec<BaseMessage>> for LanguageModelInput {
    fn from(m: Vec<BaseMessage>) -> Self {
        LanguageModelInput::Messages(m)
    }
}

impl LanguageModelInput {
    pub fn to_messages(&self) -> Vec<BaseMessage> {
        use crate::prompt_values::PromptValue;
        match self {
            LanguageModelInput::Text(s) => {
                vec![BaseMessage::Human(
                    crate::messages::HumanMessage::builder()
                        .content(s.as_str())
                        .build(),
                )]
            }
            LanguageModelInput::StringPrompt(p) => p.to_messages(),
            LanguageModelInput::ChatPrompt(p) => p.to_messages(),
            LanguageModelInput::ImagePrompt(p) => p.to_messages(),
            LanguageModelInput::Messages(m) => m.clone(),
        }
    }
}

impl std::fmt::Display for LanguageModelInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::prompt_values::PromptValue;
        match self {
            LanguageModelInput::Text(s) => write!(f, "{}", s),
            LanguageModelInput::StringPrompt(p) => write!(f, "{}", PromptValue::to_string(p)),
            LanguageModelInput::ChatPrompt(p) => write!(f, "{}", PromptValue::to_string(p)),
            LanguageModelInput::ImagePrompt(p) => write!(f, "{}", PromptValue::to_string(p)),
            LanguageModelInput::Messages(m) => {
                let joined = m
                    .iter()
                    .map(|msg| format!("{}: {}", msg.message_type(), msg.text()))
                    .collect::<Vec<_>>()
                    .join("\n");
                write!(f, "{}", joined)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum LanguageModelOutput {
    Message(Box<AIMessage>),
    Text(String),
}

impl From<AIMessage> for LanguageModelOutput {
    fn from(m: AIMessage) -> Self {
        LanguageModelOutput::Message(Box::new(m))
    }
}

impl From<String> for LanguageModelOutput {
    fn from(s: String) -> Self {
        LanguageModelOutput::Text(s)
    }
}

impl LanguageModelOutput {
    pub fn text(&self) -> String {
        match self {
            LanguageModelOutput::Message(m) => m.text(),
            LanguageModelOutput::Text(s) => s.clone(),
        }
    }

    pub fn into_text(self) -> String {
        match self {
            LanguageModelOutput::Message(m) => m.text(),
            LanguageModelOutput::Text(s) => s,
        }
    }

    pub fn message(m: AIMessage) -> Self {
        LanguageModelOutput::Message(Box::new(m))
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
        prompts: Vec<LanguageModelInput>,
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
        messages: &[BaseMessage],
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

#[allow(dead_code)]
pub type LanguageModelOutputStream =
    Pin<Box<dyn Stream<Item = Result<LanguageModelOutput>> + Send>>;

pub type LanguageModelLike = Arc<
    dyn crate::runnables::base::Runnable<Input = LanguageModelInput, Output = LanguageModelOutput>,
>;

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
    fn test_language_model_input_from_str() {
        let input: LanguageModelInput = "Hello".into();
        match input {
            LanguageModelInput::Text(s) => assert_eq!(s, "Hello"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_language_model_output_text() {
        let output = LanguageModelOutput::Text("Hello".to_string());
        assert_eq!(output.text(), "Hello");
        assert_eq!(output.into_text(), "Hello");
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
