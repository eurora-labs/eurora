//! Generic utility functions.
//!
//! This module contains generic utility functions adapted from `langchain_core/utils/utils.py`.

use std::collections::HashMap;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Validate that exactly one argument from each group is provided (not None).
///
/// This is the Rust equivalent of Python's `xor_args` decorator, but as a runtime check
/// since Rust doesn't have Python-style decorators.
///
/// # Arguments
///
/// * `arg_groups` - Groups of argument names that are mutually exclusive.
/// * `values` - The actual values of arguments as a map of name -> Option<T>.
///
/// # Returns
///
/// `Ok(())` if validation passes, `Err` with message if exactly one argument
/// in each group is not provided.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::base::validate_xor_args;
/// use std::collections::HashMap;
///
/// let mut values: HashMap<&str, Option<&str>> = HashMap::new();
/// values.insert("api_key", Some("key123"));
/// values.insert("api_key_path", None);
///
/// let groups = vec![vec!["api_key", "api_key_path"]];
/// assert!(validate_xor_args(&groups, &values).is_ok());
/// ```
pub fn validate_xor_args<T>(
    arg_groups: &[Vec<&str>],
    values: &HashMap<&str, Option<T>>,
) -> Result<(), XorArgsError> {
    let mut invalid_groups = Vec::new();

    for (i, group) in arg_groups.iter().enumerate() {
        let count = group
            .iter()
            .filter(|arg| values.get(*arg).is_some_and(|v| v.is_some()))
            .count();

        if count != 1 {
            invalid_groups.push(i);
        }
    }

    if !invalid_groups.is_empty() {
        let invalid_group_names: Vec<String> = invalid_groups
            .iter()
            .map(|&i| arg_groups[i].join(", "))
            .collect();
        return Err(XorArgsError {
            groups: invalid_group_names,
        });
    }

    Ok(())
}

/// Error returned when XOR argument validation fails.
#[derive(Debug, Clone, PartialEq)]
pub struct XorArgsError {
    /// The groups that failed validation.
    pub groups: Vec<String>,
}

impl std::fmt::Display for XorArgsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Exactly one argument in each of the following groups must be defined: {}",
            self.groups.join("; ")
        )
    }
}

impl std::error::Error for XorArgsError {}

/// Raise an error with the response text.
///
/// This is the Rust equivalent of Python's `raise_for_status_with_text`.
/// Works with HTTP status codes.
///
/// # Arguments
///
/// * `status` - The HTTP status code.
/// * `text` - The response text body.
///
/// # Returns
///
/// `Ok(())` if status is success (2xx), `Err` with the response text otherwise.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::base::raise_for_status_with_text;
///
/// // Success case
/// assert!(raise_for_status_with_text(200, "OK").is_ok());
///
/// // Error case
/// let result = raise_for_status_with_text(404, "Not Found");
/// assert!(result.is_err());
/// ```
pub fn raise_for_status_with_text(status: u16, text: &str) -> Result<(), HttpStatusError> {
    if (200..300).contains(&status) {
        Ok(())
    } else {
        Err(HttpStatusError {
            status,
            text: text.to_string(),
        })
    }
}

/// Error returned when HTTP status indicates failure.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpStatusError {
    /// The HTTP status code.
    pub status: u16,
    /// The response text.
    pub text: String,
}

impl std::fmt::Display for HttpStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HTTP {} error: {}", self.status, self.text)
    }
}

impl std::error::Error for HttpStatusError {}

/// A wrapper around a string that prevents it from being printed.
///
/// This is useful for sensitive values like API keys.
/// Equivalent to Python's `pydantic.SecretStr`.
#[derive(Clone)]
pub struct SecretString {
    value: String,
}

impl SecretString {
    /// Create a new secret string.
    pub fn new(value: String) -> Self {
        Self { value }
    }

    /// Get the secret value.
    ///
    /// Use this sparingly to avoid leaking secrets.
    pub fn expose_secret(&self) -> &str {
        &self.value
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecretString(***)")
    }
}

impl std::fmt::Display for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "***")
    }
}

