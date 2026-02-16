//! Tests for function calling utilities.
//!
//! This module tests the conversion of various types to OpenAI function format.
//! Mirrors `langchain/libs/core/tests/unit_tests/utils/test_function_calling.py`.
//!
//! # Note on Python vs Rust Differences
//!
//! The Python tests use various input types including:
//! - Pydantic models (converted via class introspection)
//! - Functions (converted via signature introspection)
//! - Method references (Dummy.dummy_function, DummyWithClassMethod.dummy_function)
//! - TypedDict classes
//!
//! In Rust, we don't have the same runtime introspection capabilities, so we:
//! - Test with JSON schemas (Value) which is the canonical representation
//! - Test with Tool/StructuredTool instances
//! - Cannot test direct function/method conversion (would require proc macros)
//!
//! The underlying conversion logic is the same - Python just has more ways to
//! create the input schemas at runtime.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use agent_chain_core::messages::BaseMessage;
use agent_chain_core::tools::{ArgsSchema, StructuredTool, Tool};
use agent_chain_core::utils::function_calling::{
    FunctionDescription, ToolDescription, convert_to_json_schema, convert_to_openai_function,
    convert_typed_dict_to_openai_function, tool_example_to_messages,
};

/// Expected JSON schema for dummy_function
fn expected_json_schema() -> Value {
    json!({
        "title": "dummy_function",
        "description": "Dummy function.",
        "type": "object",
        "properties": {
            "arg1": {"description": "foo", "type": "integer"},
            "arg2": {
                "description": "one of 'bar', 'baz'",
                "enum": ["bar", "baz"],
                "type": "string",
            },
        },
        "required": ["arg1", "arg2"],
    })
}

/// Expected OpenAI function format for dummy_function
fn expected_openai_function() -> Value {
    json!({
        "name": "dummy_function",
        "description": "Dummy function.",
        "parameters": {
            "type": "object",
            "properties": {
                "arg1": {"description": "foo", "type": "integer"},
                "arg2": {
                    "description": "one of 'bar', 'baz'",
                    "enum": ["bar", "baz"],
                    "type": "string",
                },
            },
            "required": ["arg1", "arg2"],
        },
    })
}

/// Anthropic tool format fixture
fn anthropic_tool() -> Value {
    json!({
        "name": "dummy_function",
        "description": "Dummy function.",
        "input_schema": {
            "type": "object",
            "properties": {
                "arg1": {"description": "foo", "type": "integer"},
                "arg2": {
                    "description": "one of 'bar', 'baz'",
                    "enum": ["bar", "baz"],
                    "type": "string",
                },
            },
            "required": ["arg1", "arg2"],
        },
    })
}

/// Bedrock Converse tool format fixture
fn bedrock_converse_tool() -> Value {
    json!({
        "toolSpec": {
            "name": "dummy_function",
            "description": "Dummy function.",
            "inputSchema": {
                "json": {
                    "type": "object",
                    "properties": {
                        "arg1": {"description": "foo", "type": "integer"},
                        "arg2": {
                            "description": "one of 'bar', 'baz'",
                            "enum": ["bar", "baz"],
                            "type": "string",
                        },
                    },
                    "required": ["arg1", "arg2"],
                }
            },
        }
    })
}

// =============================================================================
// TESTS: convert_to_openai_function
// =============================================================================

#[test]

fn test_convert_to_openai_function_from_json_schema() {
    let json_schema = expected_json_schema();
    let expected = expected_openai_function();

    let actual = convert_to_openai_function(&json_schema, None);

    assert_eq!(actual, expected);
}

#[test]

fn test_convert_to_openai_function_from_anthropic_tool() {
    let expected = expected_openai_function();

    let actual = convert_to_openai_function(&anthropic_tool(), None);

    assert_eq!(actual, expected);
}

#[test]

fn test_convert_to_openai_function_from_bedrock_converse_tool() {
    let expected = expected_openai_function();

    let actual = convert_to_openai_function(&bedrock_converse_tool(), None);

    assert_eq!(actual, expected);
}

#[test]

fn test_convert_to_openai_function_from_openai_function() {
    let expected = expected_openai_function();

    // Convert from already-OpenAI format should return the same
    let actual = convert_to_openai_function(&expected, None);

    assert_eq!(actual, expected);
}

#[test]
fn test_convert_to_openai_function_nested() {
    // In Rust, we'd represent the function schema differently.
    // The test verifies nested Pydantic models work correctly.
    let nested_schema = json!({
        "title": "my_function",
        "description": "Dummy function.",
        "type": "object",
        "properties": {
            "arg1": {
                "type": "object",
                "properties": {
                    "nested_arg1": {"type": "integer", "description": "foo"},
                    "nested_arg2": {
                        "type": "string",
                        "enum": ["bar", "baz"],
                        "description": "one of 'bar', 'baz'",
                    },
                },
                "required": ["nested_arg1", "nested_arg2"],
            },
        },
        "required": ["arg1"],
    });

    let expected = json!({
        "name": "my_function",
        "description": "Dummy function.",
        "parameters": {
            "type": "object",
            "properties": {
                "arg1": {
                    "type": "object",
                    "properties": {
                        "nested_arg1": {"type": "integer", "description": "foo"},
                        "nested_arg2": {
                            "type": "string",
                            "enum": ["bar", "baz"],
                            "description": "one of 'bar', 'baz'",
                        },
                    },
                    "required": ["nested_arg1", "nested_arg2"],
                },
            },
            "required": ["arg1"],
        },
    });

    let actual = convert_to_openai_function(&nested_schema, None);
    assert_eq!(actual, expected);
}

