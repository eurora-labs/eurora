use std::collections::HashMap;
use std::future::Future;
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

pub(crate) fn handle_event(
    handlers: &[Arc<dyn BaseCallbackHandler>],
    ignore_condition: Option<fn(&dyn BaseCallbackHandler) -> bool>,
    mut event_fn: impl FnMut(&dyn BaseCallbackHandler),
) {
    for handler in handlers {
        if let Some(ignore_fn) = ignore_condition
            && ignore_fn(handler.as_ref())
        {
            continue;
        }
        let result = catch_unwind(AssertUnwindSafe(|| event_fn(handler.as_ref())));
        if let Err(payload) = result {
            let msg = panic_message(&*payload);
            tracing::warn!(
                target: "agent_chain_core::callbacks",
                "Error in {}.callback: {msg}",
                handler.name(),
            );
            if handler.raise_error() {
                std::panic::resume_unwind(payload);
            }
        }
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "Unknown error".to_string())
}

#[derive(Debug, Clone, Default)]
pub struct CallbackManager {
    handlers: Vec<Arc<dyn BaseCallbackHandler>>,
    inheritable_handlers: Vec<Arc<dyn BaseCallbackHandler>>,
    parent_run_id: Option<Uuid>,
    tags: Vec<String>,
    inheritable_tags: Vec<String>,
    metadata: HashMap<String, serde_json::Value>,
    inheritable_metadata: HashMap<String, serde_json::Value>,
}

impl CallbackManager {
    pub fn new() -> Self {
        Self::default()
    }

    fn make_run_core(&self, run_id: Uuid) -> RunManagerCore {
        RunManagerCore {
            run_id,
            config: self.clone(),
        }
    }

    fn make_child(&self, parent_run_id: Uuid) -> CallbackManager {
        CallbackManager {
            handlers: self.inheritable_handlers.clone(),
            inheritable_handlers: self.inheritable_handlers.clone(),
            parent_run_id: Some(parent_run_id),
            tags: self.inheritable_tags.clone(),
            inheritable_tags: self.inheritable_tags.clone(),
            metadata: self.inheritable_metadata.clone(),
            inheritable_metadata: self.inheritable_metadata.clone(),
        }
    }

    pub fn handlers(&self) -> &[Arc<dyn BaseCallbackHandler>] {
        &self.handlers
    }

    pub fn inheritable_handlers(&self) -> &[Arc<dyn BaseCallbackHandler>] {
        &self.inheritable_handlers
    }

    pub fn parent_run_id(&self) -> Option<Uuid> {
        self.parent_run_id
    }

