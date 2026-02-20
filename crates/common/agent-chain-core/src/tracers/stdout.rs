use std::collections::HashMap;

use serde_json::Value;
use uuid::Uuid;

use crate::callbacks::base::{
    BaseCallbackHandler, CallbackManagerMixin, ChainManagerMixin, LLMManagerMixin,
    RetrieverManagerMixin, RunManagerMixin, ToolManagerMixin,
};
use crate::tracers::base::BaseTracer;
use crate::tracers::core::{TracerCore, TracerCoreConfig};
use crate::tracers::schemas::Run;
use crate::utils::input::{get_bolded_text, get_colored_text};

const MILLISECONDS_IN_SECOND: f64 = 1000.0;

pub fn try_json_stringify(obj: &Value, fallback: &str) -> String {
    serde_json::to_string_pretty(obj).unwrap_or_else(|_| fallback.to_string())
}

pub fn elapsed(run: &Run) -> String {
    if let Some(end_time) = run.end_time {
        let duration = end_time.signed_duration_since(run.start_time);
        let seconds = duration.num_milliseconds() as f64 / MILLISECONDS_IN_SECOND;
        if seconds < 1.0 {
            format!("{:.0}ms", seconds * MILLISECONDS_IN_SECOND)
        } else {
            format!("{:.2}s", seconds)
        }
    } else {
        "N/A".to_string()
    }
}

#[derive(Debug)]
pub struct FunctionCallbackHandler<F>
where
    F: Fn(&str) + Send + Sync,
{
    function_callback: F,
    config: TracerCoreConfig,
    run_map: HashMap<String, Run>,
    order_map: HashMap<Uuid, (Uuid, String)>,
}

impl<F> FunctionCallbackHandler<F>
where
    F: Fn(&str) + Send + Sync,
{
    pub fn new(function: F) -> Self {
        Self {
            function_callback: function,
            config: TracerCoreConfig::default(),
            run_map: HashMap::new(),
            order_map: HashMap::new(),
        }
    }

    pub fn get_parents(&self, run: &Run) -> Vec<Run> {
        let mut parents = Vec::new();
        let mut current_run = run.clone();

        while let Some(parent_run_id) = current_run.parent_run_id {
            if let Some(parent) = self.run_map.get(&parent_run_id.to_string()) {
                parents.push(parent.clone());
                current_run = parent.clone();
            } else {
                break;
            }
        }

        parents
    }

    pub fn get_breadcrumbs(&self, run: &Run) -> String {
        let parents: Vec<Run> = self.get_parents(run).into_iter().rev().collect();
        let mut all_runs = parents;
        all_runs.push(run.clone());

        all_runs
            .iter()
            .map(|r| format!("{}:{}", r.run_type, r.name))
            .collect::<Vec<_>>()
            .join(" > ")
    }
}

