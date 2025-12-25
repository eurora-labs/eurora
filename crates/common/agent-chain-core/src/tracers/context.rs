//! Context management for tracers.
//!
//! This module provides context management for tracers using thread-local storage.
//! Mirrors `langchain_core.tracers.context`.

use std::cell::RefCell;
use std::sync::Arc;
use uuid::Uuid;

use crate::tracers::run_collector::RunCollectorCallbackHandler;
use crate::tracers::schemas::Run;

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

/// Check if tracing v2 is enabled.
pub fn tracing_v2_is_enabled() -> bool {
    TRACING_V2_CALLBACK.with(|cell| cell.borrow().is_some())
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

/// Configuration hook for registering callback handlers.
#[derive(Debug, Clone)]
pub struct ConfigureHook {
    /// Whether the hook is inheritable.
    pub inheritable: bool,
    /// The environment variable to check.
    pub env_var: Option<String>,
}

impl ConfigureHook {
    /// Create a new configure hook.
    pub fn new(inheritable: bool, env_var: Option<String>) -> Self {
        Self {
            inheritable,
            env_var,
        }
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
/// # Arguments
///
/// * `inheritable` - Whether the hook is inheritable.
/// * `env_var` - The environment variable to check.
pub fn register_configure_hook(inheritable: bool, env_var: Option<String>) {
    if let Ok(mut registry) = CONFIGURE_HOOKS.lock() {
        registry.register(ConfigureHook::new(inheritable, env_var));
    }
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
        register_configure_hook(false, None);
        register_configure_hook(true, Some("LANGCHAIN_TRACING_V2".to_string()));

        let registry = CONFIGURE_HOOKS.lock().unwrap();
        assert!(registry.hooks().len() >= 2);
    }
}
