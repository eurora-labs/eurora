//! Callback Handler that tracks AIMessage.usage_metadata.
//!
//! This module provides a callback handler for tracking token usage
//! across chat model calls, following the Python LangChain UsageMetadataCallbackHandler pattern.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use uuid::Uuid;

use crate::chat_models::UsageMetadata;
use crate::ChatResult;

use super::base::{
    BaseCallbackHandler, CallbackManagerMixin, ChainManagerMixin, LLMManagerMixin,
    RetrieverManagerMixin, RunManagerMixin, ToolManagerMixin,
};

/// Add two usage metadata objects together.
///
/// This function combines the token counts from two usage metadata objects,
/// returning a new object with the summed values.
pub fn add_usage(left: &UsageMetadata, right: &UsageMetadata) -> UsageMetadata {
    UsageMetadata {
        input_tokens: left.input_tokens + right.input_tokens,
        output_tokens: left.output_tokens + right.output_tokens,
        total_tokens: left.total_tokens + right.total_tokens,
    }
}

/// Callback Handler that tracks AIMessage.usage_metadata.
///
/// This handler collects token usage metadata from chat model responses,
/// aggregating the usage by model name. It is thread-safe and can be used
/// across multiple concurrent LLM calls.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::callbacks::UsageMetadataCallbackHandler;
/// use std::sync::Arc;
///
/// let handler = UsageMetadataCallbackHandler::new();
/// 
/// // Use with a callback manager
/// let mut manager = CallbackManager::new();
/// manager.add_handler(Arc::new(handler.clone()), true);
///
/// // After LLM calls complete
/// let usage = handler.usage_metadata();
/// for (model, metadata) in usage.iter() {
///     println!("{}: {} tokens", model, metadata.total_tokens);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct UsageMetadataCallbackHandler {
    /// The usage metadata by model name, protected by a mutex for thread safety.
    usage_metadata: Arc<Mutex<HashMap<String, UsageMetadata>>>,
}