impl From<String> for SecretString {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for SecretString {
    fn from(value: &str) -> Self {
        Self::new(value.to_string())
    }
}

/// Convert a value to a SecretString.
///
/// This is the Rust equivalent of Python's `convert_to_secret_str`.
///
/// # Arguments
///
/// * `value` - The value to convert. Can be a String, &str, or SecretString.
///
/// # Returns
///
/// A SecretString wrapping the value.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::base::convert_to_secret_str;
///
/// let secret = convert_to_secret_str("my-api-key");
/// assert_eq!(secret.expose_secret(), "my-api-key");
/// ```
pub fn convert_to_secret_str<S: Into<SecretString>>(value: S) -> SecretString {
    value.into()
}

/// A marker type to indicate no default value is provided.
///
/// This is the Rust equivalent of Python's `_NoDefaultType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoDefault;

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

/// Create a factory function that gets a value from an environment variable.
///
/// This is the Rust equivalent of Python's `from_env`.
///
/// # Arguments
///
/// * `keys` - The environment variable(s) to look up. If multiple keys are provided,
///   the first key found in the environment will be used.
/// * `default` - The default value to return if the environment variable is not set.
/// * `error_message` - The error message to raise if the key is not found and no default is provided.
///
/// # Returns
///
/// A closure that will look up the value from the environment.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::base::from_env;
/// use std::env;
///
/// // SAFETY: This is a single-threaded doc test
/// unsafe { env::set_var("MY_TEST_VAR", "test_value"); }
/// let get_value = from_env(&["MY_TEST_VAR"], None, None);
/// assert_eq!(get_value().unwrap(), "test_value");
/// // SAFETY: This is a single-threaded doc test
/// unsafe { env::remove_var("MY_TEST_VAR"); }
/// ```
pub fn from_env<'a>(
    keys: &'a [&'a str],
    default: Option<&'a str>,
    error_message: Option<&'a str>,
) -> impl Fn() -> Result<String, EnvError> + 'a {
    move || {
        for key in keys {
            if let Ok(value) = env::var(key)
                && !value.is_empty()
            {
                return Ok(value);
            }
        }

        if let Some(default_val) = default {
            return Ok(default_val.to_string());
        }

        if let Some(msg) = error_message {
            return Err(EnvError::Custom(msg.to_string()));
        }

        let keys_str = keys.join(", ");
        Err(EnvError::NotFound {
            key: keys_str.clone(),
            env_key: keys_str,
        })
    }
}

/// Create a factory function that gets a secret value from an environment variable.
///
/// This is the Rust equivalent of Python's `secret_from_env`.
///
/// # Arguments
///
/// * `keys` - The environment variable(s) to look up.
/// * `default` - The default value to return if the environment variable is not set.
/// * `error_message` - The error message to raise if the key is not found and no default is provided.
///
/// # Returns
///
/// A closure that will look up the secret from the environment.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::base::secret_from_env;
/// use std::env;
///
/// // SAFETY: This is a single-threaded doc test
/// unsafe { env::set_var("MY_SECRET_VAR", "secret_value"); }
/// let get_secret = secret_from_env(&["MY_SECRET_VAR"], None, None);
/// assert_eq!(get_secret().unwrap().expose_secret(), "secret_value");
/// // SAFETY: This is a single-threaded doc test
/// unsafe { env::remove_var("MY_SECRET_VAR"); }
/// ```
pub fn secret_from_env<'a>(
    keys: &'a [&'a str],
    default: Option<&'a str>,
    error_message: Option<&'a str>,
) -> impl Fn() -> Result<SecretString, EnvError> + 'a {
    let get_value = from_env(keys, default, error_message);
    move || get_value().map(SecretString::new)
}

/// LangChain auto-generated ID prefix for messages and content blocks.
pub const LC_AUTO_PREFIX: &str = "lc_";

/// Internal tracing/callback system identifier.
///
/// Used for:
/// - Tracing: Every LangChain operation (LLM call, chain execution, tool use, etc.)
///   gets a unique run_id (UUID)
/// - Enables tracking parent-child relationships between operations
pub const LC_ID_PREFIX: &str = "lc_run-";

/// Ensure the ID is a valid string, generating a new UUID if not provided.
///
/// Auto-generated UUIDs are prefixed by `'lc_'` to indicate they are
/// LangChain-generated IDs.
///
/// # Arguments
///
/// * `id_val` - Optional string ID value to validate.
///
/// # Returns
///
/// A string ID, either the validated provided value or a newly generated UUID4.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::base::ensure_id;
///
/// let id = ensure_id(Some("my-custom-id".to_string()));
/// assert_eq!(id, "my-custom-id");
///
/// let generated = ensure_id(None);
/// assert!(generated.starts_with("lc_"));
/// ```
pub fn ensure_id(id_val: Option<String>) -> String {
    id_val.unwrap_or_else(|| format!("{}{}", LC_AUTO_PREFIX, Uuid::new_v4()))
}