    pub fn set_parent_run_id(&mut self, id: Option<Uuid>) {
        self.parent_run_id = id;
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    pub fn inheritable_tags(&self) -> &[String] {
        &self.inheritable_tags
    }

    pub fn metadata(&self) -> &HashMap<String, serde_json::Value> {
        &self.metadata
    }

    pub fn inheritable_metadata(&self) -> &HashMap<String, serde_json::Value> {
        &self.inheritable_metadata
    }

    pub fn add_handler(&mut self, handler: Arc<dyn BaseCallbackHandler>, inherit: bool) {
        if !self.handlers.iter().any(|h| Arc::ptr_eq(h, &handler)) {
            self.handlers.push(handler.clone());
        }
        if inherit
            && !self
                .inheritable_handlers
                .iter()
                .any(|h| Arc::ptr_eq(h, &handler))
        {
            self.inheritable_handlers.push(handler);
        }
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

    pub fn remove_handler(&mut self, handler: &Arc<dyn BaseCallbackHandler>) {
        self.handlers.retain(|h| !Arc::ptr_eq(h, handler));
        self.inheritable_handlers
            .retain(|h| !Arc::ptr_eq(h, handler));
    }

    pub fn add_tags(&mut self, tags: &[String], inherit: bool) {
        for tag in tags {
            if !self.tags.contains(tag) {
                self.tags.push(tag.clone());
            }
            if inherit && !self.inheritable_tags.contains(tag) {
                self.inheritable_tags.push(tag.clone());
            }
        }
    }

    pub fn remove_tags(&mut self, tags: &[String]) {
        for tag in tags {
            self.tags.retain(|t| t != tag);
            self.inheritable_tags.retain(|t| t != tag);
        }
    }

    pub fn add_metadata(&mut self, metadata: HashMap<String, serde_json::Value>, inherit: bool) {
        if inherit {
            self.inheritable_metadata.extend(metadata.clone());
        }
        self.metadata.extend(metadata);
    }

    pub fn remove_metadata(&mut self, keys: &[String]) {
        for key in keys {
            self.metadata.remove(key);
            self.inheritable_metadata.remove(key);
        }
    }

    pub fn on_llm_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        prompts: &[String],
        run_id: Option<Uuid>,
    ) -> Vec<CallbackManagerForLLMRun> {
        prompts
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let rid = if i == 0 {
                    run_id.unwrap_or_else(|| uuid7(None))
                } else {
                    uuid7(None)
                };
                handle_event(
                    &self.handlers,
                    Some(|h: &dyn BaseCallbackHandler| h.ignore_llm()),
                    |h| {
                        h.on_llm_start(
                            serialized,
                            prompts,
                            rid,
                            self.parent_run_id,
                            Some(&self.tags),
                            Some(&self.metadata),
                        );
                    },
                );
                CallbackManagerForLLMRun::new(self.make_run_core(rid))
            })
            .collect()
    }

    pub fn on_chat_model_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        messages: &[Vec<BaseMessage>],
        run_id: Option<Uuid>,
        name: Option<&str>,
    ) -> Vec<CallbackManagerForLLMRun> {
        let mut current_run_id = run_id;
        messages
            .iter()
            .map(|message_list| {
                let rid = current_run_id.take().unwrap_or_else(|| uuid7(None));
                let individual = std::slice::from_ref(message_list);
                handle_event(
                    &self.handlers,
                    Some(|h: &dyn BaseCallbackHandler| h.ignore_chat_model()),
                    |h| {
                        h.on_chat_model_start(
                            serialized,
                            individual,
                            rid,
                            self.parent_run_id,
                            Some(&self.tags),
                            Some(&self.metadata),
                            name,
                        );
                    },
                );
                CallbackManagerForLLMRun::new(self.make_run_core(rid))
            })
            .collect()
    }

    pub fn on_tool_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        input_str: &str,
        run_id: Option<Uuid>,
        inputs: Option<&HashMap<String, serde_json::Value>>,
    ) -> CallbackManagerForToolRun {
        let rid = run_id.unwrap_or_else(|| uuid7(None));
        handle_event(
            &self.handlers,
            Some(|h: &dyn BaseCallbackHandler| h.ignore_tool()),
            |h| {
                h.on_tool_start(
                    serialized,
                    input_str,
                    rid,
                    self.parent_run_id,
                    Some(&self.tags),
                    Some(&self.metadata),
                    inputs,
                );
            },
        );
        CallbackManagerForToolRun::new(self.make_run_core(rid))
    }

    pub fn on_custom_event(&self, name: &str, data: &serde_json::Value, run_id: Option<Uuid>) {
        if self.handlers.is_empty() {
            return;
        }
        let rid = run_id.unwrap_or_else(|| uuid7(None));
        handle_event(
            &self.handlers,
            Some(|h: &dyn BaseCallbackHandler| h.ignore_custom_event()),
            |h| h.on_custom_event(name, data, rid, None, None),
        );
    }

    pub fn merge(&self, other: &CallbackManager) -> Self {
        let mut merged = Self {
            parent_run_id: self.parent_run_id.or(other.parent_run_id),
            handlers: Vec::new(),
            inheritable_handlers: Vec::new(),
            tags: dedup_merge(&self.tags, &other.tags),
            inheritable_tags: dedup_merge(&self.inheritable_tags, &other.inheritable_tags),
            metadata: {
                let mut m = self.metadata.clone();
                m.extend(other.metadata.clone());
                m
            },
            inheritable_metadata: {
                let mut m = self.inheritable_metadata.clone();
                m.extend(other.inheritable_metadata.clone());
                m
            },
        };
        for handler in self.handlers.iter().chain(other.handlers.iter()) {
            merged.add_handler(handler.clone(), false);
        }
        for handler in self
            .inheritable_handlers
            .iter()
            .chain(other.inheritable_handlers.iter())
        {
            merged.add_handler(handler.clone(), true);
        }
        merged
    }
}

fn dedup_merge(a: &[String], b: &[String]) -> Vec<String> {
    let mut result = a.to_vec();
    for item in b {
        if !result.contains(item) {
            result.push(item.clone());
        }
    }
    result
}

#[derive(Debug, Clone)]
pub enum Callbacks {
    Handlers(Vec<Arc<dyn BaseCallbackHandler>>),
    Manager(CallbackManager),
}

