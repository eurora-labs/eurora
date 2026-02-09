//! Callback Handler that prints to stdout.
//!
//! This module provides callback handlers for printing output to stdout,
//! including a standard handler and a streaming handler.

use std::collections::HashMap;
use std::io::{self, Write};

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

/// Print text with optional color.
fn print_text(text: &str, color: Option<&str>, end: &str) {
    if let Some(c) = color {
        print!("{}{}{}{}", c, text, colors::RESET, end);
    } else {
        print!("{}{}", text, end);
    }
    let _ = io::stdout().flush();
}

/// Callback Handler that prints to stdout.
#[derive(Debug, Clone)]
pub struct StdOutCallbackHandler {
    /// The color to use for the text.
    pub color: Option<String>,
}

impl Default for StdOutCallbackHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl StdOutCallbackHandler {
    /// Create a new StdOutCallbackHandler.
    pub fn new() -> Self {
        Self { color: None }
    }

    /// Create a new StdOutCallbackHandler with a specific color.
    pub fn with_color(color: impl Into<String>) -> Self {
        Self {
            color: Some(color.into()),
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
        // Print observation prefix if provided
        if let Some(prefix) = observation_prefix {
            print_text(&format!("\n{}", prefix), None, "");
        }
        // Print output with color override or handler's default color
        let effective_color = color.or(self.get_color());
        print_text(output, effective_color, "");
        // Print LLM prefix if provided
        if let Some(prefix) = llm_prefix {
            print_text(&format!("\n{}", prefix), None, "");
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
        // Use color parameter if provided, otherwise use handler's default color
        let effective_color = color.or(self.get_color());
        print_text(text, effective_color, end);
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

        println!(
            "\n\n{}> Entering new {} chain...{}",
            colors::BOLD,
            name,
            colors::RESET
        );
    }
}

impl ChainManagerMixin for StdOutCallbackHandler {
    fn on_chain_end(
        &self,
        _outputs: &HashMap<String, serde_json::Value>,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
    ) {
        println!("\n{}> Finished chain.{}", colors::BOLD, colors::RESET);
    }

    fn on_agent_action(
        &self,
        action: &serde_json::Value,
        _run_id: Uuid,
        _parent_run_id: Option<Uuid>,
        color: Option<&str>,
    ) {
        if let Some(log) = action.get("log").and_then(|v| v.as_str()) {
            // Use color parameter if provided, otherwise use handler's default color
            let effective_color = color.or(self.get_color());
            print_text(log, effective_color, "");
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
            // Use color parameter if provided, otherwise use handler's default color
            let effective_color = color.or(self.get_color());
            print_text(log, effective_color, "\n");
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
#[derive(Debug, Clone, Default)]
pub struct StreamingStdOutCallbackHandler;

impl StreamingStdOutCallbackHandler {
    /// Create a new StreamingStdOutCallbackHandler.
    pub fn new() -> Self {
        Self
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
        print!("{}", token);
        let _ = io::stdout().flush();
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
