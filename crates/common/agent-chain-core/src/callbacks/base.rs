//! Base callback handler for LangChain.
//!
//! This module provides the base traits and types for the callback system,
//! following the LangChain pattern.

use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::messages::BaseMessage;
use crate::outputs::ChatResult;

/// Mixin for Retriever callbacks.
pub trait RetrieverManagerMixin {
    /// Run when Retriever errors.
    fn on_retriever_error(
        &self,
        error: &dyn std::error::Error,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
    ) {
        let _ = (error, run_id, parent_run_id);
    }

    /// Run when Retriever ends running.
    fn on_retriever_end(
        &self,
        documents: &[serde_json::Value],
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
    ) {
        let _ = (documents, run_id, parent_run_id);
    }
}

/// Mixin for LLM callbacks.
pub trait LLMManagerMixin {
    /// Run on new output token. Only available when streaming is enabled.
    fn on_llm_new_token(
        &self,
        token: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        chunk: Option<&serde_json::Value>,
    ) {
        let _ = (token, run_id, parent_run_id, chunk);
    }

    /// Run when LLM ends running.
    fn on_llm_end(&self, response: &ChatResult, run_id: Uuid, parent_run_id: Option<Uuid>) {
        let _ = (response, run_id, parent_run_id);
    }

    /// Run when LLM errors.
    fn on_llm_error(
        &self,
        error: &dyn std::error::Error,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
    ) {
        let _ = (error, run_id, parent_run_id);
    }
}

/// Mixin for chain callbacks.
pub trait ChainManagerMixin {
    /// Run when chain ends running.
    fn on_chain_end(
        &self,
        outputs: &HashMap<String, serde_json::Value>,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
    ) {
        let _ = (outputs, run_id, parent_run_id);
    }

    /// Run when chain errors.
    fn on_chain_error(
        &self,
        error: &dyn std::error::Error,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
    ) {
        let _ = (error, run_id, parent_run_id);
    }

    /// Run on agent action.
    fn on_agent_action(
        &self,
        action: &serde_json::Value,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        color: Option<&str>,
    ) {
        let _ = (action, run_id, parent_run_id, color);
    }

    /// Run on the agent end.
    fn on_agent_finish(
        &self,
        finish: &serde_json::Value,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        color: Option<&str>,
    ) {
        let _ = (finish, run_id, parent_run_id, color);
    }
}

/// Mixin for tool callbacks.
pub trait ToolManagerMixin {
    /// Run when the tool ends running.
    fn on_tool_end(
        &self,
        output: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        color: Option<&str>,
        observation_prefix: Option<&str>,
        llm_prefix: Option<&str>,
    ) {
        let _ = (
            output,
            run_id,
            parent_run_id,
            color,
            observation_prefix,
            llm_prefix,
        );
    }

    /// Run when tool errors.
    fn on_tool_error(
        &self,
        error: &dyn std::error::Error,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
    ) {
        let _ = (error, run_id, parent_run_id);
    }
}

/// Mixin for callback manager.
pub trait CallbackManagerMixin {
    /// Run when LLM starts running.
    #[allow(clippy::too_many_arguments)]
    fn on_llm_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        prompts: &[String],
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
        let _ = (serialized, prompts, run_id, parent_run_id, tags, metadata);
    }

    /// Run when a chat model starts running.
    ///
    /// The default implementation falls back to on_llm_start with stringified
    /// messages, matching Python's NotImplementedError fallback behavior.
    #[allow(clippy::too_many_arguments)]
    fn on_chat_model_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        messages: &[Vec<BaseMessage>],
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
        use crate::messages::utils::get_buffer_string;
        let message_strings: Vec<String> = messages
            .iter()
            .map(|m| get_buffer_string(m, "Human", "AI"))
            .collect();
        self.on_llm_start(
            serialized,
            &message_strings,
            run_id,
            parent_run_id,
            tags,
            metadata,
        );
    }

    /// Run when the Retriever starts running.
    #[allow(clippy::too_many_arguments)]
    fn on_retriever_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        query: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
        name: Option<&str>,
    ) {
        let _ = (
            serialized,
            query,
            run_id,
            parent_run_id,
            tags,
            metadata,
            name,
        );
    }

    /// Run when a chain starts running.
    #[allow(clippy::too_many_arguments)]
    fn on_chain_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        inputs: &HashMap<String, serde_json::Value>,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
        let _ = (serialized, inputs, run_id, parent_run_id, tags, metadata);
    }

    /// Run when the tool starts running.
    #[allow(clippy::too_many_arguments)]
    fn on_tool_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        input_str: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
        inputs: Option<&HashMap<String, serde_json::Value>>,
    ) {
        let _ = (
            serialized,
            input_str,
            run_id,
            parent_run_id,
            tags,
            metadata,
            inputs,
        );
    }
}

