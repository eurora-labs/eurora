use std::collections::HashMap;
use std::env;

pub fn env_var_is_set(env_var: &str) -> bool {
    match env::var(env_var) {
        Ok(value) => !value.is_empty() && value != "0" && value != "false" && value != "False",
        Err(_) => false,
    }
}

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

#[derive(Debug, Clone, PartialEq)]
pub enum EnvError {
    NotFound { key: String, env_key: String },
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
