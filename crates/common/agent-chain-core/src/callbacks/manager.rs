use std::collections::HashMap;
use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use bon::bon;
use uuid::Uuid;

use crate::messages::BaseMessage;
use crate::outputs::ChatResult;

use super::base::{BaseCallbackHandler, BaseCallbackManager, Callbacks};
use super::stdout::StdOutCallbackHandler;
use crate::globals::get_debug;
use crate::tracers::context::{
    get_configure_hooks, get_tracer_project, get_tracing_callback, tracing_v2_is_enabled,
};
use crate::tracers::stdout::ConsoleCallbackHandler;
use crate::utils::env::env_var_is_set;
use crate::utils::uuid::uuid7;

pub fn handle_event<F>(
    handlers: &[Arc<dyn BaseCallbackHandler>],
    ignore_condition: Option<fn(&dyn BaseCallbackHandler) -> bool>,
    mut event_fn: F,
) where
    F: FnMut(&Arc<dyn BaseCallbackHandler>),
{
    for handler in handlers {
        if let Some(ignore_fn) = ignore_condition
            && ignore_fn(handler.as_ref())
        {
            continue;
        }
        let result = catch_unwind(AssertUnwindSafe(|| {
            event_fn(handler);
        }));
        if let Err(panic_payload) = result {
            let error_msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown error".to_string()
            };
            tracing::warn!(
                target: "agent_chain_core::callbacks",
                "Error in {}.callback: {}",
                handler.name(),
                error_msg,
            );
            if handler.raise_error() {
                std::panic::resume_unwind(panic_payload);
            }
        }
    }
}

pub async fn ahandle_event<F, Fut>(
    handlers: &[Arc<dyn BaseCallbackHandler>],
    ignore_condition: Option<fn(&dyn BaseCallbackHandler) -> bool>,
    event_fn: F,
) where
    F: Fn(&Arc<dyn BaseCallbackHandler>) -> Fut + Send + Sync,
    Fut: Future<Output = ()> + Send,
{
    for handler in handlers.iter().filter(|h| h.run_inline()) {
        if let Some(ignore_fn) = ignore_condition
            && ignore_fn(handler.as_ref())
        {
            continue;
        }
        event_fn(handler).await;
    }

    let non_inline_futures: Vec<_> = handlers
        .iter()
        .filter(|h| !h.run_inline())
        .filter(|h| {
            if let Some(ignore_fn) = ignore_condition {
                !ignore_fn(h.as_ref())
            } else {
                true
            }
        })
        .map(event_fn)
        .collect();

    futures::future::join_all(non_inline_futures).await;
}

#[derive(Debug, Clone)]
pub struct BaseRunManager {
    pub run_id: Uuid,
    pub handlers: Vec<Arc<dyn BaseCallbackHandler>>,
    pub inheritable_handlers: Vec<Arc<dyn BaseCallbackHandler>>,
    pub parent_run_id: Option<Uuid>,
    pub tags: Vec<String>,
    pub inheritable_tags: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub inheritable_metadata: HashMap<String, serde_json::Value>,
}

