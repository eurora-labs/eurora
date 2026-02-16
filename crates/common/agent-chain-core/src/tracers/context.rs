//! Context management for tracers.
//!
//! This module provides context management for tracers using thread-local storage.
//! Mirrors `langchain_core.tracers.context`.

use std::cell::RefCell;
use std::sync::Arc;
use uuid::Uuid;

use crate::callbacks::base::BaseCallbackHandler;
use crate::tracers::run_collector::RunCollectorCallbackHandler;
use crate::tracers::schemas::Run;
use crate::utils::env::env_var_is_set;

// Thread-local storage for the tracing callback handler.
thread_local! {
    static TRACING_V2_CALLBACK: RefCell<Option<Arc<dyn TracingCallback>>> = const { RefCell::new(None) };
    static RUN_COLLECTOR: RefCell<Option<Arc<std::sync::Mutex<RunCollectorCallbackHandler>>>> = const { RefCell::new(None) };
}

/// Trait for tracing callbacks that can be stored in context.
pub trait TracingCallback: Send + Sync {
    /// Get the project name.
    fn project_name(&self) -> Option<&str>;

    /// Get the example ID.
    fn example_id(&self) -> Option<Uuid>;

    /// Get the latest run.
    fn latest_run(&self) -> Option<&Run>;

    /// Get the run URL.
    fn get_run_url(&self) -> Option<String>;
}

/// Guard that resets the tracing callback when dropped.
pub struct TracingV2Guard {
    previous: Option<Arc<dyn TracingCallback>>,
}

impl Drop for TracingV2Guard {
    fn drop(&mut self) {
        TRACING_V2_CALLBACK.with(|cell| {
            *cell.borrow_mut() = self.previous.take();
        });
    }
}

/// Guard that resets the run collector when dropped.
pub struct RunCollectorGuard {
    previous: Option<Arc<std::sync::Mutex<RunCollectorCallbackHandler>>>,
}

impl Drop for RunCollectorGuard {
    fn drop(&mut self) {
        RUN_COLLECTOR.with(|cell| {
            *cell.borrow_mut() = self.previous.take();
        });
    }
}

/// Enable tracing v2 in the current context.
///
/// # Arguments
///
/// * `callback` - The tracing callback to use.
///
/// # Returns
///
/// A guard that will reset the callback when dropped.
pub fn tracing_v2_enabled(callback: Arc<dyn TracingCallback>) -> TracingV2Guard {
    let previous = TRACING_V2_CALLBACK.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let prev = borrow.take();
        *borrow = Some(callback);
        prev
    });

    TracingV2Guard { previous }
}

/// Check if tracing v2 is enabled via context or environment variables.
///
/// Checks (in order):
/// 1. Thread-local tracing callback is set
/// 2. LANGSMITH_TRACING_V2 or LANGCHAIN_TRACING_V2 env var is "true"
/// 3. LANGSMITH_TRACING or LANGCHAIN_TRACING env var is "true"
pub fn tracing_v2_is_enabled() -> bool {
    let has_callback = TRACING_V2_CALLBACK.with(|cell| cell.borrow().is_some());
    if has_callback {
        return true;
    }
    env_var_is_set("LANGSMITH_TRACING_V2")
        || env_var_is_set("LANGCHAIN_TRACING_V2")
        || env_var_is_set("LANGSMITH_TRACING")
        || env_var_is_set("LANGCHAIN_TRACING")
}

/// Get the current tracing callback.
pub fn get_tracing_callback() -> Option<Arc<dyn TracingCallback>> {
    TRACING_V2_CALLBACK.with(|cell| cell.borrow().clone())
}

/// Collect runs in the current context.
///
/// # Arguments
///
/// * `collector` - The run collector to use.
///
/// # Returns
///
/// A guard that will reset the collector when dropped.
pub fn collect_runs(
    collector: RunCollectorCallbackHandler,
) -> (
    RunCollectorGuard,
    Arc<std::sync::Mutex<RunCollectorCallbackHandler>>,
) {
    let collector = Arc::new(std::sync::Mutex::new(collector));
    let collector_clone = collector.clone();

    let previous = RUN_COLLECTOR.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let prev = borrow.take();
        *borrow = Some(collector);
        prev
    });

    (RunCollectorGuard { previous }, collector_clone)
}