#[test]

fn test_convert_to_openai_function_nested_strict() {
    let nested_schema = json!({
        "title": "my_function",
        "description": "Dummy function.",
        "type": "object",
        "properties": {
            "arg1": {
                "type": "object",
                "properties": {
                    "nested_arg1": {"type": "integer", "description": "foo"},
                    "nested_arg2": {
                        "type": "string",
                        "enum": ["bar", "baz"],
                        "description": "one of 'bar', 'baz'",
                    },
                },
                "required": ["nested_arg1", "nested_arg2"],
            },
        },
        "required": ["arg1"],
    });

    let expected = json!({
        "name": "my_function",
        "description": "Dummy function.",
        "parameters": {
            "type": "object",
            "properties": {
                "arg1": {
                    "type": "object",
                    "properties": {
                        "nested_arg1": {"type": "integer", "description": "foo"},
                        "nested_arg2": {
                            "type": "string",
                            "enum": ["bar", "baz"],
                            "description": "one of 'bar', 'baz'",
                        },
                    },
                    "required": ["nested_arg1", "nested_arg2"],
                    "additionalProperties": false,
                },
            },
            "required": ["arg1"],
            "additionalProperties": false,
        },
        "strict": true,
    });

    let actual = convert_to_openai_function(&nested_schema, Some(true));
    assert_eq!(actual, expected);
}

#[test]

fn test_convert_to_openai_function_strict_union_of_objects_arg_type() {
    let schema = json!({
        "title": "my_function",
        "description": "Dummy function.",
        "type": "object",
        "properties": {
            "my_arg": {
                "anyOf": [
                    {
                        "properties": {"foo": {"title": "Foo", "type": "string"}},
                        "required": ["foo"],
                        "title": "NestedA",
                        "type": "object",
                    },
                    {
                        "properties": {"bar": {"title": "Bar", "type": "integer"}},
                        "required": ["bar"],
                        "title": "NestedB",
                        "type": "object",
                    },
                    {
                        "properties": {"baz": {"title": "Baz", "type": "boolean"}},
                        "required": ["baz"],
                        "title": "NestedC",
                        "type": "object",
                    },
                ]
            }
        },
        "required": ["my_arg"],
    });

    let expected = json!({
        "name": "my_function",
        "description": "Dummy function.",
        "parameters": {
            "properties": {
                "my_arg": {
                    "anyOf": [
                        {
                            "properties": {"foo": {"title": "Foo", "type": "string"}},
                            "required": ["foo"],
                            "title": "NestedA",
                            "type": "object",
                            "additionalProperties": false,
                        },
                        {
                            "properties": {"bar": {"title": "Bar", "type": "integer"}},
                            "required": ["bar"],
                            "title": "NestedB",
                            "type": "object",
                            "additionalProperties": false,
                        },
                        {
                            "properties": {"baz": {"title": "Baz", "type": "boolean"}},
                            "required": ["baz"],
                            "title": "NestedC",
                            "type": "object",
                            "additionalProperties": false,
                        },
                    ]
                }
            },
            "required": ["my_arg"],
            "type": "object",
            "additionalProperties": false,
        },
        "strict": true,
    });

    let actual = convert_to_openai_function(&schema, Some(true));
    assert_eq!(actual, expected);
}

// Tests for schemas without description

fn json_schema_no_description_no_params() -> Value {
    json!({
        "title": "dummy_function",
    })
}

fn json_schema_no_description() -> Value {
    json!({
        "title": "dummy_function",
        "type": "object",
        "properties": {
            "arg1": {"description": "foo", "type": "integer"},
            "arg2": {
                "description": "one of 'bar', 'baz'",
                "enum": ["bar", "baz"],
                "type": "string",
            },
        },
        "required": ["arg1", "arg2"],
    })
}

fn anthropic_tool_no_description() -> Value {
    json!({
        "name": "dummy_function",
        "input_schema": {
            "type": "object",
            "properties": {
                "arg1": {"description": "foo", "type": "integer"},
                "arg2": {
                    "description": "one of 'bar', 'baz'",
                    "enum": ["bar", "baz"],
                    "type": "string",
                },
            },
            "required": ["arg1", "arg2"],
        },
    })
}

fn bedrock_converse_tool_no_description() -> Value {
    json!({
        "toolSpec": {
            "name": "dummy_function",
            "inputSchema": {
                "json": {
                    "type": "object",
                    "properties": {
                        "arg1": {"description": "foo", "type": "integer"},
                        "arg2": {
                            "description": "one of 'bar', 'baz'",
                            "enum": ["bar", "baz"],
                            "type": "string",
                        },
                    },
                    "required": ["arg1", "arg2"],
                }
            },
        }
    })
}

fn openai_function_no_description() -> Value {
    json!({
        "name": "dummy_function",
        "parameters": {
            "type": "object",
            "properties": {
                "arg1": {"description": "foo", "type": "integer"},
                "arg2": {
                    "description": "one of 'bar', 'baz'",
                    "enum": ["bar", "baz"],
                    "type": "string",
                },
            },
            "required": ["arg1", "arg2"],
        },
    })
}

