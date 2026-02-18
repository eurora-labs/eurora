//! Snapshot tests for OpenAI functions output parsers.
//!
//! Ported from langchain/libs/core/tests/unit_tests/output_parsers/test_openai_functions_parser.py

use std::collections::HashMap;

use agent_chain_core::messages::AIMessage;
use agent_chain_core::output_parsers::{
    JsonKeyOutputFunctionsParser, JsonOutputFunctionsParser, OutputFunctionsParser,
    PydanticAttrOutputFunctionsParser, PydanticOutputFunctionsParser,
};
use agent_chain_core::outputs::ChatGeneration;
use serde::{Deserialize, Serialize};
use serde_json::json;


fn make_fn_message(name: &str, arguments: &str, content: &str) -> AIMessage {
    let mut additional_kwargs = HashMap::new();
    additional_kwargs.insert(
        "function_call".to_string(),
        json!({
            "name": name,
            "arguments": arguments,
        }),
    );

    AIMessage::builder()
        .content(content)
        .additional_kwargs(additional_kwargs)
        .build()
}

fn make_fn_message_default(name: &str, arguments: &str) -> AIMessage {
    make_fn_message(name, arguments, "test")
}

fn make_chat_gen(message: AIMessage) -> ChatGeneration {
    ChatGeneration::new(message.into())
}


