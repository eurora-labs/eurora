//! Tracers that call listeners.
//!
//! This module provides tracers that call listener functions on run start, end, and error.
//! Mirrors `langchain_core.tracers.root_listeners`.

use std::collections::HashMap;
use std::fmt;

use async_trait::async_trait;
use uuid::Uuid;

use crate::tracers::base::{AsyncBaseTracer, BaseTracer};
use crate::tracers::core::{SchemaFormat, TracerCore, TracerCoreConfig};
use crate::tracers::schemas::Run;

/// Type alias for a synchronous listener function.
pub type Listener = Box<dyn Fn(&Run) + Send + Sync>;

/// Type alias for an asynchronous listener function.
pub type AsyncListener = Box<dyn Fn(&Run) -> futures::future::BoxFuture<'static, ()> + Send + Sync>;

/// Tracer that calls listeners on run start, end, and error.
pub struct RootListenersTracer {
    /// The tracer configuration.
    config: TracerCoreConfig,
    /// The run map.
    run_map: HashMap<String, Run>,
    /// The order map.
    order_map: HashMap<Uuid, (Uuid, String)>,
    /// The root run ID.
    root_id: Option<Uuid>,
    /// Listener called on run start.
    #[allow(dead_code)]
    on_start: Option<Listener>,
    /// Listener called on run end.
    #[allow(dead_code)]
    on_end: Option<Listener>,
    /// Listener called on run error.
    #[allow(dead_code)]
    on_error: Option<Listener>,
}

impl fmt::Debug for RootListenersTracer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RootListenersTracer")
            .field("config", &self.config)
            .field("run_map", &self.run_map)
            .field("order_map", &self.order_map)
            .field("root_id", &self.root_id)
            .field("on_start", &self.on_start.as_ref().map(|_| "Listener"))
            .field("on_end", &self.on_end.as_ref().map(|_| "Listener"))
            .field("on_error", &self.on_error.as_ref().map(|_| "Listener"))
            .finish()
    }
}

impl RootListenersTracer {
    /// Create a new RootListenersTracer.
    ///
    /// # Arguments
    ///
    /// * `on_start` - Listener to call on run start.
    /// * `on_end` - Listener to call on run end.
    /// * `on_error` - Listener to call on run error.
    pub fn new(
        on_start: Option<Listener>,
        on_end: Option<Listener>,
        on_error: Option<Listener>,
    ) -> Self {
        Self {
            config: TracerCoreConfig {
                schema_format: SchemaFormat::OriginalChat,
                log_missing_parent: false,
            },
            run_map: HashMap::new(),
            order_map: HashMap::new(),
            root_id: None,
            on_start,
            on_end,
            on_error,
        }
    }

    /// Create a tracer with only an on_start listener.
    pub fn with_on_start(on_start: impl Fn(&Run) + Send + Sync + 'static) -> Self {
        Self::new(Some(Box::new(on_start)), None, None)
    }

    /// Create a tracer with only an on_end listener.
    pub fn with_on_end(on_end: impl Fn(&Run) + Send + Sync + 'static) -> Self {
        Self::new(None, Some(Box::new(on_end)), None)
    }

    /// Create a tracer with only an on_error listener.
    pub fn with_on_error(on_error: impl Fn(&Run) + Send + Sync + 'static) -> Self {
        Self::new(None, None, Some(Box::new(on_error)))
    }

    /// Get the root run ID.
    pub fn root_id(&self) -> Option<Uuid> {
        self.root_id
    }
}

impl TracerCore for RootListenersTracer {
    fn config(&self) -> &TracerCoreConfig {
        &self.config
    }

    fn config_mut(&mut self) -> &mut TracerCoreConfig {
        &mut self.config
    }

    fn run_map(&self) -> &HashMap<String, Run> {
        &self.run_map
    }

    fn run_map_mut(&mut self) -> &mut HashMap<String, Run> {
        &mut self.run_map
    }

    fn order_map(&self) -> &HashMap<Uuid, (Uuid, String)> {
        &self.order_map
    }

