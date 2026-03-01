use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use serde_json::Value;

use crate::error::{Error, Result};
use crate::runnables::Runnable;

use super::base::{ArgsSchema, ResponseFormat};
use super::simple::Tool;
use super::structured::{StructuredTool, create_args_schema};

#[derive(Debug, Clone, Default)]
pub struct ToolConfig {
    pub name: Option<String>,
    pub description: Option<String>,
    pub return_direct: bool,
    pub args_schema: Option<ArgsSchema>,
    pub infer_schema: bool,
    pub response_format: ResponseFormat,
    pub parse_docstring: bool,
    pub error_on_invalid_docstring: bool,
    pub extras: Option<HashMap<String, Value>>,
}

#[bon::bon]
impl ToolConfig {
    #[builder]
    pub fn new(
        #[builder(into)] name: Option<String>,
        #[builder(into)] description: Option<String>,
        #[builder(default)] return_direct: bool,
        args_schema: Option<ArgsSchema>,
        #[builder(default = true)] infer_schema: bool,
        #[builder(default)] response_format: ResponseFormat,
        #[builder(default)] parse_docstring: bool,
        #[builder(default)] error_on_invalid_docstring: bool,
        extras: Option<HashMap<String, Value>>,
    ) -> Self {
        Self {
            name,
            description,
            return_direct,
            args_schema,
            infer_schema,
            response_format,
            parse_docstring,
            error_on_invalid_docstring,
            extras,
        }
    }
}

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

pub type ToolFromSchemaFn = Box<dyn Fn(HashMap<String, Value>) -> Result<Value> + Send + Sync>;

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
        let config = ToolConfig::builder()
            .name("test")
            .description("A test tool")
            .return_direct(true)
            .response_format(ResponseFormat::ContentAndArtifact)
            .build();

        assert_eq!(config.name, Some("test".to_string()));
        assert!(config.return_direct);
        assert_eq!(config.response_format, ResponseFormat::ContentAndArtifact);
    }

    #[test]
    fn test_create_tool_with_config() {
        let config = ToolConfig::builder()
            .name("configured_tool")
            .description("A configured tool")
            .args_schema(ArgsSchema::JsonSchema(serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string"}
                }
            })))
            .build();

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
