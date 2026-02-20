use std::collections::HashMap;
use std::env;

use serde_json::json;

use agent_chain_core::outputs::GenerationChunk;
use agent_chain_core::utils::base::{EnvError, from_env, secret_from_env};
use agent_chain_core::utils::merge::{MergeError, merge_dicts};

struct MergeDictsTestCase {
    left: serde_json::Value,
    right: serde_json::Value,
    expected: Result<serde_json::Value, &'static str>,
}

fn get_merge_dicts_test_cases() -> Vec<MergeDictsTestCase> {
    vec![
        MergeDictsTestCase {
            left: json!({"a": null}),
            right: json!({"a": 1}),
            expected: Ok(json!({"a": 1})),
        },
        MergeDictsTestCase {
            left: json!({"a": 1}),
            right: json!({"a": null}),
            expected: Ok(json!({"a": 1})),
        },
        MergeDictsTestCase {
            left: json!({"a": null}),
            right: json!({"a": 0}),
            expected: Ok(json!({"a": 0})),
        },
        MergeDictsTestCase {
            left: json!({"a": null}),
            right: json!({"a": "txt"}),
            expected: Ok(json!({"a": "txt"})),
        },
        MergeDictsTestCase {
            left: json!({"a": 1}),
            right: json!({"a": 1}),
            expected: Ok(json!({"a": 1})),
        },
        MergeDictsTestCase {
            left: json!({"a": 1.5}),
            right: json!({"a": 1.5}),
            expected: Ok(json!({"a": 1.5})),
        },
        MergeDictsTestCase {
            left: json!({"a": true}),
            right: json!({"a": true}),
            expected: Ok(json!({"a": true})),
        },
        MergeDictsTestCase {
            left: json!({"a": false}),
            right: json!({"a": false}),
            expected: Ok(json!({"a": false})),
        },
        MergeDictsTestCase {
            left: json!({"a": "txt"}),
            right: json!({"a": "txt"}),
            expected: Ok(json!({"a": "txttxt"})),
        },
        MergeDictsTestCase {
            left: json!({"a": [1, 2]}),
            right: json!({"a": [1, 2]}),
            expected: Ok(json!({"a": [1, 2, 1, 2]})),
        },
        MergeDictsTestCase {
            left: json!({"a": {"b": "txt"}}),
            right: json!({"a": {"b": "txt"}}),
            expected: Ok(json!({"a": {"b": "txttxt"}})),
        },
        MergeDictsTestCase {
            left: json!({"a": "one"}),
            right: json!({"a": "two"}),
            expected: Ok(json!({"a": "onetwo"})),
        },
        MergeDictsTestCase {
            left: json!({"a": {"b": 1}}),
            right: json!({"a": {"c": 2}}),
            expected: Ok(json!({"a": {"b": 1, "c": 2}})),
        },
        MergeDictsTestCase {
            left: json!({"function_call": {"arguments": null}}),
            right: json!({"function_call": {"arguments": "{\n"}}),
            expected: Ok(json!({"function_call": {"arguments": "{\n"}})),
        },
        MergeDictsTestCase {
            left: json!({"a": [1, 2]}),
            right: json!({"a": [3]}),
            expected: Ok(json!({"a": [1, 2, 3]})),
        },
        MergeDictsTestCase {
            left: json!({"a": 1, "b": 2}),
            right: json!({"a": 1}),
            expected: Ok(json!({"a": 1, "b": 2})),
        },
        MergeDictsTestCase {
            left: json!({"a": 1, "b": 2}),
            right: json!({"c": null}),
            expected: Ok(json!({"a": 1, "b": 2, "c": null})),
        },
        MergeDictsTestCase {
            left: json!({"a": 1}),
            right: json!({"a": "1"}),
            expected: Err("TypeMismatch"),
        },
        MergeDictsTestCase {
            left: json!({"a": [{"index": 0, "b": "{"}]}),
            right: json!({"a": [{"index": 0, "b": "f"}]}),
            expected: Ok(json!({"a": [{"index": 0, "b": "{f"}]})),
        },
        MergeDictsTestCase {
            left: json!({"a": [{"idx": 0, "b": "{"}]}),
            right: json!({"a": [{"idx": 0, "b": "f"}]}),
            expected: Ok(json!({"a": [{"idx": 0, "b": "{"}, {"idx": 0, "b": "f"}]})),
        },
    ]
}

#[test]
fn test_merge_dicts() {
    for (i, test_case) in get_merge_dicts_test_cases().into_iter().enumerate() {
        let left_copy = test_case.left.clone();
        let right_copy = test_case.right.clone();

        let result = merge_dicts(test_case.left.clone(), vec![test_case.right.clone()]);

        match (result, test_case.expected) {
            (Ok(actual), Ok(expected)) => {
                assert_eq!(actual, expected, "Test case {} failed", i);
            }
            (Err(MergeError::TypeMismatch { .. }), Err("TypeMismatch")) => {}
            (Err(MergeError::UnsupportedType { .. }), Err("UnsupportedType")) => {}
            (result, expected) => {
                panic!(
                    "Test case {} - unexpected result. Got {:?}, expected {:?}",
                    i, result, expected
                );
            }
        }

        assert_eq!(
            test_case.left, left_copy,
            "Test case {} - left was mutated",
            i
        );
        assert_eq!(
            test_case.right, right_copy,
            "Test case {} - right was mutated",
            i
        );
    }
}

