//! Base classes and utilities for LangChain tools.
//!
//! This module provides the core tool abstractions, mirroring
//! `langchain_core.tools.base`.

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::callbacks::base::Callbacks;
use crate::callbacks::manager::{
    AsyncCallbackManager, AsyncCallbackManagerForToolRun, CallbackManager,
    CallbackManagerForToolRun,
};
use crate::error::{Error, Result};
use crate::messages::{BaseMessage, ToolCall, ToolMessage};
use crate::runnables::config::patch_config;
use crate::runnables::{RunnableConfig, ensure_config};

/// Arguments that are filtered out from tool schemas.
pub const FILTERED_ARGS: &[&str] = &["run_manager", "callbacks"];

/// Block types that are valid in tool messages.
pub const TOOL_MESSAGE_BLOCK_TYPES: &[&str] = &[
    "text",
    "image_url",
    "image",
    "json",
    "search_result",
    "custom_tool_call_output",
    "document",
    "file",
];

/// Error raised when args_schema is missing or has incorrect type annotation.
#[derive(Debug, Error)]
#[error("Schema annotation error: {message}")]
pub struct SchemaAnnotationError {
    pub message: String,
}

impl SchemaAnnotationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Exception thrown when a tool execution error occurs.
///
/// This exception allows tools to signal errors without stopping the agent.
/// The error is handled according to the tool's `handle_tool_error` setting,
/// and the result is returned as an observation to the agent.
#[derive(Debug, Error)]
#[error("{0}")]
pub struct ToolException(pub String);

impl ToolException {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

/// Represents the response format for a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    /// The output is interpreted as the contents of a ToolMessage.
    #[default]
    Content,
    /// The output is expected to be a tuple of (content, artifact).
    ContentAndArtifact,
}

/// Represents a tool's schema, which can be a JSON schema or a type reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgsSchema {
    /// A JSON schema definition.
    JsonSchema(Value),
    /// A type name reference.
    TypeName(String),
}

impl Default for ArgsSchema {
    fn default() -> Self {
        ArgsSchema::JsonSchema(serde_json::json!({
            "type": "object",
            "properties": {}
        }))
    }
}

impl ArgsSchema {
    /// Get the JSON schema for this args schema.
    pub fn to_json_schema(&self) -> Value {
        match self {
            ArgsSchema::JsonSchema(schema) => schema.clone(),
            ArgsSchema::TypeName(name) => serde_json::json!({
                "type": "object",
                "title": name,
                "properties": {}
            }),
        }
    }

    /// Get properties from the schema.
    pub fn properties(&self) -> HashMap<String, Value> {
        match self {
            ArgsSchema::JsonSchema(schema) => schema
                .get("properties")
                .and_then(|p| p.as_object())
                .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                .unwrap_or_default(),
            ArgsSchema::TypeName(_) => HashMap::new(),
        }
    }
}

/// Represents a tool's definition for LLM function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// The name of the tool.
    pub name: String,
    /// A description of what the tool does.
    pub description: String,
    /// JSON schema for the tool's parameters.
    pub parameters: Value,
}

/// How to handle tool errors.
///
/// Mirrors Python's `bool | str | Callable[[ToolException], str] | None`.
/// - `None` or `Bool(false)`: Don't handle errors (re-raise them)
/// - `Bool(true)`: Return the exception message
/// - `Message(str)`: Return a specific error message
/// - `Handler(fn)`: Use a custom function to handle the error
#[derive(Clone)]
pub enum HandleToolError {
    /// Return the exception message when true, don't handle when false.
    Bool(bool),
    /// Return a specific error message.
    Message(String),
    /// Use a custom function to handle the error.
    Handler(Arc<dyn Fn(&ToolException) -> String + Send + Sync>),
}

