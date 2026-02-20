use std::collections::HashMap;

use agent_chain_core::messages::{AIMessage, HumanMessage};
use agent_chain_core::output_parsers::{JsonOutputFunctionsParser, PydanticOutputFunctionsParser};
use agent_chain_core::outputs::ChatGeneration;
use serde::Deserialize;
use serde_json::json;

fn make_function_call_generation(function_name: &str, arguments: &str) -> ChatGeneration {
    let mut additional_kwargs = HashMap::new();
    additional_kwargs.insert(
        "function_call".to_string(),
        json!({
            "name": function_name,
            "arguments": arguments,
        }),
    );

    let message = AIMessage::builder()
        .content("This is a test message")
        .additional_kwargs(additional_kwargs)
        .build();

    ChatGeneration::new(message.into())
}

#[test]
fn test_json_output_function_parser() {
    let chat_generation =
        make_function_call_generation("function_name", "{\"arg1\": \"code\\ncode\"}");

    let parser = JsonOutputFunctionsParser::new(false);
    let result = parser
        .parse_result(std::slice::from_ref(&chat_generation))
        .unwrap();
    assert_eq!(
        result,
        Some(json!({"arguments": {"arg1": "code\ncode"}, "name": "function_name"}))
    );

    let parser = JsonOutputFunctionsParser::new(true);
    let result = parser
        .parse_result(std::slice::from_ref(&chat_generation))
        .unwrap();
    assert_eq!(result, Some(json!({"arg1": "code\ncode"})));

    let additional_kwargs = chat_generation.message.additional_kwargs().unwrap();
    let function_call = additional_kwargs.get("function_call").unwrap();
    assert_eq!(
        function_call,
        &json!({
            "name": "function_name",
            "arguments": "{\"arg1\": \"code\\ncode\"}",
        })
    );
}

#[test]
fn test_json_output_function_parser_strictness_full_output() {
    let chat_generation = make_function_call_generation("function_name", "{\"arg1\": \"value1\"}");

    let parser = JsonOutputFunctionsParser::new(false).with_strict(false);
    let result = parser.parse_result(&[chat_generation]).unwrap();
    assert_eq!(
        result,
        Some(json!({"arguments": {"arg1": "value1"}, "name": "function_name"}))
    );
}

#[test]
fn test_json_output_function_parser_strictness_args_only() {
    let chat_generation = make_function_call_generation("function_name", "{\"arg1\": \"value1\"}");

    let parser = JsonOutputFunctionsParser::new(true).with_strict(false);
    let result = parser.parse_result(&[chat_generation]).unwrap();
    assert_eq!(result, Some(json!({"arg1": "value1"})));
}

#[test]
fn test_json_output_function_parser_strictness_newline_lenient() {
    let chat_generation =
        make_function_call_generation("function_name", "{\"code\": \"print(2+\n2)\"}");

    let parser = JsonOutputFunctionsParser::new(true).with_strict(false);
    let result = parser.parse_result(&[chat_generation]).unwrap();
    assert_eq!(result, Some(json!({"code": "print(2+\n2)"})));
}

#[test]
fn test_json_output_function_parser_strictness_unicode() {
    let chat_generation = make_function_call_generation("function_name", "{\"code\": \"你好)\"}");

    let parser = JsonOutputFunctionsParser::new(true).with_strict(false);
    let result = parser.parse_result(&[chat_generation]).unwrap();
    assert_eq!(result, Some(json!({"code": "你好)"})));
}

#[test]
fn test_json_output_function_parser_strictness_strict_rejects_newline() {
    let chat_generation =
        make_function_call_generation("function_name", "{\"code\": \"print(2+\n2)\"}");

    let parser = JsonOutputFunctionsParser::new(true).with_strict(true);
    let result = parser.parse_result(&[chat_generation]);
    assert!(result.is_err());
}

#[test]
fn test_exception_human_message() {
    let message = HumanMessage::builder()
        .content("This is a test message")
        .build();
    let chat_generation = ChatGeneration::new(message.into());

    let parser = JsonOutputFunctionsParser::default();
    let result = parser.parse_result(&[chat_generation]);
    assert!(result.is_err());
}

#[test]
fn test_exception_ai_message_no_function_call() {
    let message = AIMessage::builder()
        .content("This is a test message")
        .build();
    let chat_generation = ChatGeneration::new(message.into());

    let parser = JsonOutputFunctionsParser::default();
    let result = parser.parse_result(&[chat_generation]);
    assert!(result.is_err());
}

#[test]
fn test_exception_arguments_not_string() {
    let mut additional_kwargs = HashMap::new();
    additional_kwargs.insert(
        "function_call".to_string(),
        json!({
            "name": "function_name",
            "arguments": {},
        }),
    );

    let message = AIMessage::builder()
        .content("This is a test message")
        .additional_kwargs(additional_kwargs)
        .build();
    let chat_generation = ChatGeneration::new(message.into());

    let parser = JsonOutputFunctionsParser::default();
    let result = parser.parse_result(&[chat_generation]);
    assert!(result.is_err());
}

#[test]
fn test_exception_arguments_invalid_json() {
    let chat_generation = make_function_call_generation("function_name", "noqweqwe");

    let parser = JsonOutputFunctionsParser::default();
    let result = parser.parse_result(&[chat_generation]);
    assert!(result.is_err());
}

#[test]
fn test_pydantic_output_functions_parser() {
    #[derive(Debug, Clone, Deserialize, PartialEq)]
    struct Model {
        name: String,
        age: i64,
    }

    let chat_generation = make_function_call_generation(
        "function_name",
        &serde_json::to_string(&json!({"name": "value", "age": 10})).unwrap(),
    );

    let parser = PydanticOutputFunctionsParser::<Model>::new();
    let result = parser.parse_result(&[chat_generation]).unwrap();
    assert_eq!(
        result,
        Model {
            name: "value".to_string(),
            age: 10,
        }
    );
}

#[test]
fn test_pydantic_output_functions_parser_multiple_schemas() {
    #[derive(Debug, Clone, Deserialize, PartialEq)]
    struct Cookie {
        name: String,
        age: i64,
    }

    #[derive(Debug, Clone, Deserialize, PartialEq)]
    struct Dog {
        species: String,
    }

    #[derive(Debug, Clone, PartialEq)]
    enum SchemaResult {
        Cookie(Cookie),
        Dog(Dog),
    }

    let chat_generation = make_function_call_generation(
        "cookie",
        &serde_json::to_string(&json!({"name": "value", "age": 10})).unwrap(),
    );

    let parser = PydanticOutputFunctionsParser::<SchemaResult>::with_multiple_schemas(
        |function_name, json_args| match function_name {
            "cookie" => {
                let cookie: Cookie = serde_json::from_str(json_args)
                    .map_err(|e| agent_chain_core::error::Error::Other(e.to_string()))?;
                Ok(SchemaResult::Cookie(cookie))
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

    let result = parser.parse_result(&[chat_generation]).unwrap();
    assert_eq!(
        result,
        SchemaResult::Cookie(Cookie {
            name: "value".to_string(),
            age: 10,
        })
    );
}
