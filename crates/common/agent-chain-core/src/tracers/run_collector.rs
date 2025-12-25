//! A tracer that collects all nested runs in a list.
//!
//! This module provides a tracer that is useful for inspection and evaluation purposes.
//! Mirrors `langchain_core.tracers.run_collector`.

use std::collections::HashMap;

use uuid::Uuid;

use crate::tracers::base::BaseTracer;
use crate::tracers::core::{TracerCore, TracerCoreConfig};
use crate::tracers::schemas::Run;

/// Tracer that collects all nested runs in a list.
///
/// This tracer is useful for inspection and evaluation purposes.
/// It collects all runs including nested child runs.
#[derive(Debug)]
pub struct RunCollectorCallbackHandler {
    /// The tracer configuration.
    config: TracerCoreConfig,
    /// The run map.
    run_map: HashMap<String, Run>,
    /// The order map.
    order_map: HashMap<Uuid, (Uuid, String)>,
    /// The example ID being traced.
    example_id: Option<Uuid>,
    /// List of traced runs.
    pub traced_runs: Vec<Run>,
}

impl RunCollectorCallbackHandler {
    /// Create a new RunCollectorCallbackHandler.
    ///
    /// # Arguments
    ///
    /// * `example_id` - The ID of the example being traced. (default: None).
    pub fn new(example_id: Option<Uuid>) -> Self {
        Self {
            config: TracerCoreConfig::default(),
            run_map: HashMap::new(),
            order_map: HashMap::new(),
            example_id,
            traced_runs: Vec::new(),
        }
    }

    /// Create a new RunCollectorCallbackHandler with an example ID from a string.
    ///
    /// # Arguments
    ///
    /// * `example_id` - The ID of the example being traced as a string.
    pub fn with_example_id_str(example_id: &str) -> Result<Self, uuid::Error> {
        let uuid = Uuid::parse_str(example_id)?;
        Ok(Self::new(Some(uuid)))
    }

    /// Get the name of the handler.
    pub fn name(&self) -> &str {
        "run-collector_callback_handler"
    }

    /// Get the example ID.
    pub fn example_id(&self) -> Option<Uuid> {
        self.example_id
    }

    /// Get the number of traced runs.
    pub fn len(&self) -> usize {
        self.traced_runs.len()
    }

    /// Check if there are any traced runs.
    pub fn is_empty(&self) -> bool {
        self.traced_runs.is_empty()
    }

    /// Clear all traced runs.
    pub fn clear(&mut self) {
        self.traced_runs.clear();
    }

    /// Get the most recent run.
    pub fn latest_run(&self) -> Option<&Run> {
        self.traced_runs.last()
    }

    /// Get runs by type.
    ///
    /// # Arguments
    ///
    /// * `run_type` - The type of runs to get (e.g., "chain", "llm", "tool").
    pub fn runs_by_type(&self, run_type: &str) -> Vec<&Run> {
        self.traced_runs
            .iter()
            .filter(|r| r.run_type == run_type)
            .collect()
    }

    /// Get runs by name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of runs to get.
    pub fn runs_by_name(&self, name: &str) -> Vec<&Run> {
        self.traced_runs.iter().filter(|r| r.name == name).collect()
    }

    /// Get runs that have errors.
    pub fn errored_runs(&self) -> Vec<&Run> {
        self.traced_runs
            .iter()
            .filter(|r| r.error.is_some())
            .collect()
    }

    /// Get successful runs (completed without error).
    pub fn successful_runs(&self) -> Vec<&Run> {
        self.traced_runs
            .iter()
            .filter(|r| r.error.is_none() && r.end_time.is_some())
            .collect()
    }
}

impl Default for RunCollectorCallbackHandler {
    fn default() -> Self {
        Self::new(None)
    }
}

