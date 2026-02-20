use std::cell::RefCell;
use std::sync::Arc;
use uuid::Uuid;

use crate::callbacks::base::BaseCallbackHandler;
use crate::tracers::run_collector::RunCollectorCallbackHandler;
use crate::tracers::schemas::Run;
use crate::utils::env::env_var_is_set;

thread_local! {
    static TRACING_V2_CALLBACK: RefCell<Option<Arc<dyn TracingCallback>>> = const { RefCell::new(None) };
    static RUN_COLLECTOR: RefCell<Option<Arc<std::sync::Mutex<RunCollectorCallbackHandler>>>> = const { RefCell::new(None) };
}

pub trait TracingCallback: Send + Sync {
    fn project_name(&self) -> Option<&str>;

    fn example_id(&self) -> Option<Uuid>;

    fn latest_run(&self) -> Option<&Run>;

    fn get_run_url(&self) -> Option<String>;
}

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

pub fn tracing_v2_enabled(callback: Arc<dyn TracingCallback>) -> TracingV2Guard {
    let previous = TRACING_V2_CALLBACK.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let prev = borrow.take();
        *borrow = Some(callback);
        prev
    });

    TracingV2Guard { previous }
}

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

pub fn get_tracing_callback() -> Option<Arc<dyn TracingCallback>> {
    TRACING_V2_CALLBACK.with(|cell| cell.borrow().clone())
}

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

pub fn get_run_collector() -> Option<Arc<std::sync::Mutex<RunCollectorCallbackHandler>>> {
    RUN_COLLECTOR.with(|cell| cell.borrow().clone())
}

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

pub struct ConfigureHook {
    pub context_getter: Box<dyn Fn() -> Option<Arc<dyn BaseCallbackHandler>> + Send + Sync>,
    pub inheritable: bool,
    pub handler_factory: Option<Box<dyn Fn() -> Arc<dyn BaseCallbackHandler> + Send + Sync>>,
    pub handler_type_name: Option<String>,
    pub env_var: Option<String>,
}

impl ConfigureHook {
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

#[derive(Debug, Default)]
pub struct ConfigureHookRegistry {
    hooks: Vec<ConfigureHook>,
}

impl ConfigureHookRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, hook: ConfigureHook) {
        self.hooks.push(hook);
    }

    pub fn hooks(&self) -> &[ConfigureHook] {
        &self.hooks
    }
}

static CONFIGURE_HOOKS: std::sync::LazyLock<std::sync::Mutex<ConfigureHookRegistry>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(ConfigureHookRegistry::new()));

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

pub fn get_configure_hooks() -> &'static std::sync::LazyLock<std::sync::Mutex<ConfigureHookRegistry>>
{
    &CONFIGURE_HOOKS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_runs() {
        let collector = RunCollectorCallbackHandler::new(None);

        {
            let (_guard, collector_arc) = collect_runs(collector);

            let current = get_run_collector();
            assert!(current.is_some());

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
        assert!(
            !tracing_v2_is_enabled() || TRACING_V2_CALLBACK.with(|cell| cell.borrow().is_some())
        );

        unsafe {
            std::env::set_var("LANGCHAIN_TRACING_V2", "true");
        }
        assert!(tracing_v2_is_enabled());
        unsafe {
            std::env::remove_var("LANGCHAIN_TRACING_V2");
        }

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
