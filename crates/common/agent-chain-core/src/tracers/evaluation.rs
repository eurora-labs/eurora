use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::tracers::base::BaseTracer;
use crate::tracers::core::{TracerCore, TracerCoreConfig};
use crate::tracers::schemas::Run;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub key: String,
    pub score: Option<f64>,
    pub value: Option<String>,
    pub comment: Option<String>,
    pub correction: Option<Value>,
    pub evaluator_info: Option<HashMap<String, Value>>,
}

pub trait RunEvaluator: Send + Sync {
    fn evaluate_run(&self, run: &Run) -> crate::error::Result<Vec<EvaluationResult>>;
}

static EVALUATOR_TRACERS: std::sync::LazyLock<Mutex<Vec<Weak<Mutex<EvaluatorCallbackHandler>>>>> =
    std::sync::LazyLock::new(|| Mutex::new(Vec::new()));

pub fn wait_for_all_evaluators() {
    if let Ok(mut tracers) = EVALUATOR_TRACERS.lock() {
        tracers.retain(|weak| weak.strong_count() > 0);
    }
}

fn register_evaluator(handler: &Arc<Mutex<EvaluatorCallbackHandler>>) {
    if let Ok(mut tracers) = EVALUATOR_TRACERS.lock() {
        tracers.push(Arc::downgrade(handler));
    }
}

pub struct EvaluatorCallbackHandler {
    config: TracerCoreConfig,
    run_map: HashMap<String, Run>,
    order_map: HashMap<Uuid, (Uuid, String)>,
    example_id: Option<Uuid>,
    evaluators: Vec<Box<dyn RunEvaluator>>,
    skip_unfinished: bool,
    logged_eval_results: HashMap<(String, String), Vec<EvaluationResult>>,
}

impl std::fmt::Debug for EvaluatorCallbackHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvaluatorCallbackHandler")
            .field("example_id", &self.example_id)
            .field("skip_unfinished", &self.skip_unfinished)
            .field("evaluator_count", &self.evaluators.len())
            .field("result_count", &self.logged_eval_results.len())
            .finish()
    }
}

impl EvaluatorCallbackHandler {
    pub fn new(
        evaluators: Vec<Box<dyn RunEvaluator>>,
        example_id: Option<Uuid>,
        skip_unfinished: bool,
    ) -> Self {
        Self {
            config: TracerCoreConfig::default(),
            run_map: HashMap::new(),
            order_map: HashMap::new(),
            example_id,
            evaluators,
            skip_unfinished,
            logged_eval_results: HashMap::new(),
        }
    }

    pub fn into_shared(self) -> Arc<Mutex<Self>> {
        let shared = Arc::new(Mutex::new(self));
        register_evaluator(&shared);
        shared
    }

    pub fn name(&self) -> &str {
        "evaluator_callback_handler"
    }

    pub fn get_results(&self) -> &HashMap<(String, String), Vec<EvaluationResult>> {
        &self.logged_eval_results
    }

    pub fn get_results_for_run(&self, run_id: &str) -> Vec<&EvaluationResult> {
        self.logged_eval_results
            .iter()
            .filter(|((rid, _), _)| rid == run_id)
            .flat_map(|(_, results)| results.iter())
            .collect()
    }

    pub fn clear_results(&mut self) {
        self.logged_eval_results.clear();
    }

    pub fn example_id(&self) -> Option<Uuid> {
        self.example_id
    }
}

