//! Snapshot tests for OpenAI tools output parsers.
//!
//! Ported from langchain/libs/core/tests/unit_tests/output_parsers/test_openai_tools_parser.py
//!
//! This file contains tests that mirror the Python test classes:
//! - TestParseToolCall
//! - TestMakeInvalidToolCall
//! - TestParseToolCalls
//! - TestJsonOutputToolsParser
//! - TestJsonOutputKeyToolsParser
//! - TestPydanticToolsParser

use std::collections::HashMap;

use agent_chain_core::messages::{AIMessage, ToolCall};
use agent_chain_core::output_parsers::{
    JsonOutputKeyToolsParser, JsonOutputToolsParser, PydanticToolsParser, make_invalid_tool_call,
    parse_tool_call, parse_tool_calls,
};
use agent_chain_core::outputs::ChatGeneration;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// Helpers (mirrors Python _tool_call_msg / _raw_tool_call_msg)
// ---------------------------------------------------------------------------

fn tool_call_generation(
    tool_calls: Vec<ToolCall>,
    content: &str,
    response_metadata: HashMap<String, Value>,
) -> ChatGeneration {
    let message = AIMessage::builder()
        .content(content)
        .tool_calls(tool_calls)
        .response_metadata(response_metadata)
        .build();
    ChatGeneration::new(message.into())
}

fn raw_tool_call_generation(raw_tool_calls: Value, content: &str) -> ChatGeneration {
    let mut additional_kwargs = HashMap::new();
    additional_kwargs.insert("tool_calls".to_string(), raw_tool_calls);
    let message = AIMessage::builder()
        .content(content)
        .additional_kwargs(additional_kwargs)
        .build();
    ChatGeneration::new(message.into())
}

fn make_tool_call(id: &str, name: &str, args: Value) -> ToolCall {
    ToolCall {
        id: Some(id.to_string()),
        name: name.to_string(),
        args,
        call_type: None,
    }
}

// ===========================================================================
// TestParseToolCall
// ===========================================================================

