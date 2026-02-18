//! Convert functions and runnables to tools.
//!
//! This module provides utilities for converting functions and runnables
//! into tools, mirroring `langchain_core.tools.convert`.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use serde_json::Value;

use crate::error::{Error, Result};
use crate::runnables::Runnable;

use super::base::{ArgsSchema, ResponseFormat};
use super::simple::Tool;
use super::structured::{StructuredTool, create_args_schema};

/// Configuration for creating a tool from a function.
#[derive(Debug, Clone, Default)]
pub struct ToolConfig {
    /// Optional name for the tool. If not provided, uses the function name.
    pub name: Option<String>,
    /// Optional description for the tool.
    pub description: Option<String>,
    /// Whether to return the tool's output directly.
    pub return_direct: bool,
    /// Optional schema for the tool's input arguments.
    pub args_schema: Option<ArgsSchema>,
    /// Whether to infer the schema from the function signature.
    pub infer_schema: bool,
    /// The tool response format.
    pub response_format: ResponseFormat,
    /// Whether to parse the docstring for parameter descriptions.
    pub parse_docstring: bool,
    /// Whether to raise an error on invalid docstring.
    pub error_on_invalid_docstring: bool,
    /// Optional provider-specific extras.
    pub extras: Option<HashMap<String, Value>>,
}

impl ToolConfig {
    /// Create a new ToolConfig with defaults.
    pub fn new() -> Self {
        Self {
            infer_schema: true,
            ..Default::default()
        }
    }

    /// Set the name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set return_direct.
    pub fn with_return_direct(mut self, return_direct: bool) -> Self {
        self.return_direct = return_direct;
        self
    }

    /// Set the args schema.
    pub fn with_args_schema(mut self, schema: ArgsSchema) -> Self {
        self.args_schema = Some(schema);
        self
    }

    /// Set infer_schema.
    pub fn with_infer_schema(mut self, infer_schema: bool) -> Self {
        self.infer_schema = infer_schema;
        self
    }

    /// Set the response format.
    pub fn with_response_format(mut self, format: ResponseFormat) -> Self {
        self.response_format = format;
        self
    }

    /// Set parse_docstring.
    pub fn with_parse_docstring(mut self, parse: bool) -> Self {
        self.parse_docstring = parse;
        self
    }

    /// Set extras.
    pub fn with_extras(mut self, extras: HashMap<String, Value>) -> Self {
        self.extras = Some(extras);
        self
    }
}

/// Create a simple string-to-string tool from a function.
///
/// This is useful for tools that take a single string input and return a string.
pub fn create_simple_tool<F>(
    name: impl Into<String>,
    description: impl Into<String>,
    func: F,
) -> Tool
where
    F: Fn(String) -> Result<String> + Send + Sync + 'static,
{
    Tool::from_function(func, name, description)
}

/// Create a simple tool with async support.
pub fn create_simple_tool_async<F, AF, Fut>(
    name: impl Into<String>,
    description: impl Into<String>,
    func: F,
    coroutine: AF,
) -> Tool
where
    F: Fn(String) -> Result<String> + Send + Sync + 'static,
    AF: Fn(String) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<String>> + Send + 'static,
{
    Tool::from_function_with_async(func, coroutine, name, description)
}

/// Create a structured tool from a function.
///
/// This is useful for tools that take multiple typed arguments.
pub fn create_structured_tool<F>(
    name: impl Into<String>,
    description: impl Into<String>,
    args_schema: ArgsSchema,
    func: F,
) -> StructuredTool
where
    F: Fn(HashMap<String, Value>) -> Result<Value> + Send + Sync + 'static,
{
    StructuredTool::from_function(func, name, description, args_schema)
}

