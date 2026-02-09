//! Unit tests for StdOutCallbackHandler.
//!
//! Ported from `langchain/libs/core/tests/unit_tests/callbacks/test_stdout.py`
//!
//! Since the handler writes to stdout, tests verify the methods execute
//! without panicking and test the handler's configuration/logic. Output
//! content is verified through the inline tests in stdout.rs that have
//! direct access to the implementation internals.

use agent_chain_core::callbacks::base::LLMManagerMixin;
use agent_chain_core::callbacks::base::{
    BaseCallbackHandler, CallbackManagerMixin, ChainManagerMixin, RunManagerMixin, ToolManagerMixin,
};
use agent_chain_core::callbacks::stdout::{
    StdOutCallbackHandler, StreamingStdOutCallbackHandler, colors,
};
use std::collections::HashMap;
use uuid::Uuid;

// ====================================================================
// Handler creation and configuration
// ====================================================================

/// Ported from `test_stdout_callback_handler_no_color`.
#[test]
fn test_stdout_handler_no_color() {
    let handler = StdOutCallbackHandler::new();
    assert!(handler.color.is_none());
    assert_eq!(handler.name(), "StdOutCallbackHandler");
}

/// Ported from `test_stdout_callback_handler_with_default_color`.
#[test]
fn test_stdout_handler_with_color() {
    let handler = StdOutCallbackHandler::with_color("blue");
    assert_eq!(handler.color, Some("blue".to_string()));
}

// ====================================================================
// Chain start name resolution
// ====================================================================

/// Ported from `test_stdout_callback_handler_chain_start`.
///
/// Verifies on_chain_start with serialized name doesn't panic.
#[test]
fn test_chain_start_with_serialized_name() {
    let handler = StdOutCallbackHandler::new();
    let mut serialized = HashMap::new();
    serialized.insert("name".to_string(), serde_json::json!("TestChain"));
    handler.on_chain_start(
        &serialized,
        &HashMap::new(),
        Uuid::new_v4(),
        None,
        None,
        None,
    );
}

/// Ported from `test_stdout_callback_handler_chain_start_with_name_kwarg`.
///
/// Verifies on_chain_start with name in metadata (kwargs equivalent).
#[test]
fn test_chain_start_with_name_in_metadata() {
    let handler = StdOutCallbackHandler::new();
    let metadata = HashMap::from([("name".to_string(), serde_json::json!("CustomName"))]);
    handler.on_chain_start(
        &HashMap::new(),
        &HashMap::new(),
        Uuid::new_v4(),
        None,
        None,
        Some(&metadata),
    );
}

/// Ported from `test_stdout_callback_handler_chain_start_with_id`.
///
/// Verifies on_chain_start falls back to serialized id.
#[test]
fn test_chain_start_with_serialized_id() {
    let handler = StdOutCallbackHandler::new();
    let mut serialized = HashMap::new();
    serialized.insert("id".to_string(), serde_json::json!(["module", "ClassName"]));
    handler.on_chain_start(
        &serialized,
        &HashMap::new(),
        Uuid::new_v4(),
        None,
        None,
        None,
    );
}

/// Ported from `test_stdout_callback_handler_chain_start_unknown`.
///
/// Verifies on_chain_start with no name information uses "<unknown>".
#[test]
fn test_chain_start_unknown_name() {
    let handler = StdOutCallbackHandler::new();
    handler.on_chain_start(
        &HashMap::new(),
        &HashMap::new(),
        Uuid::new_v4(),
        None,
        None,
        None,
    );
}

// ====================================================================
// Chain end
// ====================================================================

/// Ported from `test_stdout_callback_handler_chain_end`.
#[test]
fn test_chain_end() {
    let handler = StdOutCallbackHandler::new();
    handler.on_chain_end(
        &HashMap::from([("output".to_string(), serde_json::json!("result"))]),
        Uuid::new_v4(),
        None,
    );
}

// ====================================================================
// Agent action and finish
// ====================================================================