fn openai_function_no_description_no_params() -> Value {
    json!({
        "name": "dummy_function",
    })
}

#[test]

fn test_convert_to_openai_function_no_description_anthropic() {
    let expected = json!({
        "name": "dummy_function",
        "parameters": {
            "type": "object",
            "properties": {
                "arg1": {"description": "foo", "type": "integer"},
                "arg2": {
                    "description": "one of 'bar', 'baz'",
                    "enum": ["bar", "baz"],
                    "type": "string",
                },
            },
            "required": ["arg1", "arg2"],
        },
    });

    let actual = convert_to_openai_function(&anthropic_tool_no_description(), None);
    assert_eq!(actual, expected);
}

#[test]

fn test_convert_to_openai_function_no_description_json_schema() {
    let expected = json!({
        "name": "dummy_function",
        "parameters": {
            "type": "object",
            "properties": {
                "arg1": {"description": "foo", "type": "integer"},
                "arg2": {
                    "description": "one of 'bar', 'baz'",
                    "enum": ["bar", "baz"],
                    "type": "string",
                },
            },
            "required": ["arg1", "arg2"],
        },
    });

    let actual = convert_to_openai_function(&json_schema_no_description(), None);
    assert_eq!(actual, expected);
}

#[test]

fn test_convert_to_openai_function_no_description_bedrock() {
    let expected = json!({
        "name": "dummy_function",
        "parameters": {
            "type": "object",
            "properties": {
                "arg1": {"description": "foo", "type": "integer"},
                "arg2": {
                    "description": "one of 'bar', 'baz'",
                    "enum": ["bar", "baz"],
                    "type": "string",
                },
            },
            "required": ["arg1", "arg2"],
        },
    });

    let actual = convert_to_openai_function(&bedrock_converse_tool_no_description(), None);
    assert_eq!(actual, expected);
}

#[test]

fn test_convert_to_openai_function_no_description_openai() {
    let expected = json!({
        "name": "dummy_function",
        "parameters": {
            "type": "object",
            "properties": {
                "arg1": {"description": "foo", "type": "integer"},
                "arg2": {
                    "description": "one of 'bar', 'baz'",
                    "enum": ["bar", "baz"],
                    "type": "string",
                },
            },
            "required": ["arg1", "arg2"],
        },
    });

    let actual = convert_to_openai_function(&openai_function_no_description(), None);
    assert_eq!(actual, expected);
}

#[test]

fn test_convert_to_openai_function_no_description_no_params_json_schema() {
    let expected = json!({
        "name": "dummy_function",
    });

    let actual = convert_to_openai_function(&json_schema_no_description_no_params(), None);
    assert_eq!(actual, expected);
}

#[test]

fn test_convert_to_openai_function_no_description_no_params_openai() {
    let expected = json!({
        "name": "dummy_function",
    });

    let actual = convert_to_openai_function(&openai_function_no_description_no_params(), None);
    assert_eq!(actual, expected);
}

#[test]

fn test_function_no_params() {
    let schema = json!({
        "title": "nullary_function",
        "description": "Nullary function.",
        "type": "object",
        "properties": {},
    });

    let func = convert_to_openai_function(&schema, None);
    let required = func
        .get("parameters")
        .and_then(|p| p.get("required"))
        .and_then(|r| r.as_array());

    assert!(required.is_none() || required.unwrap().is_empty());
}

#[test]

fn test_convert_union_type() {
    // Schema representing a function with union type: int | str
    let schema = json!({
        "title": "magic_function",
        "description": "Compute a magic function.",
        "type": "object",
        "properties": {
            "value": {
                "anyOf": [{"type": "integer"}, {"type": "string"}]
            }
        },
        "required": ["value"],
    });

    let result = convert_to_openai_function(&schema, None);
    let value_prop = result
        .get("parameters")
        .and_then(|p| p.get("properties"))
        .and_then(|props| props.get("value"));

    assert_eq!(
        value_prop,
        Some(&json!({
            "anyOf": [{"type": "integer"}, {"type": "string"}]
        }))
    );
}

#[test]

fn test_convert_to_openai_function_no_args() {
    let schema = json!({
        "title": "empty_tool",
        "description": "No args.",
        "type": "object",
        "properties": {},
    });

    let actual = convert_to_openai_function(&schema, Some(true));
    let expected = json!({
        "name": "empty_tool",
        "description": "No args.",
        "parameters": {
            "properties": {},
            "additionalProperties": false,
            "type": "object",
        },
        "strict": true,
    });

    assert_eq!(actual, expected);
}

#[test]
fn test_convert_to_openai_function_nested_strict_2() {
    // Note: In Python, this test has version-dependent behavior:
    // - Pydantic >= 2.11: adds `additionalProperties: false` to bare `object` types
    // - Pydantic < 2.11: does NOT add `additionalProperties: false` to bare `object` types
    // The Rust implementation follows the Pydantic < 2.11 behavior for bare object types
    // (only adds `additionalProperties: false` to objects with defined properties).
    let schema = json!({
        "title": "my_function",
        "description": "Dummy function.",
        "type": "object",
        "properties": {
            "arg1": {"type": "object"},
            "arg2": {
                "anyOf": [
                    {"type": "object"},
                    {"type": "null"},
                ],
            },
        },
        "required": ["arg1", "arg2"],
    });

    // Expected output follows Pydantic < 2.11 behavior:
    // - Bare object types (without `properties`) do NOT get `additionalProperties: false`
    // - Only the top-level parameters object gets `additionalProperties: false`
    let expected = json!({
        "name": "my_function",
        "description": "Dummy function.",
        "parameters": {
            "type": "object",
            "properties": {
                "arg1": {"type": "object"},
                "arg2": {
                    "anyOf": [
                        {"type": "object"},
                        {"type": "null"},
                    ],
                },
            },
            "required": ["arg1", "arg2"],
            "additionalProperties": false,
        },
        "strict": true,
    });

    let actual = convert_to_openai_function(&schema, Some(true));
    assert_eq!(actual, expected);
}

