use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use bon::bon;
use serde_json::Value;

use crate::callbacks::Callbacks;
use crate::callbacks::manager::CallbackManagerForToolRun;
use crate::error::{Error, Result};
use crate::runnables::RunnableConfig;

use super::base::{
    ArgsSchema, BaseTool, ErrorHandler, FILTERED_ARGS, ResponseFormat, ToolInput, ToolOutput,
};

pub type StructuredToolFunc = Arc<dyn Fn(HashMap<String, Value>) -> Result<Value> + Send + Sync>;

pub type AsyncStructuredToolFunc = Arc<
    dyn Fn(HashMap<String, Value>) -> Pin<Box<dyn Future<Output = Result<Value>> + Send>>
        + Send
        + Sync,
>;

pub struct StructuredTool {
    name: String,
    description: String,
    func: Option<StructuredToolFunc>,
    coroutine: Option<AsyncStructuredToolFunc>,
    args_schema: ArgsSchema,
    return_direct: bool,
    verbose: bool,
    handle_tool_error: ErrorHandler,
    handle_validation_error: ErrorHandler,
    response_format: ResponseFormat,
    tags: Option<Vec<String>>,
    metadata: Option<HashMap<String, Value>>,
    extras: Option<HashMap<String, Value>>,
    callbacks: Option<Callbacks>,
}

impl Debug for StructuredTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StructuredTool")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("args_schema", &self.args_schema)
            .field("return_direct", &self.return_direct)
            .field("response_format", &self.response_format)
            .finish()
    }
}

#[bon]
impl StructuredTool {
    #[builder]
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        args_schema: ArgsSchema,
        func: Option<StructuredToolFunc>,
        coroutine: Option<AsyncStructuredToolFunc>,
        #[builder(default)] return_direct: bool,
        #[builder(default)] verbose: bool,
        #[builder(default)] handle_tool_error: ErrorHandler,
        #[builder(default)] handle_validation_error: ErrorHandler,
        #[builder(default)] response_format: ResponseFormat,
        tags: Option<Vec<String>>,
        metadata: Option<HashMap<String, Value>>,
        extras: Option<HashMap<String, Value>>,
        callbacks: Option<Callbacks>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            func,
            coroutine,
            args_schema,
            return_direct,
            verbose,
            handle_tool_error,
            handle_validation_error,
            response_format,
            tags,
            metadata,
            extras,
            callbacks,
        }
    }

    pub fn from_function<F>(
        func: F,
        name: impl Into<String>,
        description: impl Into<String>,
        args_schema: ArgsSchema,
    ) -> Self
    where
        F: Fn(HashMap<String, Value>) -> Result<Value> + Send + Sync + 'static,
    {
        Self::builder()
            .name(name)
            .description(description)
            .args_schema(args_schema)
            .func(Arc::new(func))
            .build()
    }

    pub fn from_function_with_async<F, AF, Fut>(
        func: F,
        coroutine: AF,
        name: impl Into<String>,
        description: impl Into<String>,
        args_schema: ArgsSchema,
    ) -> Self
    where
        F: Fn(HashMap<String, Value>) -> Result<Value> + Send + Sync + 'static,
        AF: Fn(HashMap<String, Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Value>> + Send + 'static,
    {
        Self::builder()
            .name(name)
            .description(description)
            .args_schema(args_schema)
            .func(Arc::new(func))
            .coroutine(Arc::new(move |args| Box::pin(coroutine(args))))
            .build()
    }

    fn extract_args(&self, input: ToolInput) -> Result<HashMap<String, Value>> {
        match input {
            ToolInput::String(s) => {
                if let Ok(Value::Object(obj)) = serde_json::from_str(&s) {
                    Ok(obj.into_iter().collect())
                } else {
                    let props = self.args_schema.properties();
                    if props.len() == 1 {
                        let key = props.into_keys().next().expect("checked len == 1");
                        Ok(HashMap::from([(key, Value::String(s))]))
                    } else {
                        Err(Error::ToolInvocation(
                            "String input not allowed for multi-argument tool".to_string(),
                        ))
                    }
                }
            }
            ToolInput::Dict(d) => Ok(d),
            ToolInput::ToolCall(tc) => tc
                .args
                .as_object()
                .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                .ok_or_else(|| {
                    Error::ToolInvocation("ToolCall args must be an object".to_string())
                }),
        }
    }
}

fn filter_args(args: HashMap<String, Value>) -> HashMap<String, Value> {
    args.into_iter()
        .filter(|(k, _)| !FILTERED_ARGS.contains(&k.as_str()))
        .collect()
}

fn value_to_tool_output(value: Value) -> ToolOutput {
    match value {
        Value::String(s) => ToolOutput::String(s),
        other => ToolOutput::Json(other),
    }
}