impl Debug for HandleToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandleToolError::Bool(b) => f.debug_tuple("HandleToolError::Bool").field(b).finish(),
            HandleToolError::Message(m) => {
                f.debug_tuple("HandleToolError::Message").field(m).finish()
            }
            HandleToolError::Handler(_) => write!(f, "HandleToolError::Handler(<function>)"),
        }
    }
}

impl Default for HandleToolError {
    fn default() -> Self {
        HandleToolError::Bool(false)
    }
}

/// How to handle validation errors.
///
/// Mirrors Python's `bool | str | Callable[[ValidationError], str] | None`.
/// - `None` or `Bool(false)`: Don't handle errors (re-raise them)
/// - `Bool(true)`: Return a generic "Tool input validation error" message
/// - `Message(str)`: Return a specific error message
/// - `Handler(fn)`: Use a custom function to handle the error
#[derive(Clone)]
pub enum HandleValidationError {
    /// Return a generic error message when true, don't handle when false.
    Bool(bool),
    /// Return a specific error message.
    Message(String),
    /// Use a custom function to handle the error.
    Handler(Arc<dyn Fn(&str) -> String + Send + Sync>),
}

impl Debug for HandleValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandleValidationError::Bool(b) => f
                .debug_tuple("HandleValidationError::Bool")
                .field(b)
                .finish(),
            HandleValidationError::Message(m) => f
                .debug_tuple("HandleValidationError::Message")
                .field(m)
                .finish(),
            HandleValidationError::Handler(_) => {
                write!(f, "HandleValidationError::Handler(<function>)")
            }
        }
    }
}

impl Default for HandleValidationError {
    fn default() -> Self {
        HandleValidationError::Bool(false)
    }
}

/// Input type for tools - can be a string, dict, or ToolCall.
#[derive(Debug, Clone)]
pub enum ToolInput {
    /// A simple string input.
    String(String),
    /// A dictionary of arguments.
    Dict(HashMap<String, Value>),
    /// A full tool call.
    ToolCall(ToolCall),
}

impl From<String> for ToolInput {
    fn from(s: String) -> Self {
        ToolInput::String(s)
    }
}

impl From<&str> for ToolInput {
    fn from(s: &str) -> Self {
        ToolInput::String(s.to_string())
    }
}

impl From<HashMap<String, Value>> for ToolInput {
    fn from(d: HashMap<String, Value>) -> Self {
        ToolInput::Dict(d)
    }
}

impl From<ToolCall> for ToolInput {
    fn from(tc: ToolCall) -> Self {
        ToolInput::ToolCall(tc)
    }
}

impl From<Value> for ToolInput {
    fn from(v: Value) -> Self {
        match v {
            Value::String(s) => ToolInput::String(s),
            Value::Object(obj) => {
                // Check if this is a tool call
                if obj.get("type").and_then(|t| t.as_str()) == Some("tool_call")
                    && let (Some(id), Some(name), Some(args)) = (
                        obj.get("id").and_then(|i| i.as_str()),
                        obj.get("name").and_then(|n| n.as_str()),
                        obj.get("args"),
                    )
                {
                    return ToolInput::ToolCall(
                        ToolCall::builder()
                            .name(name)
                            .args(args.clone())
                            .id(id.to_string())
                            .build(),
                    );
                }
                ToolInput::Dict(obj.into_iter().collect())
            }
            _ => ToolInput::String(v.to_string()),
        }
    }
}

/// Output type for tools.
#[derive(Debug, Clone)]
pub enum ToolOutput {
    /// A simple string output.
    String(String),
    /// A ToolMessage output.
    Message(ToolMessage),
    /// A content and artifact tuple.
    ContentAndArtifact { content: Value, artifact: Value },
    /// Raw JSON value.
    Json(Value),
}

impl From<String> for ToolOutput {
    fn from(s: String) -> Self {
        ToolOutput::String(s)
    }
}

impl From<&str> for ToolOutput {
    fn from(s: &str) -> Self {
        ToolOutput::String(s.to_string())
    }
}

impl From<ToolMessage> for ToolOutput {
    fn from(m: ToolMessage) -> Self {
        ToolOutput::Message(m)
    }
}