#[test]

fn test_convert_to_openai_function_strict_required() {
    let schema = json!({
        "title": "MyModel",
        "description": "Dummy schema.",
        "type": "object",
        "properties": {
            "arg1": {"type": "integer", "description": "foo"},
            "arg2": {
                "anyOf": [{"type": "string"}, {"type": "null"}],
                "description": "bar",
                "default": null,
            },
        },
        "required": ["arg1"],
    });

    let func = convert_to_openai_function(&schema, Some(true));
    let required = func
        .get("parameters")
        .and_then(|p| p.get("required"))
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();

    // With strict=true, all fields should be required
    assert_eq!(required, vec!["arg1", "arg2"]);
}

// =============================================================================
// TESTS: tool_example_to_messages
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FakeCall {
    data: String,
}

#[test]

fn test_valid_example_conversion() {
    let empty_calls: Vec<FakeCall> = vec![];
    let messages = tool_example_to_messages("This is a valid example", empty_calls, None, None);

    assert_eq!(messages.len(), 2);
    assert!(matches!(&messages[0], BaseMessage::Human(_)));
    assert!(matches!(&messages[1], BaseMessage::AI(_)));

    if let BaseMessage::Human(human_msg) = &messages[0] {
        assert_eq!(human_msg.content.as_text(), "This is a valid example");
    }

    if let BaseMessage::AI(ai_msg) = &messages[1] {
        assert_eq!(ai_msg.content, "");
        let tool_calls = ai_msg
            .additional_kwargs
            .get("tool_calls")
            .and_then(|tc| tc.as_array());
        assert!(tool_calls.map(|tc| tc.is_empty()).unwrap_or(true));
    }
}

#[test]

fn test_multiple_tool_calls() {
    let tool_calls = vec![
        FakeCall {
            data: "ToolCall1".to_string(),
        },
        FakeCall {
            data: "ToolCall2".to_string(),
        },
        FakeCall {
            data: "ToolCall3".to_string(),
        },
    ];

    let messages = tool_example_to_messages("This is an example", tool_calls, None, None);

    assert_eq!(messages.len(), 5);
    assert!(matches!(&messages[0], BaseMessage::Human(_)));
    assert!(matches!(&messages[1], BaseMessage::AI(_)));
    assert!(matches!(&messages[2], BaseMessage::Tool(_)));
    assert!(matches!(&messages[3], BaseMessage::Tool(_)));
    assert!(matches!(&messages[4], BaseMessage::Tool(_)));

    // Verify AI message has correct tool_calls structure
    if let BaseMessage::AI(ai_msg) = &messages[1] {
        let tool_calls = ai_msg
            .additional_kwargs
            .get("tool_calls")
            .and_then(|tc| tc.as_array())
            .expect("tool_calls should be an array");

        assert_eq!(tool_calls.len(), 3);

        // Check first tool call
        let first_call = &tool_calls[0];
        assert_eq!(first_call.get("type").unwrap(), "function");
        let function = first_call.get("function").unwrap();
        assert_eq!(function.get("name").unwrap(), "FakeCall");
        assert_eq!(
            function.get("arguments").unwrap(),
            r#"{"data":"ToolCall1"}"#
        );
    }
}

#[test]

fn test_tool_outputs() {
    let tool_calls = vec![FakeCall {
        data: "ToolCall1".to_string(),
    }];
    let tool_outputs = vec!["Output1".to_string()];

    let messages =
        tool_example_to_messages("This is an example", tool_calls, Some(tool_outputs), None);

    assert_eq!(messages.len(), 3);
    assert!(matches!(&messages[0], BaseMessage::Human(_)));
    assert!(matches!(&messages[1], BaseMessage::AI(_)));
    assert!(matches!(&messages[2], BaseMessage::Tool(_)));

    if let BaseMessage::Tool(tool_msg) = &messages[2] {
        assert_eq!(tool_msg.content, "Output1");
    }
}

#[test]

fn test_tool_outputs_with_ai_response() {
    let tool_calls = vec![FakeCall {
        data: "ToolCall1".to_string(),
    }];
    let tool_outputs = vec!["Output1".to_string()];
    let ai_response = "The output is Output1".to_string();

    let messages = tool_example_to_messages(
        "This is an example",
        tool_calls,
        Some(tool_outputs),
        Some(ai_response),
    );

    assert_eq!(messages.len(), 4);
    assert!(matches!(&messages[0], BaseMessage::Human(_)));
    assert!(matches!(&messages[1], BaseMessage::AI(_)));
    assert!(matches!(&messages[2], BaseMessage::Tool(_)));
    assert!(matches!(&messages[3], BaseMessage::AI(_)));

    if let BaseMessage::AI(response) = &messages[3] {
        assert_eq!(response.content, "The output is Output1");
        // Final AI response should not have tool calls
        let tool_calls = response
            .additional_kwargs
            .get("tool_calls")
            .and_then(|tc| tc.as_array());
        assert!(tool_calls.is_none() || tool_calls.unwrap().is_empty());
    }
}