/// Build extra kwargs from values, separating known fields from extra fields.
///
/// This is the Rust equivalent of Python's `_build_model_kwargs`.
///
/// # Arguments
///
/// * `values` - All init args passed in by user.
/// * `known_fields` - Set of known field names for the struct.
///
/// # Returns
///
/// A tuple of (known_kwargs, extra_kwargs) where known_kwargs contains
/// values for known fields and extra_kwargs contains the rest.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::base::build_model_kwargs;
/// use std::collections::HashMap;
/// use std::collections::HashSet;
///
/// let mut values = HashMap::new();
/// values.insert("model".to_string(), serde_json::json!("gpt-4"));
/// values.insert("temperature".to_string(), serde_json::json!(0.7));
/// values.insert("custom_param".to_string(), serde_json::json!("custom_value"));
///
/// let mut known_fields = HashSet::new();
/// known_fields.insert("model".to_string());
/// known_fields.insert("temperature".to_string());
///
/// let (known, extra) = build_model_kwargs(values, &known_fields);
/// assert!(known.contains_key("model"));
/// assert!(known.contains_key("temperature"));
/// assert!(extra.contains_key("custom_param"));
/// ```
pub fn build_model_kwargs(
    mut values: HashMap<String, serde_json::Value>,
    known_fields: &std::collections::HashSet<String>,
) -> (
    HashMap<String, serde_json::Value>,
    HashMap<String, serde_json::Value>,
) {
    let mut extra_kwargs = HashMap::new();

    if let Some(existing_extra) = values.remove("model_kwargs")
        && let Some(obj) = existing_extra.as_object()
    {
        for (k, v) in obj {
            extra_kwargs.insert(k.clone(), v.clone());
        }
    }

    let keys: Vec<String> = values.keys().cloned().collect();
    for key in keys {
        if !known_fields.contains(&key)
            && let Some(value) = values.remove(&key)
        {
            extra_kwargs.insert(key, value);
        }
    }

    (values, extra_kwargs)
}

/// A mock time provider for testing.
///
/// This is the Rust equivalent of Python's `mock_now` context manager.
/// Since Rust doesn't have monkey-patching, this uses a struct that can be
/// passed around to provide a fixed or custom time.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::base::MockTime;
///
/// // Create a mock time at a specific Unix timestamp (seconds)
/// let mock = MockTime::fixed(1609459200); // 2021-01-01 00:00:00 UTC
/// assert_eq!(mock.now_secs(), 1609459200);
/// ```
#[derive(Debug, Clone)]
pub struct MockTime {
    /// The fixed timestamp in seconds since Unix epoch.
    timestamp_secs: u64,
    /// The nanoseconds component.
    nanos: u32,
}

impl MockTime {
    /// Create a new MockTime with a fixed timestamp in seconds.
    pub fn fixed(timestamp_secs: u64) -> Self {
        Self {
            timestamp_secs,
            nanos: 0,
        }
    }

    /// Create a new MockTime with a fixed timestamp in milliseconds.
    pub fn fixed_millis(timestamp_millis: u64) -> Self {
        Self {
            timestamp_secs: timestamp_millis / 1000,
            nanos: ((timestamp_millis % 1000) * 1_000_000) as u32,
        }
    }

    /// Create a MockTime from a specific datetime components.
    ///
    /// Note: This is a simplified version that doesn't handle all datetime edge cases.
    /// For production use, consider using the `chrono` crate.
    pub fn from_components(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> Self {
        let days_before_month: [u32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];

        let year_days = (year - 1970) as u64 * 365
            + ((year - 1969) / 4) as u64 // leap years
            - ((year - 1901) / 100) as u64 // century adjustment
            + ((year - 1601) / 400) as u64; // 400-year adjustment

        let month_days = days_before_month[(month - 1) as usize] as u64;
        let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let leap_adjustment = if is_leap && month > 2 { 1 } else { 0 };

        let total_days = year_days + month_days + (day - 1) as u64 + leap_adjustment;
        let timestamp_secs =
            total_days * 86400 + hour as u64 * 3600 + minute as u64 * 60 + second as u64;

        Self {
            timestamp_secs,
            nanos: 0,
        }
    }

    /// Get the current mocked time in seconds since Unix epoch.
    pub fn now_secs(&self) -> u64 {
        self.timestamp_secs
    }

    /// Get the current mocked time in milliseconds since Unix epoch.
    pub fn now_millis(&self) -> u64 {
        self.timestamp_secs * 1000 + (self.nanos / 1_000_000) as u64
    }

    /// Get the current mocked time as (seconds, nanoseconds) tuple.
    pub fn now(&self) -> (u64, u32) {
        (self.timestamp_secs, self.nanos)
    }
}

impl Default for MockTime {
    fn default() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        Self {
            timestamp_secs: now.as_secs(),
            nanos: now.subsec_nanos(),
        }
    }
}

/// Get the current Unix timestamp in seconds.
///
/// # Returns
///
/// The current Unix timestamp as u64.
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}

