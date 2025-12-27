//! Base classes and utilities for LangChain tools.
//!
//! This module provides the core tool abstractions, mirroring
//! `langchain_core.tools.base`.

use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use crate::callbacks::{
    AsyncCallbackManager, AsyncCallbackManagerForToolRun, CallbackManager, CallbackManagerForToolRun,
    Callbacks,
};
use crate::error::Result;
use crate::messages::{BaseMessage, ToolCall, ToolMessage};
use crate::runnables::{RunnableConfig, ensure_config, patch_config};

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
            ArgsSchema::JsonSchema(schema) => {
                schema.get("properties")
                    .and_then(|p| p.as_object())
                    .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default()
            }
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
#[derive(Clone)]
pub enum HandleToolError {
    /// Don't handle errors (re-raise them).
    None,
    /// Return a generic error message.
    Bool(bool),
    /// Return a specific error message.
    Message(String),
    /// Use a custom function to handle the error.
    Handler(Arc<dyn Fn(&ToolException) -> String + Send + Sync>),
}

impl Debug for HandleToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandleToolError::None => write!(f, "HandleToolError::None"),
            HandleToolError::Bool(b) => f.debug_tuple("HandleToolError::Bool").field(b).finish(),
            HandleToolError::Message(m) => f.debug_tuple("HandleToolError::Message").field(m).finish(),
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
#[derive(Clone)]
pub enum HandleValidationError {
    /// Don't handle errors (re-raise them).
    None,
    /// Return a generic error message.
    Bool(bool),
    /// Return a specific error message.
    Message(String),
    /// Use a custom function to handle the error.
    Handler(Arc<dyn Fn(&str) -> String + Send + Sync>),
}

impl Debug for HandleValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandleValidationError::None => write!(f, "HandleValidationError::None"),
            HandleValidationError::Bool(b) => f.debug_tuple("HandleValidationError::Bool").field(b).finish(),
            HandleValidationError::Message(m) => f.debug_tuple("HandleValidationError::Message").field(m).finish(),
            HandleValidationError::Handler(_) => write!(f, "HandleValidationError::Handler(<function>)"),
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
                if obj.get("type").and_then(|t| t.as_str()) == Some("tool_call") {
                    if let (Some(id), Some(name), Some(args)) = (
                        obj.get("id").and_then(|i| i.as_str()),
                        obj.get("name").and_then(|n| n.as_str()),
                        obj.get("args"),
                    ) {
                        return ToolInput::ToolCall(ToolCall::with_id(id, name, args.clone()));
                    }
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
        self.args_schema()
            .cloned()
            .unwrap_or_default()
    }

    /// Get the tool definition for LLM function calling.
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.args_schema()
                .map(|s| s.to_json_schema())
                .unwrap_or_else(|| serde_json::json!({"type": "object", "properties": {}})),
        }
    }

    /// Get the JSON schema for the tool's parameters.
    fn parameters_schema(&self) -> Value {
        self.definition().parameters
    }

    /// Run the tool synchronously.
    fn run(
        &self,
        input: ToolInput,
        config: Option<RunnableConfig>,
    ) -> Result<ToolOutput>;

    /// Run the tool asynchronously.
    async fn arun(
        &self,
        input: ToolInput,
        config: Option<RunnableConfig>,
    ) -> Result<ToolOutput> {
        // Default implementation uses sync run
        self.run(input, config)
    }

    /// Invoke the tool with a ToolCall.
    async fn invoke(&self, tool_call: ToolCall) -> BaseMessage {
        let input = ToolInput::ToolCall(tool_call.clone());
        match self.arun(input, None).await {
            Ok(output) => match output {
                ToolOutput::String(s) => {
                    ToolMessage::new(s, tool_call.id()).into()
                }
                ToolOutput::Message(m) => m.into(),
                ToolOutput::ContentAndArtifact { content, artifact } => {
                    ToolMessage::with_artifact(
                        content.to_string(),
                        tool_call.id(),
                        artifact,
                    ).into()
                }
                ToolOutput::Json(v) => {
                    ToolMessage::new(v.to_string(), tool_call.id()).into()
                }
            },
            Err(e) => {
                ToolMessage::error(e.to_string(), tool_call.id()).into()
            }
        }
    }

    /// Invoke the tool directly with arguments.
    async fn invoke_args(&self, args: Value) -> Value {
        let tool_call = ToolCall::new(self.name(), args);
        let result = self.invoke(tool_call).await;
        Value::String(result.content().to_string())
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
pub fn handle_tool_error_impl(
    e: &ToolException,
    flag: &HandleToolError,
) -> Option<String> {
    match flag {
        HandleToolError::None => None,
        HandleToolError::Bool(false) => None,
        HandleToolError::Bool(true) => {
            Some(e.0.clone())
        }
        HandleToolError::Message(msg) => Some(msg.clone()),
        HandleToolError::Handler(f) => Some(f(e)),
    }
}

/// Handle a validation error based on the configured flag.
pub fn handle_validation_error_impl(
    e: &str,
    flag: &HandleValidationError,
) -> Option<String> {
    match flag {
        HandleValidationError::None => None,
        HandleValidationError::Bool(false) => None,
        HandleValidationError::Bool(true) => {
            Some("Tool input validation error".to_string())
        }
        HandleValidationError::Message(msg) => Some(msg.clone()),
        HandleValidationError::Handler(f) => Some(f(e)),
    }
}

/// Format tool output as appropriate.
pub fn format_output(
    content: Value,
    artifact: Option<Value>,
    tool_call_id: Option<&str>,
    name: &str,
    status: &str,
) -> ToolOutput {
    if let Some(tool_call_id) = tool_call_id {
        let msg = if let Some(artifact) = artifact {
            ToolMessage::with_artifact(
                stringify_content(&content),
                tool_call_id,
                artifact,
            )
        } else {
            ToolMessage::new(stringify_content(&content), tool_call_id)
        };
        ToolOutput::Message(msg.with_name(name))
    } else {
        match content {
            Value::String(s) => ToolOutput::String(s),
            other => ToolOutput::Json(other),
        }
    }
}

/// Check if content is a valid message content type.
pub fn is_message_content_type(obj: &Value) -> bool {
    match obj {
        Value::String(_) => true,
        Value::Array(arr) => arr.iter().all(is_message_content_block),
        _ => false,
    }
}

/// Check if object is a valid message content block.
pub fn is_message_content_block(obj: &Value) -> bool {
    match obj {
        Value::String(_) => true,
        Value::Object(map) => {
            map.get("type")
                .and_then(|t| t.as_str())
                .map(|t| TOOL_MESSAGE_BLOCK_TYPES.contains(&t))
                .unwrap_or(false)
        }
        _ => false,
    }
}

/// Convert content to string, preferring JSON format.
pub fn stringify_content(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| other.to_string()),
    }
}

/// Prepare arguments for tool execution.
pub fn prep_run_args(
    value: ToolInput,
    config: Option<RunnableConfig>,
) -> (ToolInput, Option<String>, RunnableConfig) {
    let config = ensure_config(config);
    
    match &value {
        ToolInput::ToolCall(tc) => {
            let tool_call_id = Some(tc.id().to_string());
            let input = ToolInput::Dict(
                tc.args()
                    .as_object()
                    .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                    .unwrap_or_default()
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