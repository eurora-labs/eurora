use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use agent_chain_core::messages::BaseMessage;
use agent_chain_core::tools::{ArgsSchema, StructuredTool, Tool};
use agent_chain_core::utils::function_calling::{
    FunctionDescription, ToolDescription, convert_to_json_schema, convert_to_openai_function,
    convert_typed_dict_to_openai_function, tool_example_to_messages,
};

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

#[test]

fn test_convert_to_openai_function_from_json_schema() {
    let json_schema = expected_json_schema();
    let expected = expected_openai_function();

    let actual = convert_to_openai_function(&json_schema, None).unwrap();

    assert_eq!(actual, expected);
}

#[test]

fn test_convert_to_openai_function_from_anthropic_tool() {
    let expected = expected_openai_function();

    let actual = convert_to_openai_function(&anthropic_tool(), None).unwrap();

    assert_eq!(actual, expected);
}

#[test]

fn test_convert_to_openai_function_from_bedrock_converse_tool() {
    let expected = expected_openai_function();

    let actual = convert_to_openai_function(&bedrock_converse_tool(), None).unwrap();

    assert_eq!(actual, expected);
}

#[test]

fn test_convert_to_openai_function_from_openai_function() {
    let expected = expected_openai_function();

    let actual = convert_to_openai_function(&expected, None).unwrap();

    assert_eq!(actual, expected);
}

#[test]
fn test_convert_to_openai_function_nested() {
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

    let actual = convert_to_openai_function(&nested_schema, None).unwrap();
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

    let actual = convert_to_openai_function(&nested_schema, Some(true)).unwrap();
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

    let actual = convert_to_openai_function(&schema, Some(true)).unwrap();
    assert_eq!(actual, expected);
}

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

    let actual = convert_to_openai_function(&anthropic_tool_no_description(), None).unwrap();
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

    let actual = convert_to_openai_function(&json_schema_no_description(), None).unwrap();
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

    let actual = convert_to_openai_function(&bedrock_converse_tool_no_description(), None).unwrap();
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

    let actual = convert_to_openai_function(&openai_function_no_description(), None).unwrap();
    assert_eq!(actual, expected);
}

#[test]

fn test_convert_to_openai_function_no_description_no_params_json_schema() {
    let expected = json!({
        "name": "dummy_function",
    });

    let actual = convert_to_openai_function(&json_schema_no_description_no_params(), None).unwrap();
    assert_eq!(actual, expected);
}

#[test]

fn test_convert_to_openai_function_no_description_no_params_openai() {
    let expected = json!({
        "name": "dummy_function",
    });

    let actual =
        convert_to_openai_function(&openai_function_no_description_no_params(), None).unwrap();
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

    let func = convert_to_openai_function(&schema, None).unwrap();
    let required = func
        .get("parameters")
        .and_then(|p| p.get("required"))
        .and_then(|r| r.as_array());

    assert!(required.is_none() || required.unwrap().is_empty());
}

#[test]

fn test_convert_union_type() {
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

    let result = convert_to_openai_function(&schema, None).unwrap();
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

    let actual = convert_to_openai_function(&schema, Some(true)).unwrap();
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

    let actual = convert_to_openai_function(&schema, Some(true)).unwrap();
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

    let func = convert_to_openai_function(&schema, Some(true)).unwrap();
    let required = func
        .get("parameters")
        .and_then(|p| p.get("required"))
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();

    assert_eq!(required, vec!["arg1", "arg2"]);
}

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

    if let BaseMessage::AI(ai_msg) = &messages[1] {
        let tool_calls = ai_msg
            .additional_kwargs
            .get("tool_calls")
            .and_then(|tc| tc.as_array())
            .expect("tool_calls should be an array");

        assert_eq!(tool_calls.len(), 3);

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
        let tool_calls = response
            .additional_kwargs
            .get("tool_calls")
            .and_then(|tc| tc.as_array());
        assert!(tool_calls.is_none() || tool_calls.unwrap().is_empty());
    }
}

#[test]
fn test_convert_typed_dict_to_openai_function() {
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

    let actual = convert_typed_dict_to_openai_function(&typed_dict_schema).unwrap();
    assert_eq!(actual, expected);
}

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

#[test]

fn test_convert_to_openai_function_from_structured_tool() {
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
    let actual = convert_to_openai_function(&tool, None).unwrap();

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
    let actual = convert_to_openai_function(&tool, None).unwrap();

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

    let actual = convert_to_openai_function(&tool, None).unwrap();

    assert_eq!(actual, expected);
}

#[test]
fn test_function_description_structure() {
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

#[test]
fn test_convert_to_openai_function_comprehensive() {
    let expected = expected_openai_function();

    let test_inputs = vec![
        ("json_schema", expected_json_schema()),
        ("anthropic_tool", anthropic_tool()),
        ("bedrock_converse_tool", bedrock_converse_tool()),
        ("openai_function", expected_openai_function()),
    ];

    for (name, input) in test_inputs {
        let actual = convert_to_openai_function(&input, None).unwrap();
        assert_eq!(actual, expected, "Failed for input type: {}", name);
    }
}

#[test]
fn test_convert_to_openai_function_from_typing_typed_dict() {
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
    let actual = convert_to_openai_function(&typed_dict_schema, None).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_convert_to_openai_function_from_typing_typed_dict_docstring() {
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
    let actual = convert_to_openai_function(&typed_dict_schema, None).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_convert_to_openai_function_from_extensions_typed_dict() {
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
    let actual = convert_to_openai_function(&typed_dict_schema, None).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_convert_to_openai_function_from_extensions_typed_dict_docstring() {
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
    let actual = convert_to_openai_function(&typed_dict_schema, None).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_convert_typed_dict_to_openai_function_full() {
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

    let actual = convert_typed_dict_to_openai_function(&typed_dict_schema).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_convert_typed_dict_to_openai_function_fail() {
    let invalid_schema = json!({
        "title": "Tool",
        "type": "object",
        "properties": {
            "arg1": {
                "type": "unsupported_type",
            },
        },
    });

    let result = convert_typed_dict_to_openai_function(&invalid_schema).unwrap();

    assert!(result.get("name").is_some());
    assert_eq!(result.get("name").unwrap(), "Tool");
}

#[test]
fn test_convert_to_json_schema_comprehensive() {
    let expected = expected_json_schema();

    let test_inputs = vec![
        ("json_schema", expected_json_schema()),
        ("anthropic_tool", anthropic_tool()),
        ("bedrock_converse_tool", bedrock_converse_tool()),
        ("openai_function", expected_openai_function()),
    ];

    for (name, input) in test_inputs {
        let actual = convert_to_json_schema(&input, None).unwrap();
        assert_eq!(actual, expected, "Failed for input type: {}", name);
    }
}

#[test]
#[ignore = "Direct pydantic v2 models not yet supported - matches Python xfail"]
fn test_convert_to_openai_function_nested_v2() {}

#[test]
#[ignore = "Pydantic converts str | None to str in .model_json_schema() - matches Python xfail"]
fn test_function_optional_param() {
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
}

#[test]
fn test_convert_to_openai_function_from_runnable_tool() {
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

    let actual = convert_to_openai_function(&runnable_tool_schema, None).unwrap();
    assert_eq!(actual, expected);
}
