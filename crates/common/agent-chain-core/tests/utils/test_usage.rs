//! Unit tests for usage utilities.
//!
//! Converted from langchain/libs/core/tests/unit_tests/utils/test_usage.py

use agent_chain_core::utils::usage::{UsageError, dict_int_op};
use serde_json::json;

#[test]
fn test_dict_int_op_add() {
    let left = json!({"a": 1, "b": 2});
    let right = json!({"b": 3, "c": 4});
    let result = dict_int_op(&left, &right, |x, y| x + y, 0, 100).unwrap();
    assert_eq!(result, json!({"a": 1, "b": 5, "c": 4}));
}

#[test]
fn test_dict_int_op_subtract() {
    let left = json!({"a": 5, "b": 10});
    let right = json!({"a": 2, "b": 3, "c": 1});
    let result = dict_int_op(&left, &right, |x, y| (x - y).max(0), 0, 100).unwrap();
    assert_eq!(result, json!({"a": 3, "b": 7, "c": 0}));
}

#[test]
fn test_dict_int_op_nested() {
    let left = json!({"a": 1, "b": {"c": 2, "d": 3}});
    let right = json!({"a": 2, "b": {"c": 1, "e": 4}});
    let result = dict_int_op(&left, &right, |x, y| x + y, 0, 100).unwrap();
    assert_eq!(result, json!({"a": 3, "b": {"c": 3, "d": 3, "e": 4}}));
}

#[test]
fn test_dict_int_op_max_depth_exceeded() {
    let left = json!({"a": {"b": {"c": 1}}});
    let right = json!({"a": {"b": {"c": 2}}});
    let result = dict_int_op(&left, &right, |x, y| x + y, 0, 2);
    assert!(matches!(result, Err(UsageError::MaxDepthExceeded(2))));
    // Verify the error message
    if let Err(e) = result {
        assert!(
            e.to_string().contains("max_depth=2 exceeded"),
            "Error message should contain 'max_depth=2 exceeded': {}",
            e
        );
    }
}

#[test]
fn test_dict_int_op_invalid_types() {
    let left = json!({"a": 1, "b": "string"});
    let right = json!({"a": 2, "b": 3});
    let result = dict_int_op(&left, &right, |x, y| x + y, 0, 100);
    assert!(matches!(result, Err(UsageError::TypeMismatch { .. })));
    // Verify the error message
    if let Err(e) = result {
        assert!(
            e.to_string()
                .contains("Only dict and int values are supported"),
            "Error message should contain 'Only dict and int values are supported': {}",
            e
        );
    }
}
