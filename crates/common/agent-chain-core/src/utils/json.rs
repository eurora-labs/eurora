//! Utilities for JSON parsing.
//!
//! Adapted from langchain_core/utils/json.py

use regex::Regex;
use serde_json::Value;

/// Parse a JSON string that may be missing closing braces.
///
/// This function attempts to parse a JSON string that may be incomplete,
/// such as from a streaming LLM response.
///
/// # Arguments
///
/// * `s` - The JSON string to parse.
/// * `strict` - Whether to use strict parsing (disallow control characters in strings).
///
/// # Returns
///
/// The parsed JSON value, or an error if parsing fails.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::json::parse_partial_json;
///
/// let result = parse_partial_json(r#"{"key": "value"}"#, false);
/// assert!(result.is_ok());
/// ```
pub fn parse_partial_json(s: &str, strict: bool) -> Result<Value, JsonParseError> {
    if let Ok(value) = serde_json::from_str::<Value>(s) {
        if strict && contains_control_chars(s) {
            return Err(JsonParseError::ControlCharacters);
        }
        return Ok(value);
    }

    let mut new_chars = Vec::new();
    let mut stack = Vec::new();
    let mut is_inside_string = false;
    let mut escaped = false;

    for char in s.chars() {
        let mut new_char = char.to_string();

        if is_inside_string {
            if char == '"' && !escaped {
                is_inside_string = false;
            } else if char == '\n' && !escaped {
                new_char = "\\n".to_string();
            } else if char == '\r' && !escaped {
                new_char = "\\r".to_string();
            } else if char == '\t' && !escaped {
                new_char = "\\t".to_string();
            } else if char == '\\' {
                escaped = !escaped;
            } else {
                escaped = false;
            }
        } else if char == '"' {
            is_inside_string = true;
            escaped = false;
        } else if char == '{' {
            stack.push('}');
        } else if char == '[' {
            stack.push(']');
        } else if (char == '}' || char == ']')
            && let Some(expected) = stack.last()
        {
            if *expected == char {
                stack.pop();
            } else {
                return Err(JsonParseError::MismatchedBracket);
            }
        }

        new_chars.push(new_char);
    }

    if is_inside_string {
        if escaped {
            new_chars.pop();
        }
        new_chars.push("\"".to_string());
    }

    stack.reverse();

    while !new_chars.is_empty() {
        let mut attempt = new_chars.join("");
        for closer in &stack {
            attempt.push(*closer);
        }

        match serde_json::from_str::<Value>(&attempt) {
            Ok(value) => {
                if strict && contains_control_chars(&attempt) {
                    return Err(JsonParseError::ControlCharacters);
                }
                return Ok(value);
            }
            Err(_) => {
                new_chars.pop();
            }
        }
    }

    serde_json::from_str(s).map_err(|e| JsonParseError::ParseError(e.to_string()))
}

fn contains_control_chars(s: &str) -> bool {
    s.chars()
        .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
}

/// Parse a JSON string from a Markdown string.
///
/// This function extracts JSON from a Markdown code block if present.
///
/// # Arguments
///
/// * `json_string` - The Markdown string.
///
/// # Returns
///
/// The parsed JSON value, or an error if parsing fails.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::json::parse_json_markdown;
///
/// let result = parse_json_markdown(r#"```json
/// {"key": "value"}
/// ```"#);
/// assert!(result.is_ok());
/// ```
pub fn parse_json_markdown(json_string: &str) -> Result<Value, JsonParseError> {
    if let Ok(value) = parse_json_inner(json_string) {
        return Ok(value);
    }

    let re = Regex::new(r"(?s)```(?:json)?(.*)").expect("Invalid regex");

    let json_str = if let Some(caps) = re.captures(json_string) {
        caps.get(1).map_or(json_string, |m| m.as_str())
    } else {
        json_string
    };

    parse_json_inner(json_str)
}

const JSON_STRIP_CHARS: &[char] = &[' ', '\n', '\r', '\t', '`'];

fn parse_json_inner(json_str: &str) -> Result<Value, JsonParseError> {
    let json_str = json_str.trim_matches(JSON_STRIP_CHARS);

    let json_str = custom_parser(json_str);

    parse_partial_json(&json_str, false)
}