impl TracerCore for RunCollectorCallbackHandler {
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

impl BaseTracer for RunCollectorCallbackHandler {
    fn persist_run_impl(&mut self, run: &Run) {
        let mut run_copy = run.clone();
        run_copy.reference_example_id = self.example_id;
        self.traced_runs.push(run_copy);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_collector_new() {
        let collector = RunCollectorCallbackHandler::new(None);
        assert!(collector.traced_runs.is_empty());
        assert!(collector.example_id.is_none());
    }

    #[test]
    fn test_run_collector_with_example_id() {
        let example_id = Uuid::new_v4();
        let collector = RunCollectorCallbackHandler::new(Some(example_id));
        assert_eq!(collector.example_id(), Some(example_id));
    }

    #[test]
    fn test_run_collector_with_example_id_str() {
        let uuid = Uuid::new_v4();
        let collector =
            RunCollectorCallbackHandler::with_example_id_str(&uuid.to_string()).unwrap();
        assert_eq!(collector.example_id(), Some(uuid));
    }

    #[test]
    fn test_persist_run() {
        let example_id = Uuid::new_v4();
        let mut collector = RunCollectorCallbackHandler::new(Some(example_id));

        let run = Run::new(
            Uuid::new_v4(),
            "test_run",
            "chain",
            HashMap::new(),
            HashMap::new(),
        );

        collector.persist_run_impl(&run);

        assert_eq!(collector.len(), 1);
        assert_eq!(
            collector.traced_runs[0].reference_example_id,
            Some(example_id)
        );
    }

    #[test]
    fn test_runs_by_type() {
        let mut collector = RunCollectorCallbackHandler::new(None);

        let chain_run = Run::new(
            Uuid::new_v4(),
            "chain1",
            "chain",
            HashMap::new(),
            HashMap::new(),
        );
        let tool_run = Run::new(
            Uuid::new_v4(),
            "tool1",
            "tool",
            HashMap::new(),
            HashMap::new(),
        );

        collector.persist_run_impl(&chain_run);
        collector.persist_run_impl(&tool_run);

        let chain_runs = collector.runs_by_type("chain");
        assert_eq!(chain_runs.len(), 1);
        assert_eq!(chain_runs[0].name, "chain1");

        let tool_runs = collector.runs_by_type("tool");
        assert_eq!(tool_runs.len(), 1);
        assert_eq!(tool_runs[0].name, "tool1");
    }

    #[test]
    fn test_runs_by_name() {
        let mut collector = RunCollectorCallbackHandler::new(None);

        let run1 = Run::new(
            Uuid::new_v4(),
            "my_chain",
            "chain",
            HashMap::new(),
            HashMap::new(),
        );
        let run2 = Run::new(
            Uuid::new_v4(),
            "other_chain",
            "chain",
            HashMap::new(),
            HashMap::new(),
        );

        collector.persist_run_impl(&run1);
        collector.persist_run_impl(&run2);

        let runs = collector.runs_by_name("my_chain");
        assert_eq!(runs.len(), 1);
    }

    #[test]
    fn test_errored_runs() {
        let mut collector = RunCollectorCallbackHandler::new(None);

        let mut success_run = Run::new(
            Uuid::new_v4(),
            "success",
            "chain",
            HashMap::new(),
            HashMap::new(),
        );
        success_run.set_end();

        let mut error_run = Run::new(
            Uuid::new_v4(),
            "error",
            "chain",
            HashMap::new(),
            HashMap::new(),
        );
        error_run.set_error("Something went wrong");

        collector.persist_run_impl(&success_run);
        collector.persist_run_impl(&error_run);

        let errored = collector.errored_runs();
        assert_eq!(errored.len(), 1);
        assert_eq!(errored[0].name, "error");

        let successful = collector.successful_runs();
        assert_eq!(successful.len(), 1);
        assert_eq!(successful[0].name, "success");
    }

    #[test]
    fn test_latest_run() {
        let mut collector = RunCollectorCallbackHandler::new(None);
        assert!(collector.latest_run().is_none());

        let run1 = Run::new(
            Uuid::new_v4(),
            "first",
            "chain",
            HashMap::new(),
            HashMap::new(),
        );
        let run2 = Run::new(
            Uuid::new_v4(),
            "second",
            "chain",
            HashMap::new(),
            HashMap::new(),
        );

        collector.persist_run_impl(&run1);
        collector.persist_run_impl(&run2);

        assert_eq!(collector.latest_run().unwrap().name, "second");
    }

    #[test]
    fn test_clear() {
        let mut collector = RunCollectorCallbackHandler::new(None);

        let run = Run::new(
            Uuid::new_v4(),
            "test",
            "chain",
            HashMap::new(),
            HashMap::new(),
        );
        collector.persist_run_impl(&run);

        assert!(!collector.is_empty());
        collector.clear();
        assert!(collector.is_empty());
    }
}
