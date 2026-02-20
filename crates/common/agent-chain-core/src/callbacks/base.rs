use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::messages::BaseMessage;
use crate::outputs::ChatResult;

pub trait RetrieverManagerMixin {
    fn on_retriever_error(
        &self,
        error: &dyn std::error::Error,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
    ) {
        let _ = (error, run_id, parent_run_id);
    }

    fn on_retriever_end(
        &self,
        documents: &[serde_json::Value],
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
    ) {
        let _ = (documents, run_id, parent_run_id);
    }
}

pub trait LLMManagerMixin {
    fn on_llm_new_token(
        &self,
        token: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        chunk: Option<&serde_json::Value>,
    ) {
        let _ = (token, run_id, parent_run_id, chunk);
    }

    fn on_llm_end(&self, response: &ChatResult, run_id: Uuid, parent_run_id: Option<Uuid>) {
        let _ = (response, run_id, parent_run_id);
    }

    fn on_llm_error(
        &self,
        error: &dyn std::error::Error,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
    ) {
        let _ = (error, run_id, parent_run_id);
    }
}

pub trait ChainManagerMixin {
    fn on_chain_end(
        &self,
        outputs: &HashMap<String, serde_json::Value>,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
    ) {
        let _ = (outputs, run_id, parent_run_id);
    }

    fn on_chain_error(
        &self,
        error: &dyn std::error::Error,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
    ) {
        let _ = (error, run_id, parent_run_id);
    }

    fn on_agent_action(
        &self,
        action: &serde_json::Value,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        color: Option<&str>,
    ) {
        let _ = (action, run_id, parent_run_id, color);
    }

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

pub trait ToolManagerMixin {
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

    fn on_tool_error(
        &self,
        error: &dyn std::error::Error,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
    ) {
        let _ = (error, run_id, parent_run_id);
    }
}

pub trait CallbackManagerMixin {
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

    #[allow(clippy::too_many_arguments)]
    fn on_chain_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        inputs: &HashMap<String, serde_json::Value>,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
        name: Option<&str>,
    ) {
        let _ = (
            serialized,
            inputs,
            run_id,
            parent_run_id,
            tags,
            metadata,
            name,
        );
    }

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

pub trait RunManagerMixin {
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

    fn on_retry(&self, retry_state: &dyn Any, run_id: Uuid, parent_run_id: Option<Uuid>) {
        let _ = (retry_state, run_id, parent_run_id);
    }

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
    fn raise_error(&self) -> bool {
        false
    }

    fn run_inline(&self) -> bool {
        false
    }

    fn ignore_llm(&self) -> bool {
        false
    }

    fn ignore_retry(&self) -> bool {
        false
    }

    fn ignore_chain(&self) -> bool {
        false
    }

    fn ignore_agent(&self) -> bool {
        false
    }

    fn ignore_retriever(&self) -> bool {
        false
    }

    fn ignore_chat_model(&self) -> bool {
        false
    }

    fn ignore_custom_event(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        "BaseCallbackHandler"
    }
}

#[async_trait]
pub trait AsyncCallbackHandler: BaseCallbackHandler {
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

    async fn on_llm_end_async(
        &self,
        response: &ChatResult,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (response, run_id, parent_run_id, tags);
    }

    async fn on_llm_error_async(
        &self,
        error: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (error, run_id, parent_run_id, tags);
    }

    #[allow(clippy::too_many_arguments)]
    async fn on_chain_start_async(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        inputs: &HashMap<String, serde_json::Value>,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
        metadata: Option<&HashMap<String, serde_json::Value>>,
        name: Option<&str>,
    ) {
        let _ = (
            serialized,
            inputs,
            run_id,
            parent_run_id,
            tags,
            metadata,
            name,
        );
    }

    async fn on_chain_end_async(
        &self,
        outputs: &HashMap<String, serde_json::Value>,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (outputs, run_id, parent_run_id, tags);
    }

    async fn on_chain_error_async(
        &self,
        error: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (error, run_id, parent_run_id, tags);
    }

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

    async fn on_tool_end_async(
        &self,
        output: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (output, run_id, parent_run_id, tags);
    }

    async fn on_tool_error_async(
        &self,
        error: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (error, run_id, parent_run_id, tags);
    }

    async fn on_text_async(
        &self,
        text: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (text, run_id, parent_run_id, tags);
    }

    async fn on_retry_async(
        &self,
        retry_state: &serde_json::Value,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
    ) {
        let _ = (retry_state, run_id, parent_run_id);
    }