impl Callbacks {
    pub fn into_manager(self) -> CallbackManager {
        CallbackManager::from(self)
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

impl From<Callbacks> for CallbackManager {
    fn from(callbacks: Callbacks) -> Self {
        match callbacks {
            Callbacks::Handlers(handlers) => CallbackManager {
                inheritable_handlers: handlers.clone(),
                handlers,
                ..Default::default()
            },
            Callbacks::Manager(manager) => manager,
        }
    }
}

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
    let mut manager = match inheritable_callbacks {
        Some(Callbacks::Manager(m)) => m,
        Some(Callbacks::Handlers(handlers)) => CallbackManager {
            inheritable_handlers: handlers.clone(),
            handlers,
            ..Default::default()
        },
        None => CallbackManager::new(),
    };

    if let Some(callbacks) = local_callbacks {
        let local_handlers = match callbacks {
            Callbacks::Handlers(h) => h,
            Callbacks::Manager(m) => m.handlers,
        };
        for handler in local_handlers {
            manager.add_handler(handler, false);
        }
    }

    if let Some(ref tags) = inheritable_tags {
        manager.add_tags(tags, true);
    }
    if let Some(ref tags) = local_tags {
        manager.add_tags(tags, false);
    }
    if let Some(metadata) = inheritable_metadata {
        manager.add_metadata(metadata, true);
    }
    if let Some(metadata) = local_metadata {
        manager.add_metadata(metadata, false);
    }

    let v1 = env_var_is_set("LANGCHAIN_TRACING") || env_var_is_set("LANGCHAIN_HANDLER");
    let v2 = tracing_v2_is_enabled();

    if v1 && !v2 {
        tracing::warn!(
            "Tracing using LangChainTracerV1 is no longer supported. \
             Set LANGCHAIN_TRACING_V2 to enable tracing."
        );
    }

    let debug = get_debug();

    if verbose
        && !debug
        && !manager
            .handlers
            .iter()
            .any(|h| h.name() == "StdOutCallbackHandler")
    {
        manager.add_handler(Arc::new(StdOutCallbackHandler::new()), false);
    }

    if debug
        && !manager
            .handlers
            .iter()
            .any(|h| h.name() == "ConsoleCallbackHandler")
    {
        manager.add_handler(Arc::new(ConsoleCallbackHandler::new()), true);
    }

    if v2
        && !manager
            .handlers
            .iter()
            .any(|h| h.name() == "LangChainTracer")
    {
        if get_tracing_callback().is_some() {
            tracing::debug!("LangChainTracer not yet implemented in Rust.");
        } else {
            let project = get_tracer_project();
            tracing::debug!(
                "Tracing enabled (project: {project}) but LangChainTracer not yet implemented."
            );
        }
    }

    if let Ok(registry) = get_configure_hooks().lock() {
        for hook in registry.hooks() {
            let from_env = hook.env_var.as_ref().is_some_and(|var| env_var_is_set(var))
                && hook.handler_factory.is_some();
            let context_handler = (hook.context_getter)();

            if context_handler.is_some() || from_env {
                let handler =
                    context_handler.unwrap_or_else(|| (hook.handler_factory.as_ref().unwrap())());
                let already = hook
                    .handler_type_name
                    .as_ref()
                    .is_some_and(|name| manager.handlers.iter().any(|h| h.name() == name));
                if !already {
                    manager.add_handler(handler, hook.inheritable);
                }
            }
        }
    }

    manager
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
        let rid = run_id.unwrap_or_else(|| uuid7(None));
        handle_event(
            &self.handlers,
            Some(|h: &dyn BaseCallbackHandler| h.ignore_chain()),
            |h| {
                h.on_chain_start(
                    serialized,
                    inputs,
                    rid,
                    self.parent_run_id,
                    Some(&self.tags),
                    Some(&self.metadata),
                    name,
                );
            },
        );
        CallbackManagerForChainRun::new(self.make_run_core(rid))
    }

