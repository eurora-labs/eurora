//! Methods for creating function specs in the style of OpenAI Functions.
//!
//! This module provides utilities for converting various tool representations
//! (JSON schemas, Rust structs, tools) to OpenAI function calling format.
//!
//! Mirrors `langchain_core/utils/function_calling.py`.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::messages::{AIMessage, BaseMessage, HumanMessage, ToolMessage};
use crate::tools::BaseTool;
use crate::utils::json_schema::dereference_refs;
use crate::utils::uuid::uuid7;

/// Representation of a callable function to send to an LLM.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FunctionDescription {
    /// The name of the function.
    pub name: String,
    /// A description of the function.
    pub description: String,
    /// The parameters of the function as a JSON schema.
    pub parameters: Value,
}

/// Representation of a callable function to the OpenAI API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDescription {
    /// The type of the tool (always "function").
    #[serde(rename = "type")]
    pub r#type: String,
    /// The function description.
    pub function: FunctionDescription,
}

/// Well-known OpenAI tools supported by OpenAI's chat models or responses API.
/// These tools are not expected to be supported by other chat model providers
/// that conform to the OpenAI function-calling API.
const WELL_KNOWN_OPENAI_TOOLS: &[&str] = &[
    "function",
    "file_search",
    "computer_use_preview",
    "code_interpreter",
    "mcp",
    "image_generation",
    "web_search_preview",
    "web_search",
];

/// Check if a JSON Value represents a well-known OpenAI tool that should be returned unchanged.
fn is_well_known_openai_tool(tool: &Value) -> bool {
    if let Some(tool_type) = tool.get("type").and_then(|v| v.as_str()) {
        // Check exact match against well-known tools
        if WELL_KNOWN_OPENAI_TOOLS.contains(&tool_type) {
            return true;
        }
        // As of 03.12.25 can be "web_search_preview" or "web_search_preview_2025_03_11"
        if tool_type.starts_with("web_search_preview") {
            return true;
        }
    }
    false
}

/// Recursively sets additionalProperties to false for OpenAI strict mode.
///
/// This function processes a JSON schema and adds `additionalProperties: false`
/// to any object schemas that have `required` fields or empty properties.
fn recursive_set_additional_properties_false(schema: &mut Value) {
    if let Value::Object(map) = schema {
        // Check if 'required' is a key at the current level or if the schema is empty,
        // in which case additionalProperties still needs to be specified.
        let has_required = map.contains_key("required");
        let has_empty_properties = map
            .get("properties")
            .and_then(|p| p.as_object())
            .map(|p| p.is_empty())
            .unwrap_or(false);
        let has_additional_properties = map.contains_key("additionalProperties");

        if has_required || has_empty_properties || has_additional_properties {
            map.insert("additionalProperties".to_string(), Value::Bool(false));
        }

        // Recursively check 'anyOf' if it exists
        if let Some(Value::Array(any_of)) = map.get_mut("anyOf") {
            for sub_schema in any_of {
                recursive_set_additional_properties_false(sub_schema);
            }
        }

        // Recursively check 'properties' if they exist
        if let Some(Value::Object(properties)) = map.get_mut("properties") {
            for sub_schema in properties.values_mut() {
                recursive_set_additional_properties_false(sub_schema);
            }
        }

        // Recursively check 'items' if it exists
        if let Some(items) = map.get_mut("items") {
            recursive_set_additional_properties_false(items);
        }
    }
}

/// Recursively removes "title" fields from a JSON schema dictionary.
///
/// Remove "title" fields from the input JSON schema dictionary,
/// except when a "title" appears within a property definition under "properties".
pub fn remove_titles(schema: &Value) -> Value {
    remove_titles_helper(schema, "")
}

fn remove_titles_helper(kv: &Value, prev_key: &str) -> Value {
    match kv {
        Value::Object(map) => {
            let mut new_map = Map::new();
            for (k, v) in map {
                if k == "title" {
                    if v.is_object() && prev_key == "properties" {
                        new_map.insert(k.clone(), remove_titles_helper(v, k));
                    }
                } else {
                    new_map.insert(k.clone(), remove_titles_helper(v, k));
                }
            }
            Value::Object(new_map)
        }
        Value::Array(arr) => Value::Array(
            arr.iter()
                .map(|item| remove_titles_helper(item, prev_key))
                .collect(),
        ),
        _ => kv.clone(),
    }
}

