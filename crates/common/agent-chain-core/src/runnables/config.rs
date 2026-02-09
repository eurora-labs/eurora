//! Configuration for Runnables.
//!
//! This module provides `RunnableConfig` and related utilities,
//! mirroring `langchain_core.runnables.config`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::callbacks::{CallbackManager, Callbacks};

/// Configuration for a Runnable.
///
/// This struct contains all the configuration options that can be passed
/// to a Runnable's invoke, batch, or stream methods.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnableConfig {
    /// Tags for this call and any sub-calls (e.g. a Chain calling an LLM).
    /// These can be used to filter calls.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Metadata for this call and any sub-calls (e.g. a Chain calling an LLM).
    /// Keys should be strings, values should be JSON-serializable.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,

    /// Callbacks for this call and any sub-calls.
    /// Tags and metadata are automagically inherited.
    #[serde(skip)]
    pub callbacks: Option<Callbacks>,

    /// Name for the tracer run for this call.
    /// Defaults to the name of the class.
    #[serde(default)]
    pub run_name: Option<String>,

    /// Maximum number of parallel calls to make.
    /// If not provided, defaults to ThreadPoolExecutor's default.
    #[serde(default)]
    pub max_concurrency: Option<usize>,

    /// Maximum number of times a call can recurse.
    /// If not provided, defaults to 25.
    #[serde(default = "default_recursion_limit")]
    pub recursion_limit: i32,

    /// Runtime values for configurable attributes of the Runnable.
    #[serde(default)]
    pub configurable: HashMap<String, serde_json::Value>,

    /// Unique identifier for the tracer run for this call.
    /// If not provided, a new UUID will be generated.
    #[serde(default)]
    pub run_id: Option<Uuid>,
}

fn default_recursion_limit() -> i32 {
    25
}

impl Default for RunnableConfig {
    fn default() -> Self {
        Self {
            tags: Vec::new(),
            metadata: HashMap::new(),
            callbacks: None,
            run_name: None,
            max_concurrency: None,
            recursion_limit: 25,
            configurable: HashMap::new(),
            run_id: None,
        }
    }
}

impl RunnableConfig {
    /// Create a new RunnableConfig with default values.
    pub fn new() -> Self {
        Self {
            tags: Vec::new(),
            metadata: HashMap::new(),
            callbacks: None,
            run_name: None,
            max_concurrency: None,
            recursion_limit: 25,
            configurable: HashMap::new(),
            run_id: None,
        }
    }

    /// Set the tags for this config.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set the metadata for this config.
    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set the callbacks for this config.
    pub fn with_callbacks(mut self, callbacks: Callbacks) -> Self {
        self.callbacks = Some(callbacks);
        self
    }

    /// Set the run name for this config.
    pub fn with_run_name(mut self, run_name: impl Into<String>) -> Self {
        self.run_name = Some(run_name.into());
        self
    }

    /// Set the max concurrency for this config.
    pub fn with_max_concurrency(mut self, max_concurrency: usize) -> Self {
        self.max_concurrency = Some(max_concurrency);
        self
    }

    /// Set the recursion limit for this config.
    pub fn with_recursion_limit(mut self, recursion_limit: i32) -> Self {
        self.recursion_limit = recursion_limit;
        self
    }

    /// Set the configurable values for this config.
    pub fn with_configurable(mut self, configurable: HashMap<String, serde_json::Value>) -> Self {
        self.configurable = configurable;
        self
    }

    /// Set the run ID for this config.
    pub fn with_run_id(mut self, run_id: Uuid) -> Self {
        self.run_id = Some(run_id);
        self
    }
}

/// Either a single RunnableConfig or a list of them.
#[derive(Debug, Clone)]
pub enum ConfigOrList {
    Single(Box<RunnableConfig>),
    List(Vec<RunnableConfig>),
}

impl From<RunnableConfig> for ConfigOrList {
    fn from(config: RunnableConfig) -> Self {
        ConfigOrList::Single(Box::new(config))
    }
}

impl From<Vec<RunnableConfig>> for ConfigOrList {
    fn from(configs: Vec<RunnableConfig>) -> Self {
        ConfigOrList::List(configs)
    }
}

