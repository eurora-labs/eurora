//! Callback Handler that prints to stdout.
//!
//! This module provides callback handlers for printing output to stdout,
//! including a standard handler and a streaming handler.

use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use uuid::Uuid;

use super::base::{
    BaseCallbackHandler, CallbackManagerMixin, ChainManagerMixin, LLMManagerMixin,
    RetrieverManagerMixin, RunManagerMixin, ToolManagerMixin,
};

/// ANSI color codes for terminal output.
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

/// Write text with optional color to a writer.
fn write_text(writer: &Mutex<Box<dyn Write + Send>>, text: &str, color: Option<&str>, end: &str) {
    if let Ok(mut w) = writer.lock() {
        if let Some(c) = color {
            let _ = write!(w, "{}{}{}{}", c, text, colors::RESET, end);
        } else {
            let _ = write!(w, "{}{}", text, end);
        }
        let _ = w.flush();
    }
}

/// Callback Handler that prints to stdout.
#[derive(Clone)]
pub struct StdOutCallbackHandler {
    /// The color to use for the text.
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
    /// Create a new StdOutCallbackHandler.
    pub fn new() -> Self {
        Self {
            color: None,
            writer: Arc::new(Mutex::new(Box::new(io::stdout()))),
        }
    }

    /// Create a new StdOutCallbackHandler with a specific color.
    pub fn with_color(color: impl Into<String>) -> Self {
        Self {
            color: Some(color.into()),
            writer: Arc::new(Mutex::new(Box::new(io::stdout()))),
        }
    }

    /// Create a new StdOutCallbackHandler with a custom writer.
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
        metadata: Option<&HashMap<String, serde_json::Value>>,
    ) {
        // First check metadata for "name" (equivalent to kwargs["name"] in Python)
        // Then fall back to serialized
        let name = metadata
            .and_then(|m| m.get("name"))
            .and_then(|v| v.as_str())
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
            let _ = writeln!(
                w,
                "\n\n{}> Entering new {} chain...{}",
                colors::BOLD,
                name,
                colors::RESET
            );
            let _ = w.flush();
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
            let _ = writeln!(w, "\n{}> Finished chain.{}", colors::BOLD, colors::RESET);
            let _ = w.flush();
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

/// Callback handler for streaming. Only works with LLMs that support streaming.
///
/// This handler prints tokens to stdout as they are generated.
#[derive(Clone)]
pub struct StreamingStdOutCallbackHandler {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl std::fmt::Debug for StreamingStdOutCallbackHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamingStdOutCallbackHandler").finish()
    }
}

impl Default for StreamingStdOutCallbackHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingStdOutCallbackHandler {
    /// Create a new StreamingStdOutCallbackHandler.
    pub fn new() -> Self {
        Self {
            writer: Arc::new(Mutex::new(Box::new(io::stdout()))),
        }
    }

    /// Create a new StreamingStdOutCallbackHandler with a custom writer.
    pub fn with_writer(writer: Arc<Mutex<Box<dyn Write + Send>>>) -> Self {
        Self { writer }
    }
}

impl LLMManagerMixin for StreamingStdOutCallbackHandler {
    fn on_llm_new_token(
        &self,
        token: &str,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        _chunk: Option<&serde_json::Value>,
    ) {
        if let Ok(mut w) = self.writer.lock() {
            let _ = write!(w, "{}", token);
            let _ = w.flush();
        }
    }
}

impl ChainManagerMixin for StreamingStdOutCallbackHandler {}
impl ToolManagerMixin for StreamingStdOutCallbackHandler {}
impl RetrieverManagerMixin for StreamingStdOutCallbackHandler {}
impl CallbackManagerMixin for StreamingStdOutCallbackHandler {}
impl RunManagerMixin for StreamingStdOutCallbackHandler {}

impl BaseCallbackHandler for StreamingStdOutCallbackHandler {
    fn name(&self) -> &str {
        "StreamingStdOutCallbackHandler"
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

    #[test]
    fn test_streaming_handler_creation() {
        let handler = StreamingStdOutCallbackHandler::new();
        assert_eq!(handler.name(), "StreamingStdOutCallbackHandler");
    }
}