    #[builder]
    pub fn on_retriever_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        query: &str,
        run_id: Option<Uuid>,
        name: Option<&str>,
    ) -> CallbackManagerForRetrieverRun {
        let rid = run_id.unwrap_or_else(|| uuid7(None));
        handle_event(
            &self.handlers,
            Some(|h: &dyn BaseCallbackHandler| h.ignore_retriever()),
            |h| {
                h.on_retriever_start(
                    serialized,
                    query,
                    rid,
                    self.parent_run_id,
                    Some(&self.tags),
                    Some(&self.metadata),
                    name,
                );
            },
        );
        CallbackManagerForRetrieverRun::new(self.make_run_core(rid))
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

#[derive(Debug, Clone)]
pub struct RunManagerCore {
    run_id: Uuid,
    config: CallbackManager,
}

impl Default for RunManagerCore {
    fn default() -> Self {
        Self {
            run_id: uuid7(None),
            config: CallbackManager::new(),
        }
    }
}

impl RunManagerCore {
    pub fn new(run_id: Uuid, config: CallbackManager) -> Self {
        Self { run_id, config }
    }

    pub fn noop() -> Self {
        Self::default()
    }

    pub fn run_id(&self) -> Uuid {
        self.run_id
    }

    pub fn parent_run_id(&self) -> Option<Uuid> {
        self.config.parent_run_id
    }

    pub fn handlers(&self) -> &[Arc<dyn BaseCallbackHandler>] {
        &self.config.handlers
    }

    pub fn tags(&self) -> &[String] {
        &self.config.tags
    }

    fn dispatch(
        &self,
        ignore: Option<fn(&dyn BaseCallbackHandler) -> bool>,
        f: impl FnMut(&dyn BaseCallbackHandler),
    ) {
        if !self.config.handlers.is_empty() {
            handle_event(&self.config.handlers, ignore, f);
        }
    }

    pub fn get_child_manager(&self, tag: Option<&str>) -> CallbackManager {
        let mut child = self.config.make_child(self.run_id);
        if let Some(tag) = tag {
            child.add_tags(&[tag.to_string()], false);
        }
        child
    }
}

macro_rules! define_run_manager {
    ($name:ident) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            core: RunManagerCore,
        }

        impl $name {
            pub fn new(core: RunManagerCore) -> Self {
                Self { core }
            }

            pub fn noop() -> Self {
                Self {
                    core: RunManagerCore::noop(),
                }
            }

            pub fn run_id(&self) -> Uuid {
                self.core.run_id()
            }

            pub fn parent_run_id(&self) -> Option<Uuid> {
                self.core.parent_run_id()
            }

            pub fn handlers(&self) -> &[Arc<dyn BaseCallbackHandler>] {
                self.core.handlers()
            }

            pub fn tags(&self) -> &[String] {
                self.core.tags()
            }
        }
    };
}

define_run_manager!(RunManager);

impl RunManager {
    pub fn on_text(&self, text: &str) {
        let (rid, pid) = (self.core.run_id(), self.core.parent_run_id());
        self.core
            .dispatch(None, |h| h.on_text(text, rid, pid, None, ""));
    }

    pub fn on_retry(&self, retry_state: &serde_json::Value) {
        let (rid, pid) = (self.core.run_id(), self.core.parent_run_id());
        self.core
            .dispatch(Some(|h: &dyn BaseCallbackHandler| h.ignore_retry()), |h| {
                h.on_retry(retry_state, rid, pid)
            });
    }
}

define_run_manager!(ParentRunManager);

impl ParentRunManager {
    pub fn get_child(&self, tag: Option<&str>) -> CallbackManager {
        self.core.get_child_manager(tag)
    }
}

define_run_manager!(CallbackManagerForLLMRun);

impl CallbackManagerForLLMRun {
    pub fn on_llm_new_token(&self, token: &str, chunk: Option<&serde_json::Value>) {
        let (rid, pid) = (self.core.run_id(), self.core.parent_run_id());
        self.core
            .dispatch(Some(|h: &dyn BaseCallbackHandler| h.ignore_llm()), |h| {
                h.on_llm_new_token(token, rid, pid, chunk)
            });
    }

    pub fn on_llm_end(&self, response: &ChatResult) {
        let (rid, pid) = (self.core.run_id(), self.core.parent_run_id());
        self.core
            .dispatch(Some(|h: &dyn BaseCallbackHandler| h.ignore_llm()), |h| {
                h.on_llm_end(response, rid, pid)
            });
    }

    pub fn on_llm_error(&self, error: &dyn std::error::Error) {
        let (rid, pid) = (self.core.run_id(), self.core.parent_run_id());
        self.core
            .dispatch(Some(|h: &dyn BaseCallbackHandler| h.ignore_llm()), |h| {
                h.on_llm_error(error, rid, pid)
            });
    }
}

define_run_manager!(CallbackManagerForChainRun);

impl CallbackManagerForChainRun {
    pub fn get_child(&self, tag: Option<&str>) -> CallbackManager {
        self.core.get_child_manager(tag)
    }

