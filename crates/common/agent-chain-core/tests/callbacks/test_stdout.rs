use agent_chain_core::callbacks::base::LLMManagerMixin;
use agent_chain_core::callbacks::base::{
    BaseCallbackHandler, CallbackManagerMixin, ChainManagerMixin, RunManagerMixin, ToolManagerMixin,
};
use agent_chain_core::callbacks::stdout::{StdOutCallbackHandler, colors};
use agent_chain_core::callbacks::streaming_stdout::StreamingStdOutCallbackHandler;
use std::collections::HashMap;
use uuid::Uuid;

#[test]
fn test_stdout_handler_no_color() {
    let handler = StdOutCallbackHandler::new();
    assert!(handler.color.is_none());
    assert_eq!(handler.name(), "StdOutCallbackHandler");
}

#[test]
fn test_stdout_handler_with_color() {
    let handler = StdOutCallbackHandler::with_color("blue");
    assert_eq!(handler.color, Some("blue".to_string()));
}

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
        None,
    );
}

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
        None,
    );
}

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
        None,
    );
}

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
        None,
    );
}

#[test]
fn test_chain_end() {
    let handler = StdOutCallbackHandler::new();
    handler.on_chain_end(
        &HashMap::from([("output".to_string(), serde_json::json!("result"))]),
        Uuid::new_v4(),
        None,
    );
}

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

#[test]
fn test_agent_finish() {
    let handler = StdOutCallbackHandler::new();
    let finish = serde_json::json!({
        "return_values": {"output": "final result"},
        "log": "Agent completed successfully"
    });
    handler.on_agent_finish(&finish, Uuid::new_v4(), None, None);
}

#[test]
fn test_tool_end() {
    let handler = StdOutCallbackHandler::new();
    handler.on_tool_end("Tool result", Uuid::new_v4(), None, None, None, None);
}

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

#[test]
fn test_tool_end_with_color_override() {
    let handler = StdOutCallbackHandler::with_color("green");
    handler.on_tool_end("Result", Uuid::new_v4(), None, Some("red"), None, None);
}

#[test]
fn test_on_text() {
    let handler = StdOutCallbackHandler::new();
    handler.on_text("Custom text", Uuid::new_v4(), None, None, "");
}

#[test]
fn test_on_text_with_end() {
    let handler = StdOutCallbackHandler::new();
    handler.on_text("Line 1", Uuid::new_v4(), None, None, "\n");
    handler.on_text("Line 2", Uuid::new_v4(), None, None, "");
}

#[test]
fn test_streaming_handler_creation() {
    let handler = StreamingStdOutCallbackHandler::new();
    assert_eq!(handler.name(), "StreamingStdOutCallbackHandler");
}

#[test]
fn test_streaming_handler_on_llm_new_token() {
    let handler = StreamingStdOutCallbackHandler::new();
    handler.on_llm_new_token("Hello", Uuid::new_v4(), None, None);
    handler.on_llm_new_token(" World", Uuid::new_v4(), None, None);
}

#[test]
fn test_color_constants() {
    assert!(colors::RESET.starts_with("\x1b["));
    assert!(colors::BOLD.starts_with("\x1b["));
    assert!(colors::RED.starts_with("\x1b["));
    assert!(colors::GREEN.starts_with("\x1b["));
    assert!(colors::BLUE.starts_with("\x1b["));
}
