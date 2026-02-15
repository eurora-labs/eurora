//! Parsers for OpenAI functions output.
//!
//! This module contains parsers for extracting and parsing function call
//! information from OpenAI-style chat model responses.
//! Mirrors `langchain_core.output_parsers.openai_functions`.

use std::fmt::Debug;

use serde::de::DeserializeOwned;
use serde_json::Value;

use async_trait::async_trait;

use super::base::{
    BaseGenerationOutputParser, BaseLLMOutputParser, BaseOutputParser, OutputParserError,
};
use super::transform::{BaseCumulativeTransformOutputParser, BaseTransformOutputParser};
use crate::error::{Error, Result};
use crate::messages::BaseMessage;
use crate::outputs::ChatGeneration;
use crate::outputs::Generation;
use crate::runnables::RunnableConfig;
use crate::utils::json::parse_partial_json;

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

    /// Return the output parser type for serialization.
    pub fn parser_type(&self) -> &str {
        "json_functions"
    }

    /// Parse the output of an LLM call to a JSON object.
    ///
    /// This method is not implemented because `JsonOutputFunctionsParser` works
    /// directly with `ChatGeneration` results, not raw text strings.
    pub fn parse(&self, _text: &str) -> Result<Value> {
        Err(Error::NotImplemented(
            "JsonOutputFunctionsParser.parse is not implemented".to_string(),
        ))
    }

    /// Compute a JSON patch diff between two values.
    ///
    /// Returns a list of JSON patch operations that transform `prev` into `next`.
    /// This mirrors the Python `_diff` method which uses `jsonpatch.make_patch`.
    pub fn diff(&self, prev: &Value, next: &Value) -> Vec<Value> {
        let mut ops = Vec::new();
        compute_json_diff("", prev, next, &mut ops);
        ops
    }

    /// Parse the result of an LLM call to a JSON object.
    ///
    /// When `partial` is true, attempts to parse partial JSON and returns
    /// `Ok(None)` instead of erroring on missing or invalid data.
    pub fn parse_result_with_partial(
        &self,
        result: &[ChatGeneration],
        partial: bool,
    ) -> Result<Option<Value>> {
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
                if partial {
                    return Ok(None);
                }
                return Err(OutputParserError::new(
                    "Could not parse function call: 'function_call' key not found",
                )
                .into());
            }
        };

        let arguments_value = match function_call.get("arguments") {
            Some(v) => v,
            None => return Ok(None),
        };

        let arguments_str = match arguments_value.as_str() {
            Some(s) => s,
            None => {
                if partial {
                    return Ok(None);
                }
                return Err(OutputParserError::new(
                    "Could not parse function call data: 'arguments' is not a string",
                )
                .into());
            }
        };

        if partial {
            let parsed = parse_partial_json(arguments_str, self.strict);
            match parsed {
                Ok(parsed_arguments) => {
                    if self.args_only {
                        Ok(Some(parsed_arguments))
                    } else {
                        let name = function_call.get("name").cloned().unwrap_or(Value::Null);
                        Ok(Some(serde_json::json!({
                            "arguments": parsed_arguments,
                            "name": name,
                        })))
                    }
                }
                Err(_) => Ok(None),
            }
        } else {
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
                Ok(Some(parsed_arguments))
            } else {
                let name = function_call.get("name").cloned().unwrap_or(Value::Null);
                Ok(Some(serde_json::json!({
                    "arguments": parsed_arguments,
                    "name": name,
                })))
            }
        }
    }

    /// Parse the result of an LLM call to a JSON object (non-partial).
    pub fn parse_result(&self, result: &[ChatGeneration]) -> Result<Option<Value>> {
        self.parse_result_with_partial(result, false)
    }
}

