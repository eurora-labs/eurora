//! Tests for OpenAI tools output parsers.
//!
//! Ported from langchain/libs/core/tests/unit_tests/output_parsers/test_openai_tools.py

use std::collections::HashMap;

use agent_chain_core::messages::AIMessage;
use agent_chain_core::output_parsers::{
    JsonOutputKeyToolsParser, JsonOutputToolsParser, PydanticToolsParser, parse_tool_call,
};
use agent_chain_core::outputs::ChatGeneration;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a ChatGeneration with tool calls in additional_kwargs (legacy path).
fn make_additional_kwargs_generation(tool_calls: Value) -> ChatGeneration {
    let mut additional_kwargs = HashMap::new();
    additional_kwargs.insert("tool_calls".to_string(), tool_calls);

    let message = AIMessage::builder()
        .content("")
        .additional_kwargs(additional_kwargs)
        .build();

    ChatGeneration::new(message.into())
}

/// Create a ChatGeneration with tool_calls on the AIMessage itself (modern path).
fn make_tool_calls_generation(
    tool_calls: Vec<agent_chain_core::messages::ToolCall>,
) -> ChatGeneration {
    let message = AIMessage::builder()
        .content("")
        .tool_calls(tool_calls)
        .build();

    ChatGeneration::new(message.into())
}

fn make_tool_call(id: &str, name: &str, args: Value) -> agent_chain_core::messages::ToolCall {
    agent_chain_core::messages::ToolCall {
        id: Some(id.to_string()),
        name: name.to_string(),
        args,
        call_type: None,
    }
}

// ---------------------------------------------------------------------------
// Test: JsonOutputToolsParser parse_result with additional_kwargs
// ---------------------------------------------------------------------------

/// Ported from: test_json_output_key_tools_parser_multiple_tools_first_only
/// (additional_kwargs variant)
#[test]
fn test_json_output_tools_parser_additional_kwargs() {
    let raw_tool_calls = json!([
        {
            "id": "call_OwL7f5PEPJTYzw9sQlNJtCZl",
            "function": {
                "arguments": "{\"names\": [\"suzy\", \"jermaine\", \"alex\"], \"person\": {\"age\": 39, \"hair_color\": \"brown\", \"job\": \"concierge\"}}",
                "name": "NameCollector"
            },
            "type": "function"
        }
    ]);

    let generation = make_additional_kwargs_generation(raw_tool_calls);
    let parser = JsonOutputToolsParser::new();
    let result = parser.parse_result(&[generation], false).unwrap();

    let expected = json!([{
        "type": "NameCollector",
        "args": {
            "names": ["suzy", "jermaine", "alex"],
            "person": {"age": 39, "hair_color": "brown", "job": "concierge"}
        }
    }]);
    assert_eq!(result, expected);
}

/// Ported from: test_partial_json_output_parser_return_id (additional_kwargs variant)
#[test]
fn test_json_output_tools_parser_return_id_additional_kwargs() {
    let raw_tool_calls = json!([
        {
            "id": "call_OwL7f5PEPJTYzw9sQlNJtCZl",
            "function": {
                "arguments": "{\"names\": [\"suzy\"]}",
                "name": "NameCollector"
            },
            "type": "function"
        }
    ]);

    let generation = make_additional_kwargs_generation(raw_tool_calls);
    let parser = JsonOutputToolsParser::new().with_return_id(true);
    let result = parser.parse_result(&[generation], false).unwrap();

    let expected = json!([{
        "type": "NameCollector",
        "args": {"names": ["suzy"]},
        "id": "call_OwL7f5PEPJTYzw9sQlNJtCZl"
    }]);
    assert_eq!(result, expected);
}

// ---------------------------------------------------------------------------
// Test: JsonOutputToolsParser parse_result with tool_calls
// ---------------------------------------------------------------------------

#[test]
fn test_json_output_tools_parser_tool_calls() {
    let tool_calls = vec![make_tool_call(
        "call_OwL7f5PEPJTYzw9sQlNJtCZl",
        "NameCollector",
        json!({
            "names": ["suzy", "jermaine", "alex"],
            "person": {"age": 39, "hair_color": "brown", "job": "concierge"}
        }),
    )];

    let generation = make_tool_calls_generation(tool_calls);
    let parser = JsonOutputToolsParser::new();
    let result = parser.parse_result(&[generation], false).unwrap();

    let expected = json!([{
        "type": "NameCollector",
        "args": {
            "names": ["suzy", "jermaine", "alex"],
            "person": {"age": 39, "hair_color": "brown", "job": "concierge"}
        }
    }]);
    assert_eq!(result, expected);
}

#[test]
fn test_json_output_tools_parser_return_id_tool_calls() {
    let tool_calls = vec![make_tool_call(
        "call_OwL7f5PEPJTYzw9sQlNJtCZl",
        "NameCollector",
        json!({"names": ["suzy"]}),
    )];

    let generation = make_tool_calls_generation(tool_calls);
    let parser = JsonOutputToolsParser::new().with_return_id(true);
    let result = parser.parse_result(&[generation], false).unwrap();

    let expected = json!([{
        "type": "NameCollector",
        "args": {"names": ["suzy"]},
        "id": "call_OwL7f5PEPJTYzw9sQlNJtCZl"
    }]);
    assert_eq!(result, expected);
}