/// Create a structured tool with async support.
pub fn create_structured_tool_async<F, AF, Fut>(
    name: impl Into<String>,
    description: impl Into<String>,
    args_schema: ArgsSchema,
    func: F,
    coroutine: AF,
) -> StructuredTool
where
    F: Fn(HashMap<String, Value>) -> Result<Value> + Send + Sync + 'static,
    AF: Fn(HashMap<String, Value>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Value>> + Send + 'static,
{
    StructuredTool::from_function_with_async(func, coroutine, name, description, args_schema)
}

/// Create a tool using a configuration object.
pub fn create_tool_with_config<F>(func: F, config: ToolConfig) -> Result<StructuredTool>
where
    F: Fn(HashMap<String, Value>) -> Result<Value> + Send + Sync + 'static,
{
    let name = config
        .name
        .ok_or_else(|| Error::InvalidConfig("Tool name is required".to_string()))?;
    let description = config.description.unwrap_or_default();
    let args_schema = config.args_schema.unwrap_or_default();

    let mut tool = StructuredTool::from_function(func, name, description, args_schema);

    if config.return_direct {
        tool = tool.with_return_direct(true);
    }

    tool = tool.with_response_format(config.response_format);

    if let Some(extras) = config.extras {
        tool = tool.with_extras(extras);
    }

    Ok(tool)
}

/// Convert a runnable to a tool.
///
/// This function converts a Runnable into a BaseTool.
pub fn convert_runnable_to_tool<R>(
    runnable: Arc<R>,
    name: impl Into<String>,
    description: impl Into<String>,
) -> StructuredTool
where
    R: Runnable<Input = HashMap<String, Value>, Output = Value> + Send + Sync + 'static,
{
    let name = name.into();
    let description = description.into();

    let runnable_clone = runnable.clone();
    let func = move |args: HashMap<String, Value>| runnable_clone.invoke(args, None);

    let schema = ArgsSchema::JsonSchema(serde_json::json!({
        "type": "object",
        "properties": {},
        "additionalProperties": true
    }));

    StructuredTool::from_function(func, name, description, schema)
}

/// Type alias for the tool function used in tool_from_schema.
pub type ToolFromSchemaFn = Box<dyn Fn(HashMap<String, Value>) -> Result<Value> + Send + Sync>;

/// Helper macro-like function to define a tool with a schema.
///
/// In Rust, we can't use decorators like Python's @tool,
/// but we can provide helper functions that make tool creation easier.
pub fn tool_from_schema(
    name: impl Into<String>,
    description: impl Into<String>,
    properties: Vec<(&str, &str, &str, bool)>, // (name, type, description, required)
) -> impl FnOnce(ToolFromSchemaFn) -> StructuredTool {
    let name = name.into();
    let description = description.into();

    let mut props = HashMap::new();
    let mut required = Vec::new();

    for (prop_name, prop_type, prop_desc, is_required) in properties {
        props.insert(
            prop_name.to_string(),
            serde_json::json!({
                "type": prop_type,
                "description": prop_desc
            }),
        );
        if is_required {
            required.push(prop_name.to_string());
        }
    }

    let schema = create_args_schema(&name, props, required, Some(&description));

    move |func| StructuredTool::from_function(func, name, description, schema)
}

/// Generate a placeholder description for a runnable.
pub fn get_description_from_runnable<R>(_runnable: &R) -> String
where
    R: Runnable,
{
    "Takes an input and produces an output.".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::base::BaseTool;

    #[test]
    fn test_create_simple_tool() {
        let tool = create_simple_tool("echo", "Echoes the input", |input| {
            Ok(format!("Echo: {}", input))
        });

        assert_eq!(tool.name(), "echo");
        assert_eq!(tool.description(), "Echoes the input");
    }

    #[test]
    fn test_create_structured_tool() {
        let schema = create_args_schema(
            "add",
            {
                let mut props = HashMap::new();
                props.insert("a".to_string(), serde_json::json!({"type": "number"}));
                props.insert("b".to_string(), serde_json::json!({"type": "number"}));
                props
            },
            vec!["a".to_string(), "b".to_string()],
            None,
        );

        let tool = create_structured_tool("add", "Adds two numbers", schema, |args| {
            let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
            Ok(Value::from(a + b))
        });

        assert_eq!(tool.name(), "add");
    }

    #[test]
    fn test_tool_config() {
        let config = ToolConfig::new()
            .with_name("test")
            .with_description("A test tool")
            .with_return_direct(true)
            .with_response_format(ResponseFormat::ContentAndArtifact);

        assert_eq!(config.name, Some("test".to_string()));
        assert!(config.return_direct);
        assert_eq!(config.response_format, ResponseFormat::ContentAndArtifact);
    }

    #[test]
    fn test_create_tool_with_config() {
        let config = ToolConfig::new()
            .with_name("configured_tool")
            .with_description("A configured tool")
            .with_args_schema(ArgsSchema::JsonSchema(serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string"}
                }
            })));

        let tool = create_tool_with_config(
            |args| Ok(args.get("input").cloned().unwrap_or(Value::Null)),
            config,
        )
        .unwrap();

        assert_eq!(tool.name(), "configured_tool");
    }

    #[test]
    fn test_tool_from_schema() {
        let create_tool = tool_from_schema(
            "greet",
            "Greets a person",
            vec![("name", "string", "The person's name", true)],
        );

        let tool = create_tool(Box::new(|args| {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("stranger");
            Ok(Value::String(format!("Hello, {}!", name)))
        }));

        assert_eq!(tool.name(), "greet");
    }
}