    pub fn on_chain_end(&self, outputs: &HashMap<String, serde_json::Value>) {
        let (rid, pid) = (self.core.run_id(), self.core.parent_run_id());
        self.core
            .dispatch(Some(|h: &dyn BaseCallbackHandler| h.ignore_chain()), |h| {
                h.on_chain_end(outputs, rid, pid)
            });
    }

    pub fn on_chain_error(&self, error: &dyn std::error::Error) {
        let (rid, pid) = (self.core.run_id(), self.core.parent_run_id());
        self.core
            .dispatch(Some(|h: &dyn BaseCallbackHandler| h.ignore_chain()), |h| {
                h.on_chain_error(error, rid, pid)
            });
    }

    pub fn on_agent_action(&self, action: &serde_json::Value) {
        let (rid, pid) = (self.core.run_id(), self.core.parent_run_id());
        self.core
            .dispatch(Some(|h: &dyn BaseCallbackHandler| h.ignore_agent()), |h| {
                h.on_agent_action(action, rid, pid, None)
            });
    }

    pub fn on_agent_finish(&self, finish: &serde_json::Value) {
        let (rid, pid) = (self.core.run_id(), self.core.parent_run_id());
        self.core
            .dispatch(Some(|h: &dyn BaseCallbackHandler| h.ignore_agent()), |h| {
                h.on_agent_finish(finish, rid, pid, None)
            });
    }
}

define_run_manager!(CallbackManagerForToolRun);

impl CallbackManagerForToolRun {
    pub fn get_child(&self, tag: Option<&str>) -> CallbackManager {
        self.core.get_child_manager(tag)
    }

    pub fn on_tool_end(&self, output: &str) {
        let (rid, pid) = (self.core.run_id(), self.core.parent_run_id());
        self.core
            .dispatch(Some(|h: &dyn BaseCallbackHandler| h.ignore_tool()), |h| {
                h.on_tool_end(output, rid, pid, None, None, None)
            });
    }

    pub fn on_tool_error(&self, error: &dyn std::error::Error) {
        let (rid, pid) = (self.core.run_id(), self.core.parent_run_id());
        self.core
            .dispatch(Some(|h: &dyn BaseCallbackHandler| h.ignore_tool()), |h| {
                h.on_tool_error(error, rid, pid)
            });
    }
}

define_run_manager!(CallbackManagerForRetrieverRun);

impl CallbackManagerForRetrieverRun {
    pub fn get_child(&self, tag: Option<&str>) -> CallbackManager {
        self.core.get_child_manager(tag)
    }

    pub fn on_retriever_end(&self, documents: &[serde_json::Value]) {
        let (rid, pid) = (self.core.run_id(), self.core.parent_run_id());
        self.core.dispatch(
            Some(|h: &dyn BaseCallbackHandler| h.ignore_retriever()),
            |h| h.on_retriever_end(documents, rid, pid),
        );
    }

    pub fn on_retriever_error(&self, error: &dyn std::error::Error) {
        let (rid, pid) = (self.core.run_id(), self.core.parent_run_id());
        self.core.dispatch(
            Some(|h: &dyn BaseCallbackHandler| h.ignore_retriever()),
            |h| h.on_retriever_error(error, rid, pid),
        );
    }
}

#[derive(Debug, Clone)]
pub struct CallbackManagerForChainGroup {
    inner: CallbackManager,
    parent_run_manager: CallbackManagerForChainRun,
    ended: bool,
}

impl CallbackManagerForChainGroup {
    pub fn from_parts(
        inner: CallbackManager,
        parent_run_manager: CallbackManagerForChainRun,
    ) -> Self {
        Self {
            inner,
            parent_run_manager,
            ended: false,
        }
    }