// ---------------------------------------------------------------------------
// Test: JsonOutputKeyToolsParser (additional_kwargs variant)
// ---------------------------------------------------------------------------

/// Ported from: test_partial_json_output_key_parser (additional_kwargs variant)
#[test]
fn test_json_output_key_tools_parser_additional_kwargs() {
    let raw_tool_calls = json!([
        {
            "id": "call_OwL7f5PEPJTYzw9sQlNJtCZl",
            "function": {
                "arguments": "{\"names\": [\"suzy\"]}",
                "name": "NameCollector"
            },
            "type": "function"
        }
    ]);

    let generation = make_additional_kwargs_generation(raw_tool_calls);
    let parser = JsonOutputKeyToolsParser::new("NameCollector");
    let result = parser.parse_result(&[generation], false).unwrap();

    let expected = json!([{"names": ["suzy"]}]);
    assert_eq!(result, expected);
}

/// Ported from: test_partial_json_output_key_parser_first_only (additional_kwargs)
#[test]
fn test_json_output_key_tools_parser_first_only_additional_kwargs() {
    let raw_tool_calls = json!([
        {
            "id": "call_OwL7f5PEPJTYzw9sQlNJtCZl",
            "function": {
                "arguments": "{\"names\": [\"suzy\"]}",
                "name": "NameCollector"
            },
            "type": "function"
        }
    ]);

    let generation = make_additional_kwargs_generation(raw_tool_calls);
    let parser = JsonOutputKeyToolsParser::new("NameCollector").with_first_tool_only(true);
    let result = parser.parse_result(&[generation], false).unwrap();

    let expected = json!({"names": ["suzy"]});
    assert_eq!(result, expected);
}

// ---------------------------------------------------------------------------
// Test: JsonOutputKeyToolsParser (tool_calls variant)
// ---------------------------------------------------------------------------

#[test]
fn test_json_output_key_tools_parser_tool_calls() {
    let tool_calls = vec![make_tool_call(
        "call_OwL7f5PEPJTYzw9sQlNJtCZl",
        "NameCollector",
        json!({"names": ["suzy"]}),
    )];

    let generation = make_tool_calls_generation(tool_calls);
    let parser = JsonOutputKeyToolsParser::new("NameCollector");
    let result = parser.parse_result(&[generation], false).unwrap();

    let expected = json!([{"names": ["suzy"]}]);
    assert_eq!(result, expected);
}

#[test]
fn test_json_output_key_tools_parser_first_only_tool_calls() {
    let tool_calls = vec![make_tool_call(
        "call_OwL7f5PEPJTYzw9sQlNJtCZl",
        "NameCollector",
        json!({"names": ["suzy"]}),
    )];

    let generation = make_tool_calls_generation(tool_calls);
    let parser = JsonOutputKeyToolsParser::new("NameCollector").with_first_tool_only(true);
    let result = parser.parse_result(&[generation], false).unwrap();

    let expected = json!({"names": ["suzy"]});
    assert_eq!(result, expected);
}

// ---------------------------------------------------------------------------
// Test: test_json_output_key_tools_parser_multiple_tools_first_only
// Ported from Python test of the same name.
// ---------------------------------------------------------------------------

fn run_multiple_tools_first_only_test(use_tool_calls: bool) {
    let generation = if use_tool_calls {
        make_tool_calls_generation(vec![
            make_tool_call("call_other", "other", json!({"b": 2})),
            make_tool_call("call_func", "func", json!({"a": 1})),
        ])
    } else {
        make_additional_kwargs_generation(json!([
            {
                "id": "call_other",
                "function": {"name": "other", "arguments": "{\"b\":2}"},
                "type": "function"
            },
            {
                "id": "call_func",
                "function": {"name": "func", "arguments": "{\"a\":1}"},
                "type": "function"
            }
        ]))
    };

    let result = vec![generation];

    // Test with return_id=true
    let parser = JsonOutputKeyToolsParser::new("func")
        .with_first_tool_only(true)
        .with_return_id(true);
    let output = parser.parse_result(&result, false).unwrap();

    assert!(!output.is_null());
    assert_eq!(output["type"], "func");
    assert_eq!(output["args"], json!({"a": 1}));
    assert!(output.get("id").is_some());

    // Test with return_id=false
    let parser_no_id = JsonOutputKeyToolsParser::new("func")
        .with_first_tool_only(true)
        .with_return_id(false);
    let output_no_id = parser_no_id.parse_result(&result, false).unwrap();

    assert_eq!(output_no_id, json!({"a": 1}));
}

#[test]
fn test_json_output_key_tools_parser_multiple_tools_first_only_additional_kwargs() {
    run_multiple_tools_first_only_test(false);
}

#[test]
fn test_json_output_key_tools_parser_multiple_tools_first_only_tool_calls() {
    run_multiple_tools_first_only_test(true);
}