    fn order_map_mut(&mut self) -> &mut HashMap<Uuid, (Uuid, String)> {
        &mut self.order_map
    }

    fn persist_run(&mut self, _run: &Run) {}

    fn on_run_create(&mut self, run: &Run) {
        if self.root_id.is_some() {
            return;
        }

        self.root_id = Some(run.id);

        if let Some(ref on_start) = self.on_start {
            on_start(run);
        }
    }

    fn on_run_update(&mut self, run: &Run) {
        if Some(run.id) != self.root_id {
            return;
        }

        if run.error.is_none() {
            if let Some(ref on_end) = self.on_end {
                on_end(run);
            }
        } else if let Some(ref on_error) = self.on_error {
            on_error(run);
        }
    }
}

impl BaseTracer for RootListenersTracer {
    fn persist_run_impl(&mut self, _run: &Run) {
        // This is a legacy method only called once for an entire run tree
        // therefore not useful here
    }
}

/// Async tracer that calls async listeners on run start, end, and error.
pub struct AsyncRootListenersTracer {
    /// The tracer configuration.
    config: TracerCoreConfig,
    /// The run map.
    run_map: HashMap<String, Run>,
    /// The order map.
    order_map: HashMap<Uuid, (Uuid, String)>,
    /// The root run ID.
    root_id: Option<Uuid>,
    /// Async listener called on run start.
    on_start: Option<AsyncListener>,
    /// Async listener called on run end.
    on_end: Option<AsyncListener>,
    /// Async listener called on run error.
    on_error: Option<AsyncListener>,
}

impl fmt::Debug for AsyncRootListenersTracer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AsyncRootListenersTracer")
            .field("config", &self.config)
            .field("run_map", &self.run_map)
            .field("order_map", &self.order_map)
            .field("root_id", &self.root_id)
            .field("on_start", &self.on_start.as_ref().map(|_| "AsyncListener"))
            .field("on_end", &self.on_end.as_ref().map(|_| "AsyncListener"))
            .field("on_error", &self.on_error.as_ref().map(|_| "AsyncListener"))
            .finish()
    }
}

impl AsyncRootListenersTracer {
    /// Create a new AsyncRootListenersTracer.
    ///
    /// # Arguments
    ///
    /// * `on_start` - Async listener to call on run start.
    /// * `on_end` - Async listener to call on run end.
    /// * `on_error` - Async listener to call on run error.
    pub fn new(
        on_start: Option<AsyncListener>,
        on_end: Option<AsyncListener>,
        on_error: Option<AsyncListener>,
    ) -> Self {
        Self {
            config: TracerCoreConfig {
                schema_format: SchemaFormat::OriginalChat,
                log_missing_parent: false,
            },
            run_map: HashMap::new(),
            order_map: HashMap::new(),
            root_id: None,
            on_start,
            on_end,
            on_error,
        }
    }

    /// Get the root run ID.
    pub fn root_id(&self) -> Option<Uuid> {
        self.root_id
    }
}

impl TracerCore for AsyncRootListenersTracer {
    fn config(&self) -> &TracerCoreConfig {
        &self.config
    }

    fn config_mut(&mut self) -> &mut TracerCoreConfig {
        &mut self.config
    }

    fn run_map(&self) -> &HashMap<String, Run> {
        &self.run_map
    }

    fn run_map_mut(&mut self) -> &mut HashMap<String, Run> {
        &mut self.run_map
    }

    fn order_map(&self) -> &HashMap<Uuid, (Uuid, String)> {
        &self.order_map
    }

    fn order_map_mut(&mut self) -> &mut HashMap<Uuid, (Uuid, String)> {
        &mut self.order_map
    }

    fn persist_run(&mut self, _run: &Run) {}
}

#[async_trait]
impl AsyncBaseTracer for AsyncRootListenersTracer {
    async fn persist_run_async(&mut self, _run: &Run) {
        // This is a legacy method only called once for an entire run tree
        // therefore not useful here
    }

