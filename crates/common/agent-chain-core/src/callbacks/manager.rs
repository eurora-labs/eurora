use std::collections::HashMap;
use std::future::Future;
use std::ops::Deref;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use bon::bon;
use uuid::Uuid;

use crate::messages::BaseMessage;
use crate::outputs::ChatResult;

use super::base::BaseCallbackHandler;
use super::stdout::StdOutCallbackHandler;
use crate::globals::get_debug;
use crate::tracers::context::{
    get_configure_hooks, get_tracer_project, get_tracing_callback, tracing_v2_is_enabled,
};
use crate::tracers::stdout::ConsoleCallbackHandler;
use crate::utils::env::env_var_is_set;
use crate::utils::uuid::uuid7;

// ---------------------------------------------------------------------------
// Event dispatch
// ---------------------------------------------------------------------------

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
        .map(&event_fn)
        .collect();

    futures::future::join_all(non_inline_futures).await;
}

// ---------------------------------------------------------------------------
// RunManagerCore — shared data for all run manager types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RunManagerCore {
    pub run_id: Uuid,
    pub handlers: Vec<Arc<dyn BaseCallbackHandler>>,
    pub inheritable_handlers: Vec<Arc<dyn BaseCallbackHandler>>,
    pub parent_run_id: Option<Uuid>,
    pub tags: Vec<String>,
    pub inheritable_tags: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub inheritable_metadata: HashMap<String, serde_json::Value>,
}

impl Default for RunManagerCore {
    fn default() -> Self {
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

#[bon]
impl RunManagerCore {
    #[builder]
    pub fn new(
        run_id: Uuid,
        #[builder(default)] handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        #[builder(default)] inheritable_handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        parent_run_id: Option<Uuid>,
        #[builder(default)] tags: Vec<String>,
        #[builder(default)] inheritable_tags: Vec<String>,
        #[builder(default)] metadata: HashMap<String, serde_json::Value>,
        #[builder(default)] inheritable_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            run_id,
            handlers,
            inheritable_handlers,
            parent_run_id,
            tags,
            inheritable_tags,
            metadata,
            inheritable_metadata,
        }
    }

    pub fn from_handlers(
        run_id: Uuid,
        handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        inheritable_handlers: Vec<Arc<dyn BaseCallbackHandler>>,
        parent_run_id: Option<Uuid>,
    ) -> Self {
        Self::builder()
            .run_id(run_id)
            .handlers(handlers)
            .inheritable_handlers(inheritable_handlers)
            .maybe_parent_run_id(parent_run_id)
            .build()
    }

    pub fn noop() -> Self {
        Self::default()
    }

    pub fn run_id(&self) -> Uuid {
        self.run_id
    }

    pub fn parent_run_id(&self) -> Option<Uuid> {
        self.parent_run_id
    }

    pub fn handlers(&self) -> &[Arc<dyn BaseCallbackHandler>] {
        &self.handlers
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    fn dispatch(
        &self,
        ignore: Option<fn(&dyn BaseCallbackHandler) -> bool>,
        f: impl FnMut(&Arc<dyn BaseCallbackHandler>),
    ) {
        if !self.handlers.is_empty() {
            handle_event(&self.handlers, ignore, f);
        }
    }

    pub fn get_child_manager(&self, tag: Option<&str>) -> CallbackManager {
        let mut manager = CallbackManager::new();
        manager.parent_run_id = Some(self.run_id);
        manager.set_handlers(self.inheritable_handlers.clone(), true);
        manager.add_tags(self.inheritable_tags.clone(), true);
        manager.add_metadata(self.inheritable_metadata.clone(), true);
        if let Some(tag) = tag {
            manager.add_tags(vec![tag.to_string()], false);
        }
        manager
    }
}

pub type BaseRunManager = RunManagerCore;

// ---------------------------------------------------------------------------
// RunManager — adds on_text / on_retry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RunManager {
    core: RunManagerCore,
}

impl Deref for RunManager {
    type Target = RunManagerCore;
    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

impl RunManager {
    pub fn new(core: RunManagerCore) -> Self {
        Self { core }
    }