#[test]
fn test_from_env_with_env_variable() {
    let key = "TEST_KEY_FROM_ENV";
    let value = "test_value";

    unsafe {
        env::set_var(key, value);
    }

    let keys = [key];
    let get_value = from_env(&keys, None, None);
    assert_eq!(get_value().unwrap(), value);

    unsafe {
        env::remove_var(key);
    }
}

#[test]
fn test_from_env_with_default_value() {
    let key = "TEST_KEY_NONEXISTENT_DEFAULT";
    let default_value = "default_value";

    unsafe {
        env::remove_var(key);
    }

    let keys = [key];
    let get_value = from_env(&keys, Some(default_value), None);
    assert_eq!(get_value().unwrap(), default_value);
}

#[test]
fn test_from_env_with_error_message() {
    let key = "TEST_KEY_NONEXISTENT_ERROR";
    let error_message = "Custom error message";

    unsafe {
        env::remove_var(key);
    }

    let keys = [key];
    let get_value = from_env(&keys, None, Some(error_message));
    let result = get_value();

    assert!(result.is_err());
    match result.unwrap_err() {
        EnvError::Custom(msg) => {
            assert_eq!(msg, error_message);
        }
        _ => panic!("Expected Custom error"),
    }
}

#[test]
fn test_from_env_with_default_error_message() {
    let key = "TEST_KEY_NONEXISTENT_DEFAULT_ERR";

    unsafe {
        env::remove_var(key);
    }

    let keys = [key];
    let get_value = from_env(&keys, None, None);
    let result = get_value();

    assert!(result.is_err());
    match result.unwrap_err() {
        EnvError::NotFound { key: k, .. } => {
            assert!(k.contains(key));
        }
        _ => panic!("Expected NotFound error"),
    }
}

#[test]
fn test_secret_from_env_with_env_variable() {
    let key = "TEST_SECRET_KEY";
    let value = "secret_value";

    unsafe {
        env::set_var(key, value);
    }

    let keys = [key];
    let get_secret = secret_from_env(&keys, None, None);
    let secret = get_secret().unwrap();
    assert_eq!(secret.expose_secret(), value);

    unsafe {
        env::remove_var(key);
    }
}

#[test]
fn test_secret_from_env_with_default_value() {
    let key = "TEST_SECRET_KEY_DEFAULT";
    let default_value = "default_value";

    unsafe {
        env::remove_var(key);
    }

    let keys = [key];
    let get_secret = secret_from_env(&keys, Some(default_value), None);
    let secret = get_secret().unwrap();
    assert_eq!(secret.expose_secret(), default_value);
}

#[test]
fn test_secret_from_env_without_default_raises_error() {
    let key = "TEST_SECRET_KEY_NO_DEFAULT";

    unsafe {
        env::remove_var(key);
    }

    let keys = [key];
    let get_secret = secret_from_env(&keys, None, None);
    let result = get_secret();

    assert!(result.is_err());
    match result.unwrap_err() {
        EnvError::NotFound { key: k, .. } => {
            assert!(k.contains(key));
        }
        _ => panic!("Expected NotFound error"),
    }
}

#[test]
fn test_secret_from_env_with_custom_error_message() {
    let key = "TEST_SECRET_KEY_CUSTOM_ERR";
    let error_message = "Custom error message";

    unsafe {
        env::remove_var(key);
    }

    let keys = [key];
    let get_secret = secret_from_env(&keys, None, Some(error_message));
    let result = get_secret();

    assert!(result.is_err());
    match result.unwrap_err() {
        EnvError::Custom(msg) => {
            assert_eq!(msg, error_message);
        }
        _ => panic!("Expected Custom error"),
    }
}

#[test]
fn test_generation_chunk_addition_combines_metadata() {
    let mut info1 = HashMap::new();
    info1.insert("len".to_string(), json!(0));
    let chunk1 = GenerationChunk::with_info("", info1);

    let mut info2 = HashMap::new();
    info2.insert("len".to_string(), json!(14));
    let chunk2 = GenerationChunk::with_info("Non-empty text", info2);

    let result = chunk1 + chunk2;

    let mut expected_info = HashMap::new();
    expected_info.insert("len".to_string(), json!(14));
    let expected = GenerationChunk::with_info("Non-empty text", expected_info);

    assert_eq!(result.text, expected.text);
    assert_eq!(result.generation_info, expected.generation_info);
}
