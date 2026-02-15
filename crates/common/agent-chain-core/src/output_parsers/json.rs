//! Parser for JSON output.
//!
//! This module contains the `JsonOutputParser` which parses LLM output as JSON.
//! Mirrors `langchain_core.output_parsers.json`.

use std::fmt::Debug;

use serde_json::Value;

use crate::error::{Error, Result};
use crate::outputs::Generation;
use crate::utils::json::{parse_json_markdown, parse_partial_json};

use super::base::{BaseOutputParser, OutputParserError};
use super::format_instructions::JSON_FORMAT_INSTRUCTIONS;
use super::transform::BaseCumulativeTransformOutputParser;

/// Parse the output of an LLM call to a JSON object.
///
/// Probably the most reliable output parser for getting structured data that does *not*
/// use function calling.
///
/// When used in streaming mode, it will yield partial JSON objects containing
/// all the keys that have been returned so far.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::output_parsers::JsonOutputParser;
///
/// let parser = JsonOutputParser::new();
/// let result = parser.parse(r#"{"name": "Alice", "age": 30}"#).unwrap();
/// assert_eq!(result["name"], "Alice");
/// ```
#[derive(Debug, Clone, Default)]
pub struct JsonOutputParser {
    /// Optional JSON schema to validate against.
    /// If `None`, no validation is performed.
    pub schema: Option<Value>,

    /// Whether to yield diffs between the previous and current parsed output
    /// in streaming mode.
    pub diff: bool,
}

impl JsonOutputParser {
    /// Create a new `JsonOutputParser` with no schema validation.
    pub fn new() -> Self {
        Self {
            schema: None,
            diff: false,
        }
    }

    /// Create a new `JsonOutputParser` with a schema.
    pub fn with_schema(schema: Value) -> Self {
        Self {
            schema: Some(schema),
            diff: false,
        }
    }

    /// Enable diff mode for streaming.
    pub fn with_diff(mut self) -> Self {
        self.diff = true;
        self
    }

    /// Get the JSON schema if available.
    pub fn get_schema(&self) -> Option<&Value> {
        self.schema.as_ref()
    }
}

impl BaseOutputParser for JsonOutputParser {
    type Output = Value;

    fn parse(&self, text: &str) -> Result<Value> {
        let text = text.trim();

        match parse_json_markdown(text) {
            Ok(value) => Ok(value),
            Err(e) => Err(Error::Other(format!(
                "Invalid json output: {}. Error: {}",
                text, e
            ))),
        }
    }

    fn parse_result(&self, result: &[Generation], partial: bool) -> Result<Value> {
        if result.is_empty() {
            return Err(Error::Other("No generations to parse".to_string()));
        }

        let text = result[0].text.trim();

        if partial {
            match parse_json_markdown(text) {
                Ok(value) => Ok(value),
                Err(_) => {
                    // For partial parsing, try to parse what we have
                    parse_partial_json(text, false)
                        .map_err(|e| Error::Other(format!("Partial parse failed: {}", e)))
                }
            }
        } else {
            match parse_json_markdown(text) {
                Ok(value) => Ok(value),
                Err(e) => Err(OutputParserError::parse_error(
                    format!("Invalid json output: {}", e),
                    text,
                )
                .into()),
            }
        }
    }

    fn get_format_instructions(&self) -> Result<String> {
        match self.get_schema() {
            Some(schema) => {
                // Copy schema to avoid altering original
                let mut schema_copy = schema.clone();

                // Remove extraneous fields
                if let Value::Object(ref mut map) = schema_copy {
                    map.remove("title");
                    map.remove("type");
                }

                let schema_str =
                    serde_json::to_string(&schema_copy).unwrap_or_else(|_| "{}".to_string());

                Ok(JSON_FORMAT_INSTRUCTIONS.replace("{schema}", &schema_str))
            }
            None => Ok("Return a JSON object.".to_string()),
        }
    }

    fn parser_type(&self) -> &str {
        "simple_json_output_parser"
    }
}

impl BaseCumulativeTransformOutputParser for JsonOutputParser {
    fn diff_mode(&self) -> bool {
        self.diff
    }

    fn compute_diff(&self, prev: Option<&Value>, next: Value) -> Value {
        match prev {
            Some(prev_value) => compute_json_diff(prev_value, &next),
            None => Value::Array(vec![serde_json::json!({
                "op": "replace",
                "path": "",
                "value": next,
            })]),
        }
    }
}

/// Compute a JSON diff between two values.
///
/// Returns a JSON patch-like representation of the differences.
fn compute_json_diff(prev: &Value, next: &Value) -> Value {
    let mut patches = Vec::new();
    compute_json_diff_recursive(prev, next, "", &mut patches);
    Value::Array(patches)
}