impl From<Value> for ToolOutput {
    fn from(v: Value) -> Self {
        ToolOutput::Json(v)
    }
}

/// Base trait for all LangChain tools.
///
/// This trait defines the interface that all LangChain tools must implement.
/// Tools are components that can be called by agents to perform specific actions.
#[async_trait]
pub trait BaseTool: Send + Sync + Debug {
    /// Get the unique name of the tool.
    fn name(&self) -> &str;

    /// Get the description of what the tool does.
    fn description(&self) -> &str;

    /// Get the args schema for the tool.
    fn args_schema(&self) -> Option<&ArgsSchema> {
        None
    }

    /// Whether to return the tool's output directly.
    fn return_direct(&self) -> bool {
        false
    }

    /// Whether to log the tool's progress.
    fn verbose(&self) -> bool {
        false
    }

    /// Get tags associated with the tool.
    fn tags(&self) -> Option<&[String]> {
        None
    }

    /// Get metadata associated with the tool.
    fn metadata(&self) -> Option<&HashMap<String, Value>> {
        None
    }

    /// Get how to handle tool errors.
    fn handle_tool_error(&self) -> &HandleToolError {
        &HandleToolError::Bool(false)
    }

    /// Get how to handle validation errors.
    fn handle_validation_error(&self) -> &HandleValidationError {
        &HandleValidationError::Bool(false)
    }

    /// Get the response format for the tool.
    fn response_format(&self) -> ResponseFormat {
        ResponseFormat::Content
    }

    /// Get callbacks associated with the tool.
    fn callbacks(&self) -> Option<&Callbacks> {
        None
    }

    /// Get optional provider-specific extra fields.
    fn extras(&self) -> Option<&HashMap<String, Value>> {
        None
    }

    /// Check if the tool accepts only a single input argument.
    fn is_single_input(&self) -> bool {
        let args = self.args();
        let keys: Vec<_> = args.keys().filter(|k| *k != "kwargs").collect();
        keys.len() == 1
    }

    /// Get the tool's input arguments schema.
    fn args(&self) -> HashMap<String, Value> {
        self.args_schema()
            .map(|s| s.properties())
            .unwrap_or_default()
    }

    /// Get the schema for tool calls, excluding injected arguments.
    fn tool_call_schema(&self) -> ArgsSchema {
        self.args_schema().cloned().unwrap_or_default()
    }