/// Get the current Unix timestamp in milliseconds.
///
/// # Returns
///
/// The current Unix timestamp in milliseconds as u64.
pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_xor_args_success() {
        let mut values: HashMap<&str, Option<&str>> = HashMap::new();
        values.insert("api_key", Some("key123"));
        values.insert("api_key_path", None);

        let groups = vec![vec!["api_key", "api_key_path"]];
        assert!(validate_xor_args(&groups, &values).is_ok());
    }

    #[test]
    fn test_validate_xor_args_none_provided() {
        let mut values: HashMap<&str, Option<&str>> = HashMap::new();
        values.insert("api_key", None);
        values.insert("api_key_path", None);

        let groups = vec![vec!["api_key", "api_key_path"]];
        let result = validate_xor_args(&groups, &values);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_xor_args_both_provided() {
        let mut values: HashMap<&str, Option<&str>> = HashMap::new();
        values.insert("api_key", Some("key123"));
        values.insert("api_key_path", Some("/path/to/key"));

        let groups = vec![vec!["api_key", "api_key_path"]];
        let result = validate_xor_args(&groups, &values);
        assert!(result.is_err());
    }

    #[test]
    fn test_raise_for_status_with_text_success() {
        assert!(raise_for_status_with_text(200, "OK").is_ok());
        assert!(raise_for_status_with_text(201, "Created").is_ok());
        assert!(raise_for_status_with_text(299, "Custom").is_ok());
    }

    #[test]
    fn test_raise_for_status_with_text_error() {
        let result = raise_for_status_with_text(404, "Not Found");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status, 404);
        assert_eq!(err.text, "Not Found");

        let result = raise_for_status_with_text(500, "Internal Server Error");
        assert!(result.is_err());
    }

    #[test]
    fn test_secret_string() {
        let secret = SecretString::new("my_secret".to_string());
        assert_eq!(secret.expose_secret(), "my_secret");
        assert_eq!(format!("{}", secret), "***");
        assert_eq!(format!("{:?}", secret), "SecretString(***)");
    }

    #[test]
    fn test_convert_to_secret_str() {
        let secret = convert_to_secret_str("my-secret");
        assert_eq!(secret.expose_secret(), "my-secret");

        let secret = convert_to_secret_str(String::from("another-secret"));
        assert_eq!(secret.expose_secret(), "another-secret");
    }

    #[test]
    fn test_from_env() {
        unsafe {
            env::set_var("TEST_FROM_ENV_UTILS", "test_value");
        }
        let get_value = from_env(&["TEST_FROM_ENV_UTILS"], None, None);
        assert_eq!(get_value().unwrap(), "test_value");
        unsafe {
            env::remove_var("TEST_FROM_ENV_UTILS");
        }

        let get_value = from_env(&["NONEXISTENT_UTILS"], Some("default"), None);
        assert_eq!(get_value().unwrap(), "default");
    }

    #[test]
    fn test_secret_from_env() {
        unsafe {
            env::set_var("TEST_SECRET_FROM_ENV", "secret_value");
        }
        let get_secret = secret_from_env(&["TEST_SECRET_FROM_ENV"], None, None);
        assert_eq!(get_secret().unwrap().expose_secret(), "secret_value");
        unsafe {
            env::remove_var("TEST_SECRET_FROM_ENV");
        }
    }

    #[test]
    fn test_ensure_id_with_value() {
        let id = ensure_id(Some("my-custom-id".to_string()));
        assert_eq!(id, "my-custom-id");
    }

    #[test]
    fn test_ensure_id_without_value() {
        let id = ensure_id(None);
        assert!(id.starts_with(LC_AUTO_PREFIX));
    }

    #[test]
    fn test_mock_time_fixed() {
        let mock = MockTime::fixed(1609459200);
        assert_eq!(mock.now_secs(), 1609459200);
        assert_eq!(mock.now_millis(), 1609459200000);
    }

    #[test]
    fn test_mock_time_fixed_millis() {
        let mock = MockTime::fixed_millis(1609459200500);
        assert_eq!(mock.now_secs(), 1609459200);
        assert_eq!(mock.now_millis(), 1609459200500);
    }

    #[test]
    fn test_build_model_kwargs() {
        let mut values = HashMap::new();
        values.insert("model".to_string(), serde_json::json!("gpt-4"));
        values.insert("temperature".to_string(), serde_json::json!(0.7));
        values.insert("custom_param".to_string(), serde_json::json!("custom"));

        let mut known_fields = std::collections::HashSet::new();
        known_fields.insert("model".to_string());
        known_fields.insert("temperature".to_string());

        let (known, extra) = build_model_kwargs(values, &known_fields);

        assert!(known.contains_key("model"));
        assert!(known.contains_key("temperature"));
        assert!(!known.contains_key("custom_param"));

        assert!(extra.contains_key("custom_param"));
        assert!(!extra.contains_key("model"));
    }

    #[test]
    fn test_now_functions() {
        let secs = now_secs();
        let millis = now_millis();

        assert!(secs > 1577836800);
        assert!(millis > 1577836800000);

        assert!(millis >= secs * 1000);
        assert!(millis < (secs + 2) * 1000);
    }
}