/// Ported from `test_stdout_callback_handler_agent_action`.
#[test]
fn test_agent_action() {
    let handler = StdOutCallbackHandler::new();
    let action = serde_json::json!({
        "tool": "TestTool",
        "tool_input": {"query": "test"},
        "log": "Invoking TestTool with query"
    });
    handler.on_agent_action(&action, Uuid::new_v4(), None, None);
}

/// Ported from `test_stdout_callback_handler_agent_action_with_color`.
#[test]
fn test_agent_action_with_color_override() {
    let handler = StdOutCallbackHandler::with_color("green");
    let action = serde_json::json!({
        "tool": "TestTool",
        "tool_input": {"query": "test"},
        "log": "Action log"
    });
    handler.on_agent_action(&action, Uuid::new_v4(), None, Some("red"));
}

/// Ported from `test_stdout_callback_handler_agent_finish`.
#[test]
fn test_agent_finish() {
    let handler = StdOutCallbackHandler::new();
    let finish = serde_json::json!({
        "return_values": {"output": "final result"},
        "log": "Agent completed successfully"
    });
    handler.on_agent_finish(&finish, Uuid::new_v4(), None, None);
}

// ====================================================================
// Tool end
// ====================================================================

/// Ported from `test_stdout_callback_handler_tool_end`.
#[test]
fn test_tool_end() {
    let handler = StdOutCallbackHandler::new();
    handler.on_tool_end("Tool result", Uuid::new_v4(), None, None, None, None);
}

/// Ported from `test_stdout_callback_handler_tool_end_with_prefixes`.
#[test]
fn test_tool_end_with_prefixes() {
    let handler = StdOutCallbackHandler::new();
    handler.on_tool_end(
        "Tool result",
        Uuid::new_v4(),
        None,
        None,
        Some("Observation:"),
        Some("Thought:"),
    );
}

/// Ported from `test_stdout_callback_handler_color_override`.
#[test]
fn test_tool_end_with_color_override() {
    let handler = StdOutCallbackHandler::with_color("green");
    handler.on_tool_end("Result", Uuid::new_v4(), None, Some("red"), None, None);
}

// ====================================================================
// Text output
// ====================================================================

/// Ported from `test_stdout_callback_handler_on_text`.
#[test]
fn test_on_text() {
    let handler = StdOutCallbackHandler::new();
    handler.on_text("Custom text", Uuid::new_v4(), None, None, "");
}

/// Ported from `test_stdout_callback_handler_on_text_with_end`.
#[test]
fn test_on_text_with_end() {
    let handler = StdOutCallbackHandler::new();
    handler.on_text("Line 1", Uuid::new_v4(), None, None, "\n");
    handler.on_text("Line 2", Uuid::new_v4(), None, None, "");
}

// ====================================================================
// StreamingStdOutCallbackHandler
// ====================================================================

/// Verify StreamingStdOutCallbackHandler creation and name.
#[test]
fn test_streaming_handler_creation() {
    let handler = StreamingStdOutCallbackHandler::new();
    assert_eq!(handler.name(), "StreamingStdOutCallbackHandler");
}

/// Verify on_llm_new_token doesn't panic.
#[test]
fn test_streaming_handler_on_llm_new_token() {
    let handler = StreamingStdOutCallbackHandler::new();
    handler.on_llm_new_token("Hello", Uuid::new_v4(), None, None);
    handler.on_llm_new_token(" World", Uuid::new_v4(), None, None);
}

// ====================================================================
// Color module
// ====================================================================

/// Verify color constants are non-empty ANSI escape sequences.
#[test]
fn test_color_constants() {
    assert!(colors::RESET.starts_with("\x1b["));
    assert!(colors::BOLD.starts_with("\x1b["));
    assert!(colors::RED.starts_with("\x1b["));
    assert!(colors::GREEN.starts_with("\x1b["));
    assert!(colors::BLUE.starts_with("\x1b["));
}