    /// Get the tool definition for LLM function calling.
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self
                .args_schema()
                .map(|s| s.to_json_schema())
                .unwrap_or_else(|| serde_json::json!({"type": "object", "properties": {}})),
        }
    }

    /// Get the JSON schema for the tool's parameters.
    fn parameters_schema(&self) -> Value {
        self.definition().parameters
    }

    /// The core tool implementation that concrete types override.
    ///
    /// This is the equivalent of Python's `_run()`. Concrete tool types
    /// (Tool, StructuredTool) implement this method with their actual logic.
    /// The `run_manager` can be used to get child callback managers for sub-calls.
    fn tool_run(
        &self,
        input: ToolInput,
        run_manager: Option<&CallbackManagerForToolRun>,
        config: &RunnableConfig,
    ) -> Result<ToolOutput>;

    /// The async core tool implementation that concrete types override.
    ///
    /// This is the equivalent of Python's `_arun()`. Default delegates to `tool_run`.
    async fn tool_arun(
        &self,
        input: ToolInput,
        run_manager: Option<&AsyncCallbackManagerForToolRun>,
        config: &RunnableConfig,
    ) -> Result<ToolOutput> {
        let sync_manager = run_manager.map(|rm| rm.get_sync());
        self.tool_run(input, sync_manager.as_ref(), config)
    }

    /// Run the tool synchronously with the full callback pipeline.
    ///
    /// Mirrors Python's `BaseTool.run()`:
    /// 1. Configures callback manager (merges tool + config callbacks/tags/metadata)
    /// 2. Fires `on_tool_start`
    /// 3. Calls `tool_run()` (the implementation method)
    /// 4. Handles ToolException / validation errors
    /// 5. Fires `on_tool_end` or `on_tool_error`
    /// 6. Returns formatted output
    fn run(
        &self,
        input: ToolInput,
        config: Option<RunnableConfig>,
        tool_call_id: Option<String>,
    ) -> Result<ToolOutput> {
        let config = ensure_config(config);

        // Configure callback manager
        let callback_manager = CallbackManager::configure(
            config.callbacks.clone(),
            self.callbacks().cloned(),
            self.verbose(),
            Some(config.tags.clone()),
            self.tags().map(|t| t.to_vec()),
            Some(config.metadata.clone()),
            self.metadata().cloned(),
        );

        // Build serialized info and input string for callbacks
        let mut serialized = HashMap::new();
        serialized.insert(
            "name".to_string(),
            serde_json::Value::String(self.name().to_string()),
        );
        serialized.insert(
            "description".to_string(),
            serde_json::Value::String(self.description().to_string()),
        );

        let input_str = match &input {
            ToolInput::String(s) => s.clone(),
            ToolInput::Dict(d) => format!("{:?}", d),
            ToolInput::ToolCall(tc) => tc.args.to_string(),
        };

        let run_manager =
            callback_manager.on_tool_start(&serialized, &input_str, config.run_id, None);

        // Create child config with run manager's child callback manager
        let child_config = patch_config(
            Some(config.clone()),
            Some(run_manager.get_child(None)),
            None,
            None,
            None,
            None,
        );

        // Execute tool
        let result = self.tool_run(input, Some(&run_manager), &child_config);

        // Handle result with error recovery and callback dispatch
        match result {
            Ok(output) => {
                let (content, artifact) = match self.response_format() {
                    ResponseFormat::ContentAndArtifact => {
                        // tool_run() returns raw values; for content_and_artifact
                        // we expect a JSON array [content, artifact]
                        match output {
                            ToolOutput::Json(Value::Array(ref arr)) if arr.len() == 2 => {
                                let content = match &arr[0] {
                                    Value::String(s) => ToolOutput::String(s.clone()),
                                    other => ToolOutput::Json(other.clone()),
                                };
                                (content, Some(arr[1].clone()))
                            }
                            _ => {
                                let err = Error::ToolException(
                                    "Since response_format='content_and_artifact', the tool                                      function must return a two-element JSON array                                      [content, artifact]."
                                        .to_string(),
                                );
                                run_manager.on_tool_error(&err);
                                return Err(err);
                            }
                        }
                    }
                    ResponseFormat::Content => (output, None),
                };
                let formatted = format_output(
                    content,
                    artifact,
                    tool_call_id.as_deref(),
                    self.name(),
                    "success",
                );
                let output_str = match &formatted {
                    ToolOutput::String(s) => s.clone(),
                    ToolOutput::Message(m) => m.content.to_string(),
                    ToolOutput::Json(v) => stringify(v),
                    ToolOutput::ContentAndArtifact { content, .. } => stringify(content),
                };
                run_manager.on_tool_end(&output_str);
                Ok(formatted)
            }
            Err(e) => {
                // Check if this is a ToolException
                if let Some(tool_err_msg) = e.as_tool_exception() {
                    let exc = ToolException::new(tool_err_msg);
                    if let Some(handled) = handle_tool_error_impl(&exc, self.handle_tool_error()) {
                        let formatted = format_output(
                            ToolOutput::String(handled.clone()),
                            None,
                            tool_call_id.as_deref(),
                            self.name(),
                            "error",
                        );
                        run_manager.on_tool_end(&handled);
                        return Ok(formatted);
                    }
                }
                // Check if this is a validation error
                if let Some(validation_msg) = e.as_validation_error()
                    && let Some(handled) =
                        handle_validation_error_impl(validation_msg, self.handle_validation_error())
                {
                    let formatted = format_output(
                        ToolOutput::String(handled.clone()),
                        None,
                        tool_call_id.as_deref(),
                        self.name(),
                        "error",
                    );
                    run_manager.on_tool_end(&handled);
                    return Ok(formatted);
                }
                // Unhandled error
                run_manager.on_tool_error(&e);
                Err(e)
            }
        }
    }

    /// Run the tool asynchronously with the full callback pipeline.
    ///
    /// Mirrors Python's `BaseTool.arun()`. Uses `AsyncCallbackManager` for
    /// async callback dispatch and calls `tool_arun()` for the implementation.
    async fn arun(
        &self,
        input: ToolInput,
        config: Option<RunnableConfig>,
        tool_call_id: Option<String>,
    ) -> Result<ToolOutput> {
        let config = ensure_config(config);

        // Configure async callback manager
        let async_callback_manager = AsyncCallbackManager::configure(
            config.callbacks.clone(),
            self.callbacks().cloned(),
            self.verbose(),
            Some(config.tags.clone()),
            self.tags().map(|t| t.to_vec()),
            Some(config.metadata.clone()),
            self.metadata().cloned(),
        );

        // Build serialized info and input string for callbacks
        let mut serialized = HashMap::new();
        serialized.insert(
            "name".to_string(),
            serde_json::Value::String(self.name().to_string()),
        );
        serialized.insert(
            "description".to_string(),
            serde_json::Value::String(self.description().to_string()),
        );

        let input_str = match &input {
            ToolInput::String(s) => s.clone(),
            ToolInput::Dict(d) => format!("{:?}", d),
            ToolInput::ToolCall(tc) => tc.args.to_string(),
        };

        let run_manager = async_callback_manager
            .on_tool_start(&serialized, &input_str, config.run_id, None)
            .await;

        // Create child config using the sync child from the async run manager
        let child_config = patch_config(
            Some(config.clone()),
            Some(run_manager.get_sync().get_child(None)),
            None,
            None,
            None,
            None,
        );

        // Execute tool (async)
        let result = self
            .tool_arun(input, Some(&run_manager), &child_config)
            .await;

        // Handle result with error recovery and callback dispatch
        match result {
            Ok(output) => {
                let (content, artifact) = match self.response_format() {
                    ResponseFormat::ContentAndArtifact => match output {
                        ToolOutput::Json(Value::Array(ref arr)) if arr.len() == 2 => {
                            let content = match &arr[0] {
                                Value::String(s) => ToolOutput::String(s.clone()),
                                other => ToolOutput::Json(other.clone()),
                            };
                            (content, Some(arr[1].clone()))
                        }
                        _ => {
                            let err = Error::ToolException(
                                    "Since response_format='content_and_artifact', the tool                                      function must return a two-element JSON array                                      [content, artifact]."
                                        .to_string(),
                                );
                            run_manager.get_sync().on_tool_error(&err);
                            return Err(err);
                        }
                    },
                    ResponseFormat::Content => (output, None),
                };
                let formatted = format_output(
                    content,
                    artifact,
                    tool_call_id.as_deref(),
                    self.name(),
                    "success",
                );
                let output_str = match &formatted {
                    ToolOutput::String(s) => s.clone(),
                    ToolOutput::Message(m) => m.content.to_string(),
                    ToolOutput::Json(v) => stringify(v),
                    ToolOutput::ContentAndArtifact { content, .. } => stringify(content),
                };
                run_manager.on_tool_end(&output_str).await;
                Ok(formatted)
            }
            Err(e) => {
                // Check if this is a ToolException
                if let Some(tool_err_msg) = e.as_tool_exception() {
                    let exc = ToolException::new(tool_err_msg);
                    if let Some(handled) = handle_tool_error_impl(&exc, self.handle_tool_error()) {
                        let formatted = format_output(
                            ToolOutput::String(handled.clone()),
                            None,
                            tool_call_id.as_deref(),
                            self.name(),
                            "error",
                        );
                        run_manager.on_tool_end(&handled).await;
                        return Ok(formatted);
                    }
                }
                // Check if this is a validation error
                if let Some(validation_msg) = e.as_validation_error()
                    && let Some(handled) =
                        handle_validation_error_impl(validation_msg, self.handle_validation_error())
                {
                    let formatted = format_output(
                        ToolOutput::String(handled.clone()),
                        None,
                        tool_call_id.as_deref(),
                        self.name(),
                        "error",
                    );
                    run_manager.on_tool_end(&handled).await;
                    return Ok(formatted);
                }
                // Unhandled error
                run_manager.get_sync().on_tool_error(&e);
                Err(e)
            }
        }
    }

    /// Invoke the tool, routing through the callback pipeline.
    ///
    /// Mirrors Python's `BaseTool.invoke()`: extracts tool_call_id from
    /// ToolCall input, delegates to `run()`.
    async fn invoke(&self, input: ToolInput, config: Option<RunnableConfig>) -> Result<ToolOutput> {
        let (tool_input, tool_call_id, config) = prep_run_args(input, config);
        self.arun(tool_input, Some(config), tool_call_id).await
    }

    /// Invoke the tool with a ToolCall, returning a BaseMessage.
    ///
    /// This preserves the old `invoke(ToolCall) -> BaseMessage` behavior for
    /// callers that need a ToolMessage result (e.g., agent executors).
    async fn invoke_tool_call(&self, tool_call: ToolCall) -> BaseMessage {
        let tool_call_id = tool_call.id.clone().unwrap_or_default();
        let input = ToolInput::ToolCall(tool_call);
        match self.invoke(input, None).await {
            Ok(output) => match output {
                ToolOutput::String(s) => ToolMessage::builder()
                    .content(s)
                    .tool_call_id(&tool_call_id)
                    .build()
                    .into(),
                ToolOutput::Message(m) => m.into(),
                ToolOutput::ContentAndArtifact { content, artifact } => ToolMessage::builder()
                    .content(stringify(&content))
                    .tool_call_id(&tool_call_id)
                    .artifact(artifact)
                    .build()
                    .into(),
                ToolOutput::Json(v) => ToolMessage::builder()
                    .content(v.to_string())
                    .tool_call_id(&tool_call_id)
                    .build()
                    .into(),
            },
            Err(e) => ToolMessage::builder()
                .content(e.to_string())
                .tool_call_id(&tool_call_id)
                .status(crate::messages::ToolStatus::Error)
                .build()
                .into(),
        }
    }

    /// Invoke the tool directly with arguments.
    async fn invoke_args(&self, args: Value) -> Value {
        let tool_call = ToolCall::builder().name(self.name()).args(args).build();
        let result = self.invoke_tool_call(tool_call).await;
        Value::String(result.text())
    }
}

