use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;

use crate::error::{Error, Result};
use crate::runnables::Runnable;

use super::base::{ArgsSchema, ResponseFormat};
use super::structured::{StructuredTool, create_args_schema};

#[derive(Debug, Clone, Default)]
pub struct ToolConfig {
    pub name: Option<String>,
    pub description: Option<String>,
    pub return_direct: bool,
    pub args_schema: Option<ArgsSchema>,
    pub response_format: ResponseFormat,
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
        #[builder(default)] response_format: ResponseFormat,
        extras: Option<HashMap<String, Value>>,
    ) -> Self {
        Self {
            name,
            description,
            return_direct,
            args_schema,
            response_format,
            extras,
        }
    }

    pub fn into_structured_tool<F>(self, func: F) -> Result<StructuredTool>
    where
        F: Fn(HashMap<String, Value>) -> Result<Value> + Send + Sync + 'static,
    {
        let name = self
            .name
            .ok_or_else(|| Error::InvalidConfig("Tool name is required".to_string()))?;

        Ok(StructuredTool::builder()
            .name(name)
            .description(self.description.unwrap_or_default())
            .args_schema(self.args_schema.unwrap_or_default())
            .func(Arc::new(func))
            .return_direct(self.return_direct)
            .response_format(self.response_format)
            .maybe_extras(self.extras)
            .build())
    }
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

    let func = move |args: HashMap<String, Value>| runnable.invoke(args, None);

    let schema = ArgsSchema::JsonSchema(serde_json::json!({
        "type": "object",
        "properties": {},
        "additionalProperties": true
    }));

    StructuredTool::from_function(func, name, description, schema)
}

pub struct PropertyDef<'a> {
    pub name: &'a str,
    pub r#type: &'a str,
    pub description: &'a str,
    pub required: bool,
}

pub fn tool_from_schema(
    name: impl Into<String>,
    description: impl Into<String>,
    properties: &[PropertyDef<'_>],
    func: impl Fn(HashMap<String, Value>) -> Result<Value> + Send + Sync + 'static,
) -> StructuredTool {
    let name = name.into();
    let description = description.into();

    let mut props = HashMap::new();
    let mut required = Vec::new();

    for prop in properties {
        props.insert(
            prop.name.to_string(),
            serde_json::json!({
                "type": prop.r#type,
                "description": prop.description
            }),
        );
        if prop.required {
            required.push(prop.name.to_string());
        }
    }

    let schema = create_args_schema(&name, props, required, Some(&description));
    StructuredTool::from_function(func, name, description, schema)
}

pub fn get_description_from_runnable<R>(runnable: &R) -> String
where
    R: Runnable,
{
    let input_schema = runnable.get_input_schema(None);
    format!("Takes {}.", input_schema)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::base::BaseTool;
    use crate::tools::simple::Tool;

    #[test]
    fn test_tool_config_into_structured_tool() {
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

        let tool = config
            .into_structured_tool(|args| Ok(args.get("input").cloned().unwrap_or(Value::Null)))
            .unwrap();

        assert_eq!(tool.name(), "configured_tool");
    }

    #[test]
    fn test_tool_config_requires_name() {
        let config = ToolConfig::builder().description("No name").build();

        let result = config.into_structured_tool(|_| Ok(Value::Null));
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_from_schema() {
        let tool = tool_from_schema(
            "greet",
            "Greets a person",
            &[PropertyDef {
                name: "name",
                r#type: "string",
                description: "The person's name",
                required: true,
            }],
            |args| {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("stranger");
                Ok(Value::String(format!("Hello, {}!", name)))
            },
        );

        assert_eq!(tool.name(), "greet");
    }

    #[test]
    fn test_simple_tool_creation() {
        let tool = Tool::from_function(
            |input| Ok(format!("Echo: {}", input)),
            "echo",
            "Echoes the input",
        );

        assert_eq!(tool.name(), "echo");
        assert_eq!(tool.description(), "Echoes the input");
    }

    #[test]
    fn test_tool_config_defaults() {
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
}