    pub fn inner(&self) -> &CallbackManager {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut CallbackManager {
        &mut self.inner
    }

    pub fn ended(&self) -> bool {
        self.ended
    }

    pub fn handlers(&self) -> &[Arc<dyn BaseCallbackHandler>] {
        self.inner.handlers()
    }

    pub fn tags(&self) -> &[String] {
        self.inner.tags()
    }

    pub fn parent_run_id(&self) -> Option<Uuid> {
        self.inner.parent_run_id()
    }

    pub fn merge_with(&self, other: &CallbackManager) -> Self {
        Self {
            inner: self.inner.merge(other),
            parent_run_manager: self.parent_run_manager.clone(),
            ended: self.ended,
        }
    }

    pub fn on_chain_end(&mut self, outputs: &HashMap<String, serde_json::Value>) {
        self.ended = true;
        self.parent_run_manager.on_chain_end(outputs);
    }

    pub fn on_chain_error(&mut self, error: &dyn std::error::Error) {
        self.ended = true;
        self.parent_run_manager.on_chain_error(error);
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

fn setup_chain_group(
    group_name: &str,
    callback_manager: Option<CallbackManager>,
    inputs: Option<HashMap<String, serde_json::Value>>,
    tags: Option<Vec<String>>,
    metadata: Option<HashMap<String, serde_json::Value>>,
    run_id: Option<Uuid>,
) -> (CallbackManagerForChainGroup, CallbackManagerForChainRun) {
    let cm = callback_manager.unwrap_or_else(|| {
        CallbackManager::configure()
            .maybe_inheritable_tags(tags.clone())
            .maybe_inheritable_metadata(metadata.clone())
            .call()
    });

    let serialized = HashMap::from([(
        "name".to_string(),
        serde_json::Value::String(group_name.to_string()),
    )]);

    let run_manager = cm
        .on_chain_start()
        .serialized(&serialized)
        .inputs(&inputs.unwrap_or_default())
        .maybe_run_id(run_id)
        .name(group_name)
        .call();
    let child = run_manager.get_child(None);
    let group = CallbackManagerForChainGroup::from_parts(child, run_manager.clone());
    (group, run_manager)
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
    let (mut group, run_manager) =
        setup_chain_group(group_name, callback_manager, inputs, tags, metadata, run_id);

    let result = catch_unwind(AssertUnwindSafe(|| f(&mut group)));

    match result {
        Ok(r) => {
            if !group.ended() {
                run_manager.on_chain_end(&HashMap::new());
            }
            r
        }
        Err(e) => {
            if !group.ended() {
                run_manager.on_chain_error(&ChainGroupPanicError);
            }
            std::panic::resume_unwind(e)
        }
    }
}

pub async fn atrace_as_chain_group<F, Fut, R>(
    group_name: &str,
    callback_manager: Option<CallbackManager>,
    inputs: Option<HashMap<String, serde_json::Value>>,
    tags: Option<Vec<String>>,
    metadata: Option<HashMap<String, serde_json::Value>>,
    run_id: Option<Uuid>,
    f: F,
) -> R
where
    F: FnOnce(&mut CallbackManagerForChainGroup) -> Fut,
    Fut: Future<Output = R>,
{
    let (mut group, run_manager) =
        setup_chain_group(group_name, callback_manager, inputs, tags, metadata, run_id);

    let result = f(&mut group).await;

    if !group.ended() {
        run_manager.on_chain_end(&HashMap::new());
    }

    result
}

pub fn dispatch_custom_event(
    name: &str,
    data: &serde_json::Value,
    callback_manager: &CallbackManager,
) -> Result<(), &'static str> {
    if callback_manager.handlers().is_empty() {
        return Ok(());
    }
    let run_id = callback_manager
        .parent_run_id()
        .ok_or("Unable to dispatch an adhoc event without a parent run id.")?;
    handle_event(
        callback_manager.handlers(),
        Some(|h: &dyn BaseCallbackHandler| h.ignore_custom_event()),
        |h| h.on_custom_event(name, data, run_id, None, None),
    );
    Ok(())
}

pub async fn adispatch_custom_event(
    name: &str,
    data: &serde_json::Value,
    callback_manager: &CallbackManager,
) -> Result<(), &'static str> {
    dispatch_custom_event(name, data, callback_manager)
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
        let manager = CallbackManager::configure()
            .inheritable_tags(vec!["tag1".to_string()])
            .local_tags(vec!["tag2".to_string()])
            .call();

        assert!(manager.tags().contains(&"tag1".to_string()));
        assert!(manager.tags().contains(&"tag2".to_string()));
        assert!(manager.inheritable_tags().contains(&"tag1".to_string()));
        assert!(!manager.inheritable_tags().contains(&"tag2".to_string()));
    }

    #[test]
    fn test_configure_with_verbose() {
        crate::globals::set_debug(false);

        let manager = CallbackManager::configure().verbose(true).call();
        assert!(
            manager
                .handlers()
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
            .handlers()
            .iter()
            .filter(|h| h.name() == "StdOutCallbackHandler")
            .count();
        assert_eq!(
            stdout_count, 1,
            "Should not duplicate StdOutCallbackHandler"
        );
    }
}
