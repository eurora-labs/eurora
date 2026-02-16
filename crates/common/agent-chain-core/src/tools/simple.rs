//! Tool that takes in function or coroutine directly.
//!
//! This module provides the `Tool` struct for creating simple single-input tools,
//! mirroring `langchain_core.tools.simple`.

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
    ArgsSchema, BaseTool, HandleToolError, HandleValidationError, ResponseFormat, ToolInput,
    ToolOutput,
};

/// Type alias for sync tool function.
pub type ToolFunc = Arc<dyn Fn(String) -> Result<String> + Send + Sync>;

/// Type alias for async tool function.
pub type AsyncToolFunc =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<String>> + Send>> + Send + Sync>;

/// Tool that takes in a function or coroutine directly.
///
/// This is the simplest form of tool that takes a single string input
/// and returns a string output.
pub struct Tool {
    /// The unique name of the tool.
    name: String,
    /// A description of what the tool does.
    description: String,
    /// The function to run when the tool is called.
    func: Option<ToolFunc>,
    /// The asynchronous version of the function.
    coroutine: Option<AsyncToolFunc>,
    /// Optional schema for the tool's input arguments.
    args_schema: Option<ArgsSchema>,
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

impl Debug for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tool")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("return_direct", &self.return_direct)
            .field("response_format", &self.response_format)
            .finish()
    }
}