    pub fn noop() -> Self {
        Self {
            core: RunManagerCore::noop(),
        }
    }

    #[doc(hidden)]
    pub fn get_noop_manager() -> Self {
        Self::noop()
    }

    pub fn on_text(&self, text: &str) {
        let run_id = self.core.run_id;
        let parent_run_id = self.core.parent_run_id;
        self.core.dispatch(None, |handler| {
            handler.on_text(text, run_id, parent_run_id, None, "");
        });
    }

    pub fn on_retry(&self, retry_state: &serde_json::Value) {
        let run_id = self.core.run_id;
        let parent_run_id = self.core.parent_run_id;
        self.core.dispatch(
            Some(|h: &dyn BaseCallbackHandler| h.ignore_retry()),
            |handler| {
                handler.on_retry(retry_state, run_id, parent_run_id);
            },
        );
    }
}

// ---------------------------------------------------------------------------
// AsyncRunManager
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AsyncRunManager {
    core: RunManagerCore,
}

impl Deref for AsyncRunManager {
    type Target = RunManagerCore;
    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

impl AsyncRunManager {
    pub fn new(core: RunManagerCore) -> Self {
        Self { core }
    }

    pub fn noop() -> Self {
        Self {
            core: RunManagerCore::noop(),
        }
    }

    #[doc(hidden)]
    pub fn get_noop_manager() -> Self {
        Self::noop()
    }

    pub fn get_sync(&self) -> RunManager {
        RunManager::new(self.core.clone())
    }

    pub async fn on_text(&self, text: &str) {
        if self.core.handlers.is_empty() {
            return;
        }
        let run_id = self.core.run_id;
        let parent_run_id = self.core.parent_run_id;
        ahandle_event(&self.core.handlers, None, |handler| {
            handler.on_text(text, run_id, parent_run_id, None, "");
            async {}
        })
        .await;
    }

    pub async fn on_retry(&self, retry_state: &serde_json::Value) {
        if self.core.handlers.is_empty() {
            return;
        }
        let run_id = self.core.run_id;
        let parent_run_id = self.core.parent_run_id;
        ahandle_event(
            &self.core.handlers,
            Some(|h: &dyn BaseCallbackHandler| h.ignore_retry()),
            |handler| {
                handler.on_retry(retry_state, run_id, parent_run_id);
                async {}
            },
        )
        .await;
    }
}

// ---------------------------------------------------------------------------
// ParentRunManager — adds get_child
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ParentRunManager {
    core: RunManagerCore,
}

impl Deref for ParentRunManager {
    type Target = RunManagerCore;
    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

impl ParentRunManager {
    pub fn new(core: RunManagerCore) -> Self {
        Self { core }
    }

    pub fn noop() -> Self {
        Self {
            core: RunManagerCore::noop(),
        }
    }

    #[doc(hidden)]
    pub fn get_noop_manager() -> Self {
        Self::noop()
    }

    pub fn get_child(&self, tag: Option<&str>) -> CallbackManager {
        self.core.get_child_manager(tag)
    }
}

// ---------------------------------------------------------------------------
// AsyncParentRunManager
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AsyncParentRunManager {
    core: RunManagerCore,
}

impl Deref for AsyncParentRunManager {
    type Target = RunManagerCore;
    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

impl AsyncParentRunManager {
    pub fn new(core: RunManagerCore) -> Self {
        Self { core }
    }

    pub fn noop() -> Self {
        Self {
            core: RunManagerCore::noop(),
        }
    }

    #[doc(hidden)]
    pub fn get_noop_manager() -> Self {
        Self::noop()
    }

    pub fn get_child(&self, tag: Option<&str>) -> AsyncCallbackManager {
        AsyncCallbackManager::from_callback_manager(self.core.get_child_manager(tag))
    }