impl TracerCore for EvaluatorCallbackHandler {
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

impl BaseTracer for EvaluatorCallbackHandler {
    fn persist_run_impl(&mut self, run: &Run) {
        if self.skip_unfinished && run.outputs.is_none() {
            tracing::debug!("Skipping unfinished run {}", run.id);
            return;
        }

        let mut run_copy = run.clone();
        run_copy.reference_example_id = self.example_id;

        let example_id = self.example_id.map(|id| id.to_string()).unwrap_or_default();
        let run_id = run_copy.id.to_string();

        for evaluator in &self.evaluators {
            match evaluator.evaluate_run(&run_copy) {
                Ok(eval_results) => {
                    let key = (run_id.clone(), example_id.clone());
                    self.logged_eval_results
                        .entry(key)
                        .or_default()
                        .extend(eval_results);
                }
                Err(error) => {
                    tracing::error!("Error evaluating run {}: {}", run_id, error);
                }
            }
        }
    }
}

pub struct NonEmptyOutputEvaluator;

impl std::fmt::Debug for NonEmptyOutputEvaluator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NonEmptyOutputEvaluator").finish()
    }
}

impl RunEvaluator for NonEmptyOutputEvaluator {
    fn evaluate_run(&self, run: &Run) -> crate::error::Result<Vec<EvaluationResult>> {
        let has_output = run
            .outputs
            .as_ref()
            .map(|outputs| !outputs.is_empty())
            .unwrap_or(false);

        Ok(vec![EvaluationResult {
            key: "has_output".to_string(),
            score: Some(if has_output { 1.0 } else { 0.0 }),
            value: Some(if has_output { "yes" } else { "no" }.to_string()),
            comment: None,
            correction: None,
            evaluator_info: None,
        }])
    }
}

pub struct LatencyEvaluator {
    pub max_seconds: f64,
}

impl std::fmt::Debug for LatencyEvaluator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LatencyEvaluator")
            .field("max_seconds", &self.max_seconds)
            .finish()
    }
}