#[test]
fn test_parse_tool_call_valid_arguments() {
    let raw = json!({
        "function": {"name": "myTool", "arguments": r#"{"a": 1}"#},
        "id": "call_1",
        "type": "function",
    });
    let result = parse_tool_call(&raw, false, false, true).unwrap().unwrap();
    assert_eq!(result["name"], "myTool");
    assert_eq!(result["args"], json!({"a": 1}));
    assert_eq!(result["id"], "call_1");
}

#[test]
fn test_parse_tool_call_none_arguments_non_partial() {
    let raw = json!({
        "function": {"name": "noArgs", "arguments": null},
        "id": "call_2",
    });
    let result = parse_tool_call(&raw, false, false, true).unwrap().unwrap();
    assert_eq!(result["args"], json!({}));
}

#[test]
fn test_parse_tool_call_empty_string_arguments() {
    let raw = json!({
        "function": {"name": "emptyArgs", "arguments": ""},
        "id": "call_3",
    });
    let result = parse_tool_call(&raw, false, false, true).unwrap().unwrap();
    assert_eq!(result["args"], json!({}));
}

#[test]
fn test_parse_tool_call_partial_valid() {
    let raw = json!({
        "function": {"name": "fn", "arguments": r#"{"key": "val"#},
        "id": "call_4",
    });
    let result = parse_tool_call(&raw, true, false, true).unwrap().unwrap();
    assert_eq!(result["args"], json!({"key": "val"}));
}

#[test]
fn test_parse_tool_call_partial_none_arguments_returns_none() {
    let raw = json!({
        "function": {"name": "fn", "arguments": null},
        "id": "call_5",
    });
    let result = parse_tool_call(&raw, true, false, true).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_parse_tool_call_partial_unparseable_returns_none() {
    let raw = json!({
        "function": {"name": "fn", "arguments": "{{bad"},
        "id": "call_6",
    });
    let result = parse_tool_call(&raw, true, false, true).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_parse_tool_call_invalid_json_raises() {
    let raw = json!({
        "function": {"name": "fn", "arguments": "not_json"},
        "id": "call_7",
    });
    let result = parse_tool_call(&raw, false, false, true);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("not valid JSON"));
}

#[test]
fn test_parse_tool_call_no_function_key_returns_none() {
    let raw = json!({"id": "call_8", "type": "function"});
    let result = parse_tool_call(&raw, false, false, true).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_parse_tool_call_return_id_false() {
    let raw = json!({
        "function": {"name": "fn", "arguments": r#"{"a": 1}"#},
        "id": "call_9",
    });
    let result = parse_tool_call(&raw, false, false, false).unwrap().unwrap();
    assert_eq!(result["name"], "fn");
    assert!(result.get("id").is_none());
}

#[test]
fn test_parse_tool_call_strict_mode_rejects_newlines() {
    let raw = json!({
        "function": {"name": "fn", "arguments": "{\"code\": \"a\nb\"}"},
        "id": "call_10",
    });
    let result = parse_tool_call(&raw, false, true, true);
    assert!(result.is_err());
}

#[test]
fn test_parse_tool_call_non_strict_allows_newlines() {
    let raw = json!({
        "function": {"name": "fn", "arguments": "{\"code\": \"a\nb\"}"},
        "id": "call_11",
    });
    let result = parse_tool_call(&raw, false, false, true).unwrap().unwrap();
    assert_eq!(result["args"], json!({"code": "a\nb"}));
}

#[test]
fn test_parse_tool_call_empty_function_name() {
    let raw = json!({
        "function": {"name": null, "arguments": r#"{"a": 1}"#},
        "id": "call_12",
    });
    let result = parse_tool_call(&raw, false, false, true).unwrap().unwrap();
    assert_eq!(result["name"], "");
}

// ===========================================================================
// TestMakeInvalidToolCall
// ===========================================================================

#[test]
fn test_make_invalid_tool_call_creates_invalid_tool_call() {
    let raw = json!({
        "function": {"name": "fn", "arguments": "bad_json"},
        "id": "call_1",
    });
    let result = make_invalid_tool_call(&raw, Some("Parse error"));
    assert_eq!(result.name, Some("fn".to_string()));
    assert_eq!(result.args, Some("bad_json".to_string()));
    assert_eq!(result.id, Some("call_1".to_string()));
    assert_eq!(result.error, Some("Parse error".to_string()));
    assert_eq!(result.call_type, Some("invalid_tool_call".to_string()));
}

#[test]
fn test_make_invalid_tool_call_none_error_message() {
    let raw = json!({
        "function": {"name": "fn", "arguments": "{}"},
        "id": "call_2",
    });
    let result = make_invalid_tool_call(&raw, None);
    assert!(result.error.is_none());
}

#[test]
fn test_make_invalid_tool_call_missing_id() {
    let raw = json!({
        "function": {"name": "fn", "arguments": "{}"},
    });
    let result = make_invalid_tool_call(&raw, Some("error"));
    assert!(result.id.is_none());
}

// ===========================================================================
// TestParseToolCalls
// ===========================================================================

#[test]
fn test_parse_tool_calls_multiple_valid() {
    let raw_calls = vec![
        json!({"function": {"name": "fn1", "arguments": r#"{"a": 1}"#}, "id": "c1"}),
        json!({"function": {"name": "fn2", "arguments": r#"{"b": 2}"#}, "id": "c2"}),
    ];
    let result = parse_tool_calls(&raw_calls, false, false, true).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0]["name"], "fn1");
    assert_eq!(result[1]["name"], "fn2");
}

#[test]
fn test_parse_tool_calls_empty_list() {
    let raw_calls: Vec<Value> = vec![];
    let result = parse_tool_calls(&raw_calls, false, false, true).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_parse_tool_calls_all_invalid_raises() {
    let raw_calls = vec![
        json!({"function": {"name": "fn1", "arguments": "bad1"}, "id": "c1"}),
        json!({"function": {"name": "fn2", "arguments": "bad2"}, "id": "c2"}),
    ];
    let result = parse_tool_calls(&raw_calls, false, false, true);
    assert!(result.is_err());
    let error_str = result.unwrap_err().to_string();
    assert!(error_str.contains("fn1"));
    assert!(error_str.contains("fn2"));
}

#[test]
fn test_parse_tool_calls_mixed_valid_invalid_raises() {
    let raw_calls = vec![
        json!({"function": {"name": "good", "arguments": r#"{"a": 1}"#}, "id": "c1"}),
        json!({"function": {"name": "bad", "arguments": "invalid"}, "id": "c2"}),
    ];
    let result = parse_tool_calls(&raw_calls, false, false, true);
    assert!(result.is_err());
}

#[test]
fn test_parse_tool_calls_partial_mode_skips_unparseable() {
    let raw_calls = vec![
        json!({"function": {"name": "fn1", "arguments": r#"{"a": 1}"#}, "id": "c1"}),
        json!({"function": {"name": "fn2", "arguments": null}, "id": "c2"}),
    ];
    let result = parse_tool_calls(&raw_calls, true, false, true).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0]["name"], "fn1");
}

#[test]
fn test_parse_tool_calls_no_function_key_skipped() {
    let raw_calls = vec![
        json!({"id": "c1"}),
        json!({"function": {"name": "fn", "arguments": r#"{"a": 1}"#}, "id": "c2"}),
    ];
    let result = parse_tool_calls(&raw_calls, false, false, true).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0]["name"], "fn");
}

// ===========================================================================
// TestJsonOutputToolsParser
// ===========================================================================

#[test]
fn test_json_output_tools_parser_parses_tool_calls() {
    let generation = tool_call_generation(
        vec![make_tool_call("c1", "fn", json!({"a": 1}))],
        "",
        HashMap::new(),
    );
    let parser = JsonOutputToolsParser::new();
    let result = parser.parse_result(&[generation], false).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["type"], "fn");
    assert_eq!(arr[0]["args"], json!({"a": 1}));
}

#[test]
fn test_json_output_tools_parser_multiple_tool_calls() {
    let generation = tool_call_generation(
        vec![
            make_tool_call("c1", "fn1", json!({"a": 1})),
            make_tool_call("c2", "fn2", json!({"b": 2})),
        ],
        "",
        HashMap::new(),
    );
    let parser = JsonOutputToolsParser::new();
    let result = parser.parse_result(&[generation], false).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["type"], "fn1");
    assert_eq!(arr[1]["type"], "fn2");
}

#[test]
fn test_json_output_tools_parser_return_id() {
    let generation = tool_call_generation(
        vec![make_tool_call("call_123", "fn", json!({"a": 1}))],
        "",
        HashMap::new(),
    );
    let parser = JsonOutputToolsParser::new().with_return_id(true);
    let result = parser.parse_result(&[generation], false).unwrap();
    assert_eq!(result[0]["id"], "call_123");
}

#[test]
fn test_json_output_tools_parser_no_return_id() {
    let generation = tool_call_generation(
        vec![make_tool_call("call_123", "fn", json!({"a": 1}))],
        "",
        HashMap::new(),
    );
    let parser = JsonOutputToolsParser::new().with_return_id(false);
    let result = parser.parse_result(&[generation], false).unwrap();
    assert!(result[0].get("id").is_none());
}

#[test]
fn test_json_output_tools_parser_first_tool_only() {
    let generation = tool_call_generation(
        vec![
            make_tool_call("c1", "fn1", json!({"a": 1})),
            make_tool_call("c2", "fn2", json!({"b": 2})),
        ],
        "",
        HashMap::new(),
    );
    let parser = JsonOutputToolsParser::new().with_first_tool_only(true);
    let result = parser.parse_result(&[generation], false).unwrap();
    assert!(result.is_object());
    assert_eq!(result["type"], "fn1");
}

#[test]
fn test_json_output_tools_parser_first_tool_only_empty() {
    let generation = tool_call_generation(vec![], "", HashMap::new());
    let parser = JsonOutputToolsParser::new().with_first_tool_only(true);
    let result = parser.parse_result(&[generation], false).unwrap();
    // When tool_calls is empty, first_tool_only returns null
    assert!(result.is_null());
}

#[test]
fn test_json_output_tools_parser_empty_tool_calls_returns_empty_list() {
    let generation = tool_call_generation(vec![], "", HashMap::new());
    let parser = JsonOutputToolsParser::new();
    let result = parser.parse_result(&[generation], false).unwrap();
    assert_eq!(result, json!([]));
}

#[test]
fn test_json_output_tools_parser_fallback_to_additional_kwargs() {
    let generation = raw_tool_call_generation(
        json!([{
            "id": "c1",
            "function": {"name": "fn", "arguments": r#"{"a": 1}"#},
            "type": "function",
        }]),
        "",
    );
    let parser = JsonOutputToolsParser::new();
    let result = parser.parse_result(&[generation], false).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["type"], "fn");
}

#[test]
fn test_json_output_tools_parser_no_tool_calls_or_kwargs_returns_empty() {
    let message = AIMessage::builder().content("no tools").build();
    let generation = ChatGeneration::new(message.into());
    let parser = JsonOutputToolsParser::new();
    let result = parser.parse_result(&[generation], false).unwrap();
    assert_eq!(result, json!([]));
}

// ===========================================================================
// TestJsonOutputKeyToolsParser
// ===========================================================================

#[test]
fn test_json_output_key_tools_parser_filters_by_key_name() {
    let generation = tool_call_generation(
        vec![
            make_tool_call("c1", "target", json!({"a": 1})),
            make_tool_call("c2", "other", json!({"b": 2})),
        ],
        "",
        HashMap::new(),
    );
    let parser = JsonOutputKeyToolsParser::new("target").with_return_id(false);
    let result = parser.parse_result(&[generation], false).unwrap();
    assert_eq!(result, json!([{"a": 1}]));
}

#[test]
fn test_json_output_key_tools_parser_no_match_returns_empty_list() {
    let generation = tool_call_generation(
        vec![make_tool_call("c1", "other", json!({"a": 1}))],
        "",
        HashMap::new(),
    );
    let parser = JsonOutputKeyToolsParser::new("nonexistent");
    let result = parser.parse_result(&[generation], false).unwrap();
    assert_eq!(result, json!([]));
}

#[test]
fn test_json_output_key_tools_parser_first_tool_only_returns_args() {
    let generation = tool_call_generation(
        vec![make_tool_call("c1", "target", json!({"a": 1}))],
        "",
        HashMap::new(),
    );
    let parser = JsonOutputKeyToolsParser::new("target")
        .with_first_tool_only(true)
        .with_return_id(false);
    let result = parser.parse_result(&[generation], false).unwrap();
    assert_eq!(result, json!({"a": 1}));
}

#[test]
fn test_json_output_key_tools_parser_first_tool_only_with_return_id() {
    let generation = tool_call_generation(
        vec![make_tool_call("c1", "target", json!({"a": 1}))],
        "",
        HashMap::new(),
    );
    let parser = JsonOutputKeyToolsParser::new("target")
        .with_first_tool_only(true)
        .with_return_id(true);
    let result = parser.parse_result(&[generation], false).unwrap();
    assert_eq!(result["type"], "target");
    assert_eq!(result["args"], json!({"a": 1}));
}

#[test]
fn test_json_output_key_tools_parser_first_tool_only_no_match_returns_null() {
    let generation = tool_call_generation(
        vec![make_tool_call("c1", "other", json!({"a": 1}))],
        "",
        HashMap::new(),
    );
    let parser = JsonOutputKeyToolsParser::new("missing").with_first_tool_only(true);
    let result = parser.parse_result(&[generation], false).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_json_output_key_tools_parser_multiple_matches_returns_all() {
    let generation = tool_call_generation(
        vec![
            make_tool_call("c1", "fn", json!({"a": 1})),
            make_tool_call("c2", "other", json!({"b": 2})),
            make_tool_call("c3", "fn", json!({"a": 3})),
        ],
        "",
        HashMap::new(),
    );
    let parser = JsonOutputKeyToolsParser::new("fn").with_return_id(false);
    let result = parser.parse_result(&[generation], false).unwrap();
    assert_eq!(result, json!([{"a": 1}, {"a": 3}]));
}

#[test]
fn test_json_output_key_tools_parser_empty_tool_calls_first_only_returns_null() {
    let message = AIMessage::builder().content("").build();
    let generation = ChatGeneration::new(message.into());
    let parser = JsonOutputKeyToolsParser::new("fn").with_first_tool_only(true);
    let result = parser.parse_result(&[generation], false).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_json_output_key_tools_parser_empty_tool_calls_returns_empty_list() {
    let message = AIMessage::builder().content("").build();
    let generation = ChatGeneration::new(message.into());
    let parser = JsonOutputKeyToolsParser::new("fn");
    let result = parser.parse_result(&[generation], false).unwrap();
    assert_eq!(result, json!([]));
}

// ===========================================================================
// TestPydanticToolsParser
// ===========================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct MyTool {
    value: i64,
    name: String,
}

#[test]
fn test_pydantic_parses_single_tool() {
    let parser = PydanticToolsParser::new(vec![], false).with_tool::<MyTool>("MyTool");
    let generation = tool_call_generation(
        vec![make_tool_call(
            "c1",
            "MyTool",
            json!({"value": 42, "name": "test"}),
        )],
        "",
        HashMap::new(),
    );
    let result = parser.parse_result(&[generation], false).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    let tool: MyTool = serde_json::from_value(arr[0].clone()).unwrap();
    assert_eq!(tool.value, 42);
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ToolA {
    a: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ToolB {
    b: String,
}

#[test]
fn test_pydantic_parses_multiple_tools() {
    let parser = PydanticToolsParser::new(vec![], false)
        .with_tool::<ToolA>("ToolA")
        .with_tool::<ToolB>("ToolB");
    let generation = tool_call_generation(
        vec![
            make_tool_call("c1", "ToolA", json!({"a": 1})),
            make_tool_call("c2", "ToolB", json!({"b": "hello"})),
        ],
        "",
        HashMap::new(),
    );
    let result = parser.parse_result(&[generation], false).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    let tool_a: ToolA = serde_json::from_value(arr[0].clone()).unwrap();
    assert_eq!(tool_a.a, 1);
    let tool_b: ToolB = serde_json::from_value(arr[1].clone()).unwrap();
    assert_eq!(tool_b.b, "hello");
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct XTool {
    x: i64,
}

#[test]
fn test_pydantic_first_tool_only() {
    let parser = PydanticToolsParser::new(vec![], true).with_tool::<XTool>("XTool");
    let generation = tool_call_generation(
        vec![
            make_tool_call("c1", "XTool", json!({"x": 1})),
            make_tool_call("c2", "XTool", json!({"x": 2})),
        ],
        "",
        HashMap::new(),
    );
    let result = parser.parse_result(&[generation], false).unwrap();
    assert!(result.is_object());
    let tool: XTool = serde_json::from_value(result).unwrap();
    assert_eq!(tool.x, 1);
}

#[test]
fn test_pydantic_first_tool_only_empty_returns_null() {
    let parser = PydanticToolsParser::new(vec![], true).with_tool::<XTool>("XTool");
    let generation = tool_call_generation(vec![], "", HashMap::new());
    let result = parser.parse_result(&[generation], false).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_pydantic_empty_returns_empty_list() {
    let parser = PydanticToolsParser::new(vec![], false).with_tool::<XTool>("XTool");
    let generation = tool_call_generation(vec![], "", HashMap::new());
    let result = parser.parse_result(&[generation], false).unwrap();
    assert_eq!(result, json!([]));
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct StrictTool {
    count: i64,
}

#[test]
fn test_pydantic_validation_error_raises() {
    let parser = PydanticToolsParser::new(vec![], false).with_tool::<StrictTool>("StrictTool");
    let generation = tool_call_generation(
        vec![make_tool_call(
            "c1",
            "StrictTool",
            json!({"count": "not_int"}),
        )],
        "",
        HashMap::new(),
    );
    let result = parser.parse_result(&[generation], false);
    assert!(result.is_err());
}

#[test]
fn test_pydantic_partial_skips_invalid() {
    let parser = PydanticToolsParser::new(vec![], false).with_tool::<StrictTool>("StrictTool");
    let generation = tool_call_generation(
        vec![make_tool_call("c1", "StrictTool", json!({"count": "bad"}))],
        "",
        HashMap::new(),
    );
    let result = parser.parse_result(&[generation], true).unwrap();
    assert_eq!(result, json!([]));
}

#[test]
fn test_pydantic_partial_non_dict_args_skipped() {
    let parser = PydanticToolsParser::new(vec![], false).with_tool::<XTool>("XTool");
    let generation = tool_call_generation(
        vec![make_tool_call("c1", "XTool", json!({"x": 1}))],
        "",
        HashMap::new(),
    );
    let result = parser.parse_result(&[generation], true).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 1);
}

#[test]
fn test_pydantic_unknown_tool_name_raises() {
    let parser = PydanticToolsParser::new(vec![], false).with_tool::<XTool>("XTool");
    let generation = tool_call_generation(
        vec![make_tool_call("c1", "UnknownTool", json!({"x": 1}))],
        "",
        HashMap::new(),
    );
    let result = parser.parse_result(&[generation], false);
    assert!(result.is_err());
}

#[test]
fn test_pydantic_custom_title_model() {
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct CustomTool {
        val: i64,
    }

    // Register with a custom name (equivalent to Pydantic model_config title)
    let parser = PydanticToolsParser::new(vec![], false).with_tool::<CustomTool>("MyCustomName");
    let generation = tool_call_generation(
        vec![make_tool_call("c1", "MyCustomName", json!({"val": 99}))],
        "",
        HashMap::new(),
    );
    let result = parser.parse_result(&[generation], false).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    let tool: CustomTool = serde_json::from_value(arr[0].clone()).unwrap();
    assert_eq!(tool.val, 99);
}

#[test]
fn test_pydantic_nested_models() {
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Inner {
        val: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Outer {
        inner: Inner,
        name: String,
    }

    let parser = PydanticToolsParser::new(vec![], false).with_tool::<Outer>("Outer");
    let generation = tool_call_generation(
        vec![make_tool_call(
            "c1",
            "Outer",
            json!({"inner": {"val": "deep"}, "name": "top"}),
        )],
        "",
        HashMap::new(),
    );
    let result = parser.parse_result(&[generation], false).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    let outer: Outer = serde_json::from_value(arr[0].clone()).unwrap();
    assert_eq!(outer.inner.val, "deep");
    assert_eq!(outer.name, "top");
}

#[test]
fn test_pydantic_optional_fields() {
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct OptTool {
        required: String,
        optional: Option<String>,
    }

    let parser = PydanticToolsParser::new(vec![], false).with_tool::<OptTool>("OptTool");
    let generation = tool_call_generation(
        vec![make_tool_call("c1", "OptTool", json!({"required": "yes"}))],
        "",
        HashMap::new(),
    );
    let result = parser.parse_result(&[generation], false).unwrap();
    let arr = result.as_array().unwrap();
    let tool: OptTool = serde_json::from_value(arr[0].clone()).unwrap();
    assert_eq!(tool.required, "yes");
    assert!(tool.optional.is_none());
}