// =============================================================================
// TESTS: convert_typed_dict_to_openai_function
// =============================================================================

#[test]
fn test_convert_typed_dict_to_openai_function() {
    // In Rust, TypedDict is represented as a struct with typed fields.
    // We use JSON schema representation for testing.

    // The schema that would be generated from a TypedDict-like struct
    let typed_dict_schema = json!({
        "title": "Tool",
        "description": "Docstring.",
        "type": "object",
        "properties": {
            "arg1": {"description": "foo", "type": "string"},
            "arg2": {
                "anyOf": [
                    {"type": "integer"},
                    {"type": "string"},
                    {"type": "boolean"},
                ]
            },
            "arg3": {
                "type": "array",
                "items": {
                    "description": "Subtool docstring.",
                    "type": "object",
                    "properties": {
                        "args": {
                            "description": "this does bar",
                            "default": {},
                            "type": "object",
                        }
                    },
                },
            },
            "arg4": {
                "description": "this does foo",
                "enum": ["bar", "baz"],
                "type": "string",
            },
            "arg5": {"type": "number"},
            "arg15": {"description": "flag", "default": false, "type": "boolean"},
        },
        "required": ["arg1", "arg2", "arg3", "arg4"],
    });

    let expected = json!({
        "name": "Tool",
        "description": "Docstring.",
        "parameters": {
            "type": "object",
            "properties": {
                "arg1": {"description": "foo", "type": "string"},
                "arg2": {
                    "anyOf": [
                        {"type": "integer"},
                        {"type": "string"},
                        {"type": "boolean"},
                    ]
                },
                "arg3": {
                    "type": "array",
                    "items": {
                        "description": "Subtool docstring.",
                        "type": "object",
                        "properties": {
                            "args": {
                                "description": "this does bar",
                                "default": {},
                                "type": "object",
                            }
                        },
                    },
                },
                "arg4": {
                    "description": "this does foo",
                    "enum": ["bar", "baz"],
                    "type": "string",
                },
                "arg5": {"type": "number"},
                "arg15": {"description": "flag", "default": false, "type": "boolean"},
            },
            "required": ["arg1", "arg2", "arg3", "arg4"],
        },
    });

    let actual = convert_typed_dict_to_openai_function(&typed_dict_schema);
    assert_eq!(actual, expected);
}

// =============================================================================
// TESTS: convert_to_json_schema
// =============================================================================

#[test]

fn test_convert_to_json_schema_from_json_schema() {
    let expected = expected_json_schema();

    let actual = convert_to_json_schema(&expected_json_schema(), None).unwrap();

    assert_eq!(actual, expected);
}

#[test]

fn test_convert_to_json_schema_from_anthropic_tool() {
    let expected = expected_json_schema();

    let actual = convert_to_json_schema(&anthropic_tool(), None).unwrap();

    assert_eq!(actual, expected);
}

#[test]

fn test_convert_to_json_schema_from_bedrock_converse_tool() {
    let expected = expected_json_schema();

    let actual = convert_to_json_schema(&bedrock_converse_tool(), None).unwrap();

    assert_eq!(actual, expected);
}

#[test]

fn test_convert_to_json_schema_from_openai_function() {
    let expected = expected_json_schema();

    let actual = convert_to_json_schema(&expected_openai_function(), None).unwrap();

    assert_eq!(actual, expected);
}

// =============================================================================
// TESTS: Tool conversions
// =============================================================================

#[test]

fn test_convert_to_openai_function_from_structured_tool() {
    // Create a StructuredTool with the same schema as dummy_function
    let args_schema = json!({
        "type": "object",
        "properties": {
            "arg1": {"type": "integer", "description": "foo"},
            "arg2": {
                "type": "string",
                "enum": ["bar", "baz"],
                "description": "one of 'bar', 'baz'",
            },
        },
        "required": ["arg1", "arg2"],
    });

    let tool = StructuredTool::from_function(
        |_args| Ok(Value::Null),
        "dummy_function",
        "Dummy function.",
        ArgsSchema::JsonSchema(args_schema),
    );

    let expected = expected_openai_function();
    let actual = convert_to_openai_function(&tool, None);

    assert_eq!(actual, expected);
}

#[test]

fn test_convert_to_openai_function_from_structured_tool_args_schema_dict() {
    let args_schema = json!({
        "type": "object",
        "properties": {
            "arg1": {"type": "integer", "description": "foo"},
            "arg2": {
                "type": "string",
                "enum": ["bar", "baz"],
                "description": "one of 'bar', 'baz'",
            },
        },
        "required": ["arg1", "arg2"],
    });

    let tool = StructuredTool::from_function(
        |_args| Ok(Value::Null),
        "dummy_function",
        "Dummy function.",
        ArgsSchema::JsonSchema(args_schema),
    );

    let expected = expected_openai_function();
    let actual = convert_to_openai_function(&tool, None);

    assert_eq!(actual, expected);
}
#[test]
fn test_convert_to_openai_function_from_simple_tool() {
    let tool = Tool::from_function(
        |_input: String| Ok("".to_string()),
        "dummy_function",
        "test description",
    );

    let expected = json!({
        "name": "dummy_function",
        "description": "test description",
        "parameters": {
            "properties": {"__arg1": {"title": "__arg1", "type": "string"}},
            "required": ["__arg1"],
            "type": "object",
        },
    });

    let actual = convert_to_openai_function(&tool, None);

    assert_eq!(actual, expected);
}