impl<F> TracerCore for FunctionCallbackHandler<F>
where
    F: Fn(&str) + Send + Sync + std::fmt::Debug,
{
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

    fn on_chain_start(&mut self, run: &Run) {
        let crumbs = self.get_breadcrumbs(run);
        let run_type = capitalize_first(&run.run_type);
        let inputs = serde_json::to_value(&run.inputs).unwrap_or_default();
        (self.function_callback)(&format!(
            "{} {}{}",
            get_colored_text("[chain/start]", "green"),
            get_bolded_text(&format!(
                "[{}] Entering {} run with input:\n",
                crumbs, run_type
            )),
            try_json_stringify(&inputs, "[inputs]")
        ));
    }

    fn on_chain_end(&mut self, run: &Run) {
        let crumbs = self.get_breadcrumbs(run);
        let run_type = capitalize_first(&run.run_type);
        let outputs = run
            .outputs
            .as_ref()
            .map(|o| serde_json::to_value(o).unwrap_or_default())
            .unwrap_or_default();
        (self.function_callback)(&format!(
            "{} {}{}",
            get_colored_text("[chain/end]", "blue"),
            get_bolded_text(&format!(
                "[{}] [{}] Exiting {} run with output:\n",
                crumbs,
                elapsed(run),
                run_type
            )),
            try_json_stringify(&outputs, "[outputs]")
        ));
    }

    fn on_chain_error(&mut self, run: &Run) {
        let crumbs = self.get_breadcrumbs(run);
        let run_type = capitalize_first(&run.run_type);
        let error = run
            .error
            .as_ref()
            .map(|e| Value::String(e.clone()))
            .unwrap_or_default();
        (self.function_callback)(&format!(
            "{} {}{}",
            get_colored_text("[chain/error]", "red"),
            get_bolded_text(&format!(
                "[{}] [{}] {} run errored with error:\n",
                crumbs,
                elapsed(run),
                run_type
            )),
            try_json_stringify(&error, "[error]")
        ));
    }

    fn on_llm_start(&mut self, run: &Run) {
        let crumbs = self.get_breadcrumbs(run);
        let inputs = if let Some(Value::Array(arr)) = run.inputs.get("prompts") {
            let trimmed: Vec<Value> = arr
                .iter()
                .map(|p| {
                    if let Value::String(s) = p {
                        Value::String(s.trim().to_string())
                    } else {
                        p.clone()
                    }
                })
                .collect();
            serde_json::json!({ "prompts": trimmed })
        } else {
            serde_json::to_value(&run.inputs).unwrap_or_default()
        };

        (self.function_callback)(&format!(
            "{} {}{}",
            get_colored_text("[llm/start]", "green"),
            get_bolded_text(&format!("[{}] Entering LLM run with input:\n", crumbs)),
            try_json_stringify(&inputs, "[inputs]")
        ));
    }

    fn on_llm_end(&mut self, run: &Run) {
        let crumbs = self.get_breadcrumbs(run);
        let outputs = run
            .outputs
            .as_ref()
            .map(|o| serde_json::to_value(o).unwrap_or_default())
            .unwrap_or_default();
        (self.function_callback)(&format!(
            "{} {}{}",
            get_colored_text("[llm/end]", "blue"),
            get_bolded_text(&format!(
                "[{}] [{}] Exiting LLM run with output:\n",
                crumbs,
                elapsed(run)
            )),
            try_json_stringify(&outputs, "[response]")
        ));
    }

    fn on_llm_error(&mut self, run: &Run) {
        let crumbs = self.get_breadcrumbs(run);
        let error = run
            .error
            .as_ref()
            .map(|e| Value::String(e.clone()))
            .unwrap_or_default();
        (self.function_callback)(&format!(
            "{} {}{}",
            get_colored_text("[llm/error]", "red"),
            get_bolded_text(&format!(
                "[{}] [{}] LLM run errored with error:\n",
                crumbs,
                elapsed(run)
            )),
            try_json_stringify(&error, "[error]")
        ));
    }

    fn on_tool_start(&mut self, run: &Run) {
        let crumbs = self.get_breadcrumbs(run);
        let input = run
            .inputs
            .get("input")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        (self.function_callback)(&format!(
            "{} {}\"{}\"",
            get_colored_text("[tool/start]", "green"),
            get_bolded_text(&format!("[{}] Entering Tool run with input:\n", crumbs)),
            input
        ));
    }

    fn on_tool_end(&mut self, run: &Run) {
        let crumbs = self.get_breadcrumbs(run);
        if let Some(outputs) = &run.outputs
            && let Some(output) = outputs.get("output")
        {
            let output_str = match output {
                Value::String(s) => s.trim().to_string(),
                _ => output.to_string(),
            };
            (self.function_callback)(&format!(
                "{} {}\"{}\"",
                get_colored_text("[tool/end]", "blue"),
                get_bolded_text(&format!(
                    "[{}] [{}] Exiting Tool run with output:\n",
                    crumbs,
                    elapsed(run)
                )),
                output_str
            ));
        }
    }

    fn on_tool_error(&mut self, run: &Run) {
        let crumbs = self.get_breadcrumbs(run);
        let error = run.error.as_deref().unwrap_or("");
        (self.function_callback)(&format!(
            "{} {}Tool run errored with error:\n{}",
            get_colored_text("[tool/error]", "red"),
            get_bolded_text(&format!("[{}] [{}] ", crumbs, elapsed(run))),
            error
        ));
    }
}