fn custom_parser(multiline_string: &str) -> String {
    let re = Regex::new(r#"(?s)("action_input"\s*:\s*")(.*?)(")"#).expect("Invalid regex");
    re.replace_all(multiline_string, |caps: &regex::Captures| {
        let prefix = caps.get(1).map_or("", |m| m.as_str());
        let value = caps.get(2).map_or("", |m| m.as_str());
        let suffix = caps.get(3).map_or("", |m| m.as_str());

        let value = value.replace('\n', "\\n");
        let value = value.replace('\r', "\\r");
        let value = value.replace('\t', "\\t");
        let value = escape_unescaped_quotes(&value);

        format!("{}{}{}", prefix, value, suffix)
    })
    .to_string()
}

/// Escape double quotes that are not already escaped
fn escape_unescaped_quotes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            result.push(c);
            if chars.peek().is_some() {
                result.push(chars.next().expect("checked peek above"));
            }
        } else if c == '"' {
            result.push('\\');
            result.push('"');
        } else {
            result.push(c);
        }
    }

    result
}

/// Parse a JSON string and check that it contains the expected keys.
///
/// # Arguments
///
/// * `text` - The Markdown string.
/// * `expected_keys` - The expected keys in the JSON object.
///
/// # Returns
///
/// The parsed JSON object, or an error if parsing fails or keys are missing.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::json::parse_and_check_json_markdown;
///
/// let result = parse_and_check_json_markdown(r#"{"key": "value"}"#, &["key"]);
/// assert!(result.is_ok());
/// ```
pub fn parse_and_check_json_markdown(
    text: &str,
    expected_keys: &[&str],
) -> Result<Value, JsonParseError> {
    let json_obj = parse_json_markdown(text)?;

    let obj = json_obj
        .as_object()
        .ok_or_else(|| JsonParseError::NotAnObject(format!("{:?}", json_obj)))?;

    for key in expected_keys {
        if !obj.contains_key(*key) {
            return Err(JsonParseError::MissingKey(key.to_string()));
        }
    }

    Ok(json_obj)
}

/// Error types for JSON parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonParseError {
    /// Failed to parse JSON.
    ParseError(String),
    /// Mismatched bracket in JSON.
    MismatchedBracket,
    /// Control characters found in strict mode.
    ControlCharacters,
    /// Expected an object but got something else.
    NotAnObject(String),
    /// Missing expected key.
    MissingKey(String),
}

impl std::fmt::Display for JsonParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonParseError::ParseError(msg) => write!(f, "Failed to parse JSON: {}", msg),
            JsonParseError::MismatchedBracket => write!(f, "Mismatched bracket in JSON"),
            JsonParseError::ControlCharacters => write!(f, "Control characters found in JSON"),
            JsonParseError::NotAnObject(got) => {
                write!(f, "Expected JSON object (dict), but got: {}", got)
            }
            JsonParseError::MissingKey(key) => {
                write!(f, "Missing expected key: {}", key)
            }
        }
    }
}

impl std::error::Error for JsonParseError {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_partial_json_complete() {
        let result = parse_partial_json(r#"{"key": "value"}"#, false).unwrap();
        assert_eq!(result, json!({"key": "value"}));
    }

    #[test]
    fn test_parse_partial_json_incomplete() {
        let result = parse_partial_json(r#"{"key": "value""#, false).unwrap();
        assert_eq!(result, json!({"key": "value"}));
    }

    #[test]
    fn test_parse_partial_json_array() {
        let result = parse_partial_json(r#"[1, 2, 3"#, false).unwrap();
        assert_eq!(result, json!([1, 2, 3]));
    }

    #[test]
    fn test_parse_json_markdown() {
        let markdown = r#"```json
{"key": "value"}
```"#;
        let result = parse_json_markdown(markdown).unwrap();
        assert_eq!(result, json!({"key": "value"}));
    }

    #[test]
    fn test_parse_json_markdown_no_fence() {
        let result = parse_json_markdown(r#"{"key": "value"}"#).unwrap();
        assert_eq!(result, json!({"key": "value"}));
    }

    #[test]
    fn test_parse_and_check_json_markdown() {
        let result = parse_and_check_json_markdown(r#"{"key": "value"}"#, &["key"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_and_check_json_markdown_missing_key() {
        let result = parse_and_check_json_markdown(r#"{"key": "value"}"#, &["missing"]);
        assert!(matches!(result, Err(JsonParseError::MissingKey(_))));
    }

    #[test]
    fn test_custom_parser() {
        let input = r#"{"action_input": "line1
line2"}"#;
        let result = custom_parser(input);
        assert!(result.contains("\\n"));
    }
}