/// Convert a JSON schema to an OpenAI function description.
///
/// # Arguments
///
/// * `schema` - The JSON schema to convert.
/// * `name` - Optional name for the function. If not provided, uses the schema's title.
/// * `description` - Optional description. If not provided, uses the schema's description.
/// * `rm_titles` - Whether to remove titles from the schema (default: true).
///
/// # Returns
///
/// A `FunctionDescription` compatible with OpenAI function calling.
fn convert_json_schema_to_openai_function(
    schema: &Value,
    name: Option<&str>,
    description: Option<&str>,
    rm_titles: bool,
) -> FunctionDescription {
    // Dereference refs first
    let mut schema = dereference_refs(schema, None, None);

    // Remove definitions/defs if present
    if let Value::Object(ref mut map) = schema {
        map.remove("definitions"); // pydantic 1
        map.remove("$defs"); // pydantic 2
    }

    // Extract title and description from schema
    let title = schema
        .as_object()
        .and_then(|m| m.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let default_description = schema
        .as_object()
        .and_then(|m| m.get("description"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Remove title and description from schema for parameters
    if let Value::Object(ref mut map) = schema {
        map.remove("title");
        map.remove("description");
    }

    // Apply rm_titles if needed
    let parameters = if rm_titles {
        remove_titles(&schema)
    } else {
        schema
    };

    FunctionDescription {
        name: name.map(|s| s.to_string()).unwrap_or(title),
        description: description
            .map(|s| s.to_string())
            .unwrap_or(default_description),
        parameters,
    }
}

/// Trait for types that can be converted to OpenAI function format.
pub trait ToOpenAIFunction {
    /// Convert to OpenAI function format.
    fn to_openai_function(&self, strict: Option<bool>) -> Value;
}

impl ToOpenAIFunction for Value {
    fn to_openai_function(&self, strict: Option<bool>) -> Value {
        convert_to_openai_function(self, strict)
    }
}

impl<T: BaseTool> ToOpenAIFunction for T {
    fn to_openai_function(&self, strict: Option<bool>) -> Value {
        convert_to_openai_function(self, strict)
    }
}

/// Convert a raw function/class to an OpenAI function.
///
/// This function handles various input formats:
/// - Anthropic format tools (with `name` and `input_schema`)
/// - Amazon Bedrock Converse format tools (with `toolSpec`)
/// - OpenAI format (already has `name`)
/// - JSON schema (with `title`)
/// - BaseTool implementations
///
/// # Arguments
///
/// * `function` - The function/tool specification to convert.
/// * `strict` - If `Some(true)`, model output is guaranteed to exactly match the JSON Schema.
///
/// # Returns
///
/// A JSON Value compatible with OpenAI function-calling API.
pub fn convert_to_openai_function<T>(function: &T, strict: Option<bool>) -> Value
where
    T: ConvertibleToOpenAI + ?Sized,
{
    function.convert_to_openai_function_impl(strict)
}

/// Trait for types that can be converted to OpenAI function format.
pub trait ConvertibleToOpenAI {
    /// Internal conversion implementation.
    fn convert_to_openai_function_impl(&self, strict: Option<bool>) -> Value;
}

impl ConvertibleToOpenAI for Value {
    fn convert_to_openai_function_impl(&self, strict: Option<bool>) -> Value {
        let oai_function: Value;

        // Check for Anthropic format tool (has 'name' and 'input_schema')
        if self.is_object() && self.get("name").is_some() && self.get("input_schema").is_some() {
            let mut result = Map::new();
            result.insert(
                "name".to_string(),
                self.get("name").cloned().unwrap_or(Value::Null),
            );
            result.insert(
                "parameters".to_string(),
                self.get("input_schema").cloned().unwrap_or(Value::Null),
            );
            if let Some(desc) = self.get("description") {
                result.insert("description".to_string(), desc.clone());
            }
            oai_function = Value::Object(result);
        }
        // Check for Amazon Bedrock Converse format tool (has 'toolSpec')
        else if self.is_object()
            && let Some(tool_spec) = self.get("toolSpec")
        {
            let mut result = Map::new();
            result.insert(
                "name".to_string(),
                tool_spec.get("name").cloned().unwrap_or(Value::Null),
            );
            result.insert(
                "parameters".to_string(),
                tool_spec
                    .get("inputSchema")
                    .and_then(|is| is.get("json"))
                    .cloned()
                    .unwrap_or(Value::Null),
            );
            if let Some(desc) = tool_spec.get("description") {
                result.insert("description".to_string(), desc.clone());
            }
            oai_function = Value::Object(result);
        }
        // Already in OpenAI function format (has 'name')
        else if self.is_object() && self.get("name").is_some() {
            let mut result = Map::new();
            if let Some(obj) = self.as_object() {
                for (k, v) in obj {
                    if k == "name" || k == "description" || k == "parameters" || k == "strict" {
                        result.insert(k.clone(), v.clone());
                    }
                }
            }
            oai_function = Value::Object(result);
        }
        // JSON schema with title and description
        else if self.is_object() && self.get("title").is_some() {
            let mut function_copy = self.clone();
            let mut result = Map::new();

            // Extract title as name
            if let Value::Object(ref mut map) = function_copy {
                if let Some(title) = map.remove("title") {
                    result.insert("name".to_string(), title);
                }
                if let Some(description) = map.remove("description") {
                    result.insert("description".to_string(), description);
                }
                // If there are properties left, use as parameters
                if !map.is_empty() && map.contains_key("properties") {
                    result.insert("parameters".to_string(), function_copy);
                }
            }
            oai_function = Value::Object(result);
        }
        // Unsupported format
        else {
            oai_function = serde_json::json!({
                "name": "unknown",
                "description": "",
                "parameters": {}
            });
        }

        // Handle strict mode
        let mut oai_function = oai_function;
        if let Some(strict_val) = strict {
            // Check for conflict with existing strict value
            if let Value::Object(ref existing) = oai_function
                && let Some(existing_strict) = existing.get("strict")
                && existing_strict.as_bool() != Some(strict_val)
            {
                panic!(
                    "Tool/function already has a 'strict' key with value {} which is different from the explicit strict arg {}",
                    existing_strict, strict_val
                );
            }

            // Add strict field to the result
            if let Value::Object(ref mut map) = oai_function {
                map.insert("strict".to_string(), Value::Bool(strict_val));

                // If strict is true, apply additional properties and required handling
                if strict_val && let Some(Value::Object(params)) = map.get_mut("parameters") {
                    let mut params_value = Value::Object(params.clone());
                    recursive_set_additional_properties_false(&mut params_value);
                    *params = params_value.as_object().cloned().unwrap_or_default();

                    // All fields must be required
                    if let Some(properties) = params.get("properties").cloned()
                        && let Some(props_obj) = properties.as_object()
                        && !props_obj.is_empty()
                    {
                        let required: Vec<Value> =
                            props_obj.keys().map(|k| Value::String(k.clone())).collect();
                        params.insert("required".to_string(), Value::Array(required));
                    }
                }
            }
        }

        oai_function
    }
}

impl<T: BaseTool + ?Sized> ConvertibleToOpenAI for T {
    fn convert_to_openai_function_impl(&self, strict: Option<bool>) -> Value {
        // Get the tool's args schema
        let args_schema = self.args_schema();

        // Check if this is a simple tool without args_schema
        let is_simple_tool = args_schema.is_none();

        if !is_simple_tool && let Some(schema) = args_schema {
            let json_schema = schema.to_json_schema();
            let func_desc = convert_json_schema_to_openai_function(
                &json_schema,
                Some(self.name()),
                Some(self.description()),
                true,
            );

            let mut result = serde_json::json!({
                "name": func_desc.name,
                "description": func_desc.description,
                "parameters": func_desc.parameters
            });

            // Handle strict mode
            if let Some(strict_val) = strict
                && let Value::Object(ref mut map) = result
            {
                map.insert("strict".to_string(), Value::Bool(strict_val));

                if strict_val && let Some(Value::Object(params)) = map.get_mut("parameters") {
                    let mut params_value = Value::Object(params.clone());
                    recursive_set_additional_properties_false(&mut params_value);
                    *params = params_value.as_object().cloned().unwrap_or_default();

                    // All fields must be required
                    if let Some(properties) = params.get("properties").cloned()
                        && let Some(props_obj) = properties.as_object()
                        && !props_obj.is_empty()
                    {
                        let required: Vec<Value> =
                            props_obj.keys().map(|k| Value::String(k.clone())).collect();
                        params.insert("required".to_string(), Value::Array(required));
                    }
                }
            }

            return result;
        }

        // For simple tools without args_schema, return a default schema
        let mut result = serde_json::json!({
            "name": self.name(),
            "description": self.description(),
            "parameters": {
                "properties": {
                    "__arg1": {"title": "__arg1", "type": "string"}
                },
                "required": ["__arg1"],
                "type": "object"
            }
        });

        // Handle strict mode
        if let Some(strict_val) = strict
            && let Value::Object(ref mut map) = result
        {
            map.insert("strict".to_string(), Value::Bool(strict_val));

            if strict_val && let Some(Value::Object(params)) = map.get_mut("parameters") {
                params.insert("additionalProperties".to_string(), Value::Bool(false));
            }
        }

        result
    }
}

/// Convert a TypedDict-like schema to OpenAI function format.
///
/// In Rust, TypedDict is typically represented as a JSON schema.
/// This function converts such schemas to the OpenAI function format.
///
/// # Arguments
///
/// * `schema` - The JSON schema representing the typed dict.
///
/// # Returns
///
/// A JSON Value in OpenAI function format.
pub fn convert_typed_dict_to_openai_function(schema: &Value) -> Value {
    convert_to_openai_function(schema, None)
}

/// Convert a tool-like object to an OpenAI tool schema.
///
/// [OpenAI tool schema reference](https://platform.openai.com/docs/api-reference/chat/create#chat-create-tools)
///
/// # Arguments
///
/// * `tool` - The tool specification to convert.
/// * `strict` - If `Some(true)`, model output is guaranteed to exactly match the JSON Schema.
///
/// # Returns
///
/// A JSON Value compatible with OpenAI tool-calling API.
/// Trait for types that can be converted to OpenAI tool format.
pub trait ConvertibleToOpenAITool {
    /// Internal conversion implementation.
    fn convert_to_openai_tool_impl(&self, strict: Option<bool>) -> Value;
}

impl ConvertibleToOpenAITool for Value {
    fn convert_to_openai_tool_impl(&self, strict: Option<bool>) -> Value {
        // Check if this is a well-known OpenAI tool that should be returned unchanged
        if self.is_object() && is_well_known_openai_tool(self) {
            return self.clone();
        }

        let oai_function = convert_to_openai_function(self, strict);
        serde_json::json!({
            "type": "function",
            "function": oai_function
        })
    }
}

impl<T: BaseTool + ?Sized> ConvertibleToOpenAITool for T {
    fn convert_to_openai_tool_impl(&self, strict: Option<bool>) -> Value {
        let oai_function = convert_to_openai_function(self, strict);
        serde_json::json!({
            "type": "function",
            "function": oai_function
        })
    }
}

pub fn convert_to_openai_tool<T>(tool: &T, strict: Option<bool>) -> Value
where
    T: ConvertibleToOpenAITool + ?Sized,
{
    tool.convert_to_openai_tool_impl(strict)
}

/// Convert a schema representation to a JSON schema.
///
/// # Arguments
///
/// * `schema` - The schema to convert.
/// * `strict` - If `Some(true)`, model output is guaranteed to exactly match the JSON Schema.
///
/// # Returns
///
/// A JSON schema representation of the input schema.
pub fn convert_to_json_schema<T>(schema: &T, strict: Option<bool>) -> crate::Result<Value>
where
    T: ConvertibleToOpenAITool + ?Sized,
{
    let openai_tool = convert_to_openai_tool(schema, strict);

    // Validate and extract function
    let function = openai_tool.get("function").ok_or_else(|| {
        crate::Error::InvalidConfig("Input must be a valid OpenAI-format tool".to_string())
    })?;

    let name = function
        .get("name")
        .ok_or_else(|| {
            crate::Error::InvalidConfig(
                "Input must be a valid OpenAI-format tool with name".to_string(),
            )
        })?
        .as_str()
        .ok_or_else(|| crate::Error::InvalidConfig("Tool name must be a string".to_string()))?;

    let mut json_schema = Map::new();
    json_schema.insert("title".to_string(), Value::String(name.to_string()));

    if let Some(description) = function.get("description") {
        json_schema.insert("description".to_string(), description.clone());
    }

    if let Some(parameters) = function.get("parameters")
        && let Value::Object(params) = parameters
    {
        for (k, v) in params {
            json_schema.insert(k.clone(), v.clone());
        }
    }

    Ok(Value::Object(json_schema))
}

/// Convert an example into a list of messages that can be fed into an LLM.
///
/// This code is an adapter that converts a single example to a list of messages
/// that can be fed into a chat model.
///
/// The list of messages per example by default corresponds to:
///
/// 1. `HumanMessage`: contains the content from which content should be extracted.
/// 2. `AIMessage`: contains the extracted information from the model
/// 3. `ToolMessage`: contains confirmation to the model that the model requested a
///    tool correctly.
///
/// If `ai_response` is specified, there will be a final `AIMessage` with that
/// response.
///
/// # Arguments
///
/// * `input` - The user input.
/// * `tool_calls` - Tool calls represented as serializable objects.
/// * `tool_outputs` - Optional tool call outputs. If not provided, a placeholder value will be used.
/// * `ai_response` - If provided, content for a final `AIMessage`.
///
/// # Returns
///
/// A list of messages.
pub fn tool_example_to_messages<T: Serialize>(
    input: &str,
    tool_calls: Vec<T>,
    tool_outputs: Option<Vec<String>>,
    ai_response: Option<String>,
) -> Vec<BaseMessage> {
    let mut messages: Vec<BaseMessage> =
        vec![HumanMessage::builder().content(input).build().into()];

    // Build OpenAI-style tool calls
    let openai_tool_calls: Vec<Value> = tool_calls
        .iter()
        .map(|tc| {
            let type_name = std::any::type_name::<T>()
                .split("::")
                .last()
                .unwrap_or("Unknown");

            let arguments = serde_json::to_string(tc).unwrap_or_default();

            serde_json::json!({
                "id": uuid7(None).to_string(),
                "type": "function",
                "function": {
                    "name": type_name,
                    "arguments": arguments
                }
            })
        })
        .collect();

    // Create AI message with tool calls
    let mut additional_kwargs = std::collections::HashMap::new();
    additional_kwargs.insert(
        "tool_calls".to_string(),
        Value::Array(openai_tool_calls.clone()),
    );

    let ai_msg = AIMessage::builder()
        .content("")
        .additional_kwargs(additional_kwargs)
        .build();
    messages.push(ai_msg.into());

    // Add tool messages
    let outputs = tool_outputs.unwrap_or_else(|| {
        vec!["You have correctly called this tool.".to_string(); openai_tool_calls.len()]
    });

    for (output, tool_call_dict) in outputs.iter().zip(openai_tool_calls.iter()) {
        let tool_call_id = tool_call_dict
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        messages.push(
            ToolMessage::builder()
                .content(output.clone())
                .tool_call_id(tool_call_id)
                .build()
                .into(),
        );
    }

    // Add final AI response if provided
    if let Some(response) = ai_response {
        messages.push(AIMessage::builder().content(response).build().into());
    }

    messages
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_function_description() {
        let desc = FunctionDescription {
            name: "test".to_string(),
            description: "A test function".to_string(),
            parameters: json!({"type": "object"}),
        };

        assert_eq!(desc.name, "test");
        assert_eq!(desc.description, "A test function");
    }

    #[test]
    fn test_tool_description() {
        let tool_desc = ToolDescription {
            r#type: "function".to_string(),
            function: FunctionDescription {
                name: "test".to_string(),
                description: "A test".to_string(),
                parameters: json!({}),
            },
        };

        assert_eq!(tool_desc.r#type, "function");
    }

    #[test]
    fn test_convert_json_schema() {
        let schema = json!({
            "title": "TestFunction",
            "description": "A test function.",
            "type": "object",
            "properties": {
                "arg1": {"type": "string"}
            },
            "required": ["arg1"]
        });

        let result = convert_to_openai_function(&schema, None);

        assert_eq!(result.get("name").unwrap(), "TestFunction");
        assert_eq!(result.get("description").unwrap(), "A test function.");
    }

    #[test]
    fn test_convert_anthropic_format() {
        let anthropic_tool = json!({
            "name": "my_tool",
            "description": "My tool description",
            "input_schema": {
                "type": "object",
                "properties": {
                    "query": {"type": "string"}
                }
            }
        });

        let result = convert_to_openai_function(&anthropic_tool, None);

        assert_eq!(result.get("name").unwrap(), "my_tool");
        assert_eq!(result.get("description").unwrap(), "My tool description");
        assert!(result.get("parameters").is_some());
    }

    #[test]
    fn test_strict_mode() {
        let schema = json!({
            "title": "TestFunction",
            "description": "Test",
            "type": "object",
            "properties": {
                "arg1": {"type": "string"}
            },
            "required": ["arg1"]
        });

        let result = convert_to_openai_function(&schema, Some(true));

        assert_eq!(result.get("strict").unwrap(), true);
    }

    #[test]
    fn test_recursive_additional_properties() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "nested": {
                    "type": "object",
                    "properties": {
                        "value": {"type": "string"}
                    },
                    "required": ["value"]
                }
            },
            "required": ["nested"]
        });

        recursive_set_additional_properties_false(&mut schema);

        assert_eq!(
            schema.get("additionalProperties").unwrap(),
            &Value::Bool(false)
        );
        assert_eq!(
            schema
                .get("properties")
                .unwrap()
                .get("nested")
                .unwrap()
                .get("additionalProperties")
                .unwrap(),
            &Value::Bool(false)
        );
    }
}
