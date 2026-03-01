use std::collections::HashMap;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

use uuid::Uuid;

use crate::messages::{BaseMessage, UsageMetadata};
use crate::outputs::ChatResult;

use super::base::{
    BaseCallbackHandler, CallbackManagerMixin, ChainManagerMixin, LLMManagerMixin,
    RetrieverManagerMixin, RunManagerMixin, ToolManagerMixin,
};

#[derive(Debug, Clone)]
pub struct UsageMetadataCallbackHandler {
    usage_metadata: Arc<Mutex<HashMap<String, UsageMetadata>>>,
}

impl Default for UsageMetadataCallbackHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl UsageMetadataCallbackHandler {
    pub fn new() -> Self {
        Self {
            usage_metadata: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn usage_metadata(&self) -> HashMap<String, UsageMetadata> {
        self.usage_metadata
            .lock()
            .expect("usage_metadata lock poisoned")
            .clone()
    }
}

impl fmt::Display for UsageMetadataCallbackHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}",
            self.usage_metadata
                .lock()
                .expect("usage_metadata lock poisoned")
        )
    }
}

impl LLMManagerMixin for UsageMetadataCallbackHandler {
    fn on_llm_end(&self, response: &ChatResult, _run_id: Uuid, _parent_run_id: Option<Uuid>) {
        let first_generation = response.generations.first();

        let (usage_metadata, model_name) = match first_generation {
            Some(generation) => {
                let usage = match &generation.message {
                    BaseMessage::AI(ai_msg) => ai_msg.usage_metadata.clone(),
                    _ => None,
                };

                let model = generation
                    .message
                    .response_metadata()
                    .and_then(|meta| meta.get("model_name"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                (usage, model)
            }
            None => (None, None),
        };

        if let (Some(usage), Some(model)) = (usage_metadata, model_name) {
            let mut guard = self
                .usage_metadata
                .lock()
                .expect("usage_metadata lock poisoned");
            if let Some(existing) = guard.get(&model) {
                let combined = existing.add(&usage);
                guard.insert(model, combined);
            } else {
                guard.insert(model, usage);
            }
        }
    }
}

impl ChainManagerMixin for UsageMetadataCallbackHandler {}
impl ToolManagerMixin for UsageMetadataCallbackHandler {}
impl RetrieverManagerMixin for UsageMetadataCallbackHandler {}
impl CallbackManagerMixin for UsageMetadataCallbackHandler {}
impl RunManagerMixin for UsageMetadataCallbackHandler {}

impl BaseCallbackHandler for UsageMetadataCallbackHandler {
    fn name(&self) -> &str {
        "UsageMetadataCallbackHandler"
    }
}

pub struct UsageMetadataCallbackGuard {
    handler: UsageMetadataCallbackHandler,
}

impl UsageMetadataCallbackGuard {
    fn new() -> Self {
        Self {
            handler: UsageMetadataCallbackHandler::new(),
        }
    }

    pub fn handler(&self) -> &UsageMetadataCallbackHandler {
        &self.handler
    }

    pub fn handler_mut(&mut self) -> &mut UsageMetadataCallbackHandler {
        &mut self.handler
    }

    pub fn usage_metadata(&self) -> HashMap<String, UsageMetadata> {
        self.handler.usage_metadata()
    }

    pub fn as_arc_handler(&self) -> Arc<dyn BaseCallbackHandler> {
        Arc::new(self.handler.clone()) as Arc<dyn BaseCallbackHandler>
    }
}

impl Deref for UsageMetadataCallbackGuard {
    type Target = UsageMetadataCallbackHandler;

    fn deref(&self) -> &Self::Target {
        &self.handler
    }
}

impl DerefMut for UsageMetadataCallbackGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.handler
    }
}

pub fn get_usage_metadata_callback() -> UsageMetadataCallbackGuard {
    UsageMetadataCallbackGuard::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::AIMessage;
    use crate::outputs::ChatGeneration;
    use serde_json::json;

    fn create_chat_result_with_usage(
        content: &str,
        model: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) -> ChatResult {
        let mut response_metadata = HashMap::new();
        response_metadata.insert("model_name".to_string(), json!(model));

        let ai_msg = AIMessage::builder()
            .content(content)
            .usage_metadata(UsageMetadata::new(
                input_tokens as i64,
                output_tokens as i64,
            ))
            .response_metadata(response_metadata)
            .build();

        let generation = ChatGeneration::builder().message(ai_msg.into()).build();

        ChatResult {
            generations: vec![generation],
            llm_output: None,
        }
    }

    #[test]
    fn test_usage_handler_creation() {
        let handler = UsageMetadataCallbackHandler::new();
        assert!(handler.usage_metadata().is_empty());
        assert_eq!(handler.name(), "UsageMetadataCallbackHandler");
    }

    #[test]
    fn test_on_llm_end_collects_usage() {
        let handler = UsageMetadataCallbackHandler::new();

        let result = create_chat_result_with_usage("Hello", "gpt-4", 10, 20);

        handler.on_llm_end(&result, Uuid::new_v4(), None);

        let usage = handler.usage_metadata();
        assert_eq!(usage.len(), 1);

        let gpt4_usage = usage.get("gpt-4").unwrap();
        assert_eq!(gpt4_usage.input_tokens, 10);
        assert_eq!(gpt4_usage.output_tokens, 20);
        assert_eq!(gpt4_usage.total_tokens, 30);
    }

    #[test]
    fn test_on_llm_end_accumulates_usage() {
        let handler = UsageMetadataCallbackHandler::new();

        let result1 = create_chat_result_with_usage("Hello", "gpt-4", 10, 20);
        let result2 = create_chat_result_with_usage("World", "gpt-4", 5, 15);

        handler.on_llm_end(&result1, Uuid::new_v4(), None);
        handler.on_llm_end(&result2, Uuid::new_v4(), None);

        let usage = handler.usage_metadata();
        assert_eq!(usage.len(), 1);

        let gpt4_usage = usage.get("gpt-4").unwrap();
        assert_eq!(gpt4_usage.input_tokens, 15);
        assert_eq!(gpt4_usage.output_tokens, 35);
        assert_eq!(gpt4_usage.total_tokens, 50);
    }

    #[test]
    fn test_on_llm_end_multiple_models() {
        let handler = UsageMetadataCallbackHandler::new();

        let result1 = create_chat_result_with_usage("Hello", "gpt-4", 10, 20);
        let result2 = create_chat_result_with_usage("Hello", "claude-3", 8, 25);

        handler.on_llm_end(&result1, Uuid::new_v4(), None);
        handler.on_llm_end(&result2, Uuid::new_v4(), None);

        let usage = handler.usage_metadata();
        assert_eq!(usage.len(), 2);

        let gpt4_usage = usage.get("gpt-4").unwrap();
        assert_eq!(gpt4_usage.total_tokens, 30);

        let claude_usage = usage.get("claude-3").unwrap();
        assert_eq!(claude_usage.total_tokens, 33);
    }

    #[test]
    fn test_clone_shares_state() {
        let handler1 = UsageMetadataCallbackHandler::new();
        let handler2 = handler1.clone();

        let result = create_chat_result_with_usage("Hello", "gpt-4", 10, 20);

        handler1.on_llm_end(&result, Uuid::new_v4(), None);

        assert_eq!(handler1.usage_metadata(), handler2.usage_metadata());
    }
}