/// Mixin for run manager.
pub trait RunManagerMixin {
    /// Run on an arbitrary text.
    fn on_text(
        &self,
        text: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        color: Option<&str>,
        end: &str,
    ) {
        let _ = (text, run_id, parent_run_id, color, end);
    }

    /// Run on a retry event.
    fn on_retry(&self, retry_state: &dyn Any, run_id: Uuid, parent_run_id: Option<Uuid>) {
        let _ = (retry_state, run_id, parent_run_id);
    }

    /// Override to define a handler for a custom event.
    fn on_custom_event(
        &self,
        name: &str,
        data: &dyn Any,
        run_id: Uuid,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
        let _ = (name, data, run_id, tags, metadata);
    }
}

/// Base callback handler for LangChain.
///
/// This trait combines all the mixin traits and provides the base interface
/// for callback handlers. Handlers can override specific methods they care about.
pub trait BaseCallbackHandler:
    LLMManagerMixin
    + ChainManagerMixin
    + ToolManagerMixin
    + RetrieverManagerMixin
    + CallbackManagerMixin
    + RunManagerMixin
    + Send
    + Sync
    + Debug
{
    /// Whether to raise an error if an exception occurs.
    fn raise_error(&self) -> bool {
        false
    }

    /// Whether to run the callback inline.
    fn run_inline(&self) -> bool {
        false
    }

    /// Whether to ignore LLM callbacks.
    fn ignore_llm(&self) -> bool {
        false
    }

    /// Whether to ignore retry callbacks.
    fn ignore_retry(&self) -> bool {
        false
    }

    /// Whether to ignore chain callbacks.
    fn ignore_chain(&self) -> bool {
        false
    }

    /// Whether to ignore agent callbacks.
    fn ignore_agent(&self) -> bool {
        false
    }

    /// Whether to ignore retriever callbacks.
    fn ignore_retriever(&self) -> bool {
        false
    }

    /// Whether to ignore chat model callbacks.
    fn ignore_chat_model(&self) -> bool {
        false
    }

    /// Whether to ignore custom events.
    fn ignore_custom_event(&self) -> bool {
        false
    }

    /// Get a unique name for this handler.
    /// Note: This is a Rust-specific addition for debugging purposes.
    fn name(&self) -> &str {
        "BaseCallbackHandler"
    }
}

/// Async callback handler for LangChain.
///
/// This trait provides async versions of all callback methods.
#[async_trait]
pub trait AsyncCallbackHandler: BaseCallbackHandler {
    /// Run when LLM starts running (async).
    #[allow(clippy::too_many_arguments)]
    async fn on_llm_start_async(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        prompts: &[String],
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
        let _ = (serialized, prompts, run_id, parent_run_id, tags, metadata);
    }

    /// Run when a chat model starts running (async).
    ///
    /// The default implementation falls back to on_llm_start_async with stringified
    /// messages, matching Python's NotImplementedError fallback behavior.
    #[allow(clippy::too_many_arguments)]
    async fn on_chat_model_start_async(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        messages: &[Vec<BaseMessage>],
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
        use crate::messages::utils::get_buffer_string;
        let message_strings: Vec<String> = messages
            .iter()
            .map(|m| get_buffer_string(m, "Human", "AI"))
            .collect();
        self.on_llm_start_async(
            serialized,
            &message_strings,
            run_id,
            parent_run_id,
            tags,
            metadata,
        )
        .await;
    }

    /// Run on new output token (async).
    async fn on_llm_new_token_async(
        &self,
        token: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        chunk: Option<&serde_json::Value>,
        tags: Option<&[String]>,
    ) {
        let _ = (token, run_id, parent_run_id, chunk, tags);
    }

    /// Run when LLM ends running (async).
    async fn on_llm_end_async(
        &self,
        response: &ChatResult,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (response, run_id, parent_run_id, tags);
    }

    /// Run when LLM errors (async).
    async fn on_llm_error_async(
        &self,
        error: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (error, run_id, parent_run_id, tags);
    }