// ---------------------------------------------------------------------------
// Test: test_json_output_key_tools_parser_multiple_tools_no_match
// ---------------------------------------------------------------------------

fn run_multiple_tools_no_match_test(use_tool_calls: bool) {
    let generation = if use_tool_calls {
        make_tool_calls_generation(vec![
            make_tool_call("call_other", "other", json!({"b": 2})),
            make_tool_call("call_another", "another", json!({"c": 3})),
        ])
    } else {
        make_additional_kwargs_generation(json!([
            {
                "id": "call_other",
                "function": {"name": "other", "arguments": "{\"b\":2}"},
                "type": "function"
            },
            {
                "id": "call_another",
                "function": {"name": "another", "arguments": "{\"c\":3}"},
                "type": "function"
            }
        ]))
    };

    let result = vec![generation];

    // Test with return_id=true, first_tool_only=true
    let parser = JsonOutputKeyToolsParser::new("nonexistent")
        .with_first_tool_only(true)
        .with_return_id(true);
    let output = parser.parse_result(&result, false).unwrap();
    assert!(output.is_null());

    // Test with return_id=false, first_tool_only=true
    let parser_no_id = JsonOutputKeyToolsParser::new("nonexistent")
        .with_first_tool_only(true)
        .with_return_id(false);
    let output_no_id = parser_no_id.parse_result(&result, false).unwrap();
    assert!(output_no_id.is_null());
}

#[test]
fn test_json_output_key_tools_parser_multiple_tools_no_match_additional_kwargs() {
    run_multiple_tools_no_match_test(false);
}

#[test]
fn test_json_output_key_tools_parser_multiple_tools_no_match_tool_calls() {
    run_multiple_tools_no_match_test(true);
}

// ---------------------------------------------------------------------------
// Test: test_json_output_key_tools_parser_multiple_matching_tools
// ---------------------------------------------------------------------------

fn run_multiple_matching_tools_test(use_tool_calls: bool) {
    let generation = if use_tool_calls {
        make_tool_calls_generation(vec![
            make_tool_call("call_func1", "func", json!({"a": 1})),
            make_tool_call("call_other", "other", json!({"b": 2})),
            make_tool_call("call_func2", "func", json!({"a": 3})),
        ])
    } else {
        make_additional_kwargs_generation(json!([
            {
                "id": "call_func1",
                "function": {"name": "func", "arguments": "{\"a\":1}"},
                "type": "function"
            },
            {
                "id": "call_other",
                "function": {"name": "other", "arguments": "{\"b\":2}"},
                "type": "function"
            },
            {
                "id": "call_func2",
                "function": {"name": "func", "arguments": "{\"a\":3}"},
                "type": "function"
            }
        ]))
    };

    let result = vec![generation];

    // Test with first_tool_only=true - should return first matching
    let parser = JsonOutputKeyToolsParser::new("func")
        .with_first_tool_only(true)
        .with_return_id(true);
    let output = parser.parse_result(&result, false).unwrap();

    assert!(!output.is_null());
    assert_eq!(output["type"], "func");
    assert_eq!(output["args"], json!({"a": 1}));

    // Test with first_tool_only=false - should return all matching
    let parser_all = JsonOutputKeyToolsParser::new("func")
        .with_first_tool_only(false)
        .with_return_id(true);
    let output_all = parser_all.parse_result(&result, false).unwrap();

    let arr = output_all.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["args"], json!({"a": 1}));
    assert_eq!(arr[1]["args"], json!({"a": 3}));
}

#[test]
fn test_json_output_key_tools_parser_multiple_matching_tools_additional_kwargs() {
    run_multiple_matching_tools_test(false);
}

#[test]
fn test_json_output_key_tools_parser_multiple_matching_tools_tool_calls() {
    run_multiple_matching_tools_test(true);
}

// ---------------------------------------------------------------------------
// Test: test_json_output_key_tools_parser_empty_results
// ---------------------------------------------------------------------------

fn run_empty_results_test(use_tool_calls: bool) {
    let generation = if use_tool_calls {
        make_tool_calls_generation(vec![])
    } else {
        make_additional_kwargs_generation(json!([]))
    };

    let result = vec![generation];

    // Test with first_tool_only=true
    let parser = JsonOutputKeyToolsParser::new("func")
        .with_first_tool_only(true)
        .with_return_id(true);
    let output = parser.parse_result(&result, false).unwrap();
    assert!(output.is_null());

    // Test with first_tool_only=false
    let parser_all = JsonOutputKeyToolsParser::new("func")
        .with_first_tool_only(false)
        .with_return_id(true);
    let output_all = parser_all.parse_result(&result, false).unwrap();
    assert_eq!(output_all, json!([]));
}

#[test]
fn test_json_output_key_tools_parser_empty_results_additional_kwargs() {
    run_empty_results_test(false);
}

#[test]
fn test_json_output_key_tools_parser_empty_results_tool_calls() {
    run_empty_results_test(true);
}

// ---------------------------------------------------------------------------
// Test: test_json_output_key_tools_parser_parameter_combinations
// ---------------------------------------------------------------------------

