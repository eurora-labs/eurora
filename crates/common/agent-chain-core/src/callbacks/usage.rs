//! Callback Handler that tracks AIMessage.usage_metadata.
//!
//! This module provides a callback handler for tracking token usage
//! across chat model calls, following the Python LangChain UsageMetadataCallbackHandler pattern.

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
}

impl fmt::Display for UsageMetadataCallbackHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.usage_metadata.lock().unwrap())
    }
}

impl LLMManagerMixin for UsageMetadataCallbackHandler {
    fn on_llm_end(&self, response: &ChatResult, _run_id: Uuid, _parent_run_id: Option<Uuid>) {
        // Extract usage metadata from the first generation's message
        let first_generation = response.generations.first();

        let (usage_metadata, model_name) = match first_generation {
            Some(generation) => {
                // Try to get usage from the AIMessage
                let usage = match &generation.message {
                    BaseMessage::AI(ai_msg) => ai_msg.usage_metadata.clone(),
                    _ => None,
                };

                // Get model name from response_metadata
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

        // Update shared state behind lock
        if let (Some(usage), Some(model)) = (usage_metadata, model_name) {
            let mut guard = self.usage_metadata.lock().unwrap();
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

/// Guard type for the `get_usage_metadata_callback` function.
///
/// This guard provides access to the underlying `UsageMetadataCallbackHandler`
/// and can be used with a callback manager to track usage metadata.
///
/// When the guard is dropped, any cleanup is performed automatically.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::callbacks::get_usage_metadata_callback;
///
/// let callback_guard = get_usage_metadata_callback();
/// // Use callback_guard.handler() with your callback manager
/// // The handler can be cloned and added to managers
///
/// let usage = callback_guard.usage_metadata();
/// for (model, metadata) in usage.iter() {
///     println!("{}: {} tokens", model, metadata.total_tokens);
/// }
/// ```
pub struct UsageMetadataCallbackGuard {
    handler: UsageMetadataCallbackHandler,
}

impl UsageMetadataCallbackGuard {
    /// Create a new usage metadata callback guard.
    fn new() -> Self {
        Self {
            handler: UsageMetadataCallbackHandler::new(),
        }
    }

    /// Get a reference to the underlying handler.
    pub fn handler(&self) -> &UsageMetadataCallbackHandler {
        &self.handler
    }

    /// Get a mutable reference to the underlying handler.
    pub fn handler_mut(&mut self) -> &mut UsageMetadataCallbackHandler {
        &mut self.handler
    }

    /// Get the collected usage metadata.
    pub fn usage_metadata(&self) -> HashMap<String, UsageMetadata> {
        self.handler.usage_metadata()
    }

    /// Get an Arc-wrapped handler suitable for use with callback managers.
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

/// Get a usage metadata callback handler.
///
/// This function creates a `UsageMetadataCallbackGuard` that provides access
/// to a `UsageMetadataCallbackHandler` for tracking token usage across chat
/// model calls.
///
/// The returned guard implements `Deref` and `DerefMut` to the underlying
/// handler, making it easy to use.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::callbacks::{get_usage_metadata_callback, CallbackManager};
/// use std::sync::Arc;
///
/// let callback = get_usage_metadata_callback();
///
/// // Add to a callback manager
/// let mut manager = CallbackManager::new();
/// manager.add_handler(callback.as_arc_handler(), true);
///
/// // After LLM calls complete
/// let usage = callback.usage_metadata();
/// for (model, metadata) in usage.iter() {
///     println!("{}: {} tokens", model, metadata.total_tokens);
/// }
/// ```
///
/// This is the Rust equivalent of Python's `get_usage_metadata_callback()` context manager.
pub fn get_usage_metadata_callback() -> UsageMetadataCallbackGuard {
    UsageMetadataCallbackGuard::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::AIMessage;
    use crate::outputs::ChatGeneration;
    use serde_json::json;

    /// Helper to create a ChatResult with usage metadata for testing.
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

        let generation = ChatGeneration::new(ai_msg.into());

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

        // Both handlers should see the same usage data
        assert_eq!(handler1.usage_metadata(), handler2.usage_metadata());
    }
}