    /// Run when chain starts running (async).
    #[allow(clippy::too_many_arguments)]
    async fn on_chain_start_async(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        inputs: &HashMap<String, serde_json::Value>,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
        let _ = (serialized, inputs, run_id, parent_run_id, tags, metadata);
    }

    /// Run when chain ends running (async).
    async fn on_chain_end_async(
        &self,
        outputs: &HashMap<String, serde_json::Value>,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (outputs, run_id, parent_run_id, tags);
    }

    /// Run when chain errors (async).
    async fn on_chain_error_async(
        &self,
        error: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (error, run_id, parent_run_id, tags);
    }

    /// Run when tool starts running (async).
    #[allow(clippy::too_many_arguments)]
    async fn on_tool_start_async(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        input_str: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
        inputs: Option<&HashMap<String, serde_json::Value>>,
    ) {
        let _ = (
            serialized,
            input_str,
            run_id,
            parent_run_id,
            tags,
            metadata,
            inputs,
        );
    }

    /// Run when tool ends running (async).
    async fn on_tool_end_async(
        &self,
        output: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (output, run_id, parent_run_id, tags);
    }

    /// Run when tool errors (async).
    async fn on_tool_error_async(
        &self,
        error: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (error, run_id, parent_run_id, tags);
    }

    /// Run on an arbitrary text (async).
    async fn on_text_async(
        &self,
        text: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (text, run_id, parent_run_id, tags);
    }

    /// Run on a retry event (async).
    async fn on_retry_async(
        &self,
        retry_state: &serde_json::Value,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
    ) {
        let _ = (retry_state, run_id, parent_run_id);
    }

    /// Run on agent action (async).
    async fn on_agent_action_async(
        &self,
        action: &serde_json::Value,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (action, run_id, parent_run_id, tags);
    }

    /// Run on the agent end (async).
    async fn on_agent_finish_async(
        &self,
        finish: &serde_json::Value,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (finish, run_id, parent_run_id, tags);
    }

    /// Run on the retriever start (async).
    #[allow(clippy::too_many_arguments)]
    async fn on_retriever_start_async(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        query: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
        name: Option<&str>,
    ) {
        let _ = (
            serialized,
            query,
            run_id,
            parent_run_id,
            tags,
            metadata,
            name,
        );
    }

    /// Run on the retriever end (async).
    async fn on_retriever_end_async(
        &self,
        documents: &[serde_json::Value],
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (documents, run_id, parent_run_id, tags);
    }

    /// Run on retriever error (async).
    async fn on_retriever_error_async(
        &self,
        error: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (error, run_id, parent_run_id, tags);
    }

    /// Override to define a handler for custom events (async).
    async fn on_custom_event_async(
        &self,
        name: &str,
        data: &serde_json::Value,
        run_id: Uuid,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
        let _ = (name, data, run_id, tags, metadata);
    }
}

/// Type alias for a boxed callback handler.
pub type BoxedCallbackHandler = Box<dyn BaseCallbackHandler>;

/// Type alias for an Arc-wrapped callback handler.
pub type ArcCallbackHandler = Arc<dyn BaseCallbackHandler>;

/// Base callback manager for LangChain.
///
/// Manages a collection of callback handlers and provides methods to
/// add, remove, and configure handlers.
#[derive(Debug, Clone)]
pub struct BaseCallbackManager {
    /// The handlers.
    pub handlers: Vec<Arc<dyn BaseCallbackHandler>>,
    /// The inheritable handlers.
    pub inheritable_handlers: Vec<Arc<dyn BaseCallbackHandler>>,
    /// The parent run ID.
    pub parent_run_id: Option<Uuid>,
    /// The tags.
    pub tags: Vec<String>,
    /// The inheritable tags.
    pub inheritable_tags: Vec<String>,
    /// The metadata.
    pub metadata: HashMap<String, serde_json::Value>,
    /// The inheritable metadata.
    pub inheritable_metadata: HashMap<String, serde_json::Value>,
}