/// Ensure we have a RunnableConfig, creating one if necessary.
pub fn ensure_config(config: Option<RunnableConfig>) -> RunnableConfig {
    config.unwrap_or_default()
}

/// Get a list of configs from either a single config or a list.
///
/// If a single config is provided, it will be cloned `length` times.
/// Special handling: if a single config with run_id is provided for length > 1,
/// only the first element keeps the run_id.
pub fn get_config_list(config: Option<ConfigOrList>, length: usize) -> Vec<RunnableConfig> {
    match config {
        Some(ConfigOrList::Single(c)) => {
            // Special case: if length > 1 and config has a run_id,
            // only use it for the first element
            if length > 1 && c.run_id.is_some() {
                tracing::warn!(
                    target: "agent_chain_core::runnables",
                    "Provided run_id will be used only for the first element of the batch."
                );
                let mut configs = Vec::with_capacity(length);
                configs.push((*c).clone());

                // Create subsequent configs without run_id
                let mut subsequent = *c;
                subsequent.run_id = None;
                for _ in 1..length {
                    configs.push(subsequent.clone());
                }
                configs
            } else {
                vec![*c; length]
            }
        }
        Some(ConfigOrList::List(list)) => {
            if list.len() != length {
                panic!(
                    "config must be a list of the same length as inputs, but got {} configs for {} inputs",
                    list.len(),
                    length
                );
            }
            list.into_iter().map(|c| ensure_config(Some(c))).collect()
        }
        None => (0..length).map(|_| RunnableConfig::default()).collect(),
    }
}

/// Patch a config with updates.
///
/// This allows updating specific fields of a config while keeping others.
/// When callbacks are replaced, run_name and run_id are cleared as they
/// should only apply to the same run as the original callbacks.
pub fn patch_config(
    config: Option<RunnableConfig>,
    callbacks: Option<CallbackManager>,
    run_name: Option<String>,
    max_concurrency: Option<usize>,
    recursion_limit: Option<i32>,
    configurable: Option<HashMap<String, serde_json::Value>>,
) -> RunnableConfig {
    let mut config = ensure_config(config);

    if let Some(cb) = callbacks {
        // If we're replacing callbacks, we need to unset run_name and run_id
        // As that should apply only to the same run as the original callbacks
        // Convert CallbackManager to Callbacks::Manager by extracting its fields
        let base_manager = crate::callbacks::BaseCallbackManager {
            handlers: cb.handlers,
            inheritable_handlers: cb.inheritable_handlers,
            parent_run_id: cb.parent_run_id,
            tags: cb.tags,
            inheritable_tags: cb.inheritable_tags,
            metadata: cb.metadata,
            inheritable_metadata: cb.inheritable_metadata,
        };
        config.callbacks = Some(Callbacks::Manager(base_manager));
        config.run_name = None;
        config.run_id = None;
    }
    if let Some(name) = run_name {
        config.run_name = Some(name);
    }
    if let Some(max) = max_concurrency {
        config.max_concurrency = Some(max);
    }
    if let Some(limit) = recursion_limit {
        config.recursion_limit = limit;
    }
    if let Some(cfg) = configurable {
        // Merge with existing configurable rather than replacing
        config.configurable.extend(cfg);
    }

    config
}