fn run_parameter_combinations_test(use_tool_calls: bool) {
    let generation = if use_tool_calls {
        make_tool_calls_generation(vec![
            make_tool_call("call_other", "other", json!({"b": 2})),
            make_tool_call("call_func1", "func", json!({"a": 1})),
            make_tool_call("call_func2", "func", json!({"a": 3})),
        ])
    } else {
        make_additional_kwargs_generation(json!([
            {
                "id": "call_other",
                "function": {"name": "other", "arguments": "{\"b\":2}"},
                "type": "function"
            },
            {
                "id": "call_func1",
                "function": {"name": "func", "arguments": "{\"a\":1}"},
                "type": "function"
            },
            {
                "id": "call_func2",
                "function": {"name": "func", "arguments": "{\"a\":3}"},
                "type": "function"
            }
        ]))
    };

    let result = vec![generation];

    // Test: first_tool_only=true, return_id=true
    let parser1 = JsonOutputKeyToolsParser::new("func")
        .with_first_tool_only(true)
        .with_return_id(true);
    let output1 = parser1.parse_result(&result, false).unwrap();
    assert_eq!(output1["type"], "func");
    assert_eq!(output1["args"], json!({"a": 1}));
    assert!(output1.get("id").is_some());

    // Test: first_tool_only=true, return_id=false
    let parser2 = JsonOutputKeyToolsParser::new("func")
        .with_first_tool_only(true)
        .with_return_id(false);
    let output2 = parser2.parse_result(&result, false).unwrap();
    assert_eq!(output2, json!({"a": 1}));

    // Test: first_tool_only=false, return_id=true
    let parser3 = JsonOutputKeyToolsParser::new("func")
        .with_first_tool_only(false)
        .with_return_id(true);
    let output3 = parser3.parse_result(&result, false).unwrap();
    let arr3 = output3.as_array().unwrap();
    assert_eq!(arr3.len(), 2);
    assert!(arr3.iter().all(|item| item.get("id").is_some()));
    assert_eq!(arr3[0]["args"], json!({"a": 1}));
    assert_eq!(arr3[1]["args"], json!({"a": 3}));

    // Test: first_tool_only=false, return_id=false
    let parser4 = JsonOutputKeyToolsParser::new("func")
        .with_first_tool_only(false)
        .with_return_id(false);
    let output4 = parser4.parse_result(&result, false).unwrap();
    assert_eq!(output4, json!([{"a": 1}, {"a": 3}]));
}

#[test]
fn test_json_output_key_tools_parser_parameter_combinations_additional_kwargs() {
    run_parameter_combinations_test(false);
}

#[test]
fn test_json_output_key_tools_parser_parameter_combinations_tool_calls() {
    run_parameter_combinations_test(true);
}