impl Default for BaseCallbackManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BaseCallbackManager {
    /// Create a new callback manager.
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
            inheritable_handlers: Vec::new(),
            parent_run_id: None,
            tags: Vec::new(),
            inheritable_tags: Vec::new(),
            metadata: HashMap::new(),
            inheritable_metadata: HashMap::new(),
        }
    }

    /// Create a new callback manager with handlers.
    ///
    /// This matches the Python `__init__` signature.
    #[allow(clippy::too_many_arguments)]
    pub fn with_handlers(
        handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        inheritable_handlers: Option<Vec<Arc<dyn BaseCallbackHandler>>>,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        inheritable_tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, serde_json::Value>>,
        inheritable_metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self {
            handlers,
            inheritable_handlers: inheritable_handlers.unwrap_or_default(),
            parent_run_id,
            tags: tags.unwrap_or_default(),
            inheritable_tags: inheritable_tags.unwrap_or_default(),
            metadata: metadata.unwrap_or_default(),
            inheritable_metadata: inheritable_metadata.unwrap_or_default(),
        }
    }

    /// Return a copy of the callback manager.
    pub fn copy(&self) -> Self {
        Self {
            handlers: self.handlers.clone(),
            inheritable_handlers: self.inheritable_handlers.clone(),
            parent_run_id: self.parent_run_id,
            tags: self.tags.clone(),
            inheritable_tags: self.inheritable_tags.clone(),
            metadata: self.metadata.clone(),
            inheritable_metadata: self.inheritable_metadata.clone(),
        }
    }

    /// Merge with another callback manager.
    ///
    /// Note: This matches Python's behavior which does NOT merge inheritable_metadata
    /// (this appears to be a bug in the Python implementation, but we match it for compatibility).
    pub fn merge(&self, other: &BaseCallbackManager) -> Self {
        // Use a set-like deduplication for tags (matching Python's list(set(...)))
        let mut tags_set: std::collections::HashSet<String> = self.tags.iter().cloned().collect();
        tags_set.extend(other.tags.iter().cloned());
        let tags: Vec<String> = tags_set.into_iter().collect();

        let mut inheritable_tags_set: std::collections::HashSet<String> =
            self.inheritable_tags.iter().cloned().collect();
        inheritable_tags_set.extend(other.inheritable_tags.iter().cloned());
        let inheritable_tags: Vec<String> = inheritable_tags_set.into_iter().collect();

        // Merge metadata
        let mut metadata = self.metadata.clone();
        metadata.extend(other.metadata.clone());

        // Create manager with merged values
        // Note: Python does NOT include inheritable_metadata in the constructor
        let mut manager = Self {
            handlers: Vec::new(),
            inheritable_handlers: Vec::new(),
            parent_run_id: self.parent_run_id.or(other.parent_run_id),
            tags,
            inheritable_tags,
            metadata,
            inheritable_metadata: HashMap::new(), // Python doesn't merge this
        };

        // Merge handlers
        let handlers: Vec<_> = self
            .handlers
            .iter()
            .chain(other.handlers.iter())
            .cloned()
            .collect();
        let inheritable_handlers: Vec<_> = self
            .inheritable_handlers
            .iter()
            .chain(other.inheritable_handlers.iter())
            .cloned()
            .collect();

        for handler in handlers {
            manager.add_handler(handler, false);
        }
        for handler in inheritable_handlers {
            manager.add_handler(handler, true);
        }

        manager
    }

    /// Whether the callback manager is async.
    pub fn is_async(&self) -> bool {
        false
    }

    /// Add a handler to the callback manager.
    pub fn add_handler(&mut self, handler: Arc<dyn BaseCallbackHandler>, inherit: bool) {
        if !self
            .handlers
            .iter()
            .any(|h| std::ptr::eq(h.as_ref(), handler.as_ref()))
        {
            self.handlers.push(handler.clone());
        }
        if inherit
            && !self
                .inheritable_handlers
                .iter()
                .any(|h| std::ptr::eq(h.as_ref(), handler.as_ref()))
        {
            self.inheritable_handlers.push(handler);
        }
    }

    /// Remove a handler from the callback manager.
    pub fn remove_handler(&mut self, handler: &Arc<dyn BaseCallbackHandler>) {
        self.handlers
            .retain(|h| !std::ptr::eq(h.as_ref(), handler.as_ref()));
        self.inheritable_handlers
            .retain(|h| !std::ptr::eq(h.as_ref(), handler.as_ref()));
    }

    /// Set handlers as the only handlers on the callback manager.
    pub fn set_handlers(&mut self, handlers: Vec<Arc<dyn BaseCallbackHandler>>, inherit: bool) {
        self.handlers.clear();
        self.inheritable_handlers.clear();
        for handler in handlers {
            self.add_handler(handler, inherit);
        }
    }

    /// Set a single handler as the only handler on the callback manager.
    pub fn set_handler(&mut self, handler: Arc<dyn BaseCallbackHandler>, inherit: bool) {
        self.set_handlers(vec![handler], inherit);
    }

    /// Add tags to the callback manager.
    pub fn add_tags(&mut self, tags: Vec<String>, inherit: bool) {
        for tag in &tags {
            if self.tags.contains(tag) {
                self.remove_tags(vec![tag.clone()]);
            }
        }
        self.tags.extend(tags.clone());
        if inherit {
            self.inheritable_tags.extend(tags);
        }
    }

    /// Remove tags from the callback manager.
    pub fn remove_tags(&mut self, tags: Vec<String>) {
        for tag in &tags {
            self.tags.retain(|t| t != tag);
            self.inheritable_tags.retain(|t| t != tag);
        }
    }

    /// Add metadata to the callback manager.
    pub fn add_metadata(&mut self, metadata: HashMap<String, serde_json::Value>, inherit: bool) {
        self.metadata.extend(metadata.clone());
        if inherit {
            self.inheritable_metadata.extend(metadata);
        }
    }

    /// Remove metadata from the callback manager.
    pub fn remove_metadata(&mut self, keys: Vec<String>) {
        for key in &keys {
            self.metadata.remove(key);
            self.inheritable_metadata.remove(key);
        }
    }
}