    pub fn get_sync(&self) -> ParentRunManager {
        ParentRunManager::new(self.core.clone())
    }
}

// ---------------------------------------------------------------------------
// CallbackManagerForLLMRun
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CallbackManagerForLLMRun {
    core: RunManagerCore,
}

impl Deref for CallbackManagerForLLMRun {
    type Target = RunManagerCore;
    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

impl CallbackManagerForLLMRun {
    pub fn new(core: RunManagerCore) -> Self {
        Self { core }
    }

    pub fn noop() -> Self {
        Self {
            core: RunManagerCore::noop(),
        }
    }

    #[doc(hidden)]
    pub fn get_noop_manager() -> Self {
        Self::noop()
    }

    pub fn on_llm_new_token(&self, token: &str, chunk: Option<&serde_json::Value>) {
        let run_id = self.core.run_id;
        let parent_run_id = self.core.parent_run_id;
        self.core.dispatch(
            Some(|h: &dyn BaseCallbackHandler| h.ignore_llm()),
            |handler| {
                handler.on_llm_new_token(token, run_id, parent_run_id, chunk);
            },
        );
    }

    pub fn on_llm_end(&self, response: &ChatResult) {
        let run_id = self.core.run_id;
        let parent_run_id = self.core.parent_run_id;
        self.core.dispatch(
            Some(|h: &dyn BaseCallbackHandler| h.ignore_llm()),
            |handler| {
                handler.on_llm_end(response, run_id, parent_run_id);
            },
        );
    }

    pub fn on_llm_error(&self, error: &dyn std::error::Error) {
        let run_id = self.core.run_id;
        let parent_run_id = self.core.parent_run_id;
        self.core.dispatch(
            Some(|h: &dyn BaseCallbackHandler| h.ignore_llm()),
            |handler| {
                handler.on_llm_error(error, run_id, parent_run_id);
            },
        );
    }
}

// ---------------------------------------------------------------------------
// CallbackManagerForChainRun
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CallbackManagerForChainRun {
    core: RunManagerCore,
}

impl Deref for CallbackManagerForChainRun {
    type Target = RunManagerCore;
    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

impl CallbackManagerForChainRun {
    pub fn new(core: RunManagerCore) -> Self {
        Self { core }
    }

    pub fn noop() -> Self {
        Self {
            core: RunManagerCore::noop(),
        }
    }

    #[doc(hidden)]
    pub fn get_noop_manager() -> Self {
        Self::noop()
    }

    pub fn get_child(&self, tag: Option<&str>) -> CallbackManager {
        self.core.get_child_manager(tag)
    }

    pub fn on_chain_end(&self, outputs: &HashMap<String, serde_json::Value>) {
        let run_id = self.core.run_id;
        let parent_run_id = self.core.parent_run_id;
        self.core.dispatch(
            Some(|h: &dyn BaseCallbackHandler| h.ignore_chain()),
            |handler| {
                handler.on_chain_end(outputs, run_id, parent_run_id);
            },
        );
    }

    pub fn on_chain_error(&self, error: &dyn std::error::Error) {
        let run_id = self.core.run_id;
        let parent_run_id = self.core.parent_run_id;
        self.core.dispatch(
            Some(|h: &dyn BaseCallbackHandler| h.ignore_chain()),
            |handler| {
                handler.on_chain_error(error, run_id, parent_run_id);
            },
        );
    }

    pub fn on_agent_action(&self, action: &serde_json::Value) {
        let run_id = self.core.run_id;
        let parent_run_id = self.core.parent_run_id;
        self.core.dispatch(
            Some(|h: &dyn BaseCallbackHandler| h.ignore_agent()),
            |handler| {
                handler.on_agent_action(action, run_id, parent_run_id, None);
            },
        );
    }