#[test]
fn test_output_functions_parser_args_only_returns_arguments_string() {
    let msg = make_fn_message_default("fn", r#"{"a": 1}"#);
    let parser = OutputFunctionsParser::new(true);
    let result = parser.parse_result(&[make_chat_gen(msg)]).unwrap();
    assert_eq!(result, json!(r#"{"a": 1}"#));
}

#[test]
fn test_output_functions_parser_full_output_returns_function_call() {
    let msg = make_fn_message_default("fn", r#"{"a": 1}"#);
    let parser = OutputFunctionsParser::new(false);
    let result = parser.parse_result(&[make_chat_gen(msg)]).unwrap();
    assert_eq!(result["name"], "fn");
    assert_eq!(result["arguments"], r#"{"a": 1}"#);
}

#[test]
fn test_output_functions_parser_missing_function_call_raises() {
    let msg = AIMessage::builder()
        .content("no function call")
        .additional_kwargs(HashMap::new())
        .build();
    let parser = OutputFunctionsParser::new(true);
    let result = parser.parse_result(&[make_chat_gen(msg)]);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("function_call"),
        "Error should mention 'function_call': {}",
        err_msg
    );
}

#[test]
fn test_output_functions_parser_does_not_modify_original_message() {
    let msg = make_fn_message_default("fn", r#"{"a": 1}"#);
    let original_kwargs = msg.additional_kwargs.clone();
    let parser = OutputFunctionsParser::new(false);
    let generation = make_chat_gen(msg.clone());
    parser.parse_result(&[generation]).unwrap();
    assert_eq!(msg.additional_kwargs, original_kwargs);
}


#[test]
fn test_json_output_functions_parser_args_only_parses_json() {
    let msg = make_fn_message_default("fn", r#"{"key": "value"}"#);
    let parser = JsonOutputFunctionsParser::new(true);
    let result = parser.parse_result(&[make_chat_gen(msg)]).unwrap();
    assert_eq!(result, Some(json!({"key": "value"})));
}

#[test]
fn test_json_output_functions_parser_full_output_parses_arguments() {
    let msg = make_fn_message_default("fn", r#"{"key": "value"}"#);
    let parser = JsonOutputFunctionsParser::new(false);
    let result = parser.parse_result(&[make_chat_gen(msg)]).unwrap();
    assert_eq!(
        result,
        Some(json!({"name": "fn", "arguments": {"key": "value"}}))
    );
}

#[test]
fn test_json_output_functions_parser_non_strict_allows_newlines() {
    let msg = make_fn_message_default("fn", "{\"code\": \"line1\nline2\"}");
    let parser = JsonOutputFunctionsParser::new(true).with_strict(false);
    let result = parser.parse_result(&[make_chat_gen(msg)]).unwrap();
    assert_eq!(result, Some(json!({"code": "line1\nline2"})));
}

#[test]
fn test_json_output_functions_parser_strict_rejects_newlines() {
    let msg = make_fn_message_default("fn", "{\"code\": \"line1\nline2\"}");
    let parser = JsonOutputFunctionsParser::new(true).with_strict(true);
    let result = parser.parse_result(&[make_chat_gen(msg)]);
    assert!(result.is_err());
}

#[test]
fn test_json_output_functions_parser_non_strict_allows_unicode() {
    let msg = make_fn_message_default("fn", r#"{"text": "你好"}"#);
    let parser = JsonOutputFunctionsParser::new(true).with_strict(false);
    let result = parser.parse_result(&[make_chat_gen(msg)]).unwrap();
    assert_eq!(result, Some(json!({"text": "你好"})));
}

#[test]
fn test_json_output_functions_parser_missing_function_call_raises() {
    let msg = AIMessage::builder()
        .content("no fn")
        .additional_kwargs(HashMap::new())
        .build();
    let parser = JsonOutputFunctionsParser::default();
    let result = parser.parse_result(&[make_chat_gen(msg)]);
    assert!(result.is_err());
}

#[test]
fn test_json_output_functions_parser_missing_function_call_partial_returns_none() {
    let msg = AIMessage::builder()
        .content("no fn")
        .additional_kwargs(HashMap::new())
        .build();
    let parser = JsonOutputFunctionsParser::default();
    let result = parser
        .parse_result_with_partial(&[make_chat_gen(msg)], true)
        .unwrap();
    assert!(result.is_none());
}

#[test]
fn test_json_output_functions_parser_invalid_json_raises() {
    let msg = make_fn_message_default("fn", "not_json");
    let parser = JsonOutputFunctionsParser::default();
    let result = parser.parse_result(&[make_chat_gen(msg)]);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Could not parse function call data"),
        "Error should mention parsing failure: {}",
        err_msg
    );
}

#[test]
fn test_json_output_functions_parser_invalid_json_full_output_raises() {
    let msg = make_fn_message_default("fn", "bad_json");
    let parser = JsonOutputFunctionsParser::new(false);
    let result = parser.parse_result(&[make_chat_gen(msg)]);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Could not parse function call data"),
        "Error should mention parsing failure: {}",
        err_msg
    );
}

#[test]
fn test_json_output_functions_parser_partial_valid_json() {
    let msg = make_fn_message_default("fn", r#"{"key": "val"#);
    let parser = JsonOutputFunctionsParser::default();
    let result = parser
        .parse_result_with_partial(&[make_chat_gen(msg)], true)
        .unwrap();
    assert_eq!(result, Some(json!({"key": "val"})));
}

#[test]
fn test_json_output_functions_parser_partial_invalid_json_returns_none() {
    let msg = make_fn_message_default("fn", "{{bad");
    let parser = JsonOutputFunctionsParser::default();
    let result = parser
        .parse_result_with_partial(&[make_chat_gen(msg)], true)
        .unwrap();
    assert!(result.is_none());
}

#[test]
fn test_json_output_functions_parser_partial_full_output() {
    let msg = make_fn_message_default("fn", r#"{"key": "val"#);
    let parser = JsonOutputFunctionsParser::new(false);
    let result = parser
        .parse_result_with_partial(&[make_chat_gen(msg)], true)
        .unwrap();
    assert_eq!(
        result,
        Some(json!({"name": "fn", "arguments": {"key": "val"}}))
    );
}

#[test]
fn test_json_output_functions_parser_missing_arguments_key_returns_none() {
    let mut additional_kwargs = HashMap::new();
    additional_kwargs.insert("function_call".to_string(), json!({"name": "fn"}));
    let msg = AIMessage::builder()
        .content("test")
        .additional_kwargs(additional_kwargs)
        .build();
    let parser = JsonOutputFunctionsParser::default();
    let result = parser.parse_result(&[make_chat_gen(msg)]).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_json_output_functions_parser_multiple_results_raises() {
    let msg1 = make_fn_message_default("fn", r#"{"a": 1}"#);
    let msg2 = make_fn_message_default("fn", r#"{"b": 2}"#);
    let parser = JsonOutputFunctionsParser::default();
    let result = parser.parse_result(&[make_chat_gen(msg1), make_chat_gen(msg2)]);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Expected exactly one result"),
        "Error should mention expected count: {}",
        err_msg
    );
}

#[test]
fn test_json_output_functions_parser_type_property() {
    let parser = JsonOutputFunctionsParser::default();
    assert_eq!(parser.parser_type(), "json_functions");
}

#[test]
fn test_json_output_functions_parser_parse_raises_not_implemented() {
    let parser = JsonOutputFunctionsParser::default();
    let result = parser.parse("text");
    assert!(result.is_err());
}

#[test]
fn test_json_output_functions_parser_diff_method() {
    let parser = JsonOutputFunctionsParser::default();
    let diff = parser.diff(&json!({"a": 1}), &json!({"a": 1, "b": 2}));
    assert!(!diff.is_empty());
    assert!(diff.iter().any(|op| op["op"] == "add"));
}


#[test]
fn test_json_key_output_functions_parser_extracts_key() {
    let msg = make_fn_message_default("fn", r#"{"key1": "val1", "key2": "val2"}"#);
    let parser = JsonKeyOutputFunctionsParser::new("key1");
    let result = parser.parse_result(&[make_chat_gen(msg)]).unwrap();
    assert_eq!(result, Some(json!("val1")));
}

#[test]
fn test_json_key_output_functions_parser_extracts_nested_key() {
    let msg = make_fn_message_default("fn", r#"{"data": {"nested": true}, "other": 1}"#);
    let parser = JsonKeyOutputFunctionsParser::new("data");
    let result = parser.parse_result(&[make_chat_gen(msg)]).unwrap();
    assert_eq!(result, Some(json!({"nested": true})));
}

#[test]
fn test_json_key_output_functions_parser_missing_key_raises() {
    let msg = make_fn_message_default("fn", r#"{"a": 1}"#);
    let parser = JsonKeyOutputFunctionsParser::new("missing");
    let result = parser.parse_result(&[make_chat_gen(msg)]);
    assert!(result.is_err());
}

#[test]
fn test_json_key_output_functions_parser_partial_returns_none_when_key_missing() {
    let msg = make_fn_message_default("fn", r#"{"a": 1}"#);
    let parser = JsonKeyOutputFunctionsParser::new("missing");
    let result = parser
        .parse_result_with_partial(&[make_chat_gen(msg)], true)
        .unwrap();
    assert!(result.is_none());
}

#[test]
fn test_json_key_output_functions_parser_partial_returns_value_when_key_present() {
    let msg = make_fn_message_default("fn", r#"{"target": "val"#);
    let parser = JsonKeyOutputFunctionsParser::new("target");
    let result = parser
        .parse_result_with_partial(&[make_chat_gen(msg)], true)
        .unwrap();
    assert_eq!(result, Some(json!("val")));
}

#[test]
fn test_json_key_output_functions_parser_partial_with_no_function_call_returns_none() {
    let msg = AIMessage::builder()
        .content("no fn")
        .additional_kwargs(HashMap::new())
        .build();
    let parser = JsonKeyOutputFunctionsParser::new("key");
    let result = parser
        .parse_result_with_partial(&[make_chat_gen(msg)], true)
        .unwrap();
    assert!(result.is_none());
}


#[test]
fn test_pydantic_output_functions_parser_single_schema_parses() {
    #[derive(Debug, Clone, Deserialize, PartialEq)]
    struct Model {
        name: String,
        age: i64,
    }

    let msg = make_fn_message_default(
        "Model",
        &serde_json::to_string(&json!({"name": "Alice", "age": 30})).unwrap(),
    );
    let parser = PydanticOutputFunctionsParser::<Model>::new();
    let result = parser.parse_result(&[make_chat_gen(msg)]).unwrap();
    assert_eq!(
        result,
        Model {
            name: "Alice".to_string(),
            age: 30,
        }
    );
}

#[test]
fn test_pydantic_output_functions_parser_multiple_schemas_selects_by_name() {
    #[derive(Debug, Clone, Deserialize, PartialEq)]
    struct Cat {
        breed: String,
    }

    #[derive(Debug, Clone, Deserialize, PartialEq)]
    struct Dog {
        species: String,
    }

    #[derive(Debug, Clone, PartialEq)]
    enum SchemaResult {
        Cat(Cat),
        Dog(Dog),
    }

    let msg = make_fn_message_default(
        "cat",
        &serde_json::to_string(&json!({"breed": "Siamese"})).unwrap(),
    );
    let parser = PydanticOutputFunctionsParser::<SchemaResult>::with_multiple_schemas(
        |function_name, json_args| match function_name {
            "cat" => {
                let cat: Cat = serde_json::from_str(json_args)
                    .map_err(|e| agent_chain_core::error::Error::Other(e.to_string()))?;
                Ok(SchemaResult::Cat(cat))
            }
            "dog" => {
                let dog: Dog = serde_json::from_str(json_args)
                    .map_err(|e| agent_chain_core::error::Error::Other(e.to_string()))?;
                Ok(SchemaResult::Dog(dog))
            }
            _ => Err(agent_chain_core::error::Error::Other(format!(
                "Unknown function: {}",
                function_name
            ))),
        },
    );

    let result = parser.parse_result(&[make_chat_gen(msg)]).unwrap();
    assert_eq!(
        result,
        SchemaResult::Cat(Cat {
            breed: "Siamese".to_string(),
        })
    );
}

#[test]
fn test_pydantic_output_functions_parser_validation_error_raises() {
    #[derive(Debug, Clone, Deserialize, PartialEq)]
    struct Model {
        x: i64,
    }

    let msg = make_fn_message_default("Model", r#"{"x": "not_int"}"#);
    let parser = PydanticOutputFunctionsParser::<Model>::new();
    let result = parser.parse_result(&[make_chat_gen(msg)]);
    assert!(result.is_err());
}


#[test]
fn test_pydantic_attr_output_functions_parser_extracts_attribute() {
    #[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
    struct Model {
        name: String,
        value: i64,
    }

    let msg = make_fn_message_default(
        "Model",
        &serde_json::to_string(&json!({"name": "test", "value": 42})).unwrap(),
    );
    let parser = PydanticAttrOutputFunctionsParser::<Model>::new("value");
    let result = parser.parse_result(&[make_chat_gen(msg)]).unwrap();
    assert_eq!(result, json!(42));
}

#[test]
fn test_pydantic_attr_output_functions_parser_extracts_string_attribute() {
    #[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
    struct Model {
        name: String,
        value: i64,
    }

    let msg = make_fn_message_default(
        "Model",
        &serde_json::to_string(&json!({"name": "hello", "value": 1})).unwrap(),
    );
    let parser = PydanticAttrOutputFunctionsParser::<Model>::new("name");
    let result = parser.parse_result(&[make_chat_gen(msg)]).unwrap();
    assert_eq!(result, json!("hello"));
}

#[test]
fn test_pydantic_attr_output_functions_parser_extracts_list_attribute() {
    #[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
    struct Model {
        items: Vec<String>,
        count: i64,
    }

    let msg = make_fn_message_default(
        "Model",
        &serde_json::to_string(&json!({"items": ["a", "b"], "count": 2})).unwrap(),
    );
    let parser = PydanticAttrOutputFunctionsParser::<Model>::new("items");
    let result = parser.parse_result(&[make_chat_gen(msg)]).unwrap();
    assert_eq!(result, json!(["a", "b"]));
}

#[test]
fn test_pydantic_attr_output_functions_parser_invalid_attribute_raises() {
    #[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
    struct Model {
        x: i64,
    }

    let msg = make_fn_message_default("Model", &serde_json::to_string(&json!({"x": 1})).unwrap());
    let parser = PydanticAttrOutputFunctionsParser::<Model>::new("nonexistent");
    let result = parser.parse_result(&[make_chat_gen(msg)]);
    assert!(result.is_err());
}
