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

/// Stringify a JSON object.
///
/// # Arguments
///
/// * `data` - The JSON object to stringify.
///
/// # Returns
///
/// The stringified dictionary.
pub fn stringify_json_dict(data: &serde_json::Map<String, Value>) -> String {
    data.iter()
        .map(|(k, v)| format!("{}: {}\n", k, stringify_value(v)))
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

/// Convert items that implement Display to a comma-separated string.
///
/// # Arguments
///
/// * `items` - The items to convert.
///
/// # Returns
///
/// The comma-separated string.
pub fn comma_list_display<T: std::fmt::Display>(items: &[T]) -> String {
    items
        .iter()
        .map(|item| item.to_string())
        .collect::<Vec<_>>()
        .join(", ")
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

/// Truncate a string to a maximum length.
///
/// # Arguments
///
/// * `text` - The text to truncate.
/// * `max_length` - The maximum length.
/// * `suffix` - The suffix to append if truncated (default: "...").
///
/// # Returns
///
/// The truncated string.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::strings::truncate;
///
/// assert_eq!(truncate("Hello, World!", 10, None), "Hello, ...");
/// assert_eq!(truncate("Hello", 10, None), "Hello");
/// ```
pub fn truncate(text: &str, max_length: usize, suffix: Option<&str>) -> String {
    let suffix = suffix.unwrap_or("...");
    let text_char_count = text.chars().count();
    let suffix_char_count = suffix.chars().count();

    if text_char_count <= max_length {
        text.to_string()
    } else if max_length <= suffix_char_count {
        suffix.chars().take(max_length).collect()
    } else {
        let truncate_at = max_length - suffix_char_count;
        let truncated: String = text.chars().take(truncate_at).collect();
        format!("{}{}", truncated, suffix)
    }
}

/// Remove leading and trailing whitespace from each line.
///
/// # Arguments
///
/// * `text` - The text to process.
///
/// # Returns
///
/// The text with each line trimmed.
pub fn strip_lines(text: &str) -> String {
    text.lines()
        .map(|line| line.trim())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Indent each line of text.
///
/// # Arguments
///
/// * `text` - The text to indent.
/// * `indent` - The indentation string.
///
/// # Returns
///
/// The indented text.
pub fn indent(text: &str, indent_str: &str) -> String {
    text.lines()
        .map(|line| format!("{}{}", indent_str, line))
        .collect::<Vec<_>>()
        .join("\n")
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

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("Hello, World!", 10, None), "Hello, ...");
        assert_eq!(truncate("Hello", 10, None), "Hello");
        assert_eq!(truncate("Hello, World!", 8, Some("…")), "Hello, …");
    }

    #[test]
    fn test_strip_lines() {
        assert_eq!(strip_lines("  a  \n  b  \n  c  "), "a\nb\nc");
    }

    #[test]
    fn test_indent() {
        assert_eq!(indent("a\nb\nc", "  "), "  a\n  b\n  c");
    }
}