impl Tool {
    /// Create a new Tool.
    pub fn new(
        name: impl Into<String>,
        func: Option<ToolFunc>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            func,
            coroutine: None,
            args_schema: None,
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

    /// Set the coroutine (async function).
    pub fn with_coroutine(mut self, coroutine: AsyncToolFunc) -> Self {
        self.coroutine = Some(coroutine);
        self
    }

    /// Set the args schema.
    pub fn with_args_schema(mut self, schema: ArgsSchema) -> Self {
        self.args_schema = Some(schema);
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

    /// Create a Tool from a function.
    pub fn from_function<F>(
        func: F,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self
    where
        F: Fn(String) -> Result<String> + Send + Sync + 'static,
    {
        Self::new(name, Some(Arc::new(func)), description)
    }

    /// Create a Tool from a function with additional options matching Python's
    /// `Tool.from_function(func, name, description, return_direct, args_schema, coroutine, **kwargs)`.
    pub fn from_function_full<F>(
        func: F,
        name: impl Into<String>,
        description: impl Into<String>,
        return_direct: bool,
        args_schema: Option<ArgsSchema>,
        coroutine: Option<AsyncToolFunc>,
    ) -> Self
    where
        F: Fn(String) -> Result<String> + Send + Sync + 'static,
    {
        let mut tool = Self::new(name, Some(Arc::new(func)), description);
        tool.return_direct = return_direct;
        tool.args_schema = args_schema;
        tool.coroutine = coroutine;
        tool
    }

    /// Create a Tool from a sync and async function pair.
    pub fn from_function_with_async<F, AF, Fut>(
        func: F,
        coroutine: AF,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self
    where
        F: Fn(String) -> Result<String> + Send + Sync + 'static,
        AF: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<String>> + Send + 'static,
    {
        Self::new(name, Some(Arc::new(func)), description)
            .with_coroutine(Arc::new(move |input| Box::pin(coroutine(input))))
    }

    /// Extract the single input from the tool input.
    fn extract_single_input(&self, input: ToolInput) -> Result<String> {
        match input {
            ToolInput::String(s) => Ok(s),
            ToolInput::Dict(d) => {
                // For backwards compatibility, if run_input is a dict,
                // extract the single value
                let all_args: Vec<_> = d.values().collect();
                if all_args.len() != 1 {
                    return Err(Error::ToolInvocation(format!(
                        "Too many arguments to single-input tool {}. Consider using StructuredTool instead. Args: {:?}",
                        self.name, all_args
                    )));
                }
                match all_args[0] {
                    Value::String(s) => Ok(s.clone()),
                    other => Ok(other.to_string()),
                }
            }
            ToolInput::ToolCall(tc) => {
                let args = &tc.args;
                if let Some(obj) = args.as_object() {
                    let values: Vec<_> = obj.values().collect();
                    if values.len() != 1 {
                        return Err(Error::ToolInvocation(format!(
                            "Too many arguments to single-input tool {}. Consider using StructuredTool instead.",
                            self.name,
                        )));
                    }
                    match &values[0] {
                        Value::String(s) => Ok(s.clone()),
                        other => Ok(other.to_string()),
                    }
                } else if let Some(s) = args.as_str() {
                    Ok(s.to_string())
                } else {
                    Ok(args.to_string())
                }
            }
        }
    }
}

#[async_trait]
impl BaseTool for Tool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn args_schema(&self) -> Option<&ArgsSchema> {
        self.args_schema.as_ref()
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

    fn args(&self) -> HashMap<String, Value> {
        // For backwards compatibility, if the function signature is ambiguous,
        // assume it takes a single string input.
        if let Some(args_schema) = &self.args_schema {
            return args_schema.properties();
        }
        let mut props = HashMap::new();
        props.insert(
            "tool_input".to_string(),
            serde_json::json!({"type": "string"}),
        );
        props
    }

    fn tool_run(
        &self,
        input: ToolInput,
        _run_manager: Option<&CallbackManagerForToolRun>,
        _config: &RunnableConfig,
    ) -> Result<ToolOutput> {
        let string_input = self.extract_single_input(input)?;

        if let Some(ref func) = self.func {
            let result = func(string_input)?;
            Ok(ToolOutput::String(result))
        } else {
            Err(Error::ToolInvocation(
                "Tool does not support sync invocation.".to_string(),
            ))
        }
    }

    async fn tool_arun(
        &self,
        input: ToolInput,
        _run_manager: Option<&crate::callbacks::manager::AsyncCallbackManagerForToolRun>,
        _config: &RunnableConfig,
    ) -> Result<ToolOutput> {
        let string_input = self.extract_single_input(input.clone())?;

        if let Some(ref coroutine) = self.coroutine {
            let result = coroutine(string_input).await?;
            Ok(ToolOutput::String(result))
        } else {
            // Fall back to sync implementation
            let sync_manager = _run_manager.map(|rm| rm.get_sync());
            self.tool_run(input, sync_manager.as_ref(), _config)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_creation() {
        let tool = Tool::from_function(
            |input| Ok(format!("Echo: {}", input)),
            "echo",
            "Echoes the input",
        );

        assert_eq!(tool.name(), "echo");
        assert_eq!(tool.description(), "Echoes the input");
    }

    #[test]
    fn test_tool_run() {
        let tool = Tool::from_function(
            |input| Ok(format!("Hello, {}!", input)),
            "greet",
            "Greets the user",
        );

        let result = tool
            .run(ToolInput::String("World".to_string()), None, None)
            .unwrap();
        match result {
            ToolOutput::String(s) => assert_eq!(s, "Hello, World!"),
            _ => panic!("Expected String output"),
        }
    }

    #[test]
    fn test_tool_run_with_dict() {
        let tool = Tool::from_function(
            |input| Ok(format!("Got: {}", input)),
            "process",
            "Processes input",
        );

        let mut dict = HashMap::new();
        dict.insert("query".to_string(), Value::String("test".to_string()));

        let result = tool.run(ToolInput::Dict(dict), None, None).unwrap();
        match result {
            ToolOutput::String(s) => assert_eq!(s, "Got: test"),
            _ => panic!("Expected String output"),
        }
    }

    #[test]
    fn test_tool_args() {
        let tool = Tool::from_function(Ok, "identity", "Returns input unchanged");

        let args = tool.args();
        assert!(args.contains_key("tool_input"));
    }

    #[tokio::test]
    async fn test_tool_arun() {
        let tool = Tool::from_function(
            |input| Ok(format!("Sync: {}", input)),
            "sync_tool",
            "A sync tool",
        );

        // Should fall back to sync implementation
        let result = tool
            .arun(ToolInput::String("test".to_string()), None, None)
            .await
            .unwrap();
        match result {
            ToolOutput::String(s) => assert_eq!(s, "Sync: test"),
            _ => panic!("Expected String output"),
        }
    }
}
