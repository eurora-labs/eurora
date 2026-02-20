use std::collections::HashMap;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

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

#[derive(Debug, Clone, PartialEq)]
pub struct XorArgsError {
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

#[derive(Debug, Clone, PartialEq)]
pub struct HttpStatusError {
    pub status: u16,
    pub text: String,
}

impl std::fmt::Display for HttpStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HTTP {} error: {}", self.status, self.text)
    }
}

impl std::error::Error for HttpStatusError {}

#[derive(Clone)]
pub struct SecretString {
    value: String,
}

impl SecretString {
    pub fn new(value: String) -> Self {
        Self { value }
    }

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

pub fn convert_to_secret_str<S: Into<SecretString>>(value: S) -> SecretString {
    value.into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoDefault;

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

pub fn secret_from_env<'a>(
    keys: &'a [&'a str],
    default: Option<&'a str>,
    error_message: Option<&'a str>,
) -> impl Fn() -> Result<SecretString, EnvError> + 'a {
    let get_value = from_env(keys, default, error_message);
    move || get_value().map(SecretString::new)
}

pub const LC_AUTO_PREFIX: &str = "lc_";

pub const LC_ID_PREFIX: &str = "lc_run-";

pub fn ensure_id(id_val: Option<String>) -> String {
    id_val.unwrap_or_else(|| format!("{}{}", LC_AUTO_PREFIX, Uuid::new_v4()))
}

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

#[derive(Debug, Clone)]
pub struct MockTime {
    timestamp_secs: u64,
    nanos: u32,
}

impl MockTime {
    pub fn fixed(timestamp_secs: u64) -> Self {
        Self {
            timestamp_secs,
            nanos: 0,
        }
    }

    pub fn fixed_millis(timestamp_millis: u64) -> Self {
        Self {
            timestamp_secs: timestamp_millis / 1000,
            nanos: ((timestamp_millis % 1000) * 1_000_000) as u32,
        }
    }

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

    pub fn now_secs(&self) -> u64 {
        self.timestamp_secs
    }

    pub fn now_millis(&self) -> u64 {
        self.timestamp_secs * 1000 + (self.nanos / 1_000_000) as u64
    }

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

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}

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