impl Default for UsageMetadataCallbackHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl UsageMetadataCallbackHandler {
    /// Create a new UsageMetadataCallbackHandler.
    pub fn new() -> Self {
        Self {
            usage_metadata: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get the collected usage metadata.
    ///
    /// Returns a clone of the current usage metadata map, keyed by model name.
    pub fn usage_metadata(&self) -> HashMap<String, UsageMetadata> {
        self.usage_metadata.lock().unwrap().clone()
    }

    /// Get usage metadata for a specific model.
    pub fn get_model_usage(&self, model_name: &str) -> Option<UsageMetadata> {
        self.usage_metadata.lock().unwrap().get(model_name).cloned()
    }

    /// Get total usage across all models.
    pub fn total_usage(&self) -> UsageMetadata {
        let guard = self.usage_metadata.lock().unwrap();
        let mut total = UsageMetadata::new(0, 0);
        for usage in guard.values() {
            total = add_usage(&total, usage);
        }
        total
    }

    /// Clear all collected usage metadata.
    pub fn clear(&self) {
        self.usage_metadata.lock().unwrap().clear();
    }
}

impl LLMManagerMixin for UsageMetadataCallbackHandler {
    fn on_llm_end(
        &mut self,
        response: &ChatResult,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        // Extract usage metadata from the response
        let usage_metadata = response.metadata.usage.as_ref();
        let model_name = response.metadata.model.as_deref();

        // Update shared state behind lock
        if let (Some(usage), Some(model)) = (usage_metadata, model_name) {
            let mut guard = self.usage_metadata.lock().unwrap();
            if let Some(existing) = guard.get(model) {
                let combined = add_usage(existing, usage);
                guard.insert(model.to_string(), combined);
            } else {
                guard.insert(model.to_string(), usage.clone());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_models::ChatResultMetadata;
    use crate::messages::AIMessage;

    #[test]
    fn test_usage_handler_creation() {
        let handler = UsageMetadataCallbackHandler::new();
        assert!(handler.usage_metadata().is_empty());
        assert_eq!(handler.name(), "UsageMetadataCallbackHandler");
    }

    #[test]
    fn test_add_usage() {
        let usage1 = UsageMetadata::new(10, 20);
        let usage2 = UsageMetadata::new(5, 15);
        let combined = add_usage(&usage1, &usage2);

        assert_eq!(combined.input_tokens, 15);
        assert_eq!(combined.output_tokens, 35);
        assert_eq!(combined.total_tokens, 50);
    }

    #[test]
    fn test_on_llm_end_collects_usage() {
        let mut handler = UsageMetadataCallbackHandler::new();

        let result = ChatResult {
            message: AIMessage::new("Hello"),
            metadata: ChatResultMetadata {
                model: Some("gpt-4".to_string()),
                stop_reason: None,
                usage: Some(UsageMetadata::new(10, 20)),
            },
        };

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
        let mut handler = UsageMetadataCallbackHandler::new();

        let result1 = ChatResult {
            message: AIMessage::new("Hello"),
            metadata: ChatResultMetadata {
                model: Some("gpt-4".to_string()),
                stop_reason: None,
                usage: Some(UsageMetadata::new(10, 20)),
            },
        };

        let result2 = ChatResult {
            message: AIMessage::new("World"),
            metadata: ChatResultMetadata {
                model: Some("gpt-4".to_string()),
                stop_reason: None,
                usage: Some(UsageMetadata::new(5, 15)),
            },
        };

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
        let mut handler = UsageMetadataCallbackHandler::new();

        let result1 = ChatResult {
            message: AIMessage::new("Hello"),
            metadata: ChatResultMetadata {
                model: Some("gpt-4".to_string()),
                stop_reason: None,
                usage: Some(UsageMetadata::new(10, 20)),
            },
        };

        let result2 = ChatResult {
            message: AIMessage::new("Hello"),
            metadata: ChatResultMetadata {
                model: Some("claude-3".to_string()),
                stop_reason: None,
                usage: Some(UsageMetadata::new(8, 25)),
            },
        };

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
    fn test_total_usage() {
        let mut handler = UsageMetadataCallbackHandler::new();

        let result1 = ChatResult {
            message: AIMessage::new("Hello"),
            metadata: ChatResultMetadata {
                model: Some("gpt-4".to_string()),
                stop_reason: None,
                usage: Some(UsageMetadata::new(10, 20)),
            },
        };

        let result2 = ChatResult {
            message: AIMessage::new("Hello"),
            metadata: ChatResultMetadata {
                model: Some("claude-3".to_string()),
                stop_reason: None,
                usage: Some(UsageMetadata::new(8, 25)),
            },
        };

        handler.on_llm_end(&result1, Uuid::new_v4(), None);
        handler.on_llm_end(&result2, Uuid::new_v4(), None);

        let total = handler.total_usage();
        assert_eq!(total.input_tokens, 18);
        assert_eq!(total.output_tokens, 45);
        assert_eq!(total.total_tokens, 63);
    }

    #[test]
    fn test_clear_usage() {
        let mut handler = UsageMetadataCallbackHandler::new();

        let result = ChatResult {
            message: AIMessage::new("Hello"),
            metadata: ChatResultMetadata {
                model: Some("gpt-4".to_string()),
                stop_reason: None,
                usage: Some(UsageMetadata::new(10, 20)),
            },
        };

        handler.on_llm_end(&result, Uuid::new_v4(), None);
        assert!(!handler.usage_metadata().is_empty());

        handler.clear();
        assert!(handler.usage_metadata().is_empty());
    }

    #[test]
    fn test_get_model_usage() {
        let mut handler = UsageMetadataCallbackHandler::new();

        let result = ChatResult {
            message: AIMessage::new("Hello"),
            metadata: ChatResultMetadata {
                model: Some("gpt-4".to_string()),
                stop_reason: None,
                usage: Some(UsageMetadata::new(10, 20)),
            },
        };

        handler.on_llm_end(&result, Uuid::new_v4(), None);

        assert!(handler.get_model_usage("gpt-4").is_some());
        assert!(handler.get_model_usage("claude-3").is_none());
    }

    #[test]
    fn test_clone_shares_state() {
        let mut handler1 = UsageMetadataCallbackHandler::new();
        let handler2 = handler1.clone();

        let result = ChatResult {
            message: AIMessage::new("Hello"),
            metadata: ChatResultMetadata {
                model: Some("gpt-4".to_string()),
                stop_reason: None,
                usage: Some(UsageMetadata::new(10, 20)),
            },
        };

        handler1.on_llm_end(&result, Uuid::new_v4(), None);

        // Both handlers should see the same usage data
        assert_eq!(handler1.usage_metadata(), handler2.usage_metadata());
    }
}