// =============================================================================
// FunctionDescription and ToolDescription type definitions tests
// =============================================================================

#[test]
fn test_function_description_structure() {
    // Verify the FunctionDescription structure matches Python's TypedDict
    let func_desc = FunctionDescription {
        name: "test_function".to_string(),
        description: "A test function.".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "arg1": {"type": "string"},
            },
            "required": ["arg1"],
        }),
    };

    assert_eq!(func_desc.name, "test_function");
    assert_eq!(func_desc.description, "A test function.");
    assert_eq!(
        func_desc.parameters.get("type").unwrap(),
        &Value::String("object".to_string())
    );
}

#[test]
fn test_tool_description_structure() {
    // Verify the ToolDescription structure matches Python's TypedDict
    let tool_desc = ToolDescription {
        r#type: "function".to_string(),
        function: FunctionDescription {
            name: "test_function".to_string(),
            description: "A test function.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
            }),
        },
    };

    assert_eq!(tool_desc.r#type, "function");
    assert_eq!(tool_desc.function.name, "test_function");
}

// =============================================================================
// COMPREHENSIVE TESTS: test_convert_to_openai_function (matches Python parametrized test)
// =============================================================================

/// Test convert_to_openai_function with all fixture types.
/// This matches the Python test that iterates over multiple input formats.
#[test]
fn test_convert_to_openai_function_comprehensive() {
    let expected = expected_openai_function();

    // List of all input formats that should produce the same output
    let test_inputs = vec![
        // JSON schema format (like pydantic model output)
        ("json_schema", expected_json_schema()),
        // Anthropic tool format
        ("anthropic_tool", anthropic_tool()),
        // Bedrock Converse tool format
        ("bedrock_converse_tool", bedrock_converse_tool()),
        // Already OpenAI function format
        ("openai_function", expected_openai_function()),
    ];

    for (name, input) in test_inputs {
        let actual = convert_to_openai_function(&input, None);
        assert_eq!(actual, expected, "Failed for input type: {}", name);
    }
}

/// Test for TypedDict with annotations - matches Python's dummy_typing_typed_dict
#[test]
fn test_convert_to_openai_function_from_typing_typed_dict() {
    // Represents Python's TypedDict with Annotated fields
    let typed_dict_schema = json!({
        "title": "dummy_function",
        "description": "Dummy function.",
        "type": "object",
        "properties": {
            "arg1": {"description": "foo", "type": "integer"},
            "arg2": {
                "description": "one of 'bar', 'baz'",
                "enum": ["bar", "baz"],
                "type": "string",
            },
        },
        "required": ["arg1", "arg2"],
    });

    let expected = expected_openai_function();
    let actual = convert_to_openai_function(&typed_dict_schema, None);
    assert_eq!(actual, expected);
}

/// Test for TypedDict with docstring annotations - matches Python's dummy_typing_typed_dict_docstring
#[test]
fn test_convert_to_openai_function_from_typing_typed_dict_docstring() {
    // Represents Python's TypedDict with docstring-based arg descriptions
    let typed_dict_schema = json!({
        "title": "dummy_function",
        "description": "Dummy function.",
        "type": "object",
        "properties": {
            "arg1": {"description": "foo", "type": "integer"},
            "arg2": {
                "description": "one of 'bar', 'baz'",
                "enum": ["bar", "baz"],
                "type": "string",
            },
        },
        "required": ["arg1", "arg2"],
    });

    let expected = expected_openai_function();
    let actual = convert_to_openai_function(&typed_dict_schema, None);
    assert_eq!(actual, expected);
}

/// Test for extensions TypedDict - matches Python's dummy_extensions_typed_dict
#[test]
fn test_convert_to_openai_function_from_extensions_typed_dict() {
    // Represents Python's typing_extensions.TypedDict with Annotated fields
    let typed_dict_schema = json!({
        "title": "dummy_function",
        "description": "Dummy function.",
        "type": "object",
        "properties": {
            "arg1": {"description": "foo", "type": "integer"},
            "arg2": {
                "description": "one of 'bar', 'baz'",
                "enum": ["bar", "baz"],
                "type": "string",
            },
        },
        "required": ["arg1", "arg2"],
    });

    let expected = expected_openai_function();
    let actual = convert_to_openai_function(&typed_dict_schema, None);
    assert_eq!(actual, expected);
}

/// Test for extensions TypedDict with docstring - matches Python's dummy_extensions_typed_dict_docstring
#[test]
fn test_convert_to_openai_function_from_extensions_typed_dict_docstring() {
    // Represents Python's typing_extensions.TypedDict with docstring-based arg descriptions
    let typed_dict_schema = json!({
        "title": "dummy_function",
        "description": "Dummy function.",
        "type": "object",
        "properties": {
            "arg1": {"description": "foo", "type": "integer"},
            "arg2": {
                "description": "one of 'bar', 'baz'",
                "enum": ["bar", "baz"],
                "type": "string",
            },
        },
        "required": ["arg1", "arg2"],
    });

    let expected = expected_openai_function();
    let actual = convert_to_openai_function(&typed_dict_schema, None);
    assert_eq!(actual, expected);
}