    async fn on_run_create_async(&mut self, run: &Run) {
        if self.root_id.is_some() {
            return;
        }

        self.root_id = Some(run.id);

        if let Some(ref on_start) = self.on_start {
            on_start(run).await;
        }
    }

    async fn on_run_update_async(&mut self, run: &Run) {
        if Some(run.id) != self.root_id {
            return;
        }

        if run.error.is_none() {
            if let Some(ref on_end) = self.on_end {
                on_end(run).await;
            }
        } else if let Some(ref on_error) = self.on_error {
            on_error(run).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_root_listeners_tracer_new() {
        let tracer = RootListenersTracer::new(None, None, None);
        assert!(tracer.root_id().is_none());
    }

    #[test]
    fn test_root_listeners_tracer_on_start() {
        let called = Arc::new(Mutex::new(false));
        let called_clone = called.clone();

        let mut tracer = RootListenersTracer::with_on_start(move |_run| {
            *called_clone.lock().unwrap() = true;
        });

        let run = Run::new(
            Uuid::new_v4(),
            "test",
            "chain",
            HashMap::new(),
            HashMap::new(),
        );

        tracer.on_run_create(&run);

        assert!(*called.lock().unwrap());
        assert_eq!(tracer.root_id(), Some(run.id));
    }

    #[test]
    fn test_root_listeners_tracer_on_end() {
        let called = Arc::new(Mutex::new(false));
        let called_clone = called.clone();

        let mut tracer = RootListenersTracer::with_on_end(move |_run| {
            *called_clone.lock().unwrap() = true;
        });

        let mut run = Run::new(
            Uuid::new_v4(),
            "test",
            "chain",
            HashMap::new(),
            HashMap::new(),
        );
        run.set_end();

        // Set root_id first
        tracer.on_run_create(&run);

        // Then call on_run_update
        tracer.on_run_update(&run);

        assert!(*called.lock().unwrap());
    }

    #[test]
    fn test_root_listeners_tracer_on_error() {
        let called = Arc::new(Mutex::new(false));
        let called_clone = called.clone();

        let mut tracer = RootListenersTracer::with_on_error(move |_run| {
            *called_clone.lock().unwrap() = true;
        });

        let mut run = Run::new(
            Uuid::new_v4(),
            "test",
            "chain",
            HashMap::new(),
            HashMap::new(),
        );
        run.set_error("Test error");

        // Set root_id first
        tracer.on_run_create(&run);

        // Then call on_run_update with error
        tracer.on_run_update(&run);

        assert!(*called.lock().unwrap());
    }

    #[test]
    fn test_root_listeners_only_root_run() {
        let call_count = Arc::new(Mutex::new(0));
        let call_count_clone = call_count.clone();

        let mut tracer = RootListenersTracer::with_on_start(move |_run| {
            *call_count_clone.lock().unwrap() += 1;
        });

        let run1 = Run::new(
            Uuid::new_v4(),
            "root",
            "chain",
            HashMap::new(),
            HashMap::new(),
        );
        let run2 = Run::new(
            Uuid::new_v4(),
            "child",
            "chain",
            HashMap::new(),
            HashMap::new(),
        );

        tracer.on_run_create(&run1);
        tracer.on_run_create(&run2);

        // Should only be called once for the root run
        assert_eq!(*call_count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_async_root_listeners_tracer() {
        let called = Arc::new(Mutex::new(false));
        let called_clone = called.clone();

        let on_start: AsyncListener = Box::new(move |_run: &Run| {
            let called = called_clone.clone();
            Box::pin(async move {
                *called.lock().unwrap() = true;
            })
        });

        let mut tracer = AsyncRootListenersTracer::new(Some(on_start), None, None);

        let run = Run::new(
            Uuid::new_v4(),
            "test",
            "chain",
            HashMap::new(),
            HashMap::new(),
        );

        tracer.on_run_create_async(&run).await;

        assert!(*called.lock().unwrap());
        assert_eq!(tracer.root_id(), Some(run.id));
    }
}