/// Merge multiple configs into one.
///
/// Later configs take precedence over earlier ones.
pub fn merge_configs(configs: Vec<Option<RunnableConfig>>) -> RunnableConfig {
    let mut result = RunnableConfig::default();

    for config in configs.into_iter().flatten() {
        let config = ensure_config(Some(config));

        // Merge tags (sorted and deduplicated)
        for tag in config.tags {
            if !result.tags.contains(&tag) {
                result.tags.push(tag);
            }
        }
        result.tags.sort();

        // Merge metadata
        result.metadata.extend(config.metadata);

        // Handle callbacks merging (can be list or manager)
        match (&result.callbacks, &config.callbacks) {
            (None, Some(cb)) => {
                result.callbacks = Some(cb.clone());
            }
            (Some(Callbacks::Handlers(base_handlers)), Some(Callbacks::Handlers(new_handlers))) => {
                let mut merged = base_handlers.clone();
                merged.extend(new_handlers.clone());
                result.callbacks = Some(Callbacks::Handlers(merged));
            }
            (Some(Callbacks::Manager(base_mgr)), Some(Callbacks::Handlers(new_handlers))) => {
                let mut merged = base_mgr.clone();
                for handler in new_handlers {
                    merged.add_handler(handler.clone(), true);
                }
                result.callbacks = Some(Callbacks::Manager(merged));
            }
            (Some(Callbacks::Handlers(base_handlers)), Some(Callbacks::Manager(new_mgr))) => {
                let mut merged = new_mgr.clone();
                for handler in base_handlers {
                    merged.add_handler(handler.clone(), true);
                }
                result.callbacks = Some(Callbacks::Manager(merged));
            }
            (Some(Callbacks::Manager(_base_mgr)), Some(Callbacks::Manager(new_mgr))) => {
                // For manager-to-manager merge, just take the new one
                // In Python this uses merge method, but we'll simplify
                result.callbacks = Some(Callbacks::Manager(new_mgr.clone()));
            }
            _ => {}
        }

        // Take the last non-None value for other fields
        if config.run_name.is_some() {
            result.run_name = config.run_name;
        }
        if config.max_concurrency.is_some() {
            result.max_concurrency = config.max_concurrency;
        }

        // Only update recursion_limit if it's not the default
        if config.recursion_limit != 25 {
            result.recursion_limit = config.recursion_limit;
        }

        // Merge configurable
        result.configurable.extend(config.configurable);

        if config.run_id.is_some() {
            result.run_id = config.run_id;
        }
    }

    result
}

/// A callable that takes input and optionally a config.
///
/// This enum mirrors the Python `call_func_with_variable_args` pattern,
/// where a function may or may not accept a `RunnableConfig` parameter.
#[allow(clippy::type_complexity)]
pub enum VariableArgsFn<I, O> {
    /// A function that only takes input.
    InputOnly(Box<dyn Fn(I) -> O + Send + Sync>),
    /// A function that takes input and config.
    WithConfig(Box<dyn Fn(I, &RunnableConfig) -> O + Send + Sync>),
}

/// Call a function that may optionally accept a config.
///
/// This mirrors Python's `call_func_with_variable_args`.
pub fn call_func_with_variable_args<I, O>(
    func: &VariableArgsFn<I, O>,
    input: I,
    config: &RunnableConfig,
) -> O {
    match func {
        VariableArgsFn::InputOnly(f) => f(input),
        VariableArgsFn::WithConfig(f) => f(input, config),
    }
}

/// An async callable that takes input and optionally a config.
#[allow(clippy::type_complexity)]
pub enum AsyncVariableArgsFn<I, O> {
    /// An async function that only takes input.
    InputOnly(
        Box<
            dyn Fn(I) -> std::pin::Pin<Box<dyn std::future::Future<Output = O> + Send>>
                + Send
                + Sync,
        >,
    ),
    /// An async function that takes input and config.
    WithConfig(
        Box<
            dyn Fn(
                    I,
                    RunnableConfig,
                )
                    -> std::pin::Pin<Box<dyn std::future::Future<Output = O> + Send>>
                + Send
                + Sync,
        >,
    ),
}

/// Call an async function that may optionally accept a config.
///
/// This mirrors Python's `acall_func_with_variable_args`.
pub async fn acall_func_with_variable_args<I, O>(
    func: &AsyncVariableArgsFn<I, O>,
    input: I,
    config: &RunnableConfig,
) -> O {
    match func {
        AsyncVariableArgsFn::InputOnly(f) => f(input).await,
        AsyncVariableArgsFn::WithConfig(f) => f(input, config.clone()).await,
    }
}

/// Get a callback manager configured from the given RunnableConfig.
pub fn get_callback_manager_for_config(config: &RunnableConfig) -> CallbackManager {
    CallbackManager::configure(
        config.callbacks.clone(),
        None,
        Some(config.tags.clone()),
        None,
        Some(config.metadata.clone()),
        None,
    )
}

