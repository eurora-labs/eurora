//! Utilities for environment variables.
//!
//! Adapted from `langchain_core/utils/env.py`

use std::collections::HashMap;
use std::env;

/// Check if an environment variable is set.
///
/// # Arguments
///
/// * `env_var` - The name of the environment variable.
///
/// # Returns
///
/// `true` if the environment variable is set and not falsy, `false` otherwise.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::env::env_var_is_set;
/// use std::env;
///
/// // SAFETY: This is a single-threaded doc test
/// unsafe { env::set_var("MY_TEST_VAR", "value"); }
/// assert!(env_var_is_set("MY_TEST_VAR"));
/// // SAFETY: This is a single-threaded doc test
/// unsafe { env::remove_var("MY_TEST_VAR"); }
/// ```
pub fn env_var_is_set(env_var: &str) -> bool {
    match env::var(env_var) {
        Ok(value) => !value.is_empty() && value != "0" && value != "false" && value != "False",
        Err(_) => false,
    }
}

/// Get a value from a dictionary or an environment variable.
///
/// # Arguments
///
/// * `data` - The dictionary to look up the key in.
/// * `keys` - The keys to look up in the dictionary. This can be multiple keys to try in order.
/// * `env_key` - The environment variable to look up if the key is not in the dictionary.
/// * `default` - The default value to return if the key is not in the dictionary or the environment.
///
/// # Returns
///
/// The dict value or the environment variable value, or an error if not found.
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
/// use agent_chain_core::utils::env::get_from_dict_or_env;
///
/// let mut data = HashMap::new();
/// data.insert("api_key".to_string(), "my_key".to_string());
///
/// let result = get_from_dict_or_env(&data, &["api_key"], "API_KEY", None);
/// assert_eq!(result.unwrap(), "my_key");
/// ```
pub fn get_from_dict_or_env(
    data: &HashMap<String, String>,
    keys: &[&str],
    env_key: &str,
    default: Option<&str>,
) -> Result<String, EnvError> {
    for key in keys {
        if let Some(value) = data.get(*key)
            && !value.is_empty()
        {
            return Ok(value.clone());
        }
    }

    let key_for_err = keys.first().copied().unwrap_or(env_key);
    get_from_env(key_for_err, env_key, default)
}

/// Get a value from an environment variable.
///
/// # Arguments
///
/// * `key` - The key name (used in error messages).
/// * `env_key` - The environment variable to look up.
/// * `default` - The default value to return if the environment variable is not set.
///
/// # Returns
///
/// The value of the environment variable, or an error if not found and no default provided.
///
/// # Errors
///
/// Returns `EnvError::NotFound` if the environment variable is not set and no default value is provided.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::env::get_from_env;
/// use std::env;
///
/// // SAFETY: This is a single-threaded doc test
/// unsafe { env::set_var("MY_TEST_VAR", "test_value"); }
/// let result = get_from_env("my_test", "MY_TEST_VAR", None);
/// assert_eq!(result.unwrap(), "test_value");
/// // SAFETY: This is a single-threaded doc test
/// unsafe { env::remove_var("MY_TEST_VAR"); }
/// ```
pub fn get_from_env(key: &str, env_key: &str, default: Option<&str>) -> Result<String, EnvError> {
    if let Ok(value) = env::var(env_key)
        && !value.is_empty()
    {
        return Ok(value);
    }

    if let Some(default_val) = default {
        return Ok(default_val.to_string());
    }

    Err(EnvError::NotFound {
        key: key.to_string(),
        env_key: env_key.to_string(),
    })
}

/// Error types for environment operations.
#[derive(Debug, Clone, PartialEq)]
pub enum EnvError {
    /// The environment variable was not found.
    NotFound { key: String, env_key: String },
    /// A custom error message.
    Custom(String),
}

impl std::fmt::Display for EnvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvError::NotFound { key, env_key } => {
                write!(
                    f,
                    "Did not find {}, please add an environment variable `{}` which contains it, or pass `{}` as a named parameter.",
                    key, env_key, key
                )
            }
            EnvError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for EnvError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_var_is_set() {
        unsafe {
            env::set_var("TEST_VAR_SET", "value");
        }
        assert!(env_var_is_set("TEST_VAR_SET"));
        unsafe {
            env::remove_var("TEST_VAR_SET");
        }

        unsafe {
            env::set_var("TEST_VAR_EMPTY", "");
        }
        assert!(!env_var_is_set("TEST_VAR_EMPTY"));
        unsafe {
            env::remove_var("TEST_VAR_EMPTY");
        }

        unsafe {
            env::set_var("TEST_VAR_FALSE", "false");
        }
        assert!(!env_var_is_set("TEST_VAR_FALSE"));
        unsafe {
            env::remove_var("TEST_VAR_FALSE");
        }

        unsafe {
            env::set_var("TEST_VAR_ZERO", "0");
        }
        assert!(!env_var_is_set("TEST_VAR_ZERO"));
        unsafe {
            env::remove_var("TEST_VAR_ZERO");
        }

        assert!(!env_var_is_set("NONEXISTENT_VAR_12345"));
    }

    #[test]
    fn test_get_from_dict_or_env() {
        let mut data = HashMap::new();
        data.insert("key1".to_string(), "value1".to_string());

        let result = get_from_dict_or_env(&data, &["key1"], "ENV_KEY", None);
        assert_eq!(result.unwrap(), "value1");

        unsafe {
            env::set_var("TEST_ENV_KEY", "env_value");
        }
        let result = get_from_dict_or_env(&data, &["key2"], "TEST_ENV_KEY", None);
        assert_eq!(result.unwrap(), "env_value");
        unsafe {
            env::remove_var("TEST_ENV_KEY");
        }

        let result = get_from_dict_or_env(&data, &["key3"], "NONEXISTENT", Some("default"));
        assert_eq!(result.unwrap(), "default");
    }

    #[test]
    fn test_get_from_env() {
        unsafe {
            env::set_var("TEST_GET_FROM_ENV", "test_value");
        }
        let result = get_from_env("test", "TEST_GET_FROM_ENV", None);
        assert_eq!(result.unwrap(), "test_value");
        unsafe {
            env::remove_var("TEST_GET_FROM_ENV");
        }

        let result = get_from_env("test", "NONEXISTENT_VAR", Some("default"));
        assert_eq!(result.unwrap(), "default");

        let result = get_from_env("test", "NONEXISTENT_VAR", None);
        assert!(result.is_err());
    }
}
