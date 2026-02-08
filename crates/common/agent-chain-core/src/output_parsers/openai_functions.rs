//! Parsers for OpenAI functions output.
//!
//! This module contains parsers for extracting and parsing function call
//! information from OpenAI-style chat model responses.
//! Mirrors `langchain_core.output_parsers.openai_functions`.

use std::fmt::Debug;

use serde::de::DeserializeOwned;
use serde_json::Value;

use super::base::OutputParserError;
use crate::error::{Error, Result};
use crate::outputs::ChatGeneration;

/// Parse an output that is one of sets of values.
///
/// Extracts the raw function call information from the `additional_kwargs`
/// of a `ChatGeneration`'s message.
#[derive(Debug, Clone)]
pub struct OutputFunctionsParser {
    /// Whether to only return the arguments to the function call.
    pub args_only: bool,
}

impl OutputFunctionsParser {
    pub fn new(args_only: bool) -> Self {
        Self { args_only }
    }

    /// Parse the result of an LLM call, extracting raw function call data.
    pub fn parse_result(&self, result: &[ChatGeneration]) -> Result<Value> {
        let generation = result
            .first()
            .ok_or_else(|| Error::Other("No generations to parse".to_string()))?;

        let additional_kwargs = generation
            .message
            .additional_kwargs()
            .ok_or_else(|| OutputParserError::new("Message has no additional_kwargs"))?;

        let function_call = additional_kwargs
            .get("function_call")
            .ok_or_else(|| {
                OutputParserError::new(
                    "Could not parse function call: 'function_call' key not found",
                )
            })?
            .clone();

        if self.args_only {
            let arguments = function_call.get("arguments").ok_or_else(|| {
                OutputParserError::new("Could not parse function call: missing 'arguments'")
            })?;
            Ok(arguments.clone())
        } else {
            Ok(function_call)
        }
    }
}

/// Parse an output as a JSON object from OpenAI function calling.
///
/// Extracts the function call from `additional_kwargs["function_call"]` and
/// parses the `arguments` string as JSON.
#[derive(Debug, Clone)]
pub struct JsonOutputFunctionsParser {
    /// Whether to allow non-JSON-compliant strings.
    ///
    /// When `false` (default), uses lenient parsing that handles unicode characters
    /// and newlines in strings. When `true`, uses strict JSON parsing.
    pub strict: bool,

    /// Whether to only return the parsed arguments to the function call.
    pub args_only: bool,
}

impl Default for JsonOutputFunctionsParser {
    fn default() -> Self {
        Self {
            strict: false,
            args_only: true,
        }
    }
}

impl JsonOutputFunctionsParser {
    pub fn new(args_only: bool) -> Self {
        Self {
            strict: false,
            args_only,
        }
    }

    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    /// Parse the result of an LLM call to a JSON object.
    pub fn parse_result(&self, result: &[ChatGeneration]) -> Result<Value> {
        if result.len() != 1 {
            return Err(OutputParserError::new(format!(
                "Expected exactly one result, but got {}",
                result.len()
            ))
            .into());
        }

        let generation = &result[0];
        let additional_kwargs = generation.message.additional_kwargs().ok_or_else(|| {
            OutputParserError::new("This output parser can only be used with a chat generation.")
        })?;

        let function_call = match additional_kwargs.get("function_call") {
            Some(fc) => fc,
            None => {
                return Err(OutputParserError::new(
                    "Could not parse function call: 'function_call' key not found",
                )
                .into());
            }
        };

        let arguments_value = function_call.get("arguments").ok_or_else(|| {
            OutputParserError::new("Could not parse function call data: missing 'arguments'")
        })?;

        let arguments_str = match arguments_value.as_str() {
            Some(s) => s,
            None => {
                return Err(OutputParserError::new(
                    "Could not parse function call data: 'arguments' is not a string",
                )
                .into());
            }
        };

        let parsed_arguments = if self.strict {
            serde_json::from_str::<Value>(arguments_str).map_err(|e| {
                Error::from(OutputParserError::new(format!(
                    "Could not parse function call data: {}",
                    e
                )))
            })?
        } else {
            parse_json_lenient(arguments_str).map_err(|e| {
                Error::from(OutputParserError::new(format!(
                    "Could not parse function call data: {}",
                    e
                )))
            })?
        };

        if self.args_only {
            Ok(parsed_arguments)
        } else {
            let name = function_call.get("name").cloned().unwrap_or(Value::Null);

            Ok(serde_json::json!({
                "arguments": parsed_arguments,
                "name": name,
            }))
        }
    }
}

/// Internal type-erased parser function for single-schema parsing.
#[derive(Clone)]
pub struct SingleSchemaParser<T>(
    std::sync::Arc<dyn Fn(&[ChatGeneration]) -> Result<T> + Send + Sync>,
);

impl<T> Debug for SingleSchemaParser<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SingleSchemaParser(<fn>)")
    }
}

/// Schema specification for `PydanticOutputFunctionsParser`.
///
/// In Python, `pydantic_schema` can be either a single Pydantic class or a dict
/// mapping function names to classes. In Rust, we use an enum to represent this.
#[derive(Clone)]
pub enum PydanticSchema<T> {
    /// A single schema type. `args_only` will be true.
    Single(SingleSchemaParser<T>),
    /// Multiple schemas keyed by function name.
    /// The caller provides a function that deserializes by name.
    Multiple(std::sync::Arc<dyn Fn(&str, &str) -> Result<T> + Send + Sync>),
}

impl<T> Debug for PydanticSchema<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Single(_) => f.debug_tuple("Single").finish(),
            Self::Multiple(_) => f.debug_tuple("Multiple").field(&"<resolver fn>").finish(),
        }
    }
}