/// Get the current run collector.
pub fn get_run_collector() -> Option<Arc<std::sync::Mutex<RunCollectorCallbackHandler>>> {
    RUN_COLLECTOR.with(|cell| cell.borrow().clone())
}

/// Get the project name for tracing.
///
/// Checks env vars in order: HOSTED_LANGSERVE_PROJECT_NAME,
/// LANGSMITH_PROJECT / LANGCHAIN_PROJECT,
/// LANGSMITH_SESSION / LANGCHAIN_SESSION, then falls back to "default".
pub fn get_tracer_project() -> String {
    if let Ok(val) = std::env::var("HOSTED_LANGSERVE_PROJECT_NAME")
        && !val.is_empty()
    {
        return val;
    }
    for name in &["LANGSMITH_PROJECT", "LANGCHAIN_PROJECT"] {
        if let Ok(val) = std::env::var(name)
            && !val.is_empty()
        {
            return val;
        }
    }
    for name in &["LANGSMITH_SESSION", "LANGCHAIN_SESSION"] {
        if let Ok(val) = std::env::var(name)
            && !val.is_empty()
        {
            return val;
        }
    }
    "default".to_string()
}

/// Configuration hook for registering callback handlers that get
/// auto-added during `configure()`.
pub struct ConfigureHook {
    /// Function to get the current context value (replaces Python's ContextVar.get()).
    pub context_getter: Box<dyn Fn() -> Option<Arc<dyn BaseCallbackHandler>> + Send + Sync>,
    /// Whether the handler should be inheritable.
    pub inheritable: bool,
    /// Optional factory to create a new handler (replaces Python's handler_class()).
    pub handler_factory: Option<Box<dyn Fn() -> Arc<dyn BaseCallbackHandler> + Send + Sync>>,
    /// Optional handler type name for deduplication (replaces Python's isinstance check).
    pub handler_type_name: Option<String>,
    /// Optional environment variable that triggers auto-creation.
    pub env_var: Option<String>,
}

impl ConfigureHook {
    /// Create a new configure hook.
    pub fn new(
        context_getter: impl Fn() -> Option<Arc<dyn BaseCallbackHandler>> + Send + Sync + 'static,
        inheritable: bool,
        handler_factory: Option<Box<dyn Fn() -> Arc<dyn BaseCallbackHandler> + Send + Sync>>,
        handler_type_name: Option<String>,
        env_var: Option<String>,
    ) -> Self {
        Self {
            context_getter: Box::new(context_getter),
            inheritable,
            handler_factory,
            handler_type_name,
            env_var,
        }
    }
}

impl std::fmt::Debug for ConfigureHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigureHook")
            .field("inheritable", &self.inheritable)
            .field("handler_type_name", &self.handler_type_name)
            .field("env_var", &self.env_var)
            .finish()
    }
}

/// Registry for configure hooks.
#[derive(Debug, Default)]
pub struct ConfigureHookRegistry {
    hooks: Vec<ConfigureHook>,
}

impl ConfigureHookRegistry {
    /// Create a new configure hook registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a configure hook.
    pub fn register(&mut self, hook: ConfigureHook) {
        self.hooks.push(hook);
    }

    /// Get all registered hooks.
    pub fn hooks(&self) -> &[ConfigureHook] {
        &self.hooks
    }
}

/// Global configure hook registry.
static CONFIGURE_HOOKS: std::sync::LazyLock<std::sync::Mutex<ConfigureHookRegistry>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(ConfigureHookRegistry::new()));

/// Register a configure hook.
///
/// Matches Python's `register_configure_hook(context_var, inheritable, handler_class)`.
pub fn register_configure_hook(
    context_getter: impl Fn() -> Option<Arc<dyn BaseCallbackHandler>> + Send + Sync + 'static,
    inheritable: bool,
    handler_factory: Option<Box<dyn Fn() -> Arc<dyn BaseCallbackHandler> + Send + Sync>>,
    handler_type_name: Option<String>,
    env_var: Option<String>,
) {
    if let Ok(mut registry) = CONFIGURE_HOOKS.lock() {
        registry.register(ConfigureHook::new(
            context_getter,
            inheritable,
            handler_factory,
            handler_type_name,
            env_var,
        ));
    }
}