impl RunEvaluator for LatencyEvaluator {
    fn evaluate_run(&self, run: &Run) -> crate::error::Result<Vec<EvaluationResult>> {
        let duration_secs = run
            .end_time
            .map(|end| (end - run.start_time).num_milliseconds() as f64 / 1000.0);

        let within_threshold = duration_secs
            .map(|d| d <= self.max_seconds)
            .unwrap_or(false);

        Ok(vec![EvaluationResult {
            key: "latency".to_string(),
            score: duration_secs
                .map(|d| ((self.max_seconds - d) / self.max_seconds).clamp(0.0, 1.0)),
            value: duration_secs.map(|d| format!("{:.3}s", d)),
            comment: if within_threshold {
                Some("Within threshold".to_string())
            } else {
                duration_secs
                    .map(|d| format!("Exceeded {:.1}s threshold ({:.3}s)", self.max_seconds, d))
            },
            correction: None,
            evaluator_info: None,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn make_run(name: &str, outputs: Option<HashMap<String, Value>>) -> Run {
        let mut run = Run::new(
            Uuid::new_v4(),
            name,
            "chain",
            HashMap::new(),
            HashMap::new(),
        );
        run.outputs = outputs;
        run.set_end();
        run
    }

    fn make_run_with_duration(name: &str, duration_ms: i64) -> Run {
        let mut run = Run::new(
            Uuid::new_v4(),
            name,
            "chain",
            HashMap::new(),
            HashMap::new(),
        );
        run.outputs = Some(HashMap::from([(
            "output".to_string(),
            Value::String("result".to_string()),
        )]));
        let start = Utc::now() - Duration::milliseconds(duration_ms);
        run.start_time = start;
        run.end_time = Some(Utc::now());
        run
    }

    #[test]
    fn test_evaluation_result_serialization() {
        let result = EvaluationResult {
            key: "test".to_string(),
            score: Some(0.95),
            value: Some("good".to_string()),
            comment: Some("Looks correct".to_string()),
            correction: None,
            evaluator_info: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: EvaluationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.key, "test");
        assert_eq!(deserialized.score, Some(0.95));
    }

    #[test]
    fn test_non_empty_output_evaluator_with_output() {
        let evaluator = NonEmptyOutputEvaluator;
        let run = make_run(
            "test",
            Some(HashMap::from([(
                "output".to_string(),
                Value::String("result".to_string()),
            )])),
        );
        let results = evaluator.evaluate_run(&run).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "has_output");
        assert_eq!(results[0].score, Some(1.0));
        assert_eq!(results[0].value, Some("yes".to_string()));
    }

    #[test]
    fn test_non_empty_output_evaluator_without_output() {
        let evaluator = NonEmptyOutputEvaluator;
        let run = make_run("test", None);
        let results = evaluator.evaluate_run(&run).unwrap();
        assert_eq!(results[0].score, Some(0.0));
        assert_eq!(results[0].value, Some("no".to_string()));
    }

    #[test]
    fn test_non_empty_output_evaluator_empty_map() {
        let evaluator = NonEmptyOutputEvaluator;
        let run = make_run("test", Some(HashMap::new()));
        let results = evaluator.evaluate_run(&run).unwrap();
        assert_eq!(results[0].score, Some(0.0));
        assert_eq!(results[0].value, Some("no".to_string()));
    }

    #[test]
    fn test_latency_evaluator_within_threshold() {
        let evaluator = LatencyEvaluator { max_seconds: 5.0 };
        let run = make_run_with_duration("fast_run", 100);
        let results = evaluator.evaluate_run(&run).unwrap();
        assert_eq!(results[0].key, "latency");
        let score = results[0].score.unwrap();
        assert!(score > 0.9, "Expected high score, got {}", score);
        assert_eq!(results[0].comment, Some("Within threshold".to_string()));
    }

    #[test]
    fn test_latency_evaluator_exceeds_threshold() {
        let evaluator = LatencyEvaluator { max_seconds: 0.01 };
        let run = make_run_with_duration("slow_run", 500);
        let results = evaluator.evaluate_run(&run).unwrap();
        assert_eq!(results[0].score, Some(0.0));
        assert!(results[0].comment.as_ref().unwrap().contains("Exceeded"));
    }

    #[test]
    fn test_latency_evaluator_no_end_time() {
        let evaluator = LatencyEvaluator { max_seconds: 5.0 };
        let mut run = Run::new(
            Uuid::new_v4(),
            "unfinished",
            "chain",
            HashMap::new(),
            HashMap::new(),
        );
        run.outputs = Some(HashMap::new());
        let results = evaluator.evaluate_run(&run).unwrap();
        assert_eq!(results[0].score, None);
        assert_eq!(results[0].value, None);
    }

    #[test]
    fn test_evaluator_callback_handler_basic() {
        let mut handler =
            EvaluatorCallbackHandler::new(vec![Box::new(NonEmptyOutputEvaluator)], None, true);

        let run = make_run(
            "test_chain",
            Some(HashMap::from([(
                "output".to_string(),
                Value::String("result".to_string()),
            )])),
        );

        handler.persist_run_impl(&run);

        let results = handler.get_results_for_run(&run.id.to_string());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "has_output");
        assert_eq!(results[0].score, Some(1.0));
    }

    #[test]
    fn test_evaluator_callback_handler_skip_unfinished() {
        let mut handler =
            EvaluatorCallbackHandler::new(vec![Box::new(NonEmptyOutputEvaluator)], None, true);

        let run = make_run("unfinished_chain", None);
        handler.persist_run_impl(&run);

        assert!(handler.get_results().is_empty());
    }

    #[test]
    fn test_evaluator_callback_handler_no_skip() {
        let mut handler =
            EvaluatorCallbackHandler::new(vec![Box::new(NonEmptyOutputEvaluator)], None, false);

        let run = make_run("unfinished_chain", None);
        handler.persist_run_impl(&run);

        let results = handler.get_results_for_run(&run.id.to_string());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].score, Some(0.0));
    }