    pub fn on_agent_finish(&self, finish: &serde_json::Value) {
        let run_id = self.core.run_id;
        let parent_run_id = self.core.parent_run_id;
        self.core.dispatch(
            Some(|h: &dyn BaseCallbackHandler| h.ignore_agent()),
            |handler| {
                handler.on_agent_finish(finish, run_id, parent_run_id, None);
            },
        );
    }
}

// ---------------------------------------------------------------------------
// CallbackManagerForToolRun
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CallbackManagerForToolRun {
    core: RunManagerCore,
}

impl Deref for CallbackManagerForToolRun {
    type Target = RunManagerCore;
    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

impl CallbackManagerForToolRun {
    pub fn new(core: RunManagerCore) -> Self {
        Self { core }
    }

    pub fn noop() -> Self {
        Self {
            core: RunManagerCore::noop(),
        }
    }

    #[doc(hidden)]
    pub fn get_noop_manager() -> Self {
        Self::noop()
    }

    pub fn get_child(&self, tag: Option<&str>) -> CallbackManager {
        self.core.get_child_manager(tag)
    }

    pub fn on_tool_end(&self, output: &str) {
        let run_id = self.core.run_id;
        let parent_run_id = self.core.parent_run_id;
        self.core.dispatch(
            Some(|h: &dyn BaseCallbackHandler| h.ignore_tool()),
            |handler| {
                handler.on_tool_end(output, run_id, parent_run_id, None, None, None);
            },
        );
    }

    pub fn on_tool_error(&self, error: &dyn std::error::Error) {
        let run_id = self.core.run_id;
        let parent_run_id = self.core.parent_run_id;
        self.core.dispatch(
            Some(|h: &dyn BaseCallbackHandler| h.ignore_tool()),
            |handler| {
                handler.on_tool_error(error, run_id, parent_run_id);
            },
        );
    }
}

// ---------------------------------------------------------------------------
// CallbackManagerForRetrieverRun
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CallbackManagerForRetrieverRun {
    core: RunManagerCore,
}

impl Deref for CallbackManagerForRetrieverRun {
    type Target = RunManagerCore;
    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

impl CallbackManagerForRetrieverRun {
    pub fn new(core: RunManagerCore) -> Self {
        Self { core }
    }

    pub fn noop() -> Self {
        Self {
            core: RunManagerCore::noop(),
        }
    }

    #[doc(hidden)]
    pub fn get_noop_manager() -> Self {
        Self::noop()
    }

    pub fn get_child(&self, tag: Option<&str>) -> CallbackManager {
        self.core.get_child_manager(tag)
    }

    pub fn on_retriever_end(&self, documents: &[serde_json::Value]) {
        let run_id = self.core.run_id;
        let parent_run_id = self.core.parent_run_id;
        self.core.dispatch(
            Some(|h: &dyn BaseCallbackHandler| h.ignore_retriever()),
            |handler| {
                handler.on_retriever_end(documents, run_id, parent_run_id);
            },
        );
    }

    pub fn on_retriever_error(&self, error: &dyn std::error::Error) {
        let run_id = self.core.run_id;
        let parent_run_id = self.core.parent_run_id;
        self.core.dispatch(
            Some(|h: &dyn BaseCallbackHandler| h.ignore_retriever()),
            |handler| {
                handler.on_retriever_error(error, run_id, parent_run_id);
            },
        );
    }
}

// ---------------------------------------------------------------------------
// CallbackManager — top-level configuration type
// ---------------------------------------------------------------------------

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

    fn make_run_core(&self, run_id: Uuid) -> RunManagerCore {
        RunManagerCore::builder()
            .run_id(run_id)
            .handlers(self.handlers.clone())
            .inheritable_handlers(self.inheritable_handlers.clone())
            .maybe_parent_run_id(self.parent_run_id)
            .tags(self.tags.clone())
            .inheritable_tags(self.inheritable_tags.clone())
            .metadata(self.metadata.clone())
            .inheritable_metadata(self.inheritable_metadata.clone())
            .build()
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
        for tag in tags {
            if !self.tags.contains(&tag) {
                self.tags.push(tag.clone());
            }
            if inherit && !self.inheritable_tags.contains(&tag) {
                self.inheritable_tags.push(tag);
            }
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

            managers.push(CallbackManagerForLLMRun::new(self.make_run_core(run_id)));
        }

        managers
    }