impl<F> BaseTracer for FunctionCallbackHandler<F>
where
    F: Fn(&str) + Send + Sync + std::fmt::Debug,
{
    fn persist_run_impl(&mut self, _run: &Run) {}
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[derive(Debug)]
pub struct ConsoleCallbackHandler {
    inner: FunctionCallbackHandler<fn(&str)>,
}

impl ConsoleCallbackHandler {
    pub fn new() -> Self {
        fn print_fn(s: &str) {
            println!("{}", s);
        }
        Self {
            inner: FunctionCallbackHandler::new(print_fn as fn(&str)),
        }
    }
}

impl Default for ConsoleCallbackHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl TracerCore for ConsoleCallbackHandler {
    fn config(&self) -> &TracerCoreConfig {
        self.inner.config()
    }

    fn config_mut(&mut self) -> &mut TracerCoreConfig {
        self.inner.config_mut()
    }

    fn run_map(&self) -> &HashMap<String, Run> {
        self.inner.run_map()
    }

    fn run_map_mut(&mut self) -> &mut HashMap<String, Run> {
        self.inner.run_map_mut()
    }

    fn order_map(&self) -> &HashMap<Uuid, (Uuid, String)> {
        self.inner.order_map()
    }

    fn order_map_mut(&mut self) -> &mut HashMap<Uuid, (Uuid, String)> {
        self.inner.order_map_mut()
    }

    fn persist_run(&mut self, run: &Run) {
        self.inner.persist_run(run)
    }

    fn on_chain_start(&mut self, run: &Run) {
        self.inner.on_chain_start(run)
    }

    fn on_chain_end(&mut self, run: &Run) {
        self.inner.on_chain_end(run)
    }

    fn on_chain_error(&mut self, run: &Run) {
        self.inner.on_chain_error(run)
    }

    fn on_llm_start(&mut self, run: &Run) {
        self.inner.on_llm_start(run)
    }

    fn on_llm_end(&mut self, run: &Run) {
        self.inner.on_llm_end(run)
    }

    fn on_llm_error(&mut self, run: &Run) {
        self.inner.on_llm_error(run)
    }

    fn on_tool_start(&mut self, run: &Run) {
        self.inner.on_tool_start(run)
    }

    fn on_tool_end(&mut self, run: &Run) {
        self.inner.on_tool_end(run)
    }

    fn on_tool_error(&mut self, run: &Run) {
        self.inner.on_tool_error(run)
    }
}

impl BaseTracer for ConsoleCallbackHandler {
    fn persist_run_impl(&mut self, run: &Run) {
        self.inner.persist_run_impl(run)
    }
}

impl LLMManagerMixin for ConsoleCallbackHandler {}
impl ChainManagerMixin for ConsoleCallbackHandler {}
impl ToolManagerMixin for ConsoleCallbackHandler {}
impl RetrieverManagerMixin for ConsoleCallbackHandler {}
impl CallbackManagerMixin for ConsoleCallbackHandler {}
impl RunManagerMixin for ConsoleCallbackHandler {}

impl BaseCallbackHandler for ConsoleCallbackHandler {
    fn name(&self) -> &str {
        "ConsoleCallbackHandler"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_json_stringify() {
        let obj = serde_json::json!({"key": "value"});
        let result = try_json_stringify(&obj, "fallback");
        assert!(result.contains("key"));
        assert!(result.contains("value"));
    }

    #[test]
    fn test_try_json_stringify_fallback() {
        let obj = serde_json::json!(null);
        let result = try_json_stringify(&obj, "fallback");
        assert_eq!(result, "null");
    }

    #[test]
    fn test_capitalize_first() {
        assert_eq!(capitalize_first("hello"), "Hello");
        assert_eq!(capitalize_first("HELLO"), "HELLO");
        assert_eq!(capitalize_first(""), "");
        assert_eq!(capitalize_first("chain"), "Chain");
    }

    #[test]
    fn test_elapsed() {
        let mut run = Run::default();
        assert_eq!(elapsed(&run), "N/A");

        run.end_time = Some(run.start_time + chrono::Duration::milliseconds(500));
        assert!(elapsed(&run).contains("ms"));

        run.end_time = Some(run.start_time + chrono::Duration::seconds(2));
        assert!(elapsed(&run).contains("s"));
    }

    #[test]
    fn test_console_callback_handler_creation() {
        let handler = ConsoleCallbackHandler::new();
        assert!(handler.run_map().is_empty());
    }

    #[test]
    fn test_console_callback_handler_get_breadcrumbs() {
        let handler = ConsoleCallbackHandler::new();

        let run = Run::new(
            Uuid::new_v4(),
            "test_run",
            "chain",
            HashMap::new(),
            HashMap::new(),
        );

        let breadcrumbs = handler.inner.get_breadcrumbs(&run);
        assert_eq!(breadcrumbs, "chain:test_run");
    }
}