    async fn on_agent_action_async(
        &self,
        action: &serde_json::Value,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (action, run_id, parent_run_id, tags);
    }

    async fn on_agent_finish_async(
        &self,
        finish: &serde_json::Value,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (finish, run_id, parent_run_id, tags);
    }

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

    async fn on_retriever_end_async(
        &self,
        documents: &[serde_json::Value],
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (documents, run_id, parent_run_id, tags);
    }

    async fn on_retriever_error_async(
        &self,
        error: &str,
        run_id: Uuid,
        parent_run_id: Option<Uuid>,
        tags: Option<&[String]>,
    ) {
        let _ = (error, run_id, parent_run_id, tags);
    }

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

pub type BoxedCallbackHandler = Box<dyn BaseCallbackHandler>;

pub type ArcCallbackHandler = Arc<dyn BaseCallbackHandler>;

#[derive(Debug, Clone)]
pub struct BaseCallbackManager {
    pub handlers: Vec<Arc<dyn BaseCallbackHandler>>,
    pub inheritable_handlers: Vec<Arc<dyn BaseCallbackHandler>>,
    pub parent_run_id: Option<Uuid>,
    pub tags: Vec<String>,
    pub inheritable_tags: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub inheritable_metadata: HashMap<String, serde_json::Value>,
}

impl Default for BaseCallbackManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BaseCallbackManager {
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

    pub fn merge(&self, other: &BaseCallbackManager) -> Self {
        let mut tags_set: std::collections::HashSet<String> = self.tags.iter().cloned().collect();
        tags_set.extend(other.tags.iter().cloned());
        let tags: Vec<String> = tags_set.into_iter().collect();

        let mut inheritable_tags_set: std::collections::HashSet<String> =
            self.inheritable_tags.iter().cloned().collect();
        inheritable_tags_set.extend(other.inheritable_tags.iter().cloned());
        let inheritable_tags: Vec<String> = inheritable_tags_set.into_iter().collect();

        let mut metadata = self.metadata.clone();
        metadata.extend(other.metadata.clone());

        let mut manager = Self {
            handlers: Vec::new(),
            inheritable_handlers: Vec::new(),
            parent_run_id: self.parent_run_id.or(other.parent_run_id),
            tags,
            inheritable_tags,
            metadata,
            inheritable_metadata: HashMap::new(), // Python doesn't merge this
        };

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

    pub fn is_async(&self) -> bool {
        false
    }

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

    pub fn remove_handler(&mut self, handler: &Arc<dyn BaseCallbackHandler>) {
        self.handlers
            .retain(|h| !std::ptr::eq(h.as_ref(), handler.as_ref()));
        self.inheritable_handlers
            .retain(|h| !std::ptr::eq(h.as_ref(), handler.as_ref()));
    }

    pub fn set_handlers(&mut self, handlers: Vec<Arc<dyn BaseCallbackHandler>>, inherit: bool) {
        self.handlers.clear();
        self.inheritable_handlers.clear();
        for handler in handlers {
            self.add_handler(handler, inherit);
        }
    }

    pub fn set_handler(&mut self, handler: Arc<dyn BaseCallbackHandler>, inherit: bool) {
        self.set_handlers(vec![handler], inherit);
    }

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

    pub fn remove_tags(&mut self, tags: Vec<String>) {
        for tag in &tags {
            self.tags.retain(|t| t != tag);
            self.inheritable_tags.retain(|t| t != tag);
        }
    }

    pub fn add_metadata(&mut self, metadata: HashMap<String, serde_json::Value>, inherit: bool) {
        self.metadata.extend(metadata.clone());
        if inherit {
            self.inheritable_metadata.extend(metadata);
        }
    }

    pub fn remove_metadata(&mut self, keys: Vec<String>) {
        for key in &keys {
            self.metadata.remove(key);
            self.inheritable_metadata.remove(key);
        }
    }
}

#[derive(Debug, Clone)]
pub enum Callbacks {
    Handlers(Vec<Arc<dyn BaseCallbackHandler>>),
    Manager(BaseCallbackManager),
}

impl Callbacks {
    pub fn none() -> Option<Self> {
        None
    }

    pub fn from_handlers(handlers: Vec<Arc<dyn BaseCallbackHandler>>) -> Self {
        Callbacks::Handlers(handlers)
    }

    pub fn from_manager(manager: BaseCallbackManager) -> Self {
        Callbacks::Manager(manager)
    }

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