    pub fn on_chat_model_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        messages: &[Vec<BaseMessage>],
        run_id: Option<Uuid>,
        name: Option<&str>,
    ) -> Vec<CallbackManagerForLLMRun> {
        let mut managers = Vec::new();
        let mut current_run_id = run_id;

        for message_list in messages {
            let run_id = current_run_id.unwrap_or_else(|| uuid7(None));
            current_run_id = None;

            let individual_messages = std::slice::from_ref(message_list);
            handle_event(
                &self.handlers,
                Some(|h: &dyn BaseCallbackHandler| h.ignore_chat_model()),
                |handler| {
                    handler.on_chat_model_start(
                        serialized,
                        individual_messages,
                        run_id,
                        self.parent_run_id,
                        Some(&self.tags),
                        Some(&self.metadata),
                        name,
                    );
                },
            );

            managers.push(CallbackManagerForLLMRun::new(self.make_run_core(run_id)));
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
            Some(|h: &dyn BaseCallbackHandler| h.ignore_tool()),
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

        CallbackManagerForToolRun::new(self.make_run_core(run_id))
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
}

// ---------------------------------------------------------------------------
// Callbacks enum
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Callbacks {
    Handlers(Vec<Arc<dyn BaseCallbackHandler>>),
    Manager(CallbackManager),
}

impl Callbacks {
    pub fn from_handlers(handlers: Vec<Arc<dyn BaseCallbackHandler>>) -> Self {
        Callbacks::Handlers(handlers)
    }

    pub fn from_manager(manager: CallbackManager) -> Self {
        Callbacks::Manager(manager)
    }

    pub fn into_manager(self) -> CallbackManager {
        match self {
            Callbacks::Handlers(handlers) => {
                let mut manager = CallbackManager::new();
                manager.inheritable_handlers = handlers.clone();
                manager.handlers = handlers;
                manager
            }
            Callbacks::Manager(manager) => manager,
        }
    }
}

impl From<Vec<Arc<dyn BaseCallbackHandler>>> for Callbacks {
    fn from(handlers: Vec<Arc<dyn BaseCallbackHandler>>) -> Self {
        Callbacks::Handlers(handlers)
    }
}

impl From<CallbackManager> for Callbacks {
    fn from(manager: CallbackManager) -> Self {
        Callbacks::Manager(manager)
    }
}

// ---------------------------------------------------------------------------
// configure_impl
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn configure_impl(
    inheritable_callbacks: Option<Callbacks>,
    local_callbacks: Option<Callbacks>,
    verbose: bool,
    inheritable_tags: Option<Vec<String>>,
    local_tags: Option<Vec<String>>,
    inheritable_metadata: Option<HashMap<String, serde_json::Value>>,
    local_metadata: Option<HashMap<String, serde_json::Value>>,
) -> CallbackManager {
    let mut callback_manager = CallbackManager::new();

    if let Some(callbacks) = inheritable_callbacks {
        match callbacks {
            Callbacks::Handlers(handlers) => {
                callback_manager.handlers = handlers.clone();
                callback_manager.inheritable_handlers = handlers;
            }
            Callbacks::Manager(manager) => {
                callback_manager.parent_run_id = manager.parent_run_id;
                callback_manager.handlers = manager.handlers.clone();
                callback_manager.inheritable_handlers = manager.inheritable_handlers.clone();
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

    let v1_tracing_enabled =
        env_var_is_set("LANGCHAIN_TRACING") || env_var_is_set("LANGCHAIN_HANDLER");
    let tracing_v2_enabled = tracing_v2_is_enabled();

    if v1_tracing_enabled && !tracing_v2_enabled {
        tracing::warn!(
            "Tracing using LangChainTracerV1 is no longer supported. \
             Please set the LANGCHAIN_TRACING_V2 environment variable to enable \
             tracing instead."
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
                    "Tracing is enabled but LangChainTracer is not yet \
                     implemented in Rust. Tracing callbacks will not be sent."
                );
            } else {
                let tracer_project = get_tracer_project();
                tracing::debug!(
                    "Tracing is enabled (project: {}) but LangChainTracer is not yet \
                     implemented in Rust. Tracing callbacks will not be sent.",
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

// ---------------------------------------------------------------------------
// CallbackManager builder methods (bon)
// ---------------------------------------------------------------------------

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

        CallbackManagerForChainRun::new(self.make_run_core(run_id))
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

        CallbackManagerForRetrieverRun::new(self.make_run_core(run_id))
    }

    #[builder]
    pub fn configure(
        inheritable_callbacks: Option<Callbacks>,
        local_callbacks: Option<Callbacks>,
        #[builder(default)] verbose: bool,
        inheritable_tags: Option<Vec<String>>,
        local_tags: Option<Vec<String>>,
        inheritable_metadata: Option<HashMap<String, serde_json::Value>>,
        local_metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        configure_impl(
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

// ---------------------------------------------------------------------------
// AsyncCallbackManager
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct AsyncCallbackManager {
    pub(crate) inner: CallbackManager,
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

    #[builder]
    pub fn configure(
        inheritable_callbacks: Option<Callbacks>,
        local_callbacks: Option<Callbacks>,
        #[builder(default)] verbose: bool,
        inheritable_tags: Option<Vec<String>>,
        local_tags: Option<Vec<String>>,
        inheritable_metadata: Option<HashMap<String, serde_json::Value>>,
        local_metadata: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self {
            inner: configure_impl(
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
        name: Option<&str>,
    ) -> Vec<AsyncCallbackManagerForLLMRun> {
        self.inner
            .on_chat_model_start(serialized, messages, run_id, name)
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
}

// ---------------------------------------------------------------------------
// Async run manager types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AsyncCallbackManagerForLLMRun {
    inner: CallbackManagerForLLMRun,
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

    pub fn tags(&self) -> &[String] {
        self.inner.tags()
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

    pub fn noop() -> Self {
        Self {
            inner: CallbackManagerForLLMRun::noop(),
        }
    }

    #[doc(hidden)]
    pub fn get_noop_manager() -> Self {
        Self::noop()
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

    pub fn noop() -> Self {
        Self {
            inner: CallbackManagerForChainRun::noop(),
        }
    }

    #[doc(hidden)]
    pub fn get_noop_manager() -> Self {
        Self::noop()
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

    pub fn noop() -> Self {
        Self {
            inner: CallbackManagerForToolRun::noop(),
        }
    }

    #[doc(hidden)]
    pub fn get_noop_manager() -> Self {
        Self::noop()
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

    pub fn noop() -> Self {
        Self {
            inner: CallbackManagerForRetrieverRun::noop(),
        }
    }

    #[doc(hidden)]
    pub fn get_noop_manager() -> Self {
        Self::noop()
    }
}

// ---------------------------------------------------------------------------
// Chain group types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CallbackManagerForChainGroup {
    inner: CallbackManager,
    parent_run_manager: CallbackManagerForChainRun,
    pub ended: bool,
}

#[bon]
impl CallbackManagerForChainGroup {
    #[builder]
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
        let inner = CallbackManager {
            handlers,
            inheritable_handlers: inheritable_handlers.unwrap_or_default(),
            parent_run_id,
            tags: tags.unwrap_or_default(),
            inheritable_tags: inheritable_tags.unwrap_or_default(),
            metadata: metadata.unwrap_or_default(),
            inheritable_metadata: inheritable_metadata.unwrap_or_default(),
        };

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
        name: Option<&str>,
    ) -> Vec<CallbackManagerForLLMRun> {
        self.inner
            .on_chat_model_start(serialized, messages, run_id, name)
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

#[bon]
impl AsyncCallbackManagerForChainGroup {
    #[builder]
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
        let inner_sync = CallbackManager {
            handlers,
            inheritable_handlers: inheritable_handlers.unwrap_or_default(),
            parent_run_id,
            tags: tags.unwrap_or_default(),
            inheritable_tags: inheritable_tags.unwrap_or_default(),
            metadata: metadata.unwrap_or_default(),
            inheritable_metadata: inheritable_metadata.unwrap_or_default(),
        };

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
        name: Option<&str>,
    ) -> Vec<AsyncCallbackManagerForLLMRun> {
        self.inner
            .on_chat_model_start(serialized, messages, run_id, name)
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

// ---------------------------------------------------------------------------
// Free functions
// ---------------------------------------------------------------------------

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
        CallbackManager::configure()
            .maybe_inheritable_tags(tags.clone())
            .maybe_inheritable_metadata(metadata.clone())
            .call()
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

    let mut group_cm = CallbackManagerForChainGroup::builder()
        .handlers(child_cm.handlers.clone())
        .inheritable_handlers(child_cm.inheritable_handlers.clone())
        .maybe_parent_run_id(child_cm.parent_run_id)
        .parent_run_manager(run_manager.clone())
        .tags(child_cm.tags.clone())
        .inheritable_tags(child_cm.inheritable_tags.clone())
        .metadata(child_cm.metadata.clone())
        .inheritable_metadata(child_cm.inheritable_metadata.clone())
        .build();

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
    F: FnOnce(&mut AsyncCallbackManagerForChainGroup) -> Fut,
    Fut: Future<Output = R>,
{
    let cm = callback_manager.unwrap_or_else(|| {
        AsyncCallbackManager::configure()
            .maybe_inheritable_tags(tags.clone())
            .maybe_inheritable_metadata(metadata.clone())
            .call()
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

    let mut group_cm = AsyncCallbackManagerForChainGroup::builder()
        .handlers(child_cm.handlers().to_vec())
        .inheritable_handlers(child_cm.inner.inheritable_handlers.clone())
        .maybe_parent_run_id(child_cm.parent_run_id())
        .parent_run_manager(run_manager.clone())
        .tags(child_cm.inner.tags.clone())
        .inheritable_tags(child_cm.inner.inheritable_tags.clone())
        .metadata(child_cm.inner.metadata.clone())
        .inheritable_metadata(child_cm.inner.inheritable_metadata.clone())
        .build();

    let result = f(&mut group_cm).await;

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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
        let manager = CallbackManager::configure()
            .inheritable_tags(vec!["tag1".to_string()])
            .local_tags(vec!["tag2".to_string()])
            .call();

        assert!(manager.tags.contains(&"tag1".to_string()));
        assert!(manager.tags.contains(&"tag2".to_string()));
        assert!(manager.inheritable_tags.contains(&"tag1".to_string()));
        assert!(!manager.inheritable_tags.contains(&"tag2".to_string()));
    }

    #[test]
    fn test_configure_with_verbose() {
        crate::globals::set_debug(false);

        let manager = CallbackManager::configure().verbose(true).call();
        assert!(
            manager
                .handlers
                .iter()
                .any(|h| h.name() == "StdOutCallbackHandler"),
            "StdOutCallbackHandler should be added when verbose=true"
        );
    }

    #[test]
    fn test_configure_deduplication() {
        crate::globals::set_debug(false);

        let handler: Arc<dyn BaseCallbackHandler> = Arc::new(StdOutCallbackHandler::new());
        let callbacks = Callbacks::Handlers(vec![handler]);

        let manager = CallbackManager::configure()
            .inheritable_callbacks(callbacks)
            .verbose(true)
            .call();

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
