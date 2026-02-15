//! String utilities.
//!
//! Adapted from langchain_core/utils/strings.py

use serde_json::Value;

/// Stringify a value.
///
/// # Arguments
///
/// * `val` - The value to stringify.
///
/// # Returns
///
/// The stringified value.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::strings::stringify_value;
///
/// assert_eq!(stringify_value(&serde_json::json!("hello")), "hello");
/// assert_eq!(stringify_value(&serde_json::json!(42)), "42");
/// ```
pub fn stringify_value(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Object(map) => {
            let inner = map
                .iter()
                .map(|(k, v)| format!("{}: {}", k, stringify_value(v)))
                .collect::<Vec<_>>()
                .join("\n");
            format!("\n{}", inner)
        }
        Value::Array(arr) => arr
            .iter()
            .map(stringify_value)
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
    }
}

/// Stringify a dictionary.
///
/// # Arguments
///
/// * `data` - The dictionary to stringify.
///
/// # Returns
///
/// The stringified dictionary.
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
/// use agent_chain_core::utils::strings::stringify_dict;
///
/// let mut data = HashMap::new();
/// data.insert("key".to_string(), "value".to_string());
///
/// let result = stringify_dict(&data);
/// assert!(result.contains("key: value"));
/// ```
pub fn stringify_dict(data: &std::collections::HashMap<String, String>) -> String {
    data.iter()
        .map(|(k, v)| format!("{}: {}\n", k, v))
        .collect()
}

/// Convert a list to a comma-separated string.
///
/// # Arguments
///
/// * `items` - The list to convert.
///
/// # Returns
///
/// The comma-separated string.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::strings::comma_list;
///
/// let items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
/// assert_eq!(comma_list(&items), "a, b, c");
/// ```
pub fn comma_list(items: &[String]) -> String {
    items.join(", ")
}

/// Sanitize text by removing NUL bytes that are incompatible with PostgreSQL.
///
/// PostgreSQL text fields cannot contain NUL (0x00) bytes, which can cause
/// errors when inserting documents. This function removes or replaces
/// such characters to ensure compatibility.
///
/// # Arguments
///
/// * `text` - The text to sanitize.
/// * `replacement` - String to replace NUL bytes with.
///
/// # Returns
///
/// The sanitized text with NUL bytes removed or replaced.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::strings::sanitize_for_postgres;
///
/// assert_eq!(sanitize_for_postgres("Hello\x00world", ""), "Helloworld");
/// assert_eq!(sanitize_for_postgres("Hello\x00world", " "), "Hello world");
/// ```
pub fn sanitize_for_postgres(text: &str, replacement: &str) -> String {
    text.replace('\x00', replacement)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_stringify_value_string() {
        assert_eq!(stringify_value(&json!("hello")), "hello");
    }

    #[test]
    fn test_stringify_value_number() {
        assert_eq!(stringify_value(&json!(42)), "42");
        assert_eq!(stringify_value(&json!(1.23)), "1.23");
    }

    #[test]
    fn test_stringify_value_bool() {
        assert_eq!(stringify_value(&json!(true)), "true");
        assert_eq!(stringify_value(&json!(false)), "false");
    }

    #[test]
    fn test_stringify_value_null() {
        assert_eq!(stringify_value(&json!(null)), "null");
    }

    #[test]
    fn test_stringify_value_array() {
        assert_eq!(stringify_value(&json!(["a", "b", "c"])), "a\nb\nc");
    }

    #[test]
    fn test_stringify_value_object() {
        let result = stringify_value(&json!({"key": "value"}));
        assert!(result.contains("key: value"));
    }

    #[test]
    fn test_comma_list() {
        let items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(comma_list(&items), "a, b, c");
    }

    #[test]
    fn test_comma_list_empty() {
        let items: Vec<String> = vec![];
        assert_eq!(comma_list(&items), "");
    }

    #[test]
    fn test_sanitize_for_postgres() {
        assert_eq!(sanitize_for_postgres("Hello\x00world", ""), "Helloworld");
        assert_eq!(sanitize_for_postgres("Hello\x00world", " "), "Hello world");
        assert_eq!(sanitize_for_postgres("No nulls here", ""), "No nulls here");
    }
}
