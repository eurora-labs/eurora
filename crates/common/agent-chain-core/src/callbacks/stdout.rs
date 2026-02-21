use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use uuid::Uuid;

use super::base::{
    BaseCallbackHandler, CallbackManagerMixin, ChainManagerMixin, LLMManagerMixin,
    RetrieverManagerMixin, RunManagerMixin, ToolManagerMixin,
};

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

fn write_text(writer: &Mutex<Box<dyn Write + Send>>, text: &str, color: Option<&str>, end: &str) {
    if let Ok(mut w) = writer.lock() {
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

#[derive(Clone)]
pub struct StdOutCallbackHandler {
    pub color: Option<String>,
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

    fn get_color(&self) -> Option<&str> {
        self.color.as_deref()
    }
}

impl LLMManagerMixin for StdOutCallbackHandler {}
impl RetrieverManagerMixin for StdOutCallbackHandler {}

impl ToolManagerMixin for StdOutCallbackHandler {
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
            write_text(&self.writer, &format!("\n{}", prefix), None, "");
        }
        let effective_color = color.or(self.get_color());
        write_text(&self.writer, output, effective_color, "");
        if let Some(prefix) = llm_prefix {
            write_text(&self.writer, &format!("\n{}", prefix), None, "");
        }
    }
}

impl RunManagerMixin for StdOutCallbackHandler {
    fn on_text(
        &self,
        text: &str,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        color: Option<&str>,
        end: &str,
    ) {
        let effective_color = color.or(self.get_color());
        write_text(&self.writer, text, effective_color, end);
    }
}

impl CallbackManagerMixin for StdOutCallbackHandler {
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
        let name = name
            .or_else(|| {
                if !serialized.is_empty() {
                    serialized.get("name").and_then(|v| v.as_str()).or_else(|| {
                        serialized.get("id").and_then(|v| {
                            v.as_array()
                                .and_then(|arr| arr.last())
                                .and_then(|v| v.as_str())
                        })
                    })
                } else {
                    None
                }
            })
            .unwrap_or("<unknown>");

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
}

impl ChainManagerMixin for StdOutCallbackHandler {
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
            let effective_color = color.or(self.get_color());
            write_text(&self.writer, log, effective_color, "");
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
            let effective_color = color.or(self.get_color());
            write_text(&self.writer, log, effective_color, "\n");
        }
    }
}

impl BaseCallbackHandler for StdOutCallbackHandler {
    fn name(&self) -> &str {
        "StdOutCallbackHandler"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdout_handler_creation() {
        let handler = StdOutCallbackHandler::new();
        assert!(handler.color.is_none());
        assert_eq!(handler.name(), "StdOutCallbackHandler");
    }

    #[test]
    fn test_stdout_handler_with_color() {
        let handler = StdOutCallbackHandler::with_color(colors::GREEN);
        assert_eq!(handler.color, Some(colors::GREEN.to_string()));
    }
}