/// Callbacks type alias - can be a list of handlers or a callback manager.
#[derive(Debug, Clone)]
pub enum Callbacks {
    /// A list of callback handlers.
    Handlers(Vec<Arc<dyn BaseCallbackHandler>>),
    /// A callback manager.
    Manager(BaseCallbackManager),
}

impl Callbacks {
    /// Create empty callbacks.
    pub fn none() -> Option<Self> {
        None
    }

    /// Create callbacks from handlers.
    pub fn from_handlers(handlers: Vec<Arc<dyn BaseCallbackHandler>>) -> Self {
        Callbacks::Handlers(handlers)
    }

    /// Create callbacks from a manager.
    pub fn from_manager(manager: BaseCallbackManager) -> Self {
        Callbacks::Manager(manager)
    }

    /// Convert to a callback manager.
    pub fn to_manager(&self) -> BaseCallbackManager {
        match self {
            Callbacks::Handlers(handlers) => BaseCallbackManager::with_handlers(
                handlers.clone(),
                Some(handlers.clone()),
                None,
                None,
                None,
                None,
                None,
            ),
            Callbacks::Manager(manager) => manager.clone(),
        }
    }
}

impl From<Vec<Arc<dyn BaseCallbackHandler>>> for Callbacks {
    fn from(handlers: Vec<Arc<dyn BaseCallbackHandler>>) -> Self {
        Callbacks::Handlers(handlers)
    }
}

impl From<BaseCallbackManager> for Callbacks {
    fn from(manager: BaseCallbackManager) -> Self {
        Callbacks::Manager(manager)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestHandler;

    impl LLMManagerMixin for TestHandler {}
    impl ChainManagerMixin for TestHandler {}
    impl ToolManagerMixin for TestHandler {}
    impl RetrieverManagerMixin for TestHandler {}
    impl CallbackManagerMixin for TestHandler {}
    impl RunManagerMixin for TestHandler {}

    impl BaseCallbackHandler for TestHandler {
        fn name(&self) -> &str {
            "TestHandler"
        }
    }

    #[test]
    fn test_callback_manager_add_handler() {
        let mut manager = BaseCallbackManager::new();
        let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);

        manager.add_handler(handler.clone(), true);

        assert_eq!(manager.handlers.len(), 1);
        assert_eq!(manager.inheritable_handlers.len(), 1);
    }

    #[test]
    fn test_callback_manager_add_tags() {
        let mut manager = BaseCallbackManager::new();

        manager.add_tags(vec!["tag1".to_string(), "tag2".to_string()], true);

        assert_eq!(manager.tags.len(), 2);
        assert_eq!(manager.inheritable_tags.len(), 2);
    }

    #[test]
    fn test_callback_manager_merge() {
        let mut manager1 = BaseCallbackManager::new();
        manager1.add_tags(vec!["tag1".to_string()], true);

        let mut manager2 = BaseCallbackManager::new();
        manager2.add_tags(vec!["tag2".to_string()], true);

        let merged = manager1.merge(&manager2);

        assert_eq!(merged.tags.len(), 2);
        assert!(merged.tags.contains(&"tag1".to_string()));
        assert!(merged.tags.contains(&"tag2".to_string()));
    }
}