/// Adapter that makes a `BaseTool` usable as a `Runnable`.
///
/// Mirrors how Python's `BaseTool` extends `RunnableSerializable`.
/// In Rust, we use the adapter pattern (like `ChatModelRunnable`)
/// since trait objects cannot directly implement other traits.
#[derive(Clone)]
pub struct ToolRunnable {
    tool: Arc<dyn BaseTool>,
}

impl ToolRunnable {
    pub fn new(tool: Arc<dyn BaseTool>) -> Self {
        Self { tool }
    }

    pub fn tool(&self) -> &dyn BaseTool {
        &*self.tool
    }
}

impl Debug for ToolRunnable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRunnable")
            .field("name", &self.tool.name())
            .finish()
    }
}

#[async_trait]
impl crate::runnables::base::Runnable for ToolRunnable {
    type Input = ToolInput;
    type Output = ToolOutput;

    fn name(&self) -> Option<String> {
        Some(self.tool.name().to_string())
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        let (tool_input, tool_call_id, config) = prep_run_args(input, config);
        self.tool.run(tool_input, Some(config), tool_call_id)
    }

    async fn ainvoke(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output> {
        let (tool_input, tool_call_id, config) = prep_run_args(input, config);
        self.tool.arun(tool_input, Some(config), tool_call_id).await
    }
}

/// Annotation for tool arguments that are injected at runtime.
///
/// Tool arguments annotated with this are not included in the tool
/// schema sent to language models and are instead injected during execution.
#[derive(Debug, Clone, Default)]
pub struct InjectedToolArg;

/// Annotation for injecting the tool call ID.
///
/// This annotation is used to mark a tool parameter that should receive
/// the tool call ID at runtime.
#[derive(Debug, Clone, Default)]
pub struct InjectedToolCallId;

/// Check if an input is a tool call dictionary.
pub fn is_tool_call(input: &Value) -> bool {
    input.get("type").and_then(|t| t.as_str()) == Some("tool_call")
}

/// Handle a tool exception based on the configured flag.
///
/// Mirrors Python's `_handle_tool_error`:
/// - `Bool(false)`: Returns `None` (don't handle, re-raise)
/// - `Bool(true)`: Returns the exception message (or "Tool execution error" if empty)
/// - `Message(str)`: Returns the specific message
/// - `Handler(fn)`: Calls the handler function
pub fn handle_tool_error_impl(e: &ToolException, flag: &HandleToolError) -> Option<String> {
    match flag {
        HandleToolError::Bool(false) => None,
        HandleToolError::Bool(true) => {
            if e.0.is_empty() {
                Some("Tool execution error".to_string())
            } else {
                Some(e.0.clone())
            }
        }
        HandleToolError::Message(msg) => Some(msg.clone()),
        HandleToolError::Handler(f) => Some(f(e)),
    }
}

/// Handle a validation error based on the configured flag.
///
/// Mirrors Python's `_handle_validation_error`:
/// - `Bool(false)`: Returns `None` (don't handle, re-raise)
/// - `Bool(true)`: Returns "Tool input validation error"
/// - `Message(str)`: Returns the specific message
/// - `Handler(fn)`: Calls the handler function
pub fn handle_validation_error_impl(e: &str, flag: &HandleValidationError) -> Option<String> {
    match flag {
        HandleValidationError::Bool(false) => None,
        HandleValidationError::Bool(true) => Some("Tool input validation error".to_string()),
        HandleValidationError::Message(msg) => Some(msg.clone()),
        HandleValidationError::Handler(f) => Some(f(e)),
    }
}

/// Format tool output as a ToolMessage if appropriate.
///
/// Mirrors Python's `_format_output`:
/// - If content is already a ToolMessage (ToolOutput::Message) or tool_call_id is None,
///   returns the content directly
/// - Otherwise, wraps content in a ToolMessage with the tool_call_id
pub fn format_output(
    content: ToolOutput,
    artifact: Option<Value>,
    tool_call_id: Option<&str>,
    name: &str,
    status: &str,
) -> ToolOutput {
    // If content is already a ToolMessage or tool_call_id is None, return content directly
    if matches!(content, ToolOutput::Message(_)) || tool_call_id.is_none() {
        return content;
    }

    let tool_call_id = tool_call_id.expect("tool_call_id should be Some at this point");

    // Convert content to string, using stringify if not already a valid message content type
    let content_str = match &content {
        ToolOutput::String(s) => s.clone(),
        ToolOutput::Json(v) => stringify(v),
        ToolOutput::Message(_) => return content,
        ToolOutput::ContentAndArtifact { content, .. } => stringify(content),
    };

    let status_enum = match status {
        "error" => crate::messages::ToolStatus::Error,
        _ => crate::messages::ToolStatus::Success,
    };

    let msg = ToolMessage::builder()
        .content(content_str)
        .tool_call_id(tool_call_id)
        .status(status_enum)
        .name(name.to_string())
        .maybe_artifact(artifact)
        .build();

    ToolOutput::Message(msg)
}

/// Check if content is a valid message content type.
///
/// Validates content for OpenAI or Anthropic format tool messages.
pub fn is_message_content_type(obj: &Value) -> bool {
    match obj {
        Value::String(_) => true,
        Value::Array(arr) => arr.iter().all(is_message_content_block),
        _ => false,
    }
}

/// Check if object is a valid message content block.
///
/// Validates content blocks for OpenAI or Anthropic format.
pub fn is_message_content_block(obj: &Value) -> bool {
    match obj {
        Value::String(_) => true,
        Value::Object(map) => map
            .get("type")
            .and_then(|t| t.as_str())
            .is_some_and(|t| TOOL_MESSAGE_BLOCK_TYPES.contains(&t)),
        _ => false,
    }
}

/// Convert content to string, preferring JSON format.
///
/// Mirrors Python's `_stringify`: tries JSON first, falls back to str().
pub fn stringify(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| other.to_string()),
    }
}