fn compute_json_diff_recursive(prev: &Value, next: &Value, path: &str, patches: &mut Vec<Value>) {
    if prev == next {
        return;
    }

    match (prev, next) {
        (Value::Object(prev_obj), Value::Object(next_obj)) => {
            for key in prev_obj.keys() {
                if !next_obj.contains_key(key) {
                    patches.push(serde_json::json!({
                        "op": "remove",
                        "path": format!("{}/{}", path, key),
                    }));
                }
            }

            for (key, next_val) in next_obj {
                let child_path = format!("{}/{}", path, key);
                match prev_obj.get(key) {
                    Some(prev_val) if prev_val != next_val => {
                        compute_json_diff_recursive(prev_val, next_val, &child_path, patches);
                    }
                    None => {
                        patches.push(serde_json::json!({
                            "op": "add",
                            "path": child_path,
                            "value": next_val,
                        }));
                    }
                    _ => {}
                }
            }
        }
        (Value::Array(prev_arr), Value::Array(next_arr)) => {
            let common_len = prev_arr.len().min(next_arr.len());
            for i in 0..common_len {
                let child_path = format!("{}/{}", path, i);
                compute_json_diff_recursive(&prev_arr[i], &next_arr[i], &child_path, patches);
            }
            for (i, item) in next_arr.iter().enumerate().skip(common_len) {
                patches.push(serde_json::json!({
                    "op": "add",
                    "path": format!("{}/{}", path, i),
                    "value": item,
                }));
            }
            for i in common_len..prev_arr.len() {
                patches.push(serde_json::json!({
                    "op": "remove",
                    "path": format!("{}/{}", path, i),
                }));
            }
        }
        _ => {
            patches.push(serde_json::json!({
                "op": "replace",
                "path": path,
                "value": next,
            }));
        }
    }
}

/// Type alias for backwards compatibility.
pub type SimpleJsonOutputParser = JsonOutputParser;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_output_parser_simple() {
        let parser = JsonOutputParser::new();
        let result = parser.parse(r#"{"key": "value"}"#).unwrap();
        assert_eq!(result["key"], "value");
    }

    #[test]
    fn test_json_output_parser_markdown() {
        let parser = JsonOutputParser::new();
        let result = parser
            .parse(
                r#"```json
{"key": "value"}
```"#,
            )
            .unwrap();
        assert_eq!(result["key"], "value");
    }

    #[test]
    fn test_json_output_parser_array() {
        let parser = JsonOutputParser::new();
        let result = parser.parse(r#"[1, 2, 3]"#).unwrap();
        assert!(result.is_array());
        assert_eq!(result[0], 1);
    }

    #[test]
    fn test_json_output_parser_nested() {
        let parser = JsonOutputParser::new();
        let result = parser.parse(r#"{"outer": {"inner": "value"}}"#).unwrap();
        assert_eq!(result["outer"]["inner"], "value");
    }

    #[test]
    fn test_json_output_parser_invalid() {
        let parser = JsonOutputParser::new();
        let result = parser.parse("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_json_output_parser_format_instructions_no_schema() {
        let parser = JsonOutputParser::new();
        let instructions = parser
            .get_format_instructions()
            .expect("should return format instructions");
        assert_eq!(instructions, "Return a JSON object.");
    }

    #[test]
    fn test_json_output_parser_format_instructions_with_schema() {
        let schema = serde_json::json!({
            "title": "Person",
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            }
        });
        let parser = JsonOutputParser::with_schema(schema);
        let instructions = parser
            .get_format_instructions()
            .expect("should return format instructions");
        assert!(instructions.contains("properties"));
        assert!(instructions.contains("name"));
    }

    #[test]
    fn test_json_output_parser_partial() {
        let parser = JsonOutputParser::new();
        let generations = vec![Generation::new(r#"{"key": "val"#)];
        let result = parser.parse_result(&generations, true).unwrap();
        assert_eq!(result["key"], "val");
    }

    #[test]
    fn test_json_diff() {
        let prev = serde_json::json!({"a": 1, "b": 2});
        let next = serde_json::json!({"a": 1, "b": 3, "c": 4});
        let diff = compute_json_diff(&prev, &next);

        assert!(diff.is_array());
        let patches = diff.as_array().unwrap();
        assert!(!patches.is_empty());
    }

    #[test]
    fn test_parser_type() {
        let parser = JsonOutputParser::new();
        assert_eq!(parser.parser_type(), "simple_json_output_parser");
    }
}