// ---------------------------------------------------------------------------
// Test: PydanticToolsParser
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Person {
    age: i64,
    hair_color: String,
    job: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct NameCollector {
    names: Vec<String>,
    person: Person,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Forecast {
    temperature: i64,
    forecast: String,
}

/// Ported from: test_parse_with_different_pydantic_2_proper
fn run_pydantic_tools_parser_test(use_tool_calls: bool) {
    let parser = PydanticToolsParser::new(vec![], false).with_tool::<Forecast>("Forecast");

    let generation = if use_tool_calls {
        make_tool_calls_generation(vec![make_tool_call(
            "call_OwL7f5PE",
            "Forecast",
            json!({"temperature": 20, "forecast": "Sunny"}),
        )])
    } else {
        make_additional_kwargs_generation(json!([
            {
                "id": "call_OwL7f5PE",
                "function": {"name": "Forecast", "arguments": "{\"temperature\": 20, \"forecast\": \"Sunny\"}"},
                "type": "function"
            }
        ]))
    };

    let result = parser.parse_result(&[generation], false).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 1);

    let forecast: Forecast = serde_json::from_value(arr[0].clone()).unwrap();
    assert_eq!(
        forecast,
        Forecast {
            temperature: 20,
            forecast: "Sunny".to_string(),
        }
    );
}

#[test]
fn test_pydantic_tools_parser_additional_kwargs() {
    run_pydantic_tools_parser_test(false);
}

#[test]
fn test_pydantic_tools_parser_tool_calls() {
    run_pydantic_tools_parser_test(true);
}

/// Ported from: test_pydantic_tools_parser_with_nested_models
fn run_pydantic_nested_models_test(use_tool_calls: bool) {
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Coordinates {
        latitude: f64,
        longitude: f64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Location {
        name: String,
        coordinates: Coordinates,
    }

    let parser = PydanticToolsParser::new(vec![], false).with_tool::<Location>("Location");

    let generation = if use_tool_calls {
        make_tool_calls_generation(vec![make_tool_call(
            "call_location",
            "Location",
            json!({
                "name": "Eiffel Tower",
                "coordinates": {"latitude": 48.8584, "longitude": 2.2945}
            }),
        )])
    } else {
        make_additional_kwargs_generation(json!([{
            "id": "call_location",
            "function": {
                "name": "Location",
                "arguments": "{\"name\": \"Eiffel Tower\", \"coordinates\": {\"latitude\": 48.8584, \"longitude\": 2.2945}}"
            },
            "type": "function"
        }]))
    };

    let result = parser.parse_result(&[generation], false).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 1);

    let location: Location = serde_json::from_value(arr[0].clone()).unwrap();
    assert_eq!(location.name, "Eiffel Tower");
    assert_eq!(location.coordinates.latitude, 48.8584);
    assert_eq!(location.coordinates.longitude, 2.2945);
}

#[test]
fn test_pydantic_tools_parser_nested_models_additional_kwargs() {
    run_pydantic_nested_models_test(false);
}

#[test]
fn test_pydantic_tools_parser_nested_models_tool_calls() {
    run_pydantic_nested_models_test(true);
}

/// Ported from: test_pydantic_tools_parser_with_optional_fields
fn run_pydantic_optional_fields_test(use_tool_calls: bool) {
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct User {
        username: String,
        email: String,
        bio: Option<String>,
        age: Option<i64>,
    }

    let parser = PydanticToolsParser::new(vec![], false).with_tool::<User>("User");

    // Test with all fields provided
    let generation_full = if use_tool_calls {
        make_tool_calls_generation(vec![make_tool_call(
            "call_user_full",
            "User",
            json!({
                "username": "john_doe",
                "email": "john@example.com",
                "bio": "Software developer",
                "age": 28
            }),
        )])
    } else {
        make_additional_kwargs_generation(json!([{
            "id": "call_user_full",
            "function": {
                "name": "User",
                "arguments": "{\"username\": \"john_doe\", \"email\": \"john@example.com\", \"bio\": \"Software developer\", \"age\": 28}"
            },
            "type": "function"
        }]))
    };

    let result_full = parser.parse_result(&[generation_full], false).unwrap();
    let arr = result_full.as_array().unwrap();
    assert_eq!(arr.len(), 1);

    let user: User = serde_json::from_value(arr[0].clone()).unwrap();
    assert_eq!(user.username, "john_doe");
    assert_eq!(user.email, "john@example.com");
    assert_eq!(user.bio, Some("Software developer".to_string()));
    assert_eq!(user.age, Some(28));

    // Test with only required fields
    let generation_minimal = if use_tool_calls {
        make_tool_calls_generation(vec![make_tool_call(
            "call_user_minimal",
            "User",
            json!({"username": "jane_smith", "email": "jane@example.com"}),
        )])
    } else {
        make_additional_kwargs_generation(json!([{
            "id": "call_user_minimal",
            "function": {
                "name": "User",
                "arguments": "{\"username\": \"jane_smith\", \"email\": \"jane@example.com\"}"
            },
            "type": "function"
        }]))
    };

    let result_minimal = parser.parse_result(&[generation_minimal], false).unwrap();
    let arr_min = result_minimal.as_array().unwrap();
    assert_eq!(arr_min.len(), 1);

    let user_min: User = serde_json::from_value(arr_min[0].clone()).unwrap();
    assert_eq!(user_min.username, "jane_smith");
    assert_eq!(user_min.email, "jane@example.com");
    assert!(user_min.bio.is_none());
    assert!(user_min.age.is_none());
}

#[test]
fn test_pydantic_tools_parser_optional_fields_additional_kwargs() {
    run_pydantic_optional_fields_test(false);
}

#[test]
fn test_pydantic_tools_parser_optional_fields_tool_calls() {
    run_pydantic_optional_fields_test(true);
}

/// Ported from: test_pydantic_tools_parser_with_mixed_pydantic_versions
/// In Rust we test multiple registered tools in a single parser.
fn run_pydantic_mixed_tools_test(use_tool_calls: bool) {
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Weather {
        temperature: i64,
        conditions: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Location {
        city: String,
        country: String,
    }

    let parser = PydanticToolsParser::new(vec![], false)
        .with_tool::<Weather>("Weather")
        .with_tool::<Location>("Location");

    // Test with Weather
    let generation_weather = if use_tool_calls {
        make_tool_calls_generation(vec![make_tool_call(
            "call_weather",
            "Weather",
            json!({"temperature": 25, "conditions": "sunny"}),
        )])
    } else {
        make_additional_kwargs_generation(json!([{
            "id": "call_weather",
            "function": {
                "name": "Weather",
                "arguments": "{\"temperature\": 25, \"conditions\": \"sunny\"}"
            },
            "type": "function"
        }]))
    };

    let result_weather = parser.parse_result(&[generation_weather], false).unwrap();
    let arr_w = result_weather.as_array().unwrap();
    assert_eq!(arr_w.len(), 1);
    let weather: Weather = serde_json::from_value(arr_w[0].clone()).unwrap();
    assert_eq!(weather.temperature, 25);
    assert_eq!(weather.conditions, "sunny");

    // Test with Location
    let generation_location = if use_tool_calls {
        make_tool_calls_generation(vec![make_tool_call(
            "call_location",
            "Location",
            json!({"city": "Paris", "country": "France"}),
        )])
    } else {
        make_additional_kwargs_generation(json!([{
            "id": "call_location",
            "function": {
                "name": "Location",
                "arguments": "{\"city\": \"Paris\", \"country\": \"France\"}"
            },
            "type": "function"
        }]))
    };

    let result_location = parser.parse_result(&[generation_location], false).unwrap();
    let arr_l = result_location.as_array().unwrap();
    assert_eq!(arr_l.len(), 1);
    let location: Location = serde_json::from_value(arr_l[0].clone()).unwrap();
    assert_eq!(location.city, "Paris");
    assert_eq!(location.country, "France");

    // Test with both in one message
    let generation_mixed = if use_tool_calls {
        make_tool_calls_generation(vec![
            make_tool_call(
                "call_weather",
                "Weather",
                json!({"temperature": 20, "conditions": "cloudy"}),
            ),
            make_tool_call(
                "call_location",
                "Location",
                json!({"city": "London", "country": "UK"}),
            ),
        ])
    } else {
        make_additional_kwargs_generation(json!([
            {
                "id": "call_weather",
                "function": {
                    "name": "Weather",
                    "arguments": "{\"temperature\": 20, \"conditions\": \"cloudy\"}"
                },
                "type": "function"
            },
            {
                "id": "call_location",
                "function": {
                    "name": "Location",
                    "arguments": "{\"city\": \"London\", \"country\": \"UK\"}"
                },
                "type": "function"
            }
        ]))
    };

    let result_mixed = parser.parse_result(&[generation_mixed], false).unwrap();
    let arr_m = result_mixed.as_array().unwrap();
    assert_eq!(arr_m.len(), 2);

    let weather_m: Weather = serde_json::from_value(arr_m[0].clone()).unwrap();
    assert_eq!(weather_m.temperature, 20);

    let location_m: Location = serde_json::from_value(arr_m[1].clone()).unwrap();
    assert_eq!(location_m.city, "London");
}

#[test]
fn test_pydantic_tools_parser_mixed_tools_additional_kwargs() {
    run_pydantic_mixed_tools_test(false);
}

#[test]
fn test_pydantic_tools_parser_mixed_tools_tool_calls() {
    run_pydantic_mixed_tools_test(true);
}

/// Ported from: test_pydantic_tools_parser_name_dict_fallback
#[test]
fn test_pydantic_tools_parser_name_dict_fallback() {
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct ToolWithoutTitle {
        data: String,
    }

    let parser =
        PydanticToolsParser::new(vec![], false).with_tool::<ToolWithoutTitle>("ToolWithoutTitle");

    let generation = make_tool_calls_generation(vec![make_tool_call(
        "call_no_title",
        "ToolWithoutTitle",
        json!({"data": "test_data"}),
    )]);

    let result = parser.parse_result(&[generation], false).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 1);

    let tool: ToolWithoutTitle = serde_json::from_value(arr[0].clone()).unwrap();
    assert_eq!(tool.data, "test_data");
}

/// Ported from: test_pydantic_tools_parser_with_custom_title
#[test]
fn test_pydantic_tools_parser_with_custom_title() {
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct CustomTitleTool {
        value: i64,
        description: String,
    }

    // Register with a custom name (equivalent to model_config title)
    let parser =
        PydanticToolsParser::new(vec![], false).with_tool::<CustomTitleTool>("MyCustomToolName");

    let generation = make_tool_calls_generation(vec![make_tool_call(
        "call_custom",
        "MyCustomToolName",
        json!({"value": 42, "description": "test"}),
    )]);

    let result = parser.parse_result(&[generation], false).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 1);

    let tool: CustomTitleTool = serde_json::from_value(arr[0].clone()).unwrap();
    assert_eq!(tool.value, 42);
    assert_eq!(tool.description, "test");
}