/// Internal type-erased parser function for single-schema parsing.
#[derive(Clone)]
pub struct SingleSchemaParser<T>(
    #[allow(clippy::type_complexity)]
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
    Multiple(
        #[allow(clippy::type_complexity)]
        std::sync::Arc<dyn Fn(&str, &str) -> Result<T> + Send + Sync>,
    ),
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

impl<T: DeserializeOwned + Send + Sync + Clone + Debug + 'static> Default
    for PydanticOutputFunctionsParser<T>
{
    fn default() -> Self {
        Self {
            schema: PydanticSchema::single::<T>(),
        }
    }
}

impl<T: DeserializeOwned + Send + Sync + Clone + Debug + 'static> PydanticOutputFunctionsParser<T> {
    /// Create a parser for a single schema type (args_only = true).
    pub fn new() -> Self {
        Self::default()
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

/// Parse an output as a specific key of the JSON object from OpenAI function calling.
///
/// Extracts a specific key from the parsed JSON arguments.
/// Mirrors `langchain_core.output_parsers.openai_functions.JsonKeyOutputFunctionsParser`.
#[derive(Debug, Clone)]
pub struct JsonKeyOutputFunctionsParser {
    /// The name of the key to extract from the parsed JSON.
    pub key_name: String,

    /// The underlying JSON parser.
    inner: JsonOutputFunctionsParser,
}

impl JsonKeyOutputFunctionsParser {
    pub fn new(key_name: impl Into<String>) -> Self {
        Self {
            key_name: key_name.into(),
            inner: JsonOutputFunctionsParser::default(),
        }
    }

    pub fn with_strict(mut self, strict: bool) -> Self {
        self.inner.strict = strict;
        self
    }

    /// Parse the result, extracting the value at `key_name` from the JSON.
    ///
    /// When `partial` is true, returns `Ok(None)` if the key is not present
    /// instead of erroring.
    pub fn parse_result_with_partial(
        &self,
        result: &[ChatGeneration],
        partial: bool,
    ) -> Result<Option<Value>> {
        let res = self.inner.parse_result_with_partial(result, partial)?;
        match res {
            None => Ok(None),
            Some(value) => {
                if partial {
                    Ok(value.get(&self.key_name).cloned())
                } else {
                    value
                        .get(&self.key_name)
                        .cloned()
                        .ok_or_else(|| {
                            Error::Other(format!(
                                "Key '{}' not found in parsed output",
                                self.key_name
                            ))
                        })
                        .map(Some)
                }
            }
        }
    }

    /// Parse the result (non-partial).
    pub fn parse_result(&self, result: &[ChatGeneration]) -> Result<Option<Value>> {
        self.parse_result_with_partial(result, false)
    }
}

/// Parse an output as a Pydantic object and extract a specific attribute.
///
/// Mirrors `langchain_core.output_parsers.openai_functions.PydanticAttrOutputFunctionsParser`.
#[derive(Debug, Clone)]
pub struct PydanticAttrOutputFunctionsParser<T> {
    /// The underlying pydantic parser.
    inner: PydanticOutputFunctionsParser<T>,
    /// The attribute name to extract from the parsed struct.
    pub attr_name: String,
}

impl<T: DeserializeOwned + Send + Sync + Clone + Debug + 'static>
    PydanticAttrOutputFunctionsParser<T>
{
    /// Create a new parser that extracts a specific attribute from the parsed struct.
    pub fn new(attr_name: impl Into<String>) -> Self {
        Self {
            inner: PydanticOutputFunctionsParser::new(),
            attr_name: attr_name.into(),
        }
    }

    /// Parse the result and extract the named attribute as a `Value`.
    ///
    /// First parses the result into the typed struct `T` using serde, then
    /// serializes it back to a `Value` and extracts the named field.
    pub fn parse_result(&self, result: &[ChatGeneration]) -> Result<Value>
    where
        T: serde::Serialize,
    {
        let parsed = self.inner.parse_result(result)?;
        let as_value = serde_json::to_value(&parsed)
            .map_err(|e| Error::Other(format!("Failed to serialize parsed result: {}", e)))?;
        as_value.get(&self.attr_name).cloned().ok_or_else(|| {
            Error::Other(format!(
                "Attribute '{}' not found on parsed object",
                self.attr_name
            ))
        })
    }
}

/// Compute a JSON patch-like diff between two values.
///
/// Produces a list of operations (add, remove, replace) that transform `prev` into `next`.
fn compute_json_diff(path: &str, prev: &Value, next: &Value, ops: &mut Vec<Value>) {
    if prev == next {
        return;
    }

    match (prev, next) {
        (Value::Object(prev_map), Value::Object(next_map)) => {
            for (key, next_val) in next_map {
                let child_path = if path.is_empty() {
                    format!("/{}", key)
                } else {
                    format!("{}/{}", path, key)
                };

                match prev_map.get(key) {
                    Some(prev_val) => {
                        compute_json_diff(&child_path, prev_val, next_val, ops);
                    }
                    None => {
                        ops.push(serde_json::json!({
                            "op": "add",
                            "path": child_path,
                            "value": next_val,
                        }));
                    }
                }
            }

            for key in prev_map.keys() {
                if !next_map.contains_key(key) {
                    let child_path = if path.is_empty() {
                        format!("/{}", key)
                    } else {
                        format!("{}/{}", path, key)
                    };
                    ops.push(serde_json::json!({
                        "op": "remove",
                        "path": child_path,
                    }));
                }
            }
        }
        (Value::Array(prev_arr), Value::Array(next_arr)) => {
            let min_len = prev_arr.len().min(next_arr.len());
            for i in 0..min_len {
                let child_path = format!("{}/{}", path, i);
                compute_json_diff(&child_path, &prev_arr[i], &next_arr[i], ops);
            }

            for (i, item) in next_arr.iter().enumerate().skip(min_len) {
                let child_path = format!("{}/{}", path, i);
                ops.push(serde_json::json!({
                    "op": "add",
                    "path": child_path,
                    "value": item,
                }));
            }

            for i in (min_len..prev_arr.len()).rev() {
                let child_path = format!("{}/{}", path, i);
                ops.push(serde_json::json!({
                    "op": "remove",
                    "path": child_path,
                }));
            }
        }
        _ => {
            let op_path = if path.is_empty() { "/" } else { path };
            ops.push(serde_json::json!({
                "op": "replace",
                "path": op_path,
                "value": next,
            }));
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

// --- Trait implementations to integrate with the base parser architecture ---

#[async_trait]
impl BaseLLMOutputParser for OutputFunctionsParser {
    type Output = Value;

    fn parse_result(&self, _result: &[Generation], _partial: bool) -> Result<Self::Output> {
        Err(Error::OutputParser {
            message: "This output parser can only be used with a chat generation.".to_string(),
            observation: None,
            llm_output: None,
            send_to_llm: false,
        })
    }
}

#[async_trait]
impl BaseGenerationOutputParser for OutputFunctionsParser {
    fn invoke(&self, input: BaseMessage, _config: Option<RunnableConfig>) -> Result<Self::Output> {
        let chat_gen = ChatGeneration::new(input);
        self.parse_result(&[chat_gen])
    }
}

#[async_trait]
impl BaseOutputParser for JsonOutputFunctionsParser {
    type Output = Option<Value>;

    fn parse(&self, _text: &str) -> Result<Self::Output> {
        Err(Error::NotImplemented(
            "JsonOutputFunctionsParser.parse is not implemented".to_string(),
        ))
    }

    fn parse_result(&self, _result: &[Generation], _partial: bool) -> Result<Self::Output> {
        Err(Error::OutputParser {
            message: "This output parser can only be used with a chat generation.".to_string(),
            observation: None,
            llm_output: None,
            send_to_llm: false,
        })
    }

    fn parser_type(&self) -> &str {
        "json_functions"
    }
}

impl BaseTransformOutputParser for JsonOutputFunctionsParser {}

#[async_trait]
impl BaseCumulativeTransformOutputParser for JsonOutputFunctionsParser {
    fn diff_mode(&self) -> bool {
        false
    }

    fn compute_diff(
        &self,
        prev: Option<&Self::Output>,
        next: Self::Output,
    ) -> Result<Self::Output> {
        let prev_val = prev
            .and_then(|p| p.as_ref())
            .cloned()
            .unwrap_or(Value::Null);
        let next_val = next.unwrap_or(Value::Null);
        let patch = self.diff(&prev_val, &next_val);
        Ok(Some(Value::Array(patch)))
    }
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