/// Get a reference to the global configure hooks registry.
pub fn get_configure_hooks() -> &'static std::sync::LazyLock<std::sync::Mutex<ConfigureHookRegistry>>
{
    &CONFIGURE_HOOKS
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCallback {
        project: String,
    }

    impl TracingCallback for TestCallback {
        fn project_name(&self) -> Option<&str> {
            Some(&self.project)
        }

        fn example_id(&self) -> Option<Uuid> {
            None
        }

        fn latest_run(&self) -> Option<&Run> {
            None
        }

        fn get_run_url(&self) -> Option<String> {
            None
        }
    }

    #[test]
    fn test_tracing_v2_enabled() {
        assert!(!tracing_v2_is_enabled());

        let callback = Arc::new(TestCallback {
            project: "test".to_string(),
        });

        {
            let _guard = tracing_v2_enabled(callback.clone());
            assert!(tracing_v2_is_enabled());

            let cb = get_tracing_callback().unwrap();
            assert_eq!(cb.project_name(), Some("test"));
        }

        assert!(!tracing_v2_is_enabled());
    }

    #[test]
    fn test_collect_runs() {
        let collector = RunCollectorCallbackHandler::new(None);

        {
            let (_guard, collector_arc) = collect_runs(collector);

            let current = get_run_collector();
            assert!(current.is_some());

            // Verify it's the same collector
            let collector_locked = collector_arc.lock().unwrap();
            assert!(collector_locked.is_empty());
        }

        assert!(get_run_collector().is_none());
    }

    #[test]
    fn test_register_configure_hook() {
        register_configure_hook(|| None, false, None, None, None);
        register_configure_hook(
            || None,
            true,
            None,
            None,
            Some("LANGCHAIN_TRACING_V2".to_string()),
        );

        let registry = CONFIGURE_HOOKS.lock().unwrap();
        assert!(registry.hooks().len() >= 2);
    }

    #[test]
    fn test_tracing_v2_is_enabled_env_vars() {
        // Start clean
        assert!(
            !tracing_v2_is_enabled() || TRACING_V2_CALLBACK.with(|cell| cell.borrow().is_some())
        );

        // Test LANGCHAIN_TRACING_V2
        unsafe {
            std::env::set_var("LANGCHAIN_TRACING_V2", "true");
        }
        assert!(tracing_v2_is_enabled());
        unsafe {
            std::env::remove_var("LANGCHAIN_TRACING_V2");
        }

        // Test LANGSMITH_TRACING
        unsafe {
            std::env::set_var("LANGSMITH_TRACING", "true");
        }
        assert!(tracing_v2_is_enabled());
        unsafe {
            std::env::remove_var("LANGSMITH_TRACING");
        }
    }

    #[test]
    fn test_get_tracer_project() {
        // Clean env
        unsafe {
            std::env::remove_var("HOSTED_LANGSERVE_PROJECT_NAME");
            std::env::remove_var("LANGSMITH_PROJECT");
            std::env::remove_var("LANGCHAIN_PROJECT");
            std::env::remove_var("LANGSMITH_SESSION");
            std::env::remove_var("LANGCHAIN_SESSION");
        }

        assert_eq!(get_tracer_project(), "default");

        unsafe {
            std::env::set_var("LANGCHAIN_PROJECT", "my_project");
        }
        assert_eq!(get_tracer_project(), "my_project");
        unsafe {
            std::env::remove_var("LANGCHAIN_PROJECT");
        }

        unsafe {
            std::env::set_var("HOSTED_LANGSERVE_PROJECT_NAME", "hosted_proj");
        }
        assert_eq!(get_tracer_project(), "hosted_proj");
        unsafe {
            std::env::remove_var("HOSTED_LANGSERVE_PROJECT_NAME");
        }
    }
}
