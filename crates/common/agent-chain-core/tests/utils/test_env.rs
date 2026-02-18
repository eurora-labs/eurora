//! Tests for the env module.
//!
//! These tests mirror the tests for `langchain_core/utils/env.py`.

use std::collections::HashMap;
use std::env;
use std::sync::Mutex;

use agent_chain_core::utils::env::{EnvError, env_var_is_set, get_from_dict_or_env, get_from_env};

/// Mutex to ensure env var tests don't run concurrently.
/// This is necessary because environment variables are process-global state.
static ENV_MUTEX: Mutex<()> = Mutex::new(());

/// Test `get_from_dict_or_env` with a single key that exists in the dictionary.
#[test]
fn test_get_from_dict_or_env_single_key() {
    let _guard = ENV_MUTEX.lock().unwrap();

    let mut data = HashMap::new();
    data.insert("a".to_string(), "foo".to_string());

    let result = get_from_dict_or_env(&data, &["a"], "__SOME_KEY_IN_ENV", None);
    assert_eq!(result.unwrap(), "foo");
}

/// Test `get_from_dict_or_env` with multiple keys, where the second one exists.
#[test]
fn test_get_from_dict_or_env_multiple_keys() {
    let _guard = ENV_MUTEX.lock().unwrap();

    let mut data = HashMap::new();
    data.insert("a".to_string(), "foo".to_string());

    let result = get_from_dict_or_env(&data, &["b", "a"], "__SOME_KEY_IN_ENV", None);
    assert_eq!(result.unwrap(), "foo");
}

/// Test `get_from_dict_or_env` with a default value when key doesn't exist.
#[test]
fn test_get_from_dict_or_env_with_default() {
    let _guard = ENV_MUTEX.lock().unwrap();

    let mut data = HashMap::new();
    data.insert("a".to_string(), "foo".to_string());

    let result = get_from_dict_or_env(&data, &["not exists"], "__SOME_KEY_IN_ENV", Some("default"));
    assert_eq!(result.unwrap(), "default");
}

/// Test `get_from_dict_or_env` raises error when key doesn't exist and no default.
#[test]
fn test_get_from_dict_or_env_raises_error() {
    let _guard = ENV_MUTEX.lock().unwrap();

    let mut data = HashMap::new();
    data.insert("a".to_string(), "foo".to_string());

    let result = get_from_dict_or_env(&data, &["not exists"], "__SOME_KEY_IN_ENV", None);
    assert!(result.is_err());

    match result {
        Err(EnvError::NotFound { key, env_key }) => {
            assert_eq!(key, "not exists");
            assert_eq!(env_key, "__SOME_KEY_IN_ENV");
        }
        _ => panic!("Expected EnvError::NotFound"),
    }
}

/// Test that the error message matches the Python behavior.
#[test]
fn test_get_from_dict_or_env_error_message() {
    let _guard = ENV_MUTEX.lock().unwrap();

    let mut data = HashMap::new();
    data.insert("a".to_string(), "foo".to_string());

    let result = get_from_dict_or_env(&data, &["not exists"], "__SOME_KEY_IN_ENV", None);

    let error = result.unwrap_err();
    let error_msg = error.to_string();

    assert!(error_msg.contains("not exists"));
    assert!(error_msg.contains("__SOME_KEY_IN_ENV"));
    assert!(error_msg.contains("Did not find"));
}

/// Test `env_var_is_set` returns true when variable is set with non-empty value.
#[test]
fn test_env_var_is_set_true() {
    let _guard = ENV_MUTEX.lock().unwrap();

    unsafe {
        env::set_var("TEST_ENV_VAR_IS_SET", "value");
    }

    assert!(env_var_is_set("TEST_ENV_VAR_IS_SET"));

    unsafe {
        env::remove_var("TEST_ENV_VAR_IS_SET");
    }
}

/// Test `env_var_is_set` returns false when variable is empty.
#[test]
fn test_env_var_is_set_empty() {
    let _guard = ENV_MUTEX.lock().unwrap();

    unsafe {
        env::set_var("TEST_ENV_VAR_EMPTY", "");
    }

    assert!(!env_var_is_set("TEST_ENV_VAR_EMPTY"));

    unsafe {
        env::remove_var("TEST_ENV_VAR_EMPTY");
    }
}

/// Test `env_var_is_set` returns false when variable is "0".
#[test]
fn test_env_var_is_set_zero() {
    let _guard = ENV_MUTEX.lock().unwrap();

    unsafe {
        env::set_var("TEST_ENV_VAR_ZERO", "0");
    }

    assert!(!env_var_is_set("TEST_ENV_VAR_ZERO"));

    unsafe {
        env::remove_var("TEST_ENV_VAR_ZERO");
    }
}

/// Test `env_var_is_set` returns false when variable is "false".
#[test]
fn test_env_var_is_set_false_lowercase() {
    let _guard = ENV_MUTEX.lock().unwrap();

    unsafe {
        env::set_var("TEST_ENV_VAR_FALSE_LC", "false");
    }

    assert!(!env_var_is_set("TEST_ENV_VAR_FALSE_LC"));

    unsafe {
        env::remove_var("TEST_ENV_VAR_FALSE_LC");
    }
}

