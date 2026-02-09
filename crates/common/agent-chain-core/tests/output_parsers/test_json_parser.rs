//! Snapshot tests for JsonOutputParser.
//!
//! Ported from langchain/libs/core/tests/unit_tests/output_parsers/test_json_parser.py

use agent_chain_core::output_parsers::{
    BaseCumulativeTransformOutputParser, BaseOutputParser, JsonOutputParser, SimpleJsonOutputParser,
};
use agent_chain_core::outputs::Generation;
use serde_json::json;

// --- JsonOutputParser.parse() tests ---

#[test]
fn test_parse_valid_json() {
    let parser = JsonOutputParser::new();
    let result = parser.parse(r#"{"foo": "bar"}"#).unwrap();
    assert_eq!(result, json!({"foo": "bar"}));
}

#[test]
fn test_parse_json_in_code_block() {
    let parser = JsonOutputParser::new();
    let result = parser.parse("```json\n{\"foo\": \"bar\"}\n```").unwrap();
    assert_eq!(result, json!({"foo": "bar"}));
}

#[test]
fn test_parse_json_in_plain_code_block() {
    let parser = JsonOutputParser::new();
    let result = parser.parse("```\n{\"foo\": \"bar\"}\n```").unwrap();
    assert_eq!(result, json!({"foo": "bar"}));
}

#[test]
fn test_parse_json_with_surrounding_text() {
    let parser = JsonOutputParser::new();
    let result = parser
        .parse("Some text\n```\n{\"foo\": \"bar\"}\n```\nMore text")
        .unwrap();
    assert_eq!(result, json!({"foo": "bar"}));
}

#[test]
fn test_parse_invalid_json_raises() {
    let parser = JsonOutputParser::new();
    let result = parser.parse("not json at all");
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Invalid json output"),
        "Expected 'Invalid json output' in error, got: {err_msg}"
    );
}