// =============================================================================
// COMPREHENSIVE TESTS: test__convert_typed_dict_to_openai_function
// Full test with all argument types from Python
// =============================================================================

/// Full test for convert_typed_dict_to_openai_function with all arg types.
/// Matches Python's test__convert_typed_dict_to_openai_function with:
/// - arg1: str (required)
/// - arg2: int | str | bool union
/// - arg3: list[SubTool] | None
/// - arg4: Literal["bar", "baz"] with description
/// - arg5: float | None
/// - arg6: Sequence[Mapping[str, tuple[Iterable[Any], SubTool]]] | None (complex nested)
/// - arg7: list[SubTool]
/// - arg8: tuple[SubTool]
/// - arg9: Sequence[SubTool]
/// - arg10: Iterable[SubTool]
/// - arg11: set[SubTool]
/// - arg12: dict[str, SubTool]
/// - arg13: Mapping[str, SubTool]
/// - arg14: MutableMapping[str, SubTool]
/// - arg15: bool with default False
#[test]
fn test_convert_typed_dict_to_openai_function_full() {
    // SubTool schema used in multiple args
    let subtool_schema = json!({
        "description": "Subtool docstring.",
        "type": "object",
        "properties": {
            "args": {
                "description": "this does bar",
                "default": {},
                "type": "object",
            }
        },
    });

    let subtool_schema_with_title = json!({
        "title": "SubTool",
        "description": "Subtool docstring.",
        "type": "object",
        "properties": {
            "args": {
                "title": "Args",
                "description": "this does bar",
                "default": {},
                "type": "object",
            }
        },
    });

    // Full TypedDict schema matching Python test
    let typed_dict_schema = json!({
        "title": "Tool",
        "description": "Docstring.",
        "type": "object",
        "properties": {
            "arg1": {"description": "foo", "type": "string"},
            "arg2": {
                "anyOf": [
                    {"type": "integer"},
                    {"type": "string"},
                    {"type": "boolean"},
                ]
            },
            "arg3": {
                "type": "array",
                "items": subtool_schema.clone(),
            },
            "arg4": {
                "description": "this does foo",
                "enum": ["bar", "baz"],
                "type": "string",
            },
            "arg5": {"type": "number"},
            "arg6": {
                "default": [],
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": {
                        "type": "array",
                        "minItems": 2,
                        "maxItems": 2,
                        "items": [
                            {"type": "array", "items": {}},
                            subtool_schema_with_title.clone(),
                        ],
                    },
                },
            },
            "arg7": {
                "type": "array",
                "items": subtool_schema.clone(),
            },
            "arg8": {
                "type": "array",
                "minItems": 1,
                "maxItems": 1,
                "items": [subtool_schema_with_title.clone()],
            },
            "arg9": {
                "type": "array",
                "items": subtool_schema.clone(),
            },
            "arg10": {
                "type": "array",
                "items": subtool_schema.clone(),
            },
            "arg11": {
                "type": "array",
                "items": subtool_schema.clone(),
                "uniqueItems": true,
            },
            "arg12": {
                "type": "object",
                "additionalProperties": subtool_schema.clone(),
            },
            "arg13": {
                "type": "object",
                "additionalProperties": subtool_schema.clone(),
            },
            "arg14": {
                "type": "object",
                "additionalProperties": subtool_schema.clone(),
            },
            "arg15": {"description": "flag", "default": false, "type": "boolean"},
        },
        "required": [
            "arg1",
            "arg2",
            "arg3",
            "arg4",
            "arg7",
            "arg8",
            "arg9",
            "arg10",
            "arg11",
            "arg12",
            "arg13",
            "arg14",
        ],
    });

    let expected = json!({
        "name": "Tool",
        "description": "Docstring.",
        "parameters": {
            "type": "object",
            "properties": {
                "arg1": {"description": "foo", "type": "string"},
                "arg2": {
                    "anyOf": [
                        {"type": "integer"},
                        {"type": "string"},
                        {"type": "boolean"},
                    ]
                },
                "arg3": {
                    "type": "array",
                    "items": subtool_schema.clone(),
                },
                "arg4": {
                    "description": "this does foo",
                    "enum": ["bar", "baz"],
                    "type": "string",
                },
                "arg5": {"type": "number"},
                "arg6": {
                    "default": [],
                    "type": "array",
                    "items": {
                        "type": "object",
                        "additionalProperties": {
                            "type": "array",
                            "minItems": 2,
                            "maxItems": 2,
                            "items": [
                                {"type": "array", "items": {}},
                                subtool_schema_with_title.clone(),
                            ],
                        },
                    },
                },
                "arg7": {
                    "type": "array",
                    "items": subtool_schema.clone(),
                },
                "arg8": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": 1,
                    "items": [subtool_schema_with_title.clone()],
                },
                "arg9": {
                    "type": "array",
                    "items": subtool_schema.clone(),
                },
                "arg10": {
                    "type": "array",
                    "items": subtool_schema.clone(),
                },
                "arg11": {
                    "type": "array",
                    "items": subtool_schema.clone(),
                    "uniqueItems": true,
                },
                "arg12": {
                    "type": "object",
                    "additionalProperties": subtool_schema.clone(),
                },
                "arg13": {
                    "type": "object",
                    "additionalProperties": subtool_schema.clone(),
                },
                "arg14": {
                    "type": "object",
                    "additionalProperties": subtool_schema.clone(),
                },
                "arg15": {"description": "flag", "default": false, "type": "boolean"},
            },
            "required": [
                "arg1",
                "arg2",
                "arg3",
                "arg4",
                "arg7",
                "arg8",
                "arg9",
                "arg10",
                "arg11",
                "arg12",
                "arg13",
                "arg14",
            ],
        },
    });

    let actual = convert_typed_dict_to_openai_function(&typed_dict_schema);
    assert_eq!(actual, expected);
}