/// Alias for stringify for backwards compatibility.
pub fn stringify_content(content: &Value) -> String {
    stringify(content)
}

/// Prepare arguments for tool execution.
pub fn prep_run_args(
    value: ToolInput,
    config: Option<RunnableConfig>,
) -> (ToolInput, Option<String>, RunnableConfig) {
    let config = ensure_config(config);

    match &value {
        ToolInput::ToolCall(tc) => {
            let tool_call_id = tc.id.clone();
            let input = ToolInput::Dict(
                tc.args
                    .as_object()
                    .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default(),
            );
            (input, tool_call_id, config)
        }
        _ => (value, None, config),
    }
}

/// Base class for toolkits containing related tools.
///
/// A toolkit is a collection of related tools that can be used together
/// to accomplish a specific task or work with a particular system.
pub trait BaseToolkit: Send + Sync {
    /// Get all tools in the toolkit.
    fn get_tools(&self) -> Vec<Arc<dyn BaseTool>>;
}

/// Type alias for dynamic tool reference.
pub type DynTool = Arc<dyn BaseTool>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_input_from_string() {
        let input = ToolInput::from("test");
        match input {
            ToolInput::String(s) => assert_eq!(s, "test"),
            _ => panic!("Expected String variant"),
        }
    }

    #[test]
    fn test_tool_input_from_value() {
        let value = serde_json::json!({"key": "value"});
        let input = ToolInput::from(value);
        match input {
            ToolInput::Dict(d) => {
                assert_eq!(d.get("key"), Some(&Value::String("value".to_string())));
            }
            _ => panic!("Expected Dict variant"),
        }
    }

    #[test]
    fn test_is_tool_call() {
        let tc = serde_json::json!({
            "type": "tool_call",
            "id": "123",
            "name": "test",
            "args": {}
        });
        assert!(is_tool_call(&tc));

        let not_tc = serde_json::json!({"key": "value"});
        assert!(!is_tool_call(&not_tc));
    }

    #[test]
    fn test_args_schema_properties() {
        let schema = ArgsSchema::JsonSchema(serde_json::json!({
            "type": "object",
            "properties": {
                "query": {"type": "string"}
            }
        }));
        let props = schema.properties();
        assert!(props.contains_key("query"));
    }

    #[test]
    fn test_response_format_default() {
        assert_eq!(ResponseFormat::default(), ResponseFormat::Content);
    }

    #[test]
    fn test_handle_tool_error() {
        let exc = ToolException::new("test error");

        let result = handle_tool_error_impl(&exc, &HandleToolError::Bool(false));
        assert!(result.is_none());

        let result = handle_tool_error_impl(&exc, &HandleToolError::Bool(true));
        assert_eq!(result, Some("test error".to_string()));

        let result = handle_tool_error_impl(&exc, &HandleToolError::Message("custom".to_string()));
        assert_eq!(result, Some("custom".to_string()));
    }
}