/// Ported from: test_max_tokens_error
/// Validation failure when tool call args don't match the schema.
#[test]
fn test_pydantic_tools_parser_validation_error() {
    let parser = PydanticToolsParser::new(vec![], true).with_tool::<NameCollector>("NameCollector");

    // Missing required "person" field - should fail validation
    let generation = make_tool_calls_generation(vec![make_tool_call(
        "call_OwL7f5PE",
        "NameCollector",
        json!({"names": ["suz", "jerm"]}),
    )]);

    let result = parser.parse_result(&[generation], false);
    assert!(result.is_err());
}

/// Ported from: test_pydantic_tools_parser_with_nested_models (both in one message)
#[test]
fn test_pydantic_tools_parser_nested_models_mixed_in_one_message() {
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Address {
        street: String,
        city: String,
        zip_code: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct PersonWithAddress {
        name: String,
        age: i64,
        address: Address,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Coordinates {
        latitude: f64,
        longitude: f64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct LocationWithCoords {
        name: String,
        coordinates: Coordinates,
    }

    let parser = PydanticToolsParser::new(vec![], false)
        .with_tool::<PersonWithAddress>("PersonWithAddress")
        .with_tool::<LocationWithCoords>("LocationWithCoords");

    let generation = make_tool_calls_generation(vec![
        make_tool_call(
            "call_person",
            "PersonWithAddress",
            json!({
                "name": "Bob",
                "age": 25,
                "address": {
                    "street": "456 Oak Ave",
                    "city": "Portland",
                    "zip_code": "97201"
                }
            }),
        ),
        make_tool_call(
            "call_location",
            "LocationWithCoords",
            json!({
                "name": "Golden Gate Bridge",
                "coordinates": {"latitude": 37.8199, "longitude": -122.4783}
            }),
        ),
    ]);

    let result = parser.parse_result(&[generation], false).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);

    let person: PersonWithAddress = serde_json::from_value(arr[0].clone()).unwrap();
    assert_eq!(person.name, "Bob");
    assert_eq!(person.address.city, "Portland");

    let location: LocationWithCoords = serde_json::from_value(arr[1].clone()).unwrap();
    assert_eq!(location.name, "Golden Gate Bridge");
    assert!((location.coordinates.latitude - 37.8199).abs() < 0.0001);
}

// ---------------------------------------------------------------------------
// Test: parse_tool_call function
// ---------------------------------------------------------------------------

/// Ported from: test_parse_tool_call_with_none_arguments
#[test]
fn test_parse_tool_call_with_none_arguments() {
    let raw_tool_call = json!({
        "function": {"arguments": null, "name": "orderStatus"},
        "id": "chatcmpl-tool-8b1f759d874b412e931e64cf6f57bdcc",
        "type": "function"
    });

    let result = parse_tool_call(&raw_tool_call, false, false, true)
        .unwrap()
        .unwrap();

    assert_eq!(result["name"], "orderStatus");
    assert_eq!(result["args"], json!({}));
    assert_eq!(
        result["id"],
        "chatcmpl-tool-8b1f759d874b412e931e64cf6f57bdcc"
    );
}

/// Ported from: test_parse_tool_call_with_empty_string_arguments
#[test]
fn test_parse_tool_call_with_empty_string_arguments() {
    let raw_tool_call = json!({
        "function": {"arguments": "", "name": "getStatus"},
        "id": "call_123",
        "type": "function"
    });

    let result = parse_tool_call(&raw_tool_call, false, false, true)
        .unwrap()
        .unwrap();

    assert_eq!(result["name"], "getStatus");
    assert_eq!(result["args"], json!({}));
    assert_eq!(result["id"], "call_123");
}

/// Ported from: test_parse_tool_call_with_valid_arguments
#[test]
fn test_parse_tool_call_with_valid_arguments() {
    let raw_tool_call = json!({
        "function": {"arguments": "{\"param\": \"value\"}", "name": "myTool"},
        "id": "call_456",
        "type": "function"
    });

    let result = parse_tool_call(&raw_tool_call, false, false, true)
        .unwrap()
        .unwrap();

    assert_eq!(result["name"], "myTool");
    assert_eq!(result["args"], json!({"param": "value"}));
    assert_eq!(result["id"], "call_456");
}

/// Ported from: test_parse_tool_call_partial_mode_with_none_arguments
#[test]
fn test_parse_tool_call_partial_mode_with_none_arguments() {
    let raw_tool_call = json!({
        "function": {"arguments": null, "name": "streamingTool"},
        "id": "call_789",
        "type": "function"
    });

    let result = parse_tool_call(&raw_tool_call, true, false, true).unwrap();

    // In partial mode, None arguments returns None (incomplete tool call)
    assert!(result.is_none());
}

/// Test parse_tool_call without return_id
#[test]
fn test_parse_tool_call_without_return_id() {
    let raw_tool_call = json!({
        "function": {"arguments": "{\"x\": 1}", "name": "someTool"},
        "id": "call_abc",
        "type": "function"
    });

    let result = parse_tool_call(&raw_tool_call, false, false, false)
        .unwrap()
        .unwrap();

    assert_eq!(result["name"], "someTool");
    assert_eq!(result["args"], json!({"x": 1}));
    assert!(result.get("id").is_none());
}

/// Test parse_tool_call with no function key returns None
#[test]
fn test_parse_tool_call_no_function_key() {
    let raw_tool_call = json!({"id": "call_123"});

    let result = parse_tool_call(&raw_tool_call, false, false, true).unwrap();
    assert!(result.is_none());
}

/// Test parse_tool_call with invalid JSON arguments errors
#[test]
fn test_parse_tool_call_invalid_json_arguments() {
    let raw_tool_call = json!({
        "function": {"arguments": "not valid json{{{", "name": "badTool"},
        "id": "call_bad",
        "type": "function"
    });

    let result = parse_tool_call(&raw_tool_call, false, false, true);
    assert!(result.is_err());
}

/// Test parse_tool_call partial mode with partial JSON
#[test]
fn test_parse_tool_call_partial_mode_with_partial_json() {
    let raw_tool_call = json!({
        "function": {"arguments": "{\"na", "name": "NameCollector"},
        "id": "call_partial",
        "type": "function"
    });

    let result = parse_tool_call(&raw_tool_call, true, false, true).unwrap();
    // Partial JSON should be parseable by parse_partial_json
    // "{\"na" may or may not parse depending on the partial parser
    // The important thing is it doesn't error
    // The result may be None or Some depending on parse_partial_json behavior
    let _ = result;
}

// ---------------------------------------------------------------------------
// Test: PydanticToolsParser first_tool_only
// ---------------------------------------------------------------------------

/// Ported from: test_partial_pydantic_output_parser (non-streaming version)
#[test]
fn test_pydantic_tools_parser_first_tool_only() {
    let parser = PydanticToolsParser::new(vec![], true).with_tool::<NameCollector>("NameCollector");

    let generation = make_tool_calls_generation(vec![make_tool_call(
        "call_1",
        "NameCollector",
        json!({
            "names": ["suzy", "jermaine", "alex"],
            "person": {"age": 39, "hair_color": "brown", "job": "concierge"}
        }),
    )]);

    let result = parser.parse_result(&[generation], false).unwrap();

    // first_tool_only returns a single object, not an array
    assert!(result.is_object());

    let collector: NameCollector = serde_json::from_value(result).unwrap();
    assert_eq!(collector.names, vec!["suzy", "jermaine", "alex"]);
    assert_eq!(collector.person.age, 39);
    assert_eq!(collector.person.hair_color, "brown");
    assert_eq!(collector.person.job, "concierge");
}

/// Test PydanticToolsParser with empty tool_calls and first_tool_only
#[test]
fn test_pydantic_tools_parser_first_tool_only_empty() {
    let parser = PydanticToolsParser::new(vec![], true).with_tool::<Forecast>("Forecast");

    let generation = make_tool_calls_generation(vec![]);
    let result = parser.parse_result(&[generation], false).unwrap();

    assert!(result.is_null());
}

/// Test PydanticToolsParser with empty tool_calls and first_tool_only=false
#[test]
fn test_pydantic_tools_parser_empty_list() {
    let parser = PydanticToolsParser::new(vec![], false).with_tool::<Forecast>("Forecast");

    let generation = make_tool_calls_generation(vec![]);
    let result = parser.parse_result(&[generation], false).unwrap();

    assert_eq!(result, json!([]));
}

// ---------------------------------------------------------------------------
// Test: JsonOutputToolsParser first_tool_only
// ---------------------------------------------------------------------------

#[test]
fn test_json_output_tools_parser_first_tool_only() {
    let parser = JsonOutputToolsParser::new().with_first_tool_only(true);

    let generation = make_tool_calls_generation(vec![
        make_tool_call("call_1", "func1", json!({"a": 1})),
        make_tool_call("call_2", "func2", json!({"b": 2})),
    ]);

    let result = parser.parse_result(&[generation], false).unwrap();

    assert!(result.is_object());
    assert_eq!(result["type"], "func1");
    assert_eq!(result["args"], json!({"a": 1}));
}

#[test]
fn test_json_output_tools_parser_first_tool_only_empty() {
    let parser = JsonOutputToolsParser::new().with_first_tool_only(true);

    let generation = make_tool_calls_generation(vec![]);
    let result = parser.parse_result(&[generation], false).unwrap();

    assert!(result.is_null());
}

// ---------------------------------------------------------------------------
// Test: JsonOutputToolsParser with no tool_calls and no additional_kwargs
// ---------------------------------------------------------------------------

#[test]
fn test_json_output_tools_parser_no_tool_calls_no_kwargs() {
    let message = AIMessage::builder().content("Hello").build();
    let generation = ChatGeneration::new(message.into());

    let parser = JsonOutputToolsParser::new();
    let result = parser.parse_result(&[generation], false).unwrap();

    assert_eq!(result, json!([]));
}

// ---------------------------------------------------------------------------
// Test: parse_tool_calls (list version)
// ---------------------------------------------------------------------------

#[test]
fn test_parse_tool_calls_multiple() {
    let raw = json!([
        {
            "function": {"arguments": "{\"a\": 1}", "name": "tool1"},
            "id": "call_1",
            "type": "function"
        },
        {
            "function": {"arguments": "{\"b\": 2}", "name": "tool2"},
            "id": "call_2",
            "type": "function"
        }
    ]);

    let result = agent_chain_core::output_parsers::parse_tool_calls(
        raw.as_array().unwrap(),
        false,
        false,
        true,
    )
    .unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0]["name"], "tool1");
    assert_eq!(result[0]["args"], json!({"a": 1}));
    assert_eq!(result[1]["name"], "tool2");
    assert_eq!(result[1]["args"], json!({"b": 2}));
}

#[test]
fn test_parse_tool_calls_with_invalid_json_collects_errors() {
    let raw = json!([
        {
            "function": {"arguments": "{{invalid}}", "name": "tool1"},
            "id": "call_1",
            "type": "function"
        }
    ]);

    let result = agent_chain_core::output_parsers::parse_tool_calls(
        raw.as_array().unwrap(),
        false,
        false,
        true,
    );
    assert!(result.is_err());
}

#[test]
fn test_parse_tool_calls_empty() {
    let raw: Vec<Value> = vec![];
    let result =
        agent_chain_core::output_parsers::parse_tool_calls(&raw, false, false, true).unwrap();
    assert!(result.is_empty());
}
