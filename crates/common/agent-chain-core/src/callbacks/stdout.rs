use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use uuid::Uuid;

use super::base::{BaseCallbackHandler, resolve_chain_name};

pub mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
    pub const WHITE: &str = "\x1b[37m";
}

#[derive(Clone)]
pub struct StdOutCallbackHandler {
    color: Option<String>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl std::fmt::Debug for StdOutCallbackHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StdOutCallbackHandler")
            .field("color", &self.color)
            .finish()
    }
}

impl Default for StdOutCallbackHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl StdOutCallbackHandler {
    pub fn new() -> Self {
        Self {
            color: None,
            writer: Arc::new(Mutex::new(Box::new(io::stdout()))),
        }
    }

    pub fn with_color(color: impl Into<String>) -> Self {
        Self {
            color: Some(color.into()),
            writer: Arc::new(Mutex::new(Box::new(io::stdout()))),
        }
    }

    pub fn with_writer(writer: Arc<Mutex<Box<dyn Write + Send>>>) -> Self {
        Self {
            color: None,
            writer,
        }
    }

    pub fn set_color(&mut self, color: impl Into<String>) {
        self.color = Some(color.into());
    }

    pub fn color(&self) -> Option<&str> {
        self.color.as_deref()
    }

    fn write_text(&self, text: &str, color: Option<&str>, end: &str) {
        if let Ok(mut w) = self.writer.lock() {
            let result = if let Some(c) = color {
                write!(w, "{}{}{}{}", c, text, colors::RESET, end)
            } else {
                write!(w, "{}{}", text, end)
            };
            if let Err(e) = result {
                tracing::warn!("StdOutCallbackHandler write error: {e}");
            }
            if let Err(e) = w.flush() {
                tracing::warn!("StdOutCallbackHandler flush error: {e}");
            }
        }
    }
}

impl BaseCallbackHandler for StdOutCallbackHandler {
    fn name(&self) -> &str {
        "StdOutCallbackHandler"
    }

    fn on_tool_end(
        &self,
        output: &str,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        color: Option<&str>,
        observation_prefix: Option<&str>,
        llm_prefix: Option<&str>,
    ) {
        if let Some(prefix) = observation_prefix {
            self.write_text(&format!("\n{}", prefix), None, "");
        }
        let effective_color = color.or(self.color());
        self.write_text(output, effective_color, "");
        if let Some(prefix) = llm_prefix {
            self.write_text(&format!("\n{}", prefix), None, "");
        }
    }

    fn on_text(
        &self,
        text: &str,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        color: Option<&str>,
        end: &str,
    ) {
        let effective_color = color.or(self.color());
        self.write_text(text, effective_color, end);
    }

    fn on_chain_start(
        &self,
        serialized: &HashMap<String, serde_json::Value>,
        _inputs: &HashMap<String, serde_json::Value>,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _tags: Option<&[String]>,
        _metadata: Option<&HashMap<String, serde_json::Value>>,
        name: Option<&str>,
    ) {
        let name = resolve_chain_name(serialized, name);

        if let Ok(mut w) = self.writer.lock() {
            if let Err(e) = writeln!(
                w,
                "\n\n{}> Entering new {} chain...{}",
                colors::BOLD,
                name,
                colors::RESET
            ) {
                tracing::warn!("StdOutCallbackHandler write error: {e}");
            }
            if let Err(e) = w.flush() {
                tracing::warn!("StdOutCallbackHandler flush error: {e}");
            }
        }
    }

    fn on_chain_end(
        &self,
        _outputs: &HashMap<String, serde_json::Value>,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        if let Ok(mut w) = self.writer.lock() {
            if let Err(e) = writeln!(w, "\n{}> Finished chain.{}", colors::BOLD, colors::RESET) {
                tracing::warn!("StdOutCallbackHandler write error: {e}");
            }
            if let Err(e) = w.flush() {
                tracing::warn!("StdOutCallbackHandler flush error: {e}");
            }
        }
    }

    fn on_agent_action(
        &self,
        action: &serde_json::Value,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        color: Option<&str>,
    ) {
        if let Some(log) = action.get("log").and_then(|v| v.as_str()) {
            let effective_color = color.or(self.color());
            self.write_text(log, effective_color, "");
        }
    }

    fn on_agent_finish(
        &self,
        finish: &serde_json::Value,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        color: Option<&str>,
    ) {
        if let Some(log) = finish.get("log").and_then(|v| v.as_str()) {
            let effective_color = color.or(self.color());
            self.write_text(log, effective_color, "\n");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdout_handler_creation() {
        let handler = StdOutCallbackHandler::new();
        assert!(handler.color().is_none());
        assert_eq!(handler.name(), "StdOutCallbackHandler");
    }

    #[test]
    fn test_stdout_handler_with_color() {
        let handler = StdOutCallbackHandler::with_color(colors::GREEN);
        assert_eq!(handler.color(), Some(colors::GREEN));
    }
}