/// Test `env_var_is_set` returns false when variable is "False".
#[test]
fn test_env_var_is_set_false_titlecase() {
    let _guard = ENV_MUTEX.lock().unwrap();

    unsafe {
        env::set_var("TEST_ENV_VAR_FALSE_TC", "False");
    }

    assert!(!env_var_is_set("TEST_ENV_VAR_FALSE_TC"));

    unsafe {
        env::remove_var("TEST_ENV_VAR_FALSE_TC");
    }
}

/// Test `env_var_is_set` returns false when variable doesn't exist.
#[test]
fn test_env_var_is_set_nonexistent() {
    let _guard = ENV_MUTEX.lock().unwrap();

    unsafe {
        env::remove_var("NONEXISTENT_TEST_VAR_12345");
    }

    assert!(!env_var_is_set("NONEXISTENT_TEST_VAR_12345"));
}

/// Test `get_from_env` returns the environment variable value.
#[test]
fn test_get_from_env_success() {
    let _guard = ENV_MUTEX.lock().unwrap();

    unsafe {
        env::set_var("TEST_GET_FROM_ENV_VAR", "test_value");
    }

    let result = get_from_env("test_key", "TEST_GET_FROM_ENV_VAR", None);
    assert_eq!(result.unwrap(), "test_value");

    unsafe {
        env::remove_var("TEST_GET_FROM_ENV_VAR");
    }
}

/// Test `get_from_env` returns the default value when env var not set.
#[test]
fn test_get_from_env_with_default() {
    let _guard = ENV_MUTEX.lock().unwrap();

    unsafe {
        env::remove_var("NONEXISTENT_VAR_FOR_DEFAULT");
    }

    let result = get_from_env(
        "test_key",
        "NONEXISTENT_VAR_FOR_DEFAULT",
        Some("default_value"),
    );
    assert_eq!(result.unwrap(), "default_value");
}

/// Test `get_from_env` returns error when env var not set and no default.
#[test]
fn test_get_from_env_error() {
    let _guard = ENV_MUTEX.lock().unwrap();

    unsafe {
        env::remove_var("NONEXISTENT_VAR_FOR_ERROR");
    }

    let result = get_from_env("test_key", "NONEXISTENT_VAR_FOR_ERROR", None);
    assert!(result.is_err());

    match result {
        Err(EnvError::NotFound { key, env_key }) => {
            assert_eq!(key, "test_key");
            assert_eq!(env_key, "NONEXISTENT_VAR_FOR_ERROR");
        }
        _ => panic!("Expected EnvError::NotFound"),
    }
}

/// Test `get_from_dict_or_env` falls back to env var when key not in dict.
#[test]
fn test_get_from_dict_or_env_fallback_to_env() {
    let _guard = ENV_MUTEX.lock().unwrap();

    unsafe {
        env::set_var("TEST_FALLBACK_ENV_VAR", "env_value");
    }

    let mut data = HashMap::new();
    data.insert("a".to_string(), "foo".to_string());

    let result = get_from_dict_or_env(&data, &["nonexistent"], "TEST_FALLBACK_ENV_VAR", None);
    assert_eq!(result.unwrap(), "env_value");

    unsafe {
        env::remove_var("TEST_FALLBACK_ENV_VAR");
    }
}

/// Test `get_from_dict_or_env` with empty dict.
#[test]
fn test_get_from_dict_or_env_empty_dict() {
    let _guard = ENV_MUTEX.lock().unwrap();

    let data: HashMap<String, String> = HashMap::new();

    let result = get_from_dict_or_env(&data, &["any_key"], "__NONEXISTENT_ENV", Some("default"));
    assert_eq!(result.unwrap(), "default");
}

/// Test `get_from_dict_or_env` with empty value in dict (should be treated as not found).
#[test]
fn test_get_from_dict_or_env_empty_value() {
    let _guard = ENV_MUTEX.lock().unwrap();

    let mut data = HashMap::new();
    data.insert("a".to_string(), "".to_string()); // empty value

    let result = get_from_dict_or_env(&data, &["a"], "__NONEXISTENT_ENV", Some("default"));
    assert_eq!(result.unwrap(), "default");
}

/// Test EnvError Display implementation.
#[test]
fn test_env_error_display() {
    let error = EnvError::NotFound {
        key: "api_key".to_string(),
        env_key: "API_KEY".to_string(),
    };

    let error_msg = error.to_string();
    assert!(error_msg.contains("Did not find api_key"));
    assert!(error_msg.contains("API_KEY"));

    let custom_error = EnvError::Custom("Custom error message".to_string());
    assert_eq!(custom_error.to_string(), "Custom error message");
}

/// Test that EnvError implements std::error::Error.
#[test]
fn test_env_error_is_error() {
    let error: Box<dyn std::error::Error> = Box::new(EnvError::NotFound {
        key: "test".to_string(),
        env_key: "TEST".to_string(),
    });

    let _ = error.to_string();
}