    #[test]
    fn test_evaluator_callback_handler_multiple_evaluators() {
        let mut handler = EvaluatorCallbackHandler::new(
            vec![
                Box::new(NonEmptyOutputEvaluator),
                Box::new(LatencyEvaluator { max_seconds: 5.0 }),
            ],
            None,
            true,
        );

        let run = make_run_with_duration("multi_eval", 100);
        handler.persist_run_impl(&run);

        let results = handler.get_results_for_run(&run.id.to_string());
        assert_eq!(results.len(), 2);

        let keys: Vec<&str> = results.iter().map(|r| r.key.as_str()).collect();
        assert!(keys.contains(&"has_output"));
        assert!(keys.contains(&"latency"));
    }

    #[test]
    fn test_evaluator_callback_handler_with_example_id() {
        let example_id = Uuid::new_v4();
        let mut handler = EvaluatorCallbackHandler::new(
            vec![Box::new(NonEmptyOutputEvaluator)],
            Some(example_id),
            true,
        );

        assert_eq!(handler.example_id(), Some(example_id));

        let run = make_run(
            "with_example",
            Some(HashMap::from([(
                "output".to_string(),
                Value::String("result".to_string()),
            )])),
        );
        handler.persist_run_impl(&run);

        let key = (run.id.to_string(), example_id.to_string());
        assert!(handler.get_results().contains_key(&key));
    }

    #[test]
    fn test_evaluator_callback_handler_multiple_runs() {
        let mut handler =
            EvaluatorCallbackHandler::new(vec![Box::new(NonEmptyOutputEvaluator)], None, true);

        let run1 = make_run(
            "run1",
            Some(HashMap::from([(
                "output".to_string(),
                Value::String("r1".to_string()),
            )])),
        );
        let run2 = make_run(
            "run2",
            Some(HashMap::from([(
                "output".to_string(),
                Value::String("r2".to_string()),
            )])),
        );

        handler.persist_run_impl(&run1);
        handler.persist_run_impl(&run2);

        assert_eq!(handler.get_results().len(), 2);
        assert_eq!(handler.get_results_for_run(&run1.id.to_string()).len(), 1);
        assert_eq!(handler.get_results_for_run(&run2.id.to_string()).len(), 1);
    }

    #[test]
    fn test_evaluator_callback_handler_clear_results() {
        let mut handler =
            EvaluatorCallbackHandler::new(vec![Box::new(NonEmptyOutputEvaluator)], None, true);

        let run = make_run(
            "clear_test",
            Some(HashMap::from([(
                "output".to_string(),
                Value::String("result".to_string()),
            )])),
        );
        handler.persist_run_impl(&run);
        assert!(!handler.get_results().is_empty());

        handler.clear_results();
        assert!(handler.get_results().is_empty());
    }

    #[test]
    fn test_evaluator_error_handling() {
        struct FailingEvaluator;
        impl RunEvaluator for FailingEvaluator {
            fn evaluate_run(&self, _run: &Run) -> crate::error::Result<Vec<EvaluationResult>> {
                Err(crate::error::Error::Other("evaluation failed".into()))
            }
        }

        let mut handler =
            EvaluatorCallbackHandler::new(vec![Box::new(FailingEvaluator)], None, false);

        let run = make_run(
            "error_test",
            Some(HashMap::from([(
                "output".to_string(),
                Value::String("result".to_string()),
            )])),
        );
        handler.persist_run_impl(&run);
        assert!(handler.get_results().is_empty());
    }

    #[test]
    fn test_evaluator_callback_handler_name() {
        let handler = EvaluatorCallbackHandler::new(vec![], None, true);
        assert_eq!(handler.name(), "evaluator_callback_handler");
    }

    #[test]
    fn test_wait_for_all_evaluators() {
        wait_for_all_evaluators();
    }

    #[test]
    fn test_into_shared() {
        let handler =
            EvaluatorCallbackHandler::new(vec![Box::new(NonEmptyOutputEvaluator)], None, true);
        let shared = handler.into_shared();
        let guard = shared.lock().unwrap();
        assert_eq!(guard.name(), "evaluator_callback_handler");
    }
}
