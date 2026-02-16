//! Structured tool that can operate on any number of inputs.
//!
//! This module provides the `StructuredTool` struct for creating tools
//! that accept multiple typed arguments, mirroring `langchain_core.tools.structured`.

use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::callbacks::base::Callbacks;
use crate::callbacks::manager::CallbackManagerForToolRun;
use crate::error::{Error, Result};
use crate::runnables::RunnableConfig;

use super::base::{
    ArgsSchema, BaseTool, FILTERED_ARGS, HandleToolError, HandleValidationError, ResponseFormat,
    ToolInput, ToolOutput,
};

/// Type alias for sync structured tool function.
pub type StructuredToolFunc = Arc<dyn Fn(HashMap<String, Value>) -> Result<Value> + Send + Sync>;

/// Type alias for async structured tool function.
pub type AsyncStructuredToolFunc = Arc<
    dyn Fn(HashMap<String, Value>) -> Pin<Box<dyn Future<Output = Result<Value>> + Send>>
        + Send
        + Sync,
>;

/// Tool that can operate on any number of inputs.
///
/// Unlike `Tool`, which accepts a single string input, `StructuredTool`
/// accepts a dictionary of typed arguments.
pub struct StructuredTool {
    /// The unique name of the tool.
    name: String,
    /// A description of what the tool does.
    description: String,
    /// The function to run when the tool is called.
    func: Option<StructuredToolFunc>,
    /// The asynchronous version of the function.
    coroutine: Option<AsyncStructuredToolFunc>,
    /// The input arguments' schema.
    args_schema: ArgsSchema,
    /// Whether to return the tool's output directly.
    return_direct: bool,
    /// Whether to log the tool's progress.
    verbose: bool,
    /// How to handle tool errors.
    handle_tool_error: HandleToolError,
    /// How to handle validation errors.
    handle_validation_error: HandleValidationError,
    /// The tool response format.
    response_format: ResponseFormat,
    /// Optional tags for the tool.
    tags: Option<Vec<String>>,
    /// Optional metadata for the tool.
    metadata: Option<HashMap<String, Value>>,
    /// Optional provider-specific extras.
    extras: Option<HashMap<String, Value>>,
    /// Optional callbacks for the tool.
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

impl StructuredTool {
    /// Create a new StructuredTool.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        args_schema: ArgsSchema,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            func: None,
            coroutine: None,
            args_schema,
            return_direct: false,
            verbose: false,
            handle_tool_error: HandleToolError::Bool(false),
            handle_validation_error: HandleValidationError::Bool(false),
            response_format: ResponseFormat::Content,
            tags: None,
            metadata: None,
            extras: None,
            callbacks: None,
        }
    }

    /// Set the sync function.
    pub fn with_func(mut self, func: StructuredToolFunc) -> Self {
        self.func = Some(func);
        self
    }

    /// Set the async function.
    pub fn with_coroutine(mut self, coroutine: AsyncStructuredToolFunc) -> Self {
        self.coroutine = Some(coroutine);
        self
    }

    /// Set whether to return directly.
    pub fn with_return_direct(mut self, return_direct: bool) -> Self {
        self.return_direct = return_direct;
        self
    }

    /// Set the response format.
    pub fn with_response_format(mut self, format: ResponseFormat) -> Self {
        self.response_format = format;
        self
    }

    /// Set tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: HashMap<String, Value>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Set extras.
    pub fn with_extras(mut self, extras: HashMap<String, Value>) -> Self {
        self.extras = Some(extras);
        self
    }

    /// Set callbacks.
    pub fn with_callbacks(mut self, callbacks: Callbacks) -> Self {
        self.callbacks = Some(callbacks);
        self
    }

    /// Set handle_tool_error.
    pub fn with_handle_tool_error(mut self, handler: HandleToolError) -> Self {
        self.handle_tool_error = handler;
        self
    }

    /// Set handle_validation_error.
    pub fn with_handle_validation_error(mut self, handler: HandleValidationError) -> Self {
        self.handle_validation_error = handler;
        self
    }

    /// Create a tool from a function.
    ///
    /// This is the main way to create a StructuredTool.
    pub fn from_function<F>(
        func: F,
        name: impl Into<String>,
        description: impl Into<String>,
        args_schema: ArgsSchema,
    ) -> Self
    where
        F: Fn(HashMap<String, Value>) -> Result<Value> + Send + Sync + 'static,
    {
        Self::new(name, description, args_schema).with_func(Arc::new(func))
    }

    /// Create a tool from a sync and async function pair.
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
        Self::new(name, description, args_schema)
            .with_func(Arc::new(func))
            .with_coroutine(Arc::new(move |args| Box::pin(coroutine(args))))
    }

    /// Extract the arguments from the tool input.
    fn extract_args(&self, input: ToolInput) -> Result<HashMap<String, Value>> {
        match input {
            ToolInput::String(s) => {
                // Try to parse as JSON
                if let Ok(Value::Object(obj)) = serde_json::from_str(&s) {
                    Ok(obj.into_iter().collect())
                } else {
                    // Use as single argument if schema has one field
                    let props = self.args_schema.properties();
                    if props.len() == 1 {
                        let key = props.keys().next().unwrap().clone();
                        let mut args = HashMap::new();
                        args.insert(key, Value::String(s));
                        Ok(args)
                    } else {
                        Err(Error::ToolInvocation(
                            "String input not allowed for multi-argument tool".to_string(),
                        ))
                    }
                }
            }
            ToolInput::Dict(d) => Ok(d),
            ToolInput::ToolCall(tc) => {
                let args = &tc.args;
                if let Some(obj) = args.as_object() {
                    Ok(obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                } else {
                    Err(Error::ToolInvocation(
                        "ToolCall args must be an object".to_string(),
                    ))
                }
            }
        }
    }

    /// Filter out arguments that shouldn't be passed to the function.
    fn filter_args(&self, args: HashMap<String, Value>) -> HashMap<String, Value> {
        args.into_iter()
            .filter(|(k, _)| !FILTERED_ARGS.contains(&k.as_str()))
            .collect()
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

    fn handle_tool_error(&self) -> &HandleToolError {
        &self.handle_tool_error
    }

    fn handle_validation_error(&self) -> &HandleValidationError {
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
        let args = self.extract_args(input)?;
        let filtered_args = self.filter_args(args);

        if let Some(ref func) = self.func {
            let result = func(filtered_args)?;
            // Return raw result — response_format handling is in BaseTool::run()
            match result {
                Value::String(s) => Ok(ToolOutput::String(s)),
                other => Ok(ToolOutput::Json(other)),
            }
        } else {
            Err(Error::ToolInvocation(
                "StructuredTool does not support sync invocation.".to_string(),
            ))
        }
    }

    async fn tool_arun(
        &self,
        input: ToolInput,
        _run_manager: Option<&crate::callbacks::manager::AsyncCallbackManagerForToolRun>,
        _config: &RunnableConfig,
    ) -> Result<ToolOutput> {
        let args = self.extract_args(input.clone())?;
        let filtered_args = self.filter_args(args);

        if let Some(ref coroutine) = self.coroutine {
            let result = coroutine(filtered_args).await?;
            // Return raw result — response_format handling is in BaseTool::arun()
            match result {
                Value::String(s) => Ok(ToolOutput::String(s)),
                other => Ok(ToolOutput::Json(other)),
            }
        } else {
            // Fall back to sync implementation
            let sync_manager = _run_manager.map(|rm| rm.get_sync());
            self.tool_run(input, sync_manager.as_ref(), _config)
        }
    }
}

/// Helper function to create an args schema from field definitions.
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