#[test]
fn test_parse_nested_json() {
    let parser = JsonOutputParser::new();
    let result = parser
        .parse(r#"{"outer": {"inner": {"deep": "value"}}}"#)
        .unwrap();
    assert_eq!(result, json!({"outer": {"inner": {"deep": "value"}}}));
}

#[test]
fn test_parse_json_with_array() {
    let parser = JsonOutputParser::new();
    let result = parser
        .parse(r#"{"items": [1, 2, 3], "name": "test"}"#)
        .unwrap();
    assert_eq!(result, json!({"items": [1, 2, 3], "name": "test"}));
}

#[test]
fn test_parse_json_with_newlines_in_values() {
    let parser = JsonOutputParser::new();
    let result = parser.parse(r#"{"code": "line1\nline2"}"#).unwrap();
    assert_eq!(result, json!({"code": "line1\nline2"}));
}

#[test]
fn test_parse_json_with_unicode() {
    let parser = JsonOutputParser::new();
    let result = parser.parse(r#"{"name": "你好世界"}"#).unwrap();
    assert_eq!(result, json!({"name": "你好世界"}));
}

#[test]
fn test_parse_json_with_whitespace() {
    let parser = JsonOutputParser::new();
    let result = parser.parse("  \n  {\"foo\": \"bar\"}  \n  ").unwrap();
    assert_eq!(result, json!({"foo": "bar"}));
}

#[test]
fn test_parse_json_with_boolean_and_null() {
    let parser = JsonOutputParser::new();
    let result = parser
        .parse(r#"{"active": true, "deleted": false, "metadata": null}"#)
        .unwrap();
    assert_eq!(
        result,
        json!({"active": true, "deleted": false, "metadata": null})
    );
}

#[test]
fn test_parse_json_numeric_values() {
    let parser = JsonOutputParser::new();
    let result = parser
        .parse(r#"{"int": 42, "float": 3.15, "negative": -1}"#)
        .unwrap();
    assert_eq!(result, json!({"int": 42, "float": 3.15, "negative": -1}));
}

// --- JsonOutputParser.parse_result() tests ---

#[test]
fn test_parse_result_full() {
    let parser = JsonOutputParser::new();
    let generation = Generation::new(r#"{"key": "value"}"#);
    let result = parser.parse_result(&[generation], false).unwrap();
    assert_eq!(result, json!({"key": "value"}));
}

#[test]
fn test_parse_result_partial_valid() {
    let parser = JsonOutputParser::new();
    let generation = Generation::new(r#"{"key": "val"#);
    let result = parser.parse_result(&[generation], true).unwrap();
    assert_eq!(result, json!({"key": "val"}));
}

#[test]
fn test_parse_result_partial_returns_err_for_unparseable() {
    // In Python, partial parsing of unparseable text returns None.
    // In Rust, the idiomatic equivalent is returning Err, which the streaming
    // infrastructure uses to skip unparseable intermediate states.
    let parser = JsonOutputParser::new();
    let generation = Generation::new("not json");
    let result = parser.parse_result(&[generation], true);
    assert!(result.is_err());
}

#[test]
fn test_parse_result_non_partial_raises_on_invalid() {
    let parser = JsonOutputParser::new();
    let generation = Generation::new("not json");
    let result = parser.parse_result(&[generation], false);
    assert!(result.is_err());
}

// --- JsonOutputParser.get_format_instructions() tests ---

#[test]
fn test_format_instructions_no_schema() {
    let parser = JsonOutputParser::new();
    let instructions = parser.get_format_instructions().unwrap();
    assert_eq!(instructions, "Return a JSON object.");
}

#[test]
fn test_format_instructions_with_schema() {
    let schema = json!({
        "title": "Joke",
        "type": "object",
        "properties": {
            "setup": {
                "type": "string",
                "description": "The setup of the joke"
            },
            "punchline": {
                "type": "string",
                "description": "The punchline"
            }
        }
    });
    let parser = JsonOutputParser::with_schema(schema);
    let instructions = parser.get_format_instructions().unwrap();
    assert!(
        instructions.contains("setup"),
        "Instructions should mention 'setup'"
    );
    assert!(
        instructions.contains("punchline"),
        "Instructions should mention 'punchline'"
    );
    assert!(
        !instructions.contains("{schema}"),
        "Placeholder should be filled"
    );
}

#[test]
fn test_unicode_preserved_in_instructions() {
    let schema = json!({
        "title": "UnicodeModel",
        "type": "object",
        "properties": {
            "title": {
                "type": "string",
                "description": "科学文章的标题"
            },
            "content": {
                "type": "string",
                "description": "文章内容"
            }
        }
    });
    let parser = JsonOutputParser::with_schema(schema);
    let instructions = parser.get_format_instructions().unwrap();
    assert!(
        instructions.contains("科学文章的标题"),
        "Unicode should be preserved"
    );
    assert!(
        instructions.contains("文章内容"),
        "Unicode should be preserved"
    );
}

#[test]
fn test_format_instructions_do_not_alter_schema() {
    let schema = json!({
        "title": "Joke",
        "type": "object",
        "properties": {
            "setup": {"type": "string"},
            "punchline": {"type": "string"}
        }
    });
    let initial_schema = schema.clone();
    let parser = JsonOutputParser::with_schema(schema);
    let _ = parser.get_format_instructions().unwrap();
    // The original schema stored in the parser should not be mutated
    assert_eq!(
        parser.get_schema().unwrap(),
        &initial_schema,
        "get_format_instructions should not alter the stored schema"
    );
}

// --- JsonOutputParser parser_type() tests ---

#[test]
fn test_parser_type() {
    let parser = JsonOutputParser::new();
    assert_eq!(parser.parser_type(), "simple_json_output_parser");
}

// --- JsonOutputParser.get_schema() tests ---

#[test]
fn test_get_schema_with_properties() {
    let schema = json!({
        "title": "Joke",
        "type": "object",
        "properties": {
            "setup": {
                "type": "string",
                "description": "The setup of the joke"
            },
            "punchline": {
                "type": "string",
                "description": "The punchline"
            }
        }
    });
    let parser = JsonOutputParser::with_schema(schema);
    let retrieved = parser.get_schema().unwrap();
    assert!(retrieved.get("properties").is_some());
    assert!(retrieved["properties"].get("setup").is_some());
    assert!(retrieved["properties"].get("punchline").is_some());
}

// --- SimpleJsonOutputParser alias tests ---

#[test]
fn test_simple_json_output_parser_parse() {
    let parser = SimpleJsonOutputParser::new();
    let result = parser.parse(r#"{"a": 1}"#).unwrap();
    assert_eq!(result, json!({"a": 1}));
}

#[test]
fn test_simple_json_output_parser_is_same_type() {
    // SimpleJsonOutputParser is a type alias for JsonOutputParser in Rust.
    // Verify they have the same behavior.
    let json_parser = JsonOutputParser::new();
    let simple_parser = SimpleJsonOutputParser::new();

    let input = r#"{"key": "value"}"#;
    assert_eq!(
        json_parser.parse(input).unwrap(),
        simple_parser.parse(input).unwrap()
    );
    assert_eq!(json_parser.parser_type(), simple_parser.parser_type());
}

// --- JsonOutputParser diff tests ---

#[test]
fn test_diff_add_key() {
    let parser = JsonOutputParser::new().with_diff();
    let prev = json!({"a": 1});
    let next = json!({"a": 1, "b": 2});
    let diff = parser.compute_diff(Some(&prev), next);
    let patches = diff.as_array().expect("diff should be an array");
    assert!(
        patches
            .iter()
            .any(|op| op["op"] == "add" && op["path"] == "/b"),
        "Should contain an 'add' op for /b, got: {patches:?}"
    );
}

#[test]
fn test_diff_replace_value() {
    let parser = JsonOutputParser::new().with_diff();
    let prev = json!({"a": 1});
    let next = json!({"a": 2});
    let diff = parser.compute_diff(Some(&prev), next);
    let patches = diff.as_array().expect("diff should be an array");
    assert!(
        patches.iter().any(|op| op["op"] == "replace"),
        "Should contain a 'replace' op, got: {patches:?}"
    );
}

#[test]
fn test_diff_remove_key() {
    let parser = JsonOutputParser::new().with_diff();
    let prev = json!({"a": 1, "b": 2});
    let next = json!({"a": 1});
    let diff = parser.compute_diff(Some(&prev), next);
    let patches = diff.as_array().expect("diff should be an array");
    assert!(
        patches
            .iter()
            .any(|op| op["op"] == "remove" && op["path"] == "/b"),
        "Should contain a 'remove' op for /b, got: {patches:?}"
    );
}

#[test]
fn test_diff_from_none() {
    let parser = JsonOutputParser::new().with_diff();
    let next = json!({"a": 1});
    let diff = parser.compute_diff(None, next);
    let patches = diff.as_array().expect("diff should be an array");
    assert!(!patches.is_empty(), "Diff from None should produce patches");
}

#[test]
fn test_diff_no_change() {
    let parser = JsonOutputParser::new().with_diff();
    let prev = json!({"a": 1});
    let next = json!({"a": 1});
    let diff = parser.compute_diff(Some(&prev), next);
    let patches = diff.as_array().expect("diff should be an array");
    assert!(patches.is_empty(), "No-change diff should be empty");
}
