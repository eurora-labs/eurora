//! Snapshot tests for StdOutCallbackHandler.
//!
//! These tests capture the exact output format and behavior of the
//! StdOutCallbackHandler to detect unintended changes.
//!
//! Ported from `langchain/libs/core/tests/unit_tests/callbacks/test_stdout_snapshot.py`

use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};

use agent_chain_core::callbacks::base::{
    BaseCallbackHandler, CallbackManagerMixin, ChainManagerMixin, RunManagerMixin, ToolManagerMixin,
};
use agent_chain_core::callbacks::stdout::StdOutCallbackHandler;
use uuid::Uuid;

/// A Write implementation backed by a shared buffer for test output capture.
#[derive(Clone)]
struct TestWriter {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl TestWriter {
    fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn output(&self) -> String {
        let guard = self.buffer.lock().unwrap();
        String::from_utf8(guard.clone()).unwrap()
    }
}

impl Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn create_test_handler() -> (StdOutCallbackHandler, TestWriter) {
    let writer = TestWriter::new();
    let boxed: Box<dyn Write + Send> = Box::new(writer.clone());
    let handler = StdOutCallbackHandler::with_writer(Arc::new(Mutex::new(boxed)));
    (handler, writer)
}

fn create_test_handler_with_color(color: &str) -> (StdOutCallbackHandler, TestWriter) {
    let (mut handler, writer) = create_test_handler();
    handler.color = Some(color.to_string());
    (handler, writer)
}

// ====================================================================
// TestStdOutCallbackHandlerInit
// ====================================================================

/// Ported from `test_default_color_is_none`.
#[test]
fn test_default_color_is_none() {
    let handler = StdOutCallbackHandler::new();
    assert!(handler.color.is_none());
}

/// Ported from `test_custom_color_stored`.
#[test]
fn test_custom_color_stored() {
    let handler = StdOutCallbackHandler::with_color("blue");
    assert_eq!(handler.color, Some("blue".to_string()));
}

/// Ported from `test_inherits_base_handler`.
#[test]
fn test_inherits_base_handler() {
    let handler = StdOutCallbackHandler::new();
    let _: &dyn BaseCallbackHandler = &handler;
}

/// Ported from `test_default_flags`.
#[test]
fn test_default_flags() {
    let handler = StdOutCallbackHandler::new();
    assert!(!handler.raise_error());
    assert!(!handler.run_inline());
}

// ====================================================================
// TestOnChainStart
// ====================================================================

/// Ported from `test_uses_name_from_kwargs`.
///
/// In Python, name is passed as a kwarg. In Rust, it's passed via metadata.
#[test]
fn test_chain_start_uses_name_from_kwargs() {
    let (handler, writer) = create_test_handler();
    let metadata = HashMap::from([("name".to_string(), serde_json::json!("MyChain"))]);
    handler.on_chain_start(
        &HashMap::new(),
        &HashMap::new(),
        Uuid::new_v4(),
        None,
        None,
        Some(&metadata),
    );
    let output = writer.output();
    assert!(
        output.contains("Entering new MyChain chain..."),
        "Expected 'Entering new MyChain chain...' in output: {:?}",
        output
    );
}

/// Ported from `test_uses_name_from_serialized`.
#[test]
fn test_chain_start_uses_name_from_serialized() {
    let (handler, writer) = create_test_handler();
    let serialized = HashMap::from([("name".to_string(), serde_json::json!("SerializedChain"))]);
    handler.on_chain_start(
        &serialized,
        &HashMap::new(),
        Uuid::new_v4(),
        None,
        None,
        None,
    );
    let output = writer.output();
    assert!(
        output.contains("Entering new SerializedChain chain..."),
        "Expected 'Entering new SerializedChain chain...' in output: {:?}",
        output
    );
}

/// Ported from `test_uses_id_from_serialized_as_fallback`.
#[test]
fn test_chain_start_uses_id_from_serialized_as_fallback() {
    let (handler, writer) = create_test_handler();
    let serialized = HashMap::from([(
        "id".to_string(),
        serde_json::json!(["module", "path", "ClassName"]),
    )]);
    handler.on_chain_start(
        &serialized,
        &HashMap::new(),
        Uuid::new_v4(),
        None,
        None,
        None,
    );
    let output = writer.output();
    assert!(
        output.contains("Entering new ClassName chain..."),
        "Expected 'Entering new ClassName chain...' in output: {:?}",
        output
    );
}

/// Ported from `test_uses_unknown_when_no_name`.
#[test]
fn test_chain_start_uses_unknown_when_no_name() {
    let (handler, writer) = create_test_handler();
    handler.on_chain_start(
        &HashMap::new(),
        &HashMap::new(),
        Uuid::new_v4(),
        None,
        None,
        None,
    );
    let output = writer.output();
    assert!(
        output.contains("Entering new <unknown> chain..."),
        "Expected 'Entering new <unknown> chain...' in output: {:?}",
        output
    );
}

/// Ported from `test_name_kwarg_takes_precedence_over_serialized`.
#[test]
fn test_chain_start_name_kwarg_takes_precedence_over_serialized() {
    let (handler, writer) = create_test_handler();
    let serialized = HashMap::from([("name".to_string(), serde_json::json!("Serialized"))]);
    let metadata = HashMap::from([("name".to_string(), serde_json::json!("KwargName"))]);
    handler.on_chain_start(
        &serialized,
        &HashMap::new(),
        Uuid::new_v4(),
        None,
        None,
        Some(&metadata),
    );
    let output = writer.output();
    assert!(
        output.contains("KwargName"),
        "Expected 'KwargName' in output: {:?}",
        output
    );
    assert!(
        !output.contains("Serialized"),
        "Expected 'Serialized' NOT in output: {:?}",
        output
    );
}

/// Ported from `test_output_has_bold_ansi_codes` (on_chain_start).
#[test]
fn test_chain_start_output_has_bold_ansi_codes() {
    let (handler, writer) = create_test_handler();
    let serialized = HashMap::from([("name".to_string(), serde_json::json!("Test"))]);
    handler.on_chain_start(
        &serialized,
        &HashMap::new(),
        Uuid::new_v4(),
        None,
        None,
        None,
    );
    let output = writer.output();
    assert!(
        output.contains("\x1b[1m"),
        "Expected bold ANSI code in output: {:?}",
        output
    );
    assert!(
        output.contains("\x1b[0m"),
        "Expected reset ANSI code in output: {:?}",
        output
    );
}

/// Ported from `test_serialized_none_uses_unknown`.
///
/// In Python, serialized can be None. In Rust, we pass an empty HashMap.
#[test]
fn test_chain_start_serialized_none_uses_unknown() {
    let (handler, writer) = create_test_handler();
    handler.on_chain_start(
        &HashMap::new(),
        &HashMap::new(),
        Uuid::new_v4(),
        None,
        None,
        None,
    );
    let output = writer.output();
    assert!(
        output.contains("Entering new <unknown> chain..."),
        "Expected 'Entering new <unknown> chain...' in output: {:?}",
        output
    );
}

// ====================================================================
// TestOnChainEnd
// ====================================================================

/// Ported from `test_outputs_finished_chain`.
#[test]
fn test_chain_end_outputs_finished_chain() {
    let (handler, writer) = create_test_handler();
    handler.on_chain_end(&HashMap::new(), Uuid::new_v4(), None);
    let output = writer.output();
    assert!(
        output.contains("Finished chain."),
        "Expected 'Finished chain.' in output: {:?}",
        output
    );
}

/// Ported from `test_output_has_bold_ansi_codes` (on_chain_end).
#[test]
fn test_chain_end_output_has_bold_ansi_codes() {
    let (handler, writer) = create_test_handler();
    handler.on_chain_end(&HashMap::new(), Uuid::new_v4(), None);
    let output = writer.output();
    assert!(
        output.contains("\x1b[1m"),
        "Expected bold ANSI code in output: {:?}",
        output
    );
    assert!(
        output.contains("\x1b[0m"),
        "Expected reset ANSI code in output: {:?}",
        output
    );
}

/// Ported from `test_outputs_start_with_newline`.
#[test]
fn test_chain_end_outputs_start_with_newline() {
    let (handler, writer) = create_test_handler();
    handler.on_chain_end(&HashMap::new(), Uuid::new_v4(), None);
    let output = writer.output();
    assert!(
        output.starts_with('\n'),
        "Expected output to start with newline: {:?}",
        output
    );
}

// ====================================================================
// TestOnAgentAction
// ====================================================================

/// Ported from `test_outputs_action_log`.
#[test]
fn test_agent_action_outputs_action_log() {
    let (handler, writer) = create_test_handler();
    let action = serde_json::json!({
        "tool": "search",
        "tool_input": "q",
        "log": "Using search tool"
    });
    handler.on_agent_action(&action, Uuid::new_v4(), None, None);
    let output = writer.output();
    assert!(
        output.contains("Using search tool"),
        "Expected 'Using search tool' in output: {:?}",
        output
    );
}

/// Ported from `test_color_override` (on_agent_action).
#[test]
fn test_agent_action_color_override() {
    let (handler, writer) = create_test_handler_with_color("green");
    let action = serde_json::json!({
        "tool": "t",
        "tool_input": "i",
        "log": "log text"
    });
    handler.on_agent_action(&action, Uuid::new_v4(), None, Some("red"));
    let output = writer.output();
    assert!(
        output.contains("log text"),
        "Expected 'log text' in output: {:?}",
        output
    );
}

/// Ported from `test_uses_default_color_when_no_override`.
#[test]
fn test_agent_action_uses_default_color_when_no_override() {
    let (handler, writer) = create_test_handler_with_color("green");
    let action = serde_json::json!({
        "tool": "t",
        "tool_input": "i",
        "log": "log"
    });
    handler.on_agent_action(&action, Uuid::new_v4(), None, None);
    let output = writer.output();
    assert!(
        output.contains("log"),
        "Expected 'log' in output: {:?}",
        output
    );
}

// ====================================================================
// TestOnToolEnd
// ====================================================================

/// Ported from `test_outputs_tool_result`.
#[test]
fn test_tool_end_outputs_tool_result() {
    let (handler, writer) = create_test_handler();
    handler.on_tool_end("result text", Uuid::new_v4(), None, None, None, None);
    let output = writer.output();
    assert!(
        output.contains("result text"),
        "Expected 'result text' in output: {:?}",
        output
    );
}

/// Ported from `test_with_observation_prefix`.
#[test]
fn test_tool_end_with_observation_prefix() {
    let (handler, writer) = create_test_handler();
    handler.on_tool_end("result", Uuid::new_v4(), None, None, Some("Obs:"), None);
    let output = writer.output();
    assert!(
        output.contains("Obs:"),
        "Expected 'Obs:' in output: {:?}",
        output
    );
    assert!(
        output.contains("result"),
        "Expected 'result' in output: {:?}",
        output
    );
}

/// Ported from `test_with_llm_prefix`.
#[test]
fn test_tool_end_with_llm_prefix() {
    let (handler, writer) = create_test_handler();
    handler.on_tool_end("result", Uuid::new_v4(), None, None, None, Some("Think:"));
    let output = writer.output();
    assert!(
        output.contains("Think:"),
        "Expected 'Think:' in output: {:?}",
        output
    );
    assert!(
        output.contains("result"),
        "Expected 'result' in output: {:?}",
        output
    );
}

/// Ported from `test_with_both_prefixes`.
#[test]
fn test_tool_end_with_both_prefixes() {
    let (handler, writer) = create_test_handler();
    handler.on_tool_end(
        "result",
        Uuid::new_v4(),
        None,
        None,
        Some("Obs:"),
        Some("Think:"),
    );
    let output = writer.output();
    assert!(
        output.contains("Obs:"),
        "Expected 'Obs:' in output: {:?}",
        output
    );
    assert!(
        output.contains("result"),
        "Expected 'result' in output: {:?}",
        output
    );
    assert!(
        output.contains("Think:"),
        "Expected 'Think:' in output: {:?}",
        output
    );
}

/// Ported from `test_no_prefix_when_none`.
#[test]
fn test_tool_end_no_prefix_when_none() {
    let (handler, writer) = create_test_handler();
    handler.on_tool_end("result", Uuid::new_v4(), None, None, None, None);
    let output = writer.output();
    assert!(
        !output.contains("Obs:"),
        "Expected no 'Obs:' in output: {:?}",
        output
    );
}

/// Ported from `test_non_string_output_converted`.
///
/// In Python, on_tool_end calls str(output). In Rust, the parameter is
/// already `&str`, so callers convert before passing. This test verifies
/// the pattern works.
#[test]
fn test_tool_end_non_string_output_converted() {
    let (handler, writer) = create_test_handler();
    handler.on_tool_end(&42.to_string(), Uuid::new_v4(), None, None, None, None);
    let output = writer.output();
    assert!(
        output.contains("42"),
        "Expected '42' in output: {:?}",
        output
    );
}

/// Ported from `test_color_override` (on_tool_end).
#[test]
fn test_tool_end_color_override() {
    let (handler, writer) = create_test_handler_with_color("green");
    handler.on_tool_end("result", Uuid::new_v4(), None, Some("red"), None, None);
    let output = writer.output();
    assert!(
        output.contains("result"),
        "Expected 'result' in output: {:?}",
        output
    );
}

// ====================================================================
// TestOnText
// ====================================================================

/// Ported from `test_outputs_text`.
#[test]
fn test_on_text_outputs_text() {
    let (handler, writer) = create_test_handler();
    handler.on_text("hello world", Uuid::new_v4(), None, None, "");
    let output = writer.output();
    assert!(
        output.contains("hello world"),
        "Expected 'hello world' in output: {:?}",
        output
    );
}

/// Ported from `test_custom_end_character`.
#[test]
fn test_on_text_custom_end_character() {
    let (handler, writer) = create_test_handler();
    handler.on_text("line1", Uuid::new_v4(), None, None, "\n");
    handler.on_text("line2", Uuid::new_v4(), None, None, "");
    let output = writer.output();
    assert!(
        output.contains("line1\n"),
        "Expected 'line1\\n' in output: {:?}",
        output
    );
    assert!(
        output.contains("line2"),
        "Expected 'line2' in output: {:?}",
        output
    );
}

/// Ported from `test_default_end_is_empty`.
#[test]
fn test_on_text_default_end_is_empty() {
    let (handler, writer) = create_test_handler();
    handler.on_text("a", Uuid::new_v4(), None, None, "");
    handler.on_text("b", Uuid::new_v4(), None, None, "");
    let output = writer.output();
    assert!(output.contains("a"), "Expected 'a' in output: {:?}", output);
    assert!(output.contains("b"), "Expected 'b' in output: {:?}", output);
}

/// Ported from `test_empty_text`.
#[test]
fn test_on_text_empty_text() {
    let (handler, _writer) = create_test_handler();
    // Should not panic
    handler.on_text("", Uuid::new_v4(), None, None, "");
}

// ====================================================================
// TestOnAgentFinish
// ====================================================================

/// Ported from `test_outputs_finish_log`.
#[test]
fn test_agent_finish_outputs_finish_log() {
    let (handler, writer) = create_test_handler();
    let finish = serde_json::json!({
        "return_values": {"output": "done"},
        "log": "Final answer: done"
    });
    handler.on_agent_finish(&finish, Uuid::new_v4(), None, None);
    let output = writer.output();
    assert!(
        output.contains("Final answer: done"),
        "Expected 'Final answer: done' in output: {:?}",
        output
    );
}

/// Ported from `test_color_override` (on_agent_finish).
#[test]
fn test_agent_finish_color_override() {
    let (handler, writer) = create_test_handler_with_color("green");
    let finish = serde_json::json!({
        "return_values": {},
        "log": "log"
    });
    handler.on_agent_finish(&finish, Uuid::new_v4(), None, Some("red"));
    let output = writer.output();
    assert!(
        output.contains("log"),
        "Expected 'log' in output: {:?}",
        output
    );
}

/// Ported from `test_ends_with_newline`.
#[test]
fn test_agent_finish_ends_with_newline() {
    let (handler, writer) = create_test_handler();
    let finish = serde_json::json!({
        "return_values": {},
        "log": "log"
    });
    handler.on_agent_finish(&finish, Uuid::new_v4(), None, None);
    let output = writer.output();
    assert!(
        output.ends_with('\n'),
        "Expected output to end with newline: {:?}",
        output
    );
}