impl BaseRunManager {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        run_id: Uuid,
        handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        inheritable_handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        inheritable_tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, serde_json::Value>>,
        inheritable_metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self {
            run_id,
            handlers,
            inheritable_handlers,
            parent_run_id,
            tags: tags.unwrap_or_default(),
            inheritable_tags: inheritable_tags.unwrap_or_default(),
            metadata: metadata.unwrap_or_default(),
            inheritable_metadata: inheritable_metadata.unwrap_or_default(),
        }
    }

    pub fn get_noop_manager() -> Self {
        Self {
            run_id: uuid7(None),
            handlers: Vec::new(),
            inheritable_handlers: Vec::new(),
            parent_run_id: None,
            tags: Vec::new(),
            inheritable_tags: Vec::new(),
            metadata: HashMap::new(),
            inheritable_metadata: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RunManager {
    inner: BaseRunManager,
}

impl RunManager {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        run_id: Uuid,
        handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        inheritable_handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        inheritable_tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, serde_json::Value>>,
        inheritable_metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self {
            inner: BaseRunManager::new(
                run_id,
                handlers,
                inheritable_handlers,
                parent_run_id,
                tags,
                inheritable_tags,
                metadata,
                inheritable_metadata,
            ),
        }
    }

    pub fn run_id(&self) -> Uuid {
        self.inner.run_id
    }

    pub fn parent_run_id(&self) -> Option<Uuid> {
        self.inner.parent_run_id
    }

    pub fn handlers(&self) -> &[Arc<dyn BaseCallbackHandler>] {
        &self.inner.handlers
    }

    pub fn tags(&self) -> &[String] {
        &self.inner.tags
    }

    pub fn on_text(&self, text: &str) {
        if self.inner.handlers.is_empty() {
            return;
        }
        let run_id = self.inner.run_id;
        let parent_run_id = self.inner.parent_run_id;
        let _tags = self.inner.tags.clone();
        handle_event(&self.inner.handlers, None, |handler| {
            handler.on_text(text, run_id, parent_run_id, None, "");
        });
    }

    pub fn on_retry(&self, retry_state: &serde_json::Value) {
        if self.inner.handlers.is_empty() {
            return;
        }
        let run_id = self.inner.run_id;
        let parent_run_id = self.inner.parent_run_id;
        let _tags = self.inner.tags.clone();
        handle_event(
            &self.inner.handlers,
            Some(|h: &dyn BaseCallbackHandler| h.ignore_retry()),
            |handler| {
                handler.on_retry(retry_state, run_id, parent_run_id);
            },
        );
    }

    pub fn get_noop_manager() -> Self {
        Self {
            inner: BaseRunManager::get_noop_manager(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AsyncRunManager {
    inner: BaseRunManager,
}

impl AsyncRunManager {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        run_id: Uuid,
        handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        inheritable_handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        inheritable_tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, serde_json::Value>>,
        inheritable_metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self {
            inner: BaseRunManager::new(
                run_id,
                handlers,
                inheritable_handlers,
                parent_run_id,
                tags,
                inheritable_tags,
                metadata,
                inheritable_metadata,
            ),
        }
    }

    pub fn get_sync(&self) -> RunManager {
        RunManager::new(
            self.inner.run_id,
            self.inner.handlers.clone(),
            self.inner.inheritable_handlers.clone(),
            self.inner.parent_run_id,
            Some(self.inner.tags.clone()),
            Some(self.inner.inheritable_tags.clone()),
            Some(self.inner.metadata.clone()),
            Some(self.inner.inheritable_metadata.clone()),
        )
    }

    pub fn run_id(&self) -> Uuid {
        self.inner.run_id
    }

    pub fn parent_run_id(&self) -> Option<Uuid> {
        self.inner.parent_run_id
    }

    pub fn handlers(&self) -> &[Arc<dyn BaseCallbackHandler>] {
        &self.inner.handlers
    }

    pub fn tags(&self) -> &[String] {
        &self.inner.tags
    }

    pub async fn on_text(&self, text: &str) {
        if self.inner.handlers.is_empty() {
            return;
        }
        let run_id = self.inner.run_id;
        let parent_run_id = self.inner.parent_run_id;
        let _tags = self.inner.tags.clone();
        ahandle_event(&self.inner.handlers, None, |handler| {
            handler.on_text(text, run_id, parent_run_id, None, "");
            async {}
        })
        .await;
    }

    pub async fn on_retry(&self, retry_state: &serde_json::Value) {
        if self.inner.handlers.is_empty() {
            return;
        }
        let run_id = self.inner.run_id;
        let parent_run_id = self.inner.parent_run_id;
        let _tags = self.inner.tags.clone();
        ahandle_event(
            &self.inner.handlers,
            Some(|h: &dyn BaseCallbackHandler| h.ignore_retry()),
            |handler| {
                handler.on_retry(retry_state, run_id, parent_run_id);
                async {}
            },
        )
        .await;
    }

    pub fn get_noop_manager() -> Self {
        Self {
            inner: BaseRunManager::get_noop_manager(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AsyncParentRunManager {
    inner: AsyncRunManager,
}

impl AsyncParentRunManager {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        run_id: Uuid,
        handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        inheritable_handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        inheritable_tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, serde_json::Value>>,
        inheritable_metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self {
            inner: AsyncRunManager::new(
                run_id,
                handlers,
                inheritable_handlers,
                parent_run_id,
                tags,
                inheritable_tags,
                metadata,
                inheritable_metadata,
            ),
        }
    }

    pub fn get_child(&self, tag: Option<&str>) -> AsyncCallbackManager {
        let mut manager = AsyncCallbackManager::new();
        manager.inner.parent_run_id = Some(self.inner.run_id());
        manager.set_handlers(self.inner.inner.inheritable_handlers.clone(), true);
        manager.add_tags(self.inner.inner.inheritable_tags.clone(), true);
        manager.add_metadata(self.inner.inner.inheritable_metadata.clone(), true);
        if let Some(tag) = tag {
            manager.add_tags(vec![tag.to_string()], false);
        }
        manager
    }

    pub fn get_sync(&self) -> ParentRunManager {
        ParentRunManager::new(
            self.inner.inner.run_id,
            self.inner.inner.handlers.clone(),
            self.inner.inner.inheritable_handlers.clone(),
            self.inner.inner.parent_run_id,
            Some(self.inner.inner.tags.clone()),
            Some(self.inner.inner.inheritable_tags.clone()),
            Some(self.inner.inner.metadata.clone()),
            Some(self.inner.inner.inheritable_metadata.clone()),
        )
    }

    pub fn run_id(&self) -> Uuid {
        self.inner.run_id()
    }

    pub fn parent_run_id(&self) -> Option<Uuid> {
        self.inner.parent_run_id()
    }

    pub fn handlers(&self) -> &[Arc<dyn BaseCallbackHandler>] {
        self.inner.handlers()
    }

    pub fn tags(&self) -> &[String] {
        self.inner.tags()
    }

    pub fn get_noop_manager() -> Self {
        Self {
            inner: AsyncRunManager::get_noop_manager(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParentRunManager {
    inner: RunManager,
}

impl ParentRunManager {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        run_id: Uuid,
        handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        inheritable_handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        inheritable_tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, serde_json::Value>>,
        inheritable_metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self {
            inner: RunManager::new(
                run_id,
                handlers,
                inheritable_handlers,
                parent_run_id,
                tags,
                inheritable_tags,
                metadata,
                inheritable_metadata,
            ),
        }
    }

    pub fn get_child(&self, tag: Option<&str>) -> CallbackManager {
        let mut manager = CallbackManager::new();
        manager.parent_run_id = Some(self.inner.run_id());
        manager.set_handlers(self.inner.inner.inheritable_handlers.clone(), true);
        manager.add_tags(self.inner.inner.inheritable_tags.clone(), true);
        manager.add_metadata(self.inner.inner.inheritable_metadata.clone(), true);
        if let Some(tag) = tag {
            manager.add_tags(vec![tag.to_string()], false);
        }
        manager
    }

    pub fn run_id(&self) -> Uuid {
        self.inner.run_id()
    }

    pub fn parent_run_id(&self) -> Option<Uuid> {
        self.inner.parent_run_id()
    }

    pub fn handlers(&self) -> &[Arc<dyn BaseCallbackHandler>] {
        self.inner.handlers()
    }

    pub fn tags(&self) -> &[String] {
        self.inner.tags()
    }

    pub fn get_noop_manager() -> Self {
        Self {
            inner: RunManager::get_noop_manager(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CallbackManagerForLLMRun {
    inner: RunManager,
}

impl CallbackManagerForLLMRun {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        run_id: Uuid,
        handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        inheritable_handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        inheritable_tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, serde_json::Value>>,
        inheritable_metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self {
            inner: RunManager::new(
                run_id,
                handlers,
                inheritable_handlers,
                parent_run_id,
                tags,
                inheritable_tags,
                metadata,
                inheritable_metadata,
            ),
        }
    }

    pub fn run_id(&self) -> Uuid {
        self.inner.run_id()
    }

    pub fn parent_run_id(&self) -> Option<Uuid> {
        self.inner.parent_run_id()
    }

    pub fn handlers(&self) -> &[Arc<dyn BaseCallbackHandler>] {
        self.inner.handlers()
    }

    pub fn tags(&self) -> &[String] {
        self.inner.tags()
    }

    pub fn on_llm_new_token(&self, token: &str, chunk: Option<&serde_json::Value>) {
        if self.inner.inner.handlers.is_empty() {
            return;
        }
        let run_id = self.inner.run_id();
        let parent_run_id = self.inner.parent_run_id();
        let _tags = self.inner.tags().to_vec();
        handle_event(
            self.inner.handlers(),
            Some(|h: &dyn BaseCallbackHandler| h.ignore_llm()),
            |handler| {
                handler.on_llm_new_token(token, run_id, parent_run_id, chunk);
            },
        );
    }

    pub fn on_llm_end(&self, response: &ChatResult) {
        if self.inner.inner.handlers.is_empty() {
            return;
        }
        let run_id = self.inner.run_id();
        let parent_run_id = self.inner.parent_run_id();
        let _tags = self.inner.tags().to_vec();
        handle_event(
            self.inner.handlers(),
            Some(|h: &dyn BaseCallbackHandler| h.ignore_llm()),
            |handler| {
                handler.on_llm_end(response, run_id, parent_run_id);
            },
        );
    }

    pub fn on_llm_error(&self, error: &dyn std::error::Error) {
        if self.inner.inner.handlers.is_empty() {
            return;
        }
        let run_id = self.inner.run_id();
        let parent_run_id = self.inner.parent_run_id();
        let _tags = self.inner.tags().to_vec();
        handle_event(
            self.inner.handlers(),
            Some(|h: &dyn BaseCallbackHandler| h.ignore_llm()),
            |handler| {
                handler.on_llm_error(error, run_id, parent_run_id);
            },
        );
    }

    pub fn get_noop_manager() -> Self {
        Self {
            inner: RunManager::get_noop_manager(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CallbackManagerForChainRun {
    inner: ParentRunManager,
}

impl CallbackManagerForChainRun {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        run_id: Uuid,
        handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        inheritable_handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        inheritable_tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, serde_json::Value>>,
        inheritable_metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self {
            inner: ParentRunManager::new(
                run_id,
                handlers,
                inheritable_handlers,
                parent_run_id,
                tags,
                inheritable_tags,
                metadata,
                inheritable_metadata,
            ),
        }
    }

    pub fn run_id(&self) -> Uuid {
        self.inner.run_id()
    }

    pub fn parent_run_id(&self) -> Option<Uuid> {
        self.inner.parent_run_id()
    }

    pub fn handlers(&self) -> &[Arc<dyn BaseCallbackHandler>] {
        self.inner.handlers()
    }

    pub fn tags(&self) -> &[String] {
        self.inner.tags()
    }

    pub fn get_child(&self, tag: Option<&str>) -> CallbackManager {
        self.inner.get_child(tag)
    }

    pub fn on_chain_end(&self, outputs: &HashMap<String, serde_json::Value>) {
        if self.inner.inner.inner.handlers.is_empty() {
            return;
        }
        let run_id = self.inner.run_id();
        let parent_run_id = self.inner.parent_run_id();
        let _tags = self.inner.tags().to_vec();
        handle_event(
            self.inner.handlers(),
            Some(|h: &dyn BaseCallbackHandler| h.ignore_chain()),
            |handler| {
                handler.on_chain_end(outputs, run_id, parent_run_id);
            },
        );
    }

    pub fn on_chain_error(&self, error: &dyn std::error::Error) {
        if self.inner.inner.inner.handlers.is_empty() {
            return;
        }
        let run_id = self.inner.run_id();
        let parent_run_id = self.inner.parent_run_id();
        let _tags = self.inner.tags().to_vec();
        handle_event(
            self.inner.handlers(),
            Some(|h: &dyn BaseCallbackHandler| h.ignore_chain()),
            |handler| {
                handler.on_chain_error(error, run_id, parent_run_id);
            },
        );
    }

    pub fn on_agent_action(&self, action: &serde_json::Value) {
        if self.inner.inner.inner.handlers.is_empty() {
            return;
        }
        let run_id = self.inner.run_id();
        let parent_run_id = self.inner.parent_run_id();
        let _tags = self.inner.tags().to_vec();
        handle_event(
            self.inner.handlers(),
            Some(|h: &dyn BaseCallbackHandler| h.ignore_agent()),
            |handler| {
                handler.on_agent_action(action, run_id, parent_run_id, None);
            },
        );
    }

    pub fn on_agent_finish(&self, finish: &serde_json::Value) {
        if self.inner.inner.inner.handlers.is_empty() {
            return;
        }
        let run_id = self.inner.run_id();
        let parent_run_id = self.inner.parent_run_id();
        let _tags = self.inner.tags().to_vec();
        handle_event(
            self.inner.handlers(),
            Some(|h: &dyn BaseCallbackHandler| h.ignore_agent()),
            |handler| {
                handler.on_agent_finish(finish, run_id, parent_run_id, None);
            },
        );
    }

    pub fn get_noop_manager() -> Self {
        Self {
            inner: ParentRunManager::get_noop_manager(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CallbackManagerForToolRun {
    inner: ParentRunManager,
}

impl CallbackManagerForToolRun {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        run_id: Uuid,
        handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        inheritable_handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        inheritable_tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, serde_json::Value>>,
        inheritable_metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self {
            inner: ParentRunManager::new(
                run_id,
                handlers,
                inheritable_handlers,
                parent_run_id,
                tags,
                inheritable_tags,
                metadata,
                inheritable_metadata,
            ),
        }
    }

    pub fn run_id(&self) -> Uuid {
        self.inner.run_id()
    }

    pub fn parent_run_id(&self) -> Option<Uuid> {
        self.inner.parent_run_id()
    }

    pub fn handlers(&self) -> &[Arc<dyn BaseCallbackHandler>] {
        self.inner.handlers()
    }

    pub fn tags(&self) -> &[String] {
        self.inner.tags()
    }

    pub fn get_child(&self, tag: Option<&str>) -> CallbackManager {
        self.inner.get_child(tag)
    }

    pub fn on_tool_end(&self, output: &str) {
        if self.inner.inner.inner.handlers.is_empty() {
            return;
        }
        let run_id = self.inner.run_id();
        let parent_run_id = self.inner.parent_run_id();
        let _tags = self.inner.tags().to_vec();
        handle_event(
            self.inner.handlers(),
            Some(|h: &dyn BaseCallbackHandler| h.ignore_agent()),
            |handler| {
                handler.on_tool_end(output, run_id, parent_run_id, None, None, None);
            },
        );
    }

    pub fn on_tool_error(&self, error: &dyn std::error::Error) {
        if self.inner.inner.inner.handlers.is_empty() {
            return;
        }
        let run_id = self.inner.run_id();
        let parent_run_id = self.inner.parent_run_id();
        let _tags = self.inner.tags().to_vec();
        handle_event(
            self.inner.handlers(),
            Some(|h: &dyn BaseCallbackHandler| h.ignore_agent()),
            |handler| {
                handler.on_tool_error(error, run_id, parent_run_id);
            },
        );
    }

    pub fn get_noop_manager() -> Self {
        Self {
            inner: ParentRunManager::get_noop_manager(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CallbackManagerForRetrieverRun {
    inner: ParentRunManager,
}

impl CallbackManagerForRetrieverRun {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        run_id: Uuid,
        handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        inheritable_handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        parent_run_id: Option<Uuid>,
        tags: Option<Vec<String>>,
        inheritable_tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, serde_json::Value>>,
        inheritable_metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self {
            inner: ParentRunManager::new(
                run_id,
                handlers,
                inheritable_handlers,
                parent_run_id,
                tags,
                inheritable_tags,
                metadata,
                inheritable_metadata,
            ),
        }
    }

    pub fn run_id(&self) -> Uuid {
        self.inner.run_id()
    }

    pub fn parent_run_id(&self) -> Option<Uuid> {
        self.inner.parent_run_id()
    }

    pub fn handlers(&self) -> &[Arc<dyn BaseCallbackHandler>] {
        self.inner.handlers()
    }

    pub fn tags(&self) -> &[String] {
        self.inner.tags()
    }

    pub fn get_child(&self, tag: Option<&str>) -> CallbackManager {
        self.inner.get_child(tag)
    }

    pub fn on_retriever_end(&self, documents: &[serde_json::Value]) {
        if self.inner.inner.inner.handlers.is_empty() {
            return;
        }
        let run_id = self.inner.run_id();
        let parent_run_id = self.inner.parent_run_id();
        let _tags = self.inner.tags().to_vec();
        handle_event(
            self.inner.handlers(),
            Some(|h: &dyn BaseCallbackHandler| h.ignore_retriever()),
            |handler| {
                handler.on_retriever_end(documents, run_id, parent_run_id);
            },
        );
    }

    pub fn on_retriever_error(&self, error: &dyn std::error::Error) {
        if self.inner.inner.inner.handlers.is_empty() {
            return;
        }
        let run_id = self.inner.run_id();
        let parent_run_id = self.inner.parent_run_id();
        let _tags = self.inner.tags().to_vec();
        handle_event(
            self.inner.handlers(),
            Some(|h: &dyn BaseCallbackHandler| h.ignore_retriever()),
            |handler| {
                handler.on_retriever_error(error, run_id, parent_run_id);
            },
        );
    }

    pub fn get_noop_manager() -> Self {
        Self {
            inner: ParentRunManager::get_noop_manager(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CallbackManager {
    pub handlers: Vec<Arc<dyn BaseCallbackHandler>>,
    pub inheritable_handlers: Vec<Arc<dyn BaseCallbackHandler>>,
    pub parent_run_id: Option<Uuid>,
    pub tags: Vec<String>,
    pub inheritable_tags: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub inheritable_metadata: HashMap<String, serde_json::Value>,
}

impl CallbackManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_base(base: BaseCallbackManager) -> Self {
        Self {
            handlers: base.handlers,
            inheritable_handlers: base.inheritable_handlers,
            parent_run_id: base.parent_run_id,
            tags: base.tags,
            inheritable_tags: base.inheritable_tags,
            metadata: base.metadata,
            inheritable_metadata: base.inheritable_metadata,
        }
    }

    pub fn set_handlers(&mut self, handlers: Vec<Arc<dyn BaseCallbackHandler>>, inherit: bool) {
        self.handlers = Vec::new();
        self.inheritable_handlers = Vec::new();
        for handler in handlers {
            self.add_handler(handler, inherit);
        }
    }

    pub fn set_handler(&mut self, handler: Arc<dyn BaseCallbackHandler>, inherit: bool) {
        self.set_handlers(vec![handler], inherit);
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
    pub fn add_tags(&mut self, tags: Vec<String>, inherit: bool) {
        for tag in &tags {
            if self.tags.contains(tag) {
                self.remove_tags(std::slice::from_ref(tag));
            }
        }
        self.tags.extend(tags.clone());
        if inherit {
            self.inheritable_tags.extend(tags);
        }
    }

    pub fn remove_tags(&mut self, tags: &[String]) {
        for tag in tags {
            self.tags.retain(|t| t != tag);
            self.inheritable_tags.retain(|t| t != tag);
        }
    }

    pub fn remove_metadata(&mut self, keys: &[String]) {
        for key in keys {
            self.metadata.remove(key);
            self.inheritable_metadata.remove(key);
        }
    }

    pub fn add_metadata(&mut self, metadata: HashMap<String, serde_json::Value>, inherit: bool) {
        self.metadata.extend(metadata.clone());
        if inherit {
            self.inheritable_metadata.extend(metadata);
        }
    }

    pub fn on_llm_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        prompts: &[String],
        run_id: Option<Uuid>,
    ) -> Vec<CallbackManagerForLLMRun> {
        let mut managers = Vec::new();

        for (i, _prompt) in prompts.iter().enumerate() {
            let run_id = if i == 0
                && let Some(run_id) = run_id
            {
                run_id
            } else {
                uuid7(None)
            };

            handle_event(
                &self.handlers,
                Some(|h: &dyn BaseCallbackHandler| h.ignore_llm()),
                |handler| {
                    handler.on_llm_start(
                        serialized,
                        prompts,
                        run_id,
                        self.parent_run_id,
                        Some(&self.tags),
                        Some(&self.metadata),
                    );
                },
            );

            managers.push(CallbackManagerForLLMRun::new(
                run_id,
                self.handlers.clone(),
                self.inheritable_handlers.clone(),
                self.parent_run_id,
                Some(self.tags.clone()),
                Some(self.inheritable_tags.clone()),
                Some(self.metadata.clone()),
                Some(self.inheritable_metadata.clone()),
            ));
        }

        managers
    }

    pub fn on_chat_model_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        messages: &[Vec<BaseMessage>],
        run_id: Option<Uuid>,
    ) -> Vec<CallbackManagerForLLMRun> {
        let mut managers = Vec::new();
        let mut current_run_id = run_id;

        for _message_list in messages {
            let run_id = current_run_id.unwrap_or_else(|| uuid7(None));
            current_run_id = None;

            handle_event(
                &self.handlers,
                Some(|h: &dyn BaseCallbackHandler| h.ignore_chat_model()),
                |handler| {
                    handler.on_chat_model_start(
                        serialized,
                        messages,
                        run_id,
                        self.parent_run_id,
                        Some(&self.tags),
                        Some(&self.metadata),
                    );
                },
            );

            managers.push(CallbackManagerForLLMRun::new(
                run_id,
                self.handlers.clone(),
                self.inheritable_handlers.clone(),
                self.parent_run_id,
                Some(self.tags.clone()),
                Some(self.inheritable_tags.clone()),
                Some(self.metadata.clone()),
                Some(self.inheritable_metadata.clone()),
            ));
        }

        managers
    }

    pub fn on_tool_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        input_str: &str,
        run_id: Option<Uuid>,
        inputs: Option<&HashMap<String, serde_json::Value>>,
    ) -> CallbackManagerForToolRun {
        let run_id = run_id.unwrap_or_else(|| uuid7(None));

        handle_event(
            &self.handlers,
            Some(|h: &dyn BaseCallbackHandler| h.ignore_agent()),
            |handler| {
                handler.on_tool_start(
                    serialized,
                    input_str,
                    run_id,
                    self.parent_run_id,
                    Some(&self.tags),
                    Some(&self.metadata),
                    inputs,
                );
            },
        );

        CallbackManagerForToolRun::new(
            run_id,
            self.handlers.clone(),
            self.inheritable_handlers.clone(),
            self.parent_run_id,
            Some(self.tags.clone()),
            Some(self.inheritable_tags.clone()),
            Some(self.metadata.clone()),
            Some(self.inheritable_metadata.clone()),
        )
    }

    pub fn on_custom_event(&self, name: &str, data: &serde_json::Value, run_id: Option<Uuid>) {
        if self.handlers.is_empty() {
            return;
        }

        let run_id = run_id.unwrap_or_else(|| uuid7(None));

        handle_event(
            &self.handlers,
            Some(|h: &dyn BaseCallbackHandler| h.ignore_custom_event()),
            |handler| {
                handler.on_custom_event(name, data, run_id, None, None);
            },
        );
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

    pub fn merge(&self, other: &CallbackManager) -> Self {
        let mut tags: Vec<String> = self.tags.clone();
        for tag in &other.tags {
            if !tags.contains(tag) {
                tags.push(tag.clone());
            }
        }

        let mut inheritable_tags: Vec<String> = self.inheritable_tags.clone();
        for tag in &other.inheritable_tags {
            if !inheritable_tags.contains(tag) {
                inheritable_tags.push(tag.clone());
            }
        }

        let mut metadata = self.metadata.clone();
        metadata.extend(other.metadata.clone());

        let mut inheritable_metadata = self.inheritable_metadata.clone();
        inheritable_metadata.extend(other.inheritable_metadata.clone());

        let mut manager = Self {
            parent_run_id: self.parent_run_id.or(other.parent_run_id),
            handlers: Vec::new(),
            inheritable_handlers: Vec::new(),
            tags,
            inheritable_tags,
            metadata,
            inheritable_metadata,
        };

        let handlers: Vec<Arc<dyn BaseCallbackHandler>> = self
            .handlers
            .iter()
            .chain(other.handlers.iter())
            .cloned()
            .collect();

        let inheritable_handlers: Vec<Arc<dyn BaseCallbackHandler>> = self
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

    pub fn configure(
        inheritable_callbacks: Option<Callbacks>,
        local_callbacks: Option<Callbacks>,
        verbose: bool,
        inheritable_tags: Option<Vec<String>>,
        local_tags: Option<Vec<String>>,
        inheritable_metadata: Option<HashMap<String, serde_json::Value>>,
        local_metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        _configure(
            inheritable_callbacks,
            local_callbacks,
            verbose,
            inheritable_tags,
            local_tags,
            inheritable_metadata,
            local_metadata,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn _configure(
    inheritable_callbacks: Option<Callbacks>,
    local_callbacks: Option<Callbacks>,
    verbose: bool,
    inheritable_tags: Option<Vec<String>>,
    local_tags: Option<Vec<String>>,
    inheritable_metadata: Option<HashMap<String, serde_json::Value>>,
    local_metadata: Option<HashMap<String, serde_json::Value>>,
) -> CallbackManager {
    let tracing_metadata: HashMap<String, serde_json::Value> = HashMap::new();
    let tracing_tags: Vec<String> = Vec::new();
    let parent_run_id: Option<Uuid> = None;

    let mut callback_manager = CallbackManager::new();
    callback_manager.parent_run_id = parent_run_id;

    if let Some(callbacks) = inheritable_callbacks {
        match callbacks {
            Callbacks::Handlers(handlers) => {
                callback_manager.handlers = handlers.clone();
                callback_manager.inheritable_handlers = handlers;
            }
            Callbacks::Manager(manager) => {
                let parent_run_id_ = if parent_run_id.is_some() {
                    parent_run_id
                } else {
                    manager.parent_run_id
                };
                callback_manager.handlers = manager.handlers.clone();
                callback_manager.inheritable_handlers = manager.inheritable_handlers.clone();
                callback_manager.parent_run_id = parent_run_id_;
                callback_manager.tags = manager.tags.clone();
                callback_manager.inheritable_tags = manager.inheritable_tags.clone();
                callback_manager.metadata = manager.metadata.clone();
                callback_manager.inheritable_metadata = manager.inheritable_metadata.clone();
            }
        }
    }

    if let Some(callbacks) = local_callbacks {
        let local_handlers = match callbacks {
            Callbacks::Handlers(handlers) => handlers,
            Callbacks::Manager(manager) => manager.handlers,
        };
        for handler in local_handlers {
            callback_manager.add_handler(handler, false);
        }
    }

    if let Some(tags) = inheritable_tags {
        callback_manager.add_tags(tags, true);
    }
    if let Some(tags) = local_tags {
        callback_manager.add_tags(tags, false);
    }
    if let Some(metadata) = inheritable_metadata {
        callback_manager.add_metadata(metadata, true);
    }
    if let Some(metadata) = local_metadata {
        callback_manager.add_metadata(metadata, false);
    }

    if !tracing_metadata.is_empty() {
        callback_manager.add_metadata(tracing_metadata, true);
    }
    if !tracing_tags.is_empty() {
        callback_manager.add_tags(tracing_tags, true);
    }

    let v1_tracing_enabled =
        env_var_is_set("LANGCHAIN_TRACING") || env_var_is_set("LANGCHAIN_HANDLER");
    let tracing_v2_enabled = tracing_v2_is_enabled();

    if v1_tracing_enabled && !tracing_v2_enabled {
        tracing::warn!(
            "Tracing using LangChainTracerV1 is no longer supported.              Please set the LANGCHAIN_TRACING_V2 environment variable to enable              tracing instead."
        );
    }

    let debug = get_debug();

    if verbose || debug || tracing_v2_enabled {
        if verbose
            && !callback_manager
                .handlers
                .iter()
                .any(|h| h.name() == "StdOutCallbackHandler")
            && !debug
        {
            callback_manager.add_handler(Arc::new(StdOutCallbackHandler::new()), false);
        }

        if debug
            && !callback_manager
                .handlers
                .iter()
                .any(|h| h.name() == "ConsoleCallbackHandler")
        {
            callback_manager.add_handler(Arc::new(ConsoleCallbackHandler::new()), true);
        }

        if tracing_v2_enabled
            && !callback_manager
                .handlers
                .iter()
                .any(|h| h.name() == "LangChainTracer")
        {
            if let Some(_tracer) = get_tracing_callback() {
                tracing::debug!(
                    "Tracing is enabled but LangChainTracer is not yet                      implemented in Rust. Tracing callbacks will not be sent."
                );
            } else {
                let tracer_project = get_tracer_project();
                tracing::debug!(
                    "Tracing is enabled (project: {}) but LangChainTracer is not yet                      implemented in Rust. Tracing callbacks will not be sent.",
                    tracer_project
                );
            }
        }
    }

    if let Ok(registry) = get_configure_hooks().lock() {
        for hook in registry.hooks() {
            let create_from_env = hook.env_var.as_ref().is_some_and(|var| env_var_is_set(var))
                && hook.handler_factory.is_some();

            let context_handler = (hook.context_getter)();

            if context_handler.is_some() || create_from_env {
                let handler = context_handler
                    .unwrap_or_else(|| (hook.handler_factory.as_ref().expect("checked above"))());

                let already_present = if let Some(type_name) = &hook.handler_type_name {
                    callback_manager
                        .handlers
                        .iter()
                        .any(|h| h.name() == type_name)
                } else {
                    callback_manager
                        .handlers
                        .iter()
                        .any(|h| std::ptr::eq(h.as_ref(), handler.as_ref()))
                };

                if !already_present {
                    callback_manager.add_handler(handler, hook.inheritable);
                }
            }
        }
    }

    callback_manager
}

#[derive(Debug, Clone, Default)]
pub struct AsyncCallbackManager {
    inner: CallbackManager,
}

#[bon]
impl CallbackManager {
    #[builder]
    pub fn on_chain_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        inputs: &HashMap<String, serde_json::Value>,
        run_id: Option<Uuid>,
        name: Option<&str>,
    ) -> CallbackManagerForChainRun {
        let run_id = run_id.unwrap_or_else(|| uuid7(None));

        handle_event(
            &self.handlers,
            Some(|h: &dyn BaseCallbackHandler| h.ignore_chain()),
            |handler| {
                handler.on_chain_start(
                    serialized,
                    inputs,
                    run_id,
                    self.parent_run_id,
                    Some(&self.tags),
                    Some(&self.metadata),
                    name,
                );
            },
        );

        CallbackManagerForChainRun::new(
            run_id,
            self.handlers.clone(),
            self.inheritable_handlers.clone(),
            self.parent_run_id,
            Some(self.tags.clone()),
            Some(self.inheritable_tags.clone()),
            Some(self.metadata.clone()),
            Some(self.inheritable_metadata.clone()),
        )
    }

    #[builder]
    pub fn on_retriever_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        query: &str,
        run_id: Option<Uuid>,
        name: Option<&str>,
    ) -> CallbackManagerForRetrieverRun {
        let run_id = run_id.unwrap_or_else(|| uuid7(None));

        handle_event(
            &self.handlers,
            Some(|h: &dyn BaseCallbackHandler| h.ignore_retriever()),
            |handler| {
                handler.on_retriever_start(
                    serialized,
                    query,
                    run_id,
                    self.parent_run_id,
                    Some(&self.tags),
                    Some(&self.metadata),
                    name,
                );
            },
        );

        CallbackManagerForRetrieverRun::new(
            run_id,
            self.handlers.clone(),
            self.inheritable_handlers.clone(),
            self.parent_run_id,
            Some(self.tags.clone()),
            Some(self.inheritable_tags.clone()),
            Some(self.metadata.clone()),
            Some(self.inheritable_metadata.clone()),
        )
    }
}

impl AsyncCallbackManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_callback_manager(manager: CallbackManager) -> Self {
        Self { inner: manager }
    }

    pub fn handlers(&self) -> &[Arc<dyn BaseCallbackHandler>] {
        &self.inner.handlers
    }

    pub fn parent_run_id(&self) -> Option<Uuid> {
        self.inner.parent_run_id
    }

    pub fn set_handlers(&mut self, handlers: Vec<Arc<dyn BaseCallbackHandler>>, inherit: bool) {
        self.inner.set_handlers(handlers, inherit);
    }

    pub fn add_handler(&mut self, handler: Arc<dyn BaseCallbackHandler>, inherit: bool) {
        self.inner.add_handler(handler, inherit);
    }

    pub fn remove_handler(&mut self, handler: &Arc<dyn BaseCallbackHandler>) {
        self.inner.remove_handler(handler);
    }

    pub fn to_callback_manager(&self) -> CallbackManager {
        self.inner.clone()
    }

    pub fn add_tags(&mut self, tags: Vec<String>, inherit: bool) {
        self.inner.add_tags(tags, inherit);
    }

    pub fn add_metadata(&mut self, metadata: HashMap<String, serde_json::Value>, inherit: bool) {
        self.inner.add_metadata(metadata, inherit);
    }

    pub fn is_async(&self) -> bool {
        true
    }

    pub async fn on_llm_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        prompts: &[String],
        run_id: Option<Uuid>,
    ) -> Vec<AsyncCallbackManagerForLLMRun> {
        self.inner
            .on_llm_start(serialized, prompts, run_id)
            .into_iter()
            .map(AsyncCallbackManagerForLLMRun::from_sync)
            .collect()
    }

    pub async fn on_chat_model_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        messages: &[Vec<BaseMessage>],
        run_id: Option<Uuid>,
    ) -> Vec<AsyncCallbackManagerForLLMRun> {
        self.inner
            .on_chat_model_start(serialized, messages, run_id)
            .into_iter()
            .map(AsyncCallbackManagerForLLMRun::from_sync)
            .collect()
    }

    pub async fn on_chain_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        inputs: &HashMap<String, serde_json::Value>,
        run_id: Option<Uuid>,
        name: Option<&str>,
    ) -> AsyncCallbackManagerForChainRun {
        AsyncCallbackManagerForChainRun::from_sync(
            self.inner
                .on_chain_start()
                .serialized(serialized)
                .inputs(inputs)
                .maybe_run_id(run_id)
                .maybe_name(name)
                .call(),
        )
    }

    pub async fn on_tool_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        input_str: &str,
        run_id: Option<Uuid>,
        inputs: Option<&HashMap<String, serde_json::Value>>,
    ) -> AsyncCallbackManagerForToolRun {
        AsyncCallbackManagerForToolRun::from_sync(
            self.inner
                .on_tool_start(serialized, input_str, run_id, inputs),
        )
    }

    pub async fn on_custom_event(
        &self,
        name: &str,
        data: &serde_json::Value,
        run_id: Option<Uuid>,
    ) {
        if self.inner.handlers.is_empty() {
            return;
        }

        let run_id = run_id.unwrap_or_else(|| uuid7(None));

        handle_event(
            &self.inner.handlers,
            Some(|h: &dyn BaseCallbackHandler| h.ignore_custom_event()),
            |handler| {
                handler.on_custom_event(name, data, run_id, None, None);
            },
        );
    }

    pub fn configure(
        inheritable_callbacks: Option<Callbacks>,
        local_callbacks: Option<Callbacks>,
        verbose: bool,
        inheritable_tags: Option<Vec<String>>,
        local_tags: Option<Vec<String>>,
        inheritable_metadata: Option<HashMap<String, serde_json::Value>>,
        local_metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self {
            inner: _configure(
                inheritable_callbacks,
                local_callbacks,
                verbose,
                inheritable_tags,
                local_tags,
                inheritable_metadata,
                local_metadata,
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AsyncCallbackManagerForLLMRun {
    inner: CallbackManagerForLLMRun,
}

#[bon]
impl AsyncCallbackManager {
    #[builder]
    pub async fn on_retriever_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        query: &str,
        run_id: Option<Uuid>,
        name: Option<&str>,
    ) -> AsyncCallbackManagerForRetrieverRun {
        AsyncCallbackManagerForRetrieverRun::from_sync(
            self.inner
                .on_retriever_start()
                .serialized(serialized)
                .query(query)
                .maybe_run_id(run_id)
                .maybe_name(name)
                .call(),
        )
    }
}

impl AsyncCallbackManagerForLLMRun {
    pub fn from_sync(inner: CallbackManagerForLLMRun) -> Self {
        Self { inner }
    }

    pub fn get_sync(&self) -> CallbackManagerForLLMRun {
        self.inner.clone()
    }

    pub fn run_id(&self) -> Uuid {
        self.inner.run_id()
    }

    pub fn parent_run_id(&self) -> Option<Uuid> {
        self.inner.parent_run_id()
    }

    pub fn handlers(&self) -> &[Arc<dyn BaseCallbackHandler>] {
        self.inner.handlers()
    }

    pub async fn on_llm_new_token(&self, token: &str, chunk: Option<&serde_json::Value>) {
        self.inner.on_llm_new_token(token, chunk);
    }

    pub async fn on_llm_end(&self, response: &ChatResult) {
        self.inner.on_llm_end(response);
    }

    pub async fn on_llm_error(&self, error: &dyn std::error::Error) {
        self.inner.on_llm_error(error);
    }

    pub fn get_noop_manager() -> Self {
        Self {
            inner: CallbackManagerForLLMRun::get_noop_manager(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AsyncCallbackManagerForChainRun {
    inner: CallbackManagerForChainRun,
}

impl AsyncCallbackManagerForChainRun {
    pub fn from_sync(inner: CallbackManagerForChainRun) -> Self {
        Self { inner }
    }

    pub fn get_sync(&self) -> CallbackManagerForChainRun {
        self.inner.clone()
    }

    pub fn run_id(&self) -> Uuid {
        self.inner.run_id()
    }

    pub fn parent_run_id(&self) -> Option<Uuid> {
        self.inner.parent_run_id()
    }

    pub fn handlers(&self) -> &[Arc<dyn BaseCallbackHandler>] {
        self.inner.handlers()
    }

    pub fn get_child(&self, tag: Option<&str>) -> AsyncCallbackManager {
        AsyncCallbackManager::from_callback_manager(self.inner.get_child(tag))
    }

    pub async fn on_chain_end(&self, outputs: &HashMap<String, serde_json::Value>) {
        self.inner.on_chain_end(outputs);
    }

    pub async fn on_chain_error(&self, error: &dyn std::error::Error) {
        self.inner.on_chain_error(error);
    }

    pub async fn on_agent_action(&self, action: &serde_json::Value) {
        self.inner.on_agent_action(action);
    }

    pub async fn on_agent_finish(&self, finish: &serde_json::Value) {
        self.inner.on_agent_finish(finish);
    }

    pub fn get_noop_manager() -> Self {
        Self {
            inner: CallbackManagerForChainRun::get_noop_manager(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AsyncCallbackManagerForToolRun {
    inner: CallbackManagerForToolRun,
}

impl AsyncCallbackManagerForToolRun {
    pub fn from_sync(inner: CallbackManagerForToolRun) -> Self {
        Self { inner }
    }

    pub fn get_sync(&self) -> CallbackManagerForToolRun {
        self.inner.clone()
    }

    pub fn run_id(&self) -> Uuid {
        self.inner.run_id()
    }

    pub fn parent_run_id(&self) -> Option<Uuid> {
        self.inner.parent_run_id()
    }

    pub fn handlers(&self) -> &[Arc<dyn BaseCallbackHandler>] {
        self.inner.handlers()
    }

    pub fn get_child(&self, tag: Option<&str>) -> AsyncCallbackManager {
        AsyncCallbackManager::from_callback_manager(self.inner.get_child(tag))
    }

    pub async fn on_tool_end(&self, output: &str) {
        self.inner.on_tool_end(output);
    }

    pub async fn on_tool_error(&self, error: &dyn std::error::Error) {
        self.inner.on_tool_error(error);
    }

    pub fn get_noop_manager() -> Self {
        Self {
            inner: CallbackManagerForToolRun::get_noop_manager(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AsyncCallbackManagerForRetrieverRun {
    inner: CallbackManagerForRetrieverRun,
}

impl AsyncCallbackManagerForRetrieverRun {
    pub fn from_sync(inner: CallbackManagerForRetrieverRun) -> Self {
        Self { inner }
    }

    pub fn get_sync(&self) -> CallbackManagerForRetrieverRun {
        self.inner.clone()
    }

    pub fn run_id(&self) -> Uuid {
        self.inner.run_id()
    }

    pub fn parent_run_id(&self) -> Option<Uuid> {
        self.inner.parent_run_id()
    }

    pub fn handlers(&self) -> &[Arc<dyn BaseCallbackHandler>] {
        self.inner.handlers()
    }

    pub fn get_child(&self, tag: Option<&str>) -> AsyncCallbackManager {
        AsyncCallbackManager::from_callback_manager(self.inner.get_child(tag))
    }

    pub async fn on_retriever_end(&self, documents: &[serde_json::Value]) {
        self.inner.on_retriever_end(documents);
    }

    pub async fn on_retriever_error(&self, error: &dyn std::error::Error) {
        self.inner.on_retriever_error(error);
    }

    pub fn get_noop_manager() -> Self {
        Self {
            inner: CallbackManagerForRetrieverRun::get_noop_manager(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_callback_manager_on_chain_start() {
        let manager = CallbackManager::new();
        let run_manager = manager
            .on_chain_start()
            .serialized(&HashMap::new())
            .inputs(&HashMap::new())
            .call();

        assert!(!run_manager.run_id().is_nil());
    }

    #[test]
    fn test_callback_manager_configure() {
        let manager = CallbackManager::configure(
            None,
            None,
            false,
            Some(vec!["tag1".to_string()]),
            Some(vec!["tag2".to_string()]),
            None,
            None,
        );

        assert!(manager.tags.contains(&"tag1".to_string()));
        assert!(manager.tags.contains(&"tag2".to_string()));
        assert!(manager.inheritable_tags.contains(&"tag1".to_string()));
        assert!(!manager.inheritable_tags.contains(&"tag2".to_string()));
    }

    #[test]
    fn test_configure_with_verbose() {
        crate::globals::set_debug(false);

        let manager = CallbackManager::configure(None, None, true, None, None, None, None);
        assert!(
            manager
                .handlers
                .iter()
                .any(|h| h.name() == "StdOutCallbackHandler"),
            "StdOutCallbackHandler should be added when verbose=true"
        );
    }

    // #[test]
    // fn test_configure_verbose_not_added_when_debug() {
    //     crate::globals::set_debug(true);

    //     let manager = CallbackManager::configure(None, None, true, None, None, None, None);
    //     assert!(
    //         !manager
    //             .handlers
    //             .iter()
    //             .any(|h| h.name() == "StdOutCallbackHandler"),
    //         "StdOutCallbackHandler should NOT be added when debug=true (debug supersedes verbose)"
    //     );
    //     assert!(
    //         manager
    //             .handlers
    //             .iter()
    //             .any(|h| h.name() == "ConsoleCallbackHandler"),
    //         "ConsoleCallbackHandler should be added when debug=true"
    //     );

    //     crate::globals::set_debug(false);
    // }

    #[test]
    fn test_configure_deduplication() {
        crate::globals::set_debug(false);

        let handler: Arc<dyn BaseCallbackHandler> = Arc::new(StdOutCallbackHandler::new());
        let callbacks = Callbacks::Handlers(vec![handler]);

        let manager =
            CallbackManager::configure(Some(callbacks), None, true, None, None, None, None);

        let stdout_count = manager
            .handlers
            .iter()
            .filter(|h| h.name() == "StdOutCallbackHandler")
            .count();
        assert_eq!(
            stdout_count, 1,
            "Should not duplicate StdOutCallbackHandler"
        );
    }
}

#[derive(Debug, Clone)]
pub struct CallbackManagerForChainGroup {
    inner: CallbackManager,
    parent_run_manager: CallbackManagerForChainRun,
    pub ended: bool,
}

impl CallbackManagerForChainGroup {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        inheritable_handlers: Option<Vec<Arc<dyn BaseCallbackHandler>>>,
        parent_run_id: Option<Uuid>,
        parent_run_manager: CallbackManagerForChainRun,
        tags: Option<Vec<String>>,
        inheritable_tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, serde_json::Value>>,
        inheritable_metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        let mut inner = CallbackManager::new();
        inner.handlers = handlers;
        inner.inheritable_handlers = inheritable_handlers.unwrap_or_default();
        inner.parent_run_id = parent_run_id;
        inner.tags = tags.unwrap_or_default();
        inner.inheritable_tags = inheritable_tags.unwrap_or_default();
        inner.metadata = metadata.unwrap_or_default();
        inner.inheritable_metadata = inheritable_metadata.unwrap_or_default();

        Self {
            inner,
            parent_run_manager,
            ended: false,
        }
    }

    pub fn handlers(&self) -> &[Arc<dyn BaseCallbackHandler>] {
        &self.inner.handlers
    }

    pub fn parent_run_id(&self) -> Option<Uuid> {
        self.inner.parent_run_id
    }

    pub fn tags(&self) -> &[String] {
        &self.inner.tags
    }

    pub fn copy(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            parent_run_manager: self.parent_run_manager.clone(),
            ended: self.ended,
        }
    }

    pub fn merge(&self, other: &CallbackManager) -> Self {
        let mut merged_inner = self.inner.clone();

        for tag in &other.tags {
            if !merged_inner.tags.contains(tag) {
                merged_inner.tags.push(tag.clone());
            }
        }
        for tag in &other.inheritable_tags {
            if !merged_inner.inheritable_tags.contains(tag) {
                merged_inner.inheritable_tags.push(tag.clone());
            }
        }

        merged_inner.metadata.extend(other.metadata.clone());

        for handler in &other.handlers {
            merged_inner.add_handler(handler.clone(), false);
        }

        Self {
            inner: merged_inner,
            parent_run_manager: self.parent_run_manager.clone(),
            ended: self.ended,
        }
    }

    pub fn set_handlers(&mut self, handlers: Vec<Arc<dyn BaseCallbackHandler>>, inherit: bool) {
        self.inner.set_handlers(handlers, inherit);
    }

    pub fn add_handler(&mut self, handler: Arc<dyn BaseCallbackHandler>, inherit: bool) {
        self.inner.add_handler(handler, inherit);
    }

    pub fn add_tags(&mut self, tags: Vec<String>, inherit: bool) {
        self.inner.add_tags(tags, inherit);
    }

    pub fn add_metadata(&mut self, metadata: HashMap<String, serde_json::Value>, inherit: bool) {
        self.inner.add_metadata(metadata, inherit);
    }

    pub fn on_chain_end(&mut self, outputs: &HashMap<String, serde_json::Value>) {
        self.ended = true;
        self.parent_run_manager.on_chain_end(outputs);
    }

    pub fn on_chain_error(&mut self, error: &dyn std::error::Error) {
        self.ended = true;
        self.parent_run_manager.on_chain_error(error);
    }

    pub fn on_llm_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        prompts: &[String],
        run_id: Option<Uuid>,
    ) -> Vec<CallbackManagerForLLMRun> {
        self.inner.on_llm_start(serialized, prompts, run_id)
    }

    pub fn on_chat_model_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        messages: &[Vec<BaseMessage>],
        run_id: Option<Uuid>,
    ) -> Vec<CallbackManagerForLLMRun> {
        self.inner.on_chat_model_start(serialized, messages, run_id)
    }

    pub fn on_chain_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        inputs: &HashMap<String, serde_json::Value>,
        run_id: Option<Uuid>,
        name: Option<&str>,
    ) -> CallbackManagerForChainRun {
        self.inner
            .on_chain_start()
            .serialized(serialized)
            .inputs(inputs)
            .maybe_run_id(run_id)
            .maybe_name(name)
            .call()
    }

    pub fn on_tool_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        input_str: &str,
        run_id: Option<Uuid>,
        inputs: Option<&HashMap<String, serde_json::Value>>,
    ) -> CallbackManagerForToolRun {
        self.inner
            .on_tool_start(serialized, input_str, run_id, inputs)
    }

    pub fn on_retriever_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        query: &str,
        run_id: Option<Uuid>,
        name: Option<&str>,
    ) -> CallbackManagerForRetrieverRun {
        self.inner
            .on_retriever_start()
            .serialized(serialized)
            .query(query)
            .maybe_run_id(run_id)
            .maybe_name(name)
            .call()
    }
}

#[derive(Debug, Clone)]
pub struct AsyncCallbackManagerForChainGroup {
    inner: AsyncCallbackManager,
    parent_run_manager: AsyncCallbackManagerForChainRun,
    pub ended: bool,
}

impl AsyncCallbackManagerForChainGroup {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        inheritable_handlers: Option<Vec<Arc<dyn BaseCallbackHandler>>>,
        parent_run_id: Option<Uuid>,
        parent_run_manager: AsyncCallbackManagerForChainRun,
        tags: Option<Vec<String>>,
        inheritable_tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, serde_json::Value>>,
        inheritable_metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        let mut inner_sync = CallbackManager::new();
        inner_sync.handlers = handlers;
        inner_sync.inheritable_handlers = inheritable_handlers.unwrap_or_default();
        inner_sync.parent_run_id = parent_run_id;
        inner_sync.tags = tags.unwrap_or_default();
        inner_sync.inheritable_tags = inheritable_tags.unwrap_or_default();
        inner_sync.metadata = metadata.unwrap_or_default();
        inner_sync.inheritable_metadata = inheritable_metadata.unwrap_or_default();

        Self {
            inner: AsyncCallbackManager::from_callback_manager(inner_sync),
            parent_run_manager,
            ended: false,
        }
    }

    pub fn handlers(&self) -> &[Arc<dyn BaseCallbackHandler>] {
        self.inner.handlers()
    }

    pub fn parent_run_id(&self) -> Option<Uuid> {
        self.inner.parent_run_id()
    }

    pub fn copy(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            parent_run_manager: self.parent_run_manager.clone(),
            ended: self.ended,
        }
    }

    pub fn merge(&self, other: &CallbackManager) -> Self {
        let mut inner_sync = self.inner.inner.clone();

        for tag in &other.tags {
            if !inner_sync.tags.contains(tag) {
                inner_sync.tags.push(tag.clone());
            }
        }
        for tag in &other.inheritable_tags {
            if !inner_sync.inheritable_tags.contains(tag) {
                inner_sync.inheritable_tags.push(tag.clone());
            }
        }

        inner_sync.metadata.extend(other.metadata.clone());

        for handler in &other.handlers {
            inner_sync.add_handler(handler.clone(), false);
        }

        Self {
            inner: AsyncCallbackManager::from_callback_manager(inner_sync),
            parent_run_manager: self.parent_run_manager.clone(),
            ended: self.ended,
        }
    }

    pub fn set_handlers(&mut self, handlers: Vec<Arc<dyn BaseCallbackHandler>>, inherit: bool) {
        self.inner.set_handlers(handlers, inherit);
    }

    pub fn add_handler(&mut self, handler: Arc<dyn BaseCallbackHandler>, inherit: bool) {
        self.inner.add_handler(handler, inherit);
    }

    pub fn add_tags(&mut self, tags: Vec<String>, inherit: bool) {
        self.inner.add_tags(tags, inherit);
    }

    pub fn add_metadata(&mut self, metadata: HashMap<String, serde_json::Value>, inherit: bool) {
        self.inner.add_metadata(metadata, inherit);
    }

    pub async fn on_chain_end(&mut self, outputs: &HashMap<String, serde_json::Value>) {
        self.ended = true;
        self.parent_run_manager.on_chain_end(outputs).await;
    }

    pub async fn on_chain_error(&mut self, error: &dyn std::error::Error) {
        self.ended = true;
        self.parent_run_manager.on_chain_error(error).await;
    }

    pub async fn on_llm_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        prompts: &[String],
        run_id: Option<Uuid>,
    ) -> Vec<AsyncCallbackManagerForLLMRun> {
        self.inner.on_llm_start(serialized, prompts, run_id).await
    }

    pub async fn on_chat_model_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        messages: &[Vec<BaseMessage>],
        run_id: Option<Uuid>,
    ) -> Vec<AsyncCallbackManagerForLLMRun> {
        self.inner
            .on_chat_model_start(serialized, messages, run_id)
            .await
    }

    pub async fn on_chain_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        inputs: &HashMap<String, serde_json::Value>,
        run_id: Option<Uuid>,
        name: Option<&str>,
    ) -> AsyncCallbackManagerForChainRun {
        self.inner
            .on_chain_start(serialized, inputs, run_id, name)
            .await
    }

    pub async fn on_tool_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        input_str: &str,
        run_id: Option<Uuid>,
        inputs: Option<&HashMap<String, serde_json::Value>>,
    ) -> AsyncCallbackManagerForToolRun {
        self.inner
            .on_tool_start(serialized, input_str, run_id, inputs)
            .await
    }

    pub async fn on_retriever_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        query: &str,
        run_id: Option<Uuid>,
        name: Option<&str>,
    ) -> AsyncCallbackManagerForRetrieverRun {
        self.inner
            .on_retriever_start()
            .serialized(serialized)
            .query(query)
            .maybe_run_id(run_id)
            .maybe_name(name)
            .call()
            .await
    }
}

pub fn trace_as_chain_group<F, R>(
    group_name: &str,
    callback_manager: Option<CallbackManager>,
    inputs: Option<HashMap<String, serde_json::Value>>,
    tags: Option<Vec<String>>,
    metadata: Option<HashMap<String, serde_json::Value>>,
    run_id: Option<Uuid>,
    f: F,
) -> R
where
    F: FnOnce(&mut CallbackManagerForChainGroup) -> R,
{
    let cm = callback_manager.unwrap_or_else(|| {
        CallbackManager::configure(
            None,
            None,
            false,
            tags.clone(),
            None,
            metadata.clone(),
            None,
        )
    });

    let mut serialized = HashMap::new();
    serialized.insert(
        "name".to_string(),
        serde_json::Value::String(group_name.to_string()),
    );

    let run_manager = cm
        .on_chain_start()
        .serialized(&serialized)
        .inputs(&inputs.clone().unwrap_or_default())
        .maybe_run_id(run_id)
        .name(group_name)
        .call();
    let child_cm = run_manager.get_child(None);

    let mut group_cm = CallbackManagerForChainGroup::new(
        child_cm.handlers.clone(),
        Some(child_cm.inheritable_handlers.clone()),
        child_cm.parent_run_id,
        run_manager.clone(),
        Some(child_cm.tags.clone()),
        Some(child_cm.inheritable_tags.clone()),
        Some(child_cm.metadata.clone()),
        Some(child_cm.inheritable_metadata.clone()),
    );

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&mut group_cm)));

    match result {
        Ok(r) => {
            if !group_cm.ended {
                run_manager.on_chain_end(&HashMap::new());
            }
            r
        }
        Err(e) => {
            if !group_cm.ended {
                run_manager.on_chain_error(&ChainGroupPanicError);
            }
            std::panic::resume_unwind(e)
        }
    }
}

#[derive(Debug)]
struct ChainGroupPanicError;

impl std::fmt::Display for ChainGroupPanicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Chain group panicked")
    }
}

impl std::error::Error for ChainGroupPanicError {}

pub fn dispatch_custom_event(
    name: &str,
    data: &serde_json::Value,
    callback_manager: &CallbackManager,
) -> Result<(), &'static str> {
    if callback_manager.handlers.is_empty() {
        return Ok(());
    }

    let parent_run_id = callback_manager
        .parent_run_id
        .ok_or("Unable to dispatch an adhoc event without a parent run id.")?;

    let run_id = parent_run_id;

    handle_event(
        &callback_manager.handlers,
        Some(|h: &dyn BaseCallbackHandler| h.ignore_custom_event()),
        |handler| {
            handler.on_custom_event(name, data, run_id, None, None);
        },
    );

    Ok(())
}

pub async fn atrace_as_chain_group<F, Fut, R>(
    group_name: &str,
    callback_manager: Option<AsyncCallbackManager>,
    inputs: Option<HashMap<String, serde_json::Value>>,
    tags: Option<Vec<String>>,
    metadata: Option<HashMap<String, serde_json::Value>>,
    run_id: Option<Uuid>,
    f: F,
) -> R
where
    F: FnOnce(AsyncCallbackManagerForChainGroup) -> Fut,
    Fut: Future<Output = R>,
{
    let cm = callback_manager.unwrap_or_else(|| {
        AsyncCallbackManager::configure(
            None,
            None,
            false,
            tags.clone(),
            None,
            metadata.clone(),
            None,
        )
    });

    let mut serialized = HashMap::new();
    serialized.insert(
        "name".to_string(),
        serde_json::Value::String(group_name.to_string()),
    );

    let run_manager = cm
        .on_chain_start(
            &serialized,
            &inputs.clone().unwrap_or_default(),
            run_id,
            Some(group_name),
        )
        .await;
    let child_cm = run_manager.get_child(None);

    let group_cm = AsyncCallbackManagerForChainGroup::new(
        child_cm.handlers().to_vec(),
        Some(child_cm.inner.inheritable_handlers.clone()),
        child_cm.parent_run_id(),
        run_manager.clone(),
        Some(child_cm.inner.tags.clone()),
        Some(child_cm.inner.inheritable_tags.clone()),
        Some(child_cm.inner.metadata.clone()),
        Some(child_cm.inner.inheritable_metadata.clone()),
    );

    let result = f(group_cm.clone()).await;

    if !group_cm.ended {
        run_manager.on_chain_end(&HashMap::new()).await;
    }

    result
}

pub async fn adispatch_custom_event(
    name: &str,
    data: &serde_json::Value,
    callback_manager: &AsyncCallbackManager,
) -> Result<(), &'static str> {
    if callback_manager.handlers().is_empty() {
        return Ok(());
    }

    let parent_run_id = callback_manager
        .parent_run_id()
        .ok_or("Unable to dispatch an adhoc event without a parent run id.")?;

    let run_id = parent_run_id;

    handle_event(
        callback_manager.handlers(),
        Some(|h: &dyn BaseCallbackHandler| h.ignore_custom_event()),
        |handler| {
            handler.on_custom_event(name, data, run_id, None, None);
        },
    );

    Ok(())
}