/// Test for convert_typed_dict_to_openai_function error case.
/// Matches Python's test__convert_typed_dict_to_openai_function_fail.
/// Note: In Python, this tests MutableSet which is not supported.
/// In Rust, we test that invalid schemas fail appropriately.
#[test]
fn test_convert_typed_dict_to_openai_function_fail() {
    // Schema with an unsupported/invalid type
    // Note: The Python test uses MutableSet which Pydantic v1 doesn't support.
    // In Rust, we test that invalid schemas are handled gracefully.
    let invalid_schema = json!({
        "title": "Tool",
        "type": "object",
        "properties": {
            "arg1": {
                // Invalid type that should cause issues
                "type": "unsupported_type",
            },
        },
    });

    // The function should still produce output (it may just pass through the invalid type)
    // since JSON schema allows custom types
    let result = convert_typed_dict_to_openai_function(&invalid_schema);

    // Verify it produces some output rather than panicking
    assert!(result.get("name").is_some());
    assert_eq!(result.get("name").unwrap(), "Tool");
}

// =============================================================================
// COMPREHENSIVE TESTS: test_convert_to_json_schema (matches Python parametrized test)
// =============================================================================

/// Test convert_to_json_schema with all fixture types.
/// This matches the Python test that iterates over multiple input formats.
#[test]
fn test_convert_to_json_schema_comprehensive() {
    let expected = expected_json_schema();

    // List of all input formats that should produce the same output
    let test_inputs = vec![
        // JSON schema format (like pydantic model output)
        ("json_schema", expected_json_schema()),
        // Anthropic tool format
        ("anthropic_tool", anthropic_tool()),
        // Bedrock Converse tool format
        ("bedrock_converse_tool", bedrock_converse_tool()),
        // OpenAI function format
        ("openai_function", expected_openai_function()),
    ];

    for (name, input) in test_inputs {
        let actual = convert_to_json_schema(&input, None).unwrap();
        assert_eq!(actual, expected, "Failed for input type: {}", name);
    }
}

// =============================================================================
// XFAIL TESTS: Tests that are expected to fail (marked as xfail in Python)
// =============================================================================

/// Test for nested pydantic v2 models.
/// Marked as xfail in Python: "Direct pydantic v2 models not yet supported"
/// In Rust, we note this limitation.
#[test]
#[ignore = "Direct pydantic v2 models not yet supported - matches Python xfail"]
fn test_convert_to_openai_function_nested_v2() {
    // This test is ignored to match Python's @pytest.mark.xfail behavior
    // The functionality would involve direct conversion of pydantic v2 models
    // which is not yet supported.
}

/// Test for optional param handling.
/// Marked as xfail in Python: "Pydantic converts str | None to str in .model_json_schema()"
/// In Rust, we note this limitation.
#[test]
#[ignore = "Pydantic converts str | None to str in .model_json_schema() - matches Python xfail"]
fn test_function_optional_param() {
    // This test is ignored to match Python's @pytest.mark.xfail behavior
    // The functionality involves proper handling of Optional types
    // which Pydantic doesn't preserve in the JSON schema.

    // The schema would look like:
    let _schema = json!({
        "title": "func5",
        "description": "A test function.",
        "type": "object",
        "properties": {
            "a": {"type": "string"},  // Should be string | null
            "b": {"type": "string"},
            "c": {
                "type": "array",
                "items": {"type": "string"},  // Should be string | null
            },  // Should be array | null
        },
        "required": ["b"],  // Only b should be required
    });

    // The test would verify that only "b" is in required
    // but this doesn't work due to Pydantic's handling of Optional
}

// =============================================================================
// ADDITIONAL TESTS: Runnable conversion (matches Python test)
// =============================================================================

/// Test convert_to_openai_function with a runnable-like tool.
/// Matches Python test for runnable.as_tool() conversion.
#[test]
fn test_convert_to_openai_function_from_runnable_tool() {
    // When converting a runnable to a tool, descriptions may not be preserved
    // in the parameters if they come from TypedDict annotations rather than
    // Pydantic Field descriptions.
    let runnable_tool_schema = json!({
        "title": "dummy_function",
        "description": "Dummy function.",
        "type": "object",
        "properties": {
            "arg1": {"type": "integer"},
            "arg2": {
                "enum": ["bar", "baz"],
                "type": "string",
            },
        },
        "required": ["arg1", "arg2"],
    });

    let expected = json!({
        "name": "dummy_function",
        "description": "Dummy function.",
        "parameters": {
            "type": "object",
            "properties": {
                "arg1": {"type": "integer"},
                "arg2": {
                    "enum": ["bar", "baz"],
                    "type": "string",
                },
            },
            "required": ["arg1", "arg2"],
        },
    });

    let actual = convert_to_openai_function(&runnable_tool_schema, None);
    assert_eq!(actual, expected);
}
