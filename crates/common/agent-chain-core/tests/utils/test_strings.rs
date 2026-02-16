//! Unit tests for string utilities.
//!
//! Converted from langchain/libs/core/tests/unit_tests/utils/test_strings.py

use agent_chain_core::utils::strings::{
    comma_list, sanitize_for_postgres, stringify_dict, stringify_value,
};
use serde_json::json;

#[test]
fn test_sanitize_for_postgres() {
    // Test with NUL bytes
    let text_with_nul = "Hello\x00world\x00test";
    let expected = "Helloworldtest";
    assert_eq!(sanitize_for_postgres(text_with_nul, ""), expected);

    // Test with replacement character
    let expected_with_replacement = "Hello world test";
    assert_eq!(
        sanitize_for_postgres(text_with_nul, " "),
        expected_with_replacement
    );

    // Test with text without NUL bytes
    let clean_text = "Hello world";
    assert_eq!(sanitize_for_postgres(clean_text, ""), clean_text);

    // Test empty string
    assert!(sanitize_for_postgres("", "").is_empty());

    // Test with multiple consecutive NUL bytes
    let text_with_multiple_nuls = "Hello\x00\x00\x00world";
    assert_eq!(
        sanitize_for_postgres(text_with_multiple_nuls, ""),
        "Helloworld"
    );
    assert_eq!(
        sanitize_for_postgres(text_with_multiple_nuls, "-"),
        "Hello---world"
    );
}

#[test]
fn test_existing_string_functions() {
    // Test comma_list with numbers as strings
    let nums = vec!["1".to_string(), "2".to_string(), "3".to_string()];
    assert_eq!(comma_list(&nums), "1, 2, 3");

    // Test comma_list with strings
    let items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    assert_eq!(comma_list(&items), "a, b, c");

    // Test stringify_value
    assert_eq!(stringify_value(&json!("hello")), "hello");
    assert_eq!(stringify_value(&json!(42)), "42");

    // Test stringify_dict
    let mut data = std::collections::HashMap::new();
    data.insert("key".to_string(), "value".to_string());
    data.insert("number".to_string(), "123".to_string());
    let result = stringify_dict(&data);
    assert!(result.contains("key: value"));
    assert!(result.contains("number: 123"));
}

#[test]
fn test_stringify_value_nested_structures() {
    // Test nested dict in list
    let nested_data = json!({
        "users": [
            {"name": "Alice", "age": 25},
            {"name": "Bob", "age": 30}
        ],
        "metadata": {"total_users": 2, "active": true}
    });

    let result = stringify_value(&nested_data);

    // Should contain all the nested values
    assert!(
        result.contains("users:"),
        "Result should contain 'users:': {}",
        result
    );
    assert!(
        result.contains("name: Alice"),
        "Result should contain 'name: Alice': {}",
        result
    );
    assert!(
        result.contains("name: Bob"),
        "Result should contain 'name: Bob': {}",
        result
    );
    assert!(
        result.contains("metadata:"),
        "Result should contain 'metadata:': {}",
        result
    );
    assert!(
        result.contains("total_users: 2"),
        "Result should contain 'total_users: 2': {}",
        result
    );
    assert!(
        result.contains("active: true"),
        "Result should contain 'active: true': {}",
        result
    );

    // Test list of mixed types
    let mixed_list = json!(["string", 42, {"key": "value"}, ["nested", "list"]]);
    let result = stringify_value(&mixed_list);

    assert!(
        result.contains("string"),
        "Result should contain 'string': {}",
        result
    );
    assert!(
        result.contains("42"),
        "Result should contain '42': {}",
        result
    );
    assert!(
        result.contains("key: value"),
        "Result should contain 'key: value': {}",
        result
    );
    assert!(
        result.contains("nested"),
        "Result should contain 'nested': {}",
        result
    );
    assert!(
        result.contains("list"),
        "Result should contain 'list': {}",
        result
    );
}