/// Get an async callback manager configured from the given RunnableConfig.
pub fn get_async_callback_manager_for_config(
    config: &RunnableConfig,
) -> crate::callbacks::AsyncCallbackManager {
    crate::callbacks::AsyncCallbackManager::configure(
        config.callbacks.clone(),
        None,
        Some(config.tags.clone()),
        None,
        Some(config.metadata.clone()),
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runnable_config_default() {
        let config = RunnableConfig::default();
        assert!(config.tags.is_empty());
        assert!(config.metadata.is_empty());
        assert!(config.callbacks.is_none());
        assert!(config.run_name.is_none());
        assert!(config.max_concurrency.is_none());
        assert_eq!(config.recursion_limit, 25);
        assert!(config.configurable.is_empty());
        assert!(config.run_id.is_none());
    }

    #[test]
    fn test_runnable_config_builder() {
        let config = RunnableConfig::new()
            .with_tags(vec!["tag1".to_string(), "tag2".to_string()])
            .with_run_name("test_run")
            .with_max_concurrency(4)
            .with_recursion_limit(10);

        assert_eq!(config.tags, vec!["tag1", "tag2"]);
        assert_eq!(config.run_name, Some("test_run".to_string()));
        assert_eq!(config.max_concurrency, Some(4));
        assert_eq!(config.recursion_limit, 10);
    }

    #[test]
    fn test_ensure_config() {
        let config = ensure_config(None);
        assert_eq!(config.recursion_limit, 25);

        let custom = RunnableConfig::new().with_recursion_limit(10);
        let config = ensure_config(Some(custom));
        assert_eq!(config.recursion_limit, 10);
    }

    #[test]
    fn test_get_config_list() {
        let configs = get_config_list(None, 3);
        assert_eq!(configs.len(), 3);

        let single = RunnableConfig::new().with_recursion_limit(10);
        let configs = get_config_list(Some(ConfigOrList::Single(Box::new(single))), 3);
        assert_eq!(configs.len(), 3);
        assert!(configs.iter().all(|c| c.recursion_limit == 10));
    }

    #[test]
    fn test_merge_configs() {
        let config1 = RunnableConfig::new()
            .with_tags(vec!["tag1".to_string()])
            .with_recursion_limit(10);

        let config2 = RunnableConfig::new()
            .with_tags(vec!["tag2".to_string()])
            .with_run_name("test");

        let merged = merge_configs(vec![Some(config1), Some(config2)]);

        // Tags should be sorted
        assert_eq!(merged.tags, vec!["tag1", "tag2"]);
        // Recursion limit should be from config1 (10) since config2 has default (25)
        assert_eq!(merged.recursion_limit, 10);
        assert_eq!(merged.run_name, Some("test".to_string()));
    }

    #[test]
    fn test_patch_config() {
        let config = RunnableConfig::new().with_recursion_limit(10);

        let patched = patch_config(
            Some(config),
            None,
            Some("new_name".to_string()),
            Some(8),
            None,
            None,
        );

        assert_eq!(patched.run_name, Some("new_name".to_string()));
        assert_eq!(patched.max_concurrency, Some(8));
        assert_eq!(patched.recursion_limit, 10); // Unchanged
    }

    #[test]
    fn test_get_config_list_with_run_id() {
        let config = RunnableConfig::new()
            .with_run_id(uuid::Uuid::new_v4())
            .with_recursion_limit(10);

        let configs = get_config_list(Some(ConfigOrList::Single(Box::new(config.clone()))), 3);
        assert_eq!(configs.len(), 3);
        // First config should have the run_id
        assert!(configs[0].run_id.is_some());
        assert_eq!(configs[0].recursion_limit, 10);
        // Subsequent configs should not have the run_id
        assert!(configs[1].run_id.is_none());
        assert!(configs[2].run_id.is_none());
        // But should have other fields
        assert_eq!(configs[1].recursion_limit, 10);
        assert_eq!(configs[2].recursion_limit, 10);
    }

    #[test]
    fn test_patch_config_callbacks_clear_run_info() {
        let run_id = uuid::Uuid::new_v4();
        let config = RunnableConfig::new()
            .with_run_name("original")
            .with_run_id(run_id)
            .with_recursion_limit(10);

        let new_manager = CallbackManager::new();
        let patched = patch_config(Some(config), Some(new_manager), None, None, None, None);

        // run_name and run_id should be cleared when callbacks are replaced
        assert!(patched.run_name.is_none());
        assert!(patched.run_id.is_none());
        // Other fields should be preserved
        assert_eq!(patched.recursion_limit, 10);
    }
}