#[async_trait]
impl BaseTool for StructuredTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn args_schema(&self) -> Option<&ArgsSchema> {
        Some(&self.args_schema)
    }

    fn return_direct(&self) -> bool {
        self.return_direct
    }

    fn verbose(&self) -> bool {
        self.verbose
    }

    fn tags(&self) -> Option<&[String]> {
        self.tags.as_deref()
    }

    fn metadata(&self) -> Option<&HashMap<String, Value>> {
        self.metadata.as_ref()
    }

    fn handle_tool_error(&self) -> &ErrorHandler {
        &self.handle_tool_error
    }

    fn handle_validation_error(&self) -> &ErrorHandler {
        &self.handle_validation_error
    }

    fn response_format(&self) -> ResponseFormat {
        self.response_format
    }

    fn extras(&self) -> Option<&HashMap<String, Value>> {
        self.extras.as_ref()
    }

    fn callbacks(&self) -> Option<&Callbacks> {
        self.callbacks.as_ref()
    }

    fn tool_run(
        &self,
        input: ToolInput,
        _run_manager: Option<&CallbackManagerForToolRun>,
        _config: &RunnableConfig,
    ) -> Result<ToolOutput> {
        let args = filter_args(self.extract_args(input)?);

        let func = self.func.as_ref().ok_or_else(|| {
            Error::ToolInvocation("StructuredTool does not support sync invocation.".to_string())
        })?;

        func(args).map(value_to_tool_output)
    }

    async fn tool_arun(
        &self,
        input: ToolInput,
        run_manager: Option<&CallbackManagerForToolRun>,
        config: &RunnableConfig,
    ) -> Result<ToolOutput> {
        if let Some(coroutine) = &self.coroutine {
            let args = filter_args(self.extract_args(input)?);
            coroutine(args).await.map(value_to_tool_output)
        } else {
            self.tool_run(input, run_manager, config)
        }
    }
}

pub fn create_args_schema(
    name: &str,
    properties: HashMap<String, Value>,
    required: Vec<String>,
    description: Option<&str>,
) -> ArgsSchema {
    let mut schema = serde_json::json!({
        "type": "object",
        "title": name,
        "properties": properties,
        "required": required,
    });

    if let Some(desc) = description {
        schema["description"] = Value::String(desc.to_string());
    }

    ArgsSchema::JsonSchema(schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_structured_tool_creation() {
        let schema = create_args_schema(
            "add_numbers",
            {
                let mut props = HashMap::new();
                props.insert("a".to_string(), serde_json::json!({"type": "number"}));
                props.insert("b".to_string(), serde_json::json!({"type": "number"}));
                props
            },
            vec!["a".to_string(), "b".to_string()],
            Some("Add two numbers"),
        );

        let tool = StructuredTool::from_function(
            |args| {
                let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                Ok(Value::from(a + b))
            },
            "add",
            "Adds two numbers together",
            schema,
        );

        assert_eq!(tool.name(), "add");
        assert_eq!(tool.description(), "Adds two numbers together");
    }

    #[test]
    fn test_structured_tool_run() {
        let schema = create_args_schema(
            "multiply",
            {
                let mut props = HashMap::new();
                props.insert("x".to_string(), serde_json::json!({"type": "number"}));
                props.insert("y".to_string(), serde_json::json!({"type": "number"}));
                props
            },
            vec!["x".to_string(), "y".to_string()],
            None,
        );

        let tool = StructuredTool::from_function(
            |args| {
                let x = args.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let y = args.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                Ok(Value::from(x * y))
            },
            "multiply",
            "Multiplies two numbers",
            schema,
        );

        let mut input = HashMap::new();
        input.insert("x".to_string(), Value::from(3.0));
        input.insert("y".to_string(), Value::from(4.0));

        let result = tool.run(ToolInput::Dict(input), None, None).unwrap();
        match result {
            ToolOutput::Json(v) => assert_eq!(v.as_f64().unwrap(), 12.0),
            _ => panic!("Expected Json output"),
        }
    }

    #[test]
    fn test_create_args_schema() {
        let schema = create_args_schema(
            "test_schema",
            {
                let mut props = HashMap::new();
                props.insert("field1".to_string(), serde_json::json!({"type": "string"}));
                props
            },
            vec!["field1".to_string()],
            Some("Test description"),
        );

        let json = schema.to_json_schema();
        assert_eq!(json["title"], "test_schema");
        assert_eq!(json["description"], "Test description");
        assert!(json["properties"]["field1"].is_object());
    }

    #[tokio::test]
    async fn test_structured_tool_arun() {
        let schema = create_args_schema(
            "concat",
            {
                let mut props = HashMap::new();
                props.insert("a".to_string(), serde_json::json!({"type": "string"}));
                props.insert("b".to_string(), serde_json::json!({"type": "string"}));
                props
            },
            vec!["a".to_string(), "b".to_string()],
            None,
        );

        let tool = StructuredTool::from_function(
            |args| {
                let a = args.get("a").and_then(|v| v.as_str()).unwrap_or("");
                let b = args.get("b").and_then(|v| v.as_str()).unwrap_or("");
                Ok(Value::String(format!("{}{}", a, b)))
            },
            "concat",
            "Concatenates two strings",
            schema,
        );

        let mut input = HashMap::new();
        input.insert("a".to_string(), Value::String("Hello".to_string()));
        input.insert("b".to_string(), Value::String("World".to_string()));

        let result = tool.arun(ToolInput::Dict(input), None, None).await.unwrap();
        match result {
            ToolOutput::String(s) => assert_eq!(s, "HelloWorld"),
            _ => panic!("Expected String output"),
        }
    }
}