impl<T: Send + Sync + 'static> PydanticSchema<T> {
    /// Create a single-schema variant that deserializes via `serde_json`.
    pub fn single<D: DeserializeOwned + Into<T> + 'static>() -> Self {
        Self::Single(SingleSchemaParser(std::sync::Arc::new(|result| {
            let base_parser = OutputFunctionsParser::new(true);
            let raw = base_parser.parse_result(result)?;
            let json_str = match raw.as_str() {
                Some(s) => s.to_string(),
                None => raw.to_string(),
            };
            let parsed: D = serde_json::from_str(&json_str).map_err(|e| {
                Error::from(OutputParserError::new(format!(
                    "Could not parse function call into schema: {}",
                    e
                )))
            })?;
            Ok(parsed.into())
        })))
    }

    /// Create a multiple-schema variant with a resolver function.
    ///
    /// The resolver takes `(function_name, json_args_string)` and returns
    /// the deserialized value.
    pub fn multiple(resolver: impl Fn(&str, &str) -> Result<T> + Send + Sync + 'static) -> Self {
        Self::Multiple(std::sync::Arc::new(resolver))
    }
}

/// Parse an output as a deserialized struct from OpenAI function calling.
///
/// This is the Rust equivalent of `PydanticOutputFunctionsParser`. Instead of
/// Pydantic models, it uses `serde::Deserialize` to parse the function call
/// arguments into a typed struct.
#[derive(Debug, Clone)]
pub struct PydanticOutputFunctionsParser<T> {
    pub schema: PydanticSchema<T>,
}

impl<T: DeserializeOwned + Send + Sync + Clone + Debug + 'static> PydanticOutputFunctionsParser<T> {
    /// Create a parser for a single schema type (args_only = true).
    pub fn new() -> Self {
        Self {
            schema: PydanticSchema::single::<T>(),
        }
    }
}

impl<T: Send + Sync + Clone + Debug + 'static> PydanticOutputFunctionsParser<T> {
    /// Create a parser for multiple schemas keyed by function name.
    pub fn with_multiple_schemas(
        resolver: impl Fn(&str, &str) -> Result<T> + Send + Sync + 'static,
    ) -> Self {
        Self {
            schema: PydanticSchema::multiple(resolver),
        }
    }

    /// Parse the result of an LLM call into a typed struct.
    ///
    /// For `Single` schemas, `T` must implement `DeserializeOwned` (enforced at
    /// construction via `new()`). For `Multiple` schemas, deserialization is
    /// handled by the user-provided resolver function.
    pub fn parse_result(&self, result: &[ChatGeneration]) -> Result<T> {
        match &self.schema {
            PydanticSchema::Single(parse_fn) => (parse_fn.0)(result),
            PydanticSchema::Multiple(resolver) => {
                let base_parser = OutputFunctionsParser::new(false);
                let raw = base_parser.parse_result(result)?;
                let function_name = raw.get("name").and_then(|v| v.as_str()).ok_or_else(|| {
                    OutputParserError::new("Missing function name in function call")
                })?;
                let arguments = raw
                    .get("arguments")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| OutputParserError::new("Missing arguments in function call"))?;
                resolver(function_name, arguments)
            }
        }
    }
}

/// Parse JSON leniently, handling newlines and special characters inside strings.
///
/// Python's `json.loads` with `strict=False` allows control characters (like
/// literal newlines) inside JSON strings. Rust's `serde_json` is strict by
/// default. This function preprocesses the input to escape unescaped control
/// characters inside JSON string values before parsing.
fn parse_json_lenient(input: &str) -> std::result::Result<Value, String> {
    // First try standard parsing
    if let Ok(value) = serde_json::from_str::<Value>(input) {
        return Ok(value);
    }

    // If that fails, try escaping control characters inside string values
    let mut result = String::with_capacity(input.len());
    let mut in_string = false;
    let mut prev_was_backslash = false;
    let chars: Vec<char> = input.chars().collect();

    for &character in &chars {
        if prev_was_backslash {
            result.push(character);
            prev_was_backslash = false;
            continue;
        }

        if character == '\\' && in_string {
            result.push(character);
            prev_was_backslash = true;
            continue;
        }

        if character == '"' {
            in_string = !in_string;
            result.push(character);
            continue;
        }

        if in_string && character == '\n' {
            result.push_str("\\n");
            continue;
        }

        if in_string && character == '\r' {
            result.push_str("\\r");
            continue;
        }

        if in_string && character == '\t' {
            result.push_str("\\t");
            continue;
        }

        result.push(character);
    }

    serde_json::from_str::<Value>(&result).map_err(|e| format!("JSON parse error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_lenient_basic() {
        let result = parse_json_lenient(r#"{"key": "value"}"#).unwrap();
        assert_eq!(result["key"], "value");
    }

    #[test]
    fn test_parse_json_lenient_with_newlines() {
        let input = "{\"code\": \"print(2+\n2)\"}";
        let result = parse_json_lenient(input).unwrap();
        assert_eq!(result["code"], "print(2+\n2)");
    }

    #[test]
    fn test_parse_json_lenient_unicode() {
        let input = "{\"code\": \"你好)\"}";
        let result = parse_json_lenient(input).unwrap();
        assert_eq!(result["code"], "你好)");
    }

    #[test]
    fn test_parse_json_strict_rejects_newlines() {
        let input = "{\"code\": \"print(2+\n2)\"}";
        let result = serde_json::from_str::<Value>(input);
        assert!(result.is_err());
    }
}
