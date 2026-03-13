use std::collections::HashMap;
use std::fmt::{self, Debug, Display};
use std::sync::Arc;

use crate::callbacks::Callbacks;
use crate::callbacks::manager::{CallbackManager, CallbackManagerForToolRun};
use crate::error::{Error, Result};
use crate::messages::{AnyMessage, ToolCall, ToolMessage};
use crate::runnables::config::patch_config;
use crate::runnables::{RunnableConfig, ensure_config};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const FILTERED_ARGS: &[&str] = &["run_manager", "callbacks"];

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    #[default]
    Content,
    ContentAndArtifact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgsSchema {
    JsonSchema(Value),
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

    pub fn properties(&self) -> HashMap<String, Value> {
        match self {
            ArgsSchema::JsonSchema(schema) => schema
                .get("properties")
                .and_then(|p| p.as_object())
                .map(|obj| obj.clone().into_iter().collect())
                .unwrap_or_default(),
            ArgsSchema::TypeName(_) => HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Clone, Default)]
pub enum ErrorHandler {
    #[default]
    Ignore,
    UseDefault,
    Message(String),
    Handler(Arc<dyn Fn(&str) -> String + Send + Sync>),
}

impl Debug for ErrorHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorHandler::Ignore => write!(f, "ErrorHandler::Ignore"),
            ErrorHandler::UseDefault => write!(f, "ErrorHandler::UseDefault"),
            ErrorHandler::Message(m) => f.debug_tuple("ErrorHandler::Message").field(m).finish(),
            ErrorHandler::Handler(_) => write!(f, "ErrorHandler::Handler(<fn>)"),
        }
    }
}

impl ErrorHandler {
    pub fn handle(&self, error_msg: &str, default_msg: &str) -> Option<String> {
        match self {
            ErrorHandler::Ignore => None,
            ErrorHandler::UseDefault => {
                if error_msg.is_empty() {
                    Some(default_msg.to_string())
                } else {
                    Some(error_msg.to_string())
                }
            }
            ErrorHandler::Message(msg) => Some(msg.clone()),
            ErrorHandler::Handler(f) => Some(f(error_msg)),
        }
    }
}

pub struct ToolMeta {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) return_direct: bool,
    pub(crate) verbose: bool,
    pub(crate) handle_tool_error: ErrorHandler,
    pub(crate) handle_validation_error: ErrorHandler,
    pub(crate) response_format: ResponseFormat,
    pub(crate) tags: Option<Vec<String>>,
    pub(crate) metadata: Option<HashMap<String, Value>>,
    pub(crate) extras: Option<HashMap<String, Value>>,
    pub(crate) callbacks: Option<Callbacks>,
}

impl Debug for ToolMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToolMeta")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("return_direct", &self.return_direct)
            .field("response_format", &self.response_format)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub enum ToolInput {
    String(String),
    Dict(HashMap<String, Value>),
    ToolCall(ToolCall),
}

impl Display for ToolInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolInput::String(s) => f.write_str(s),
            ToolInput::Dict(d) => {
                let v = serde_json::to_string(d).unwrap_or_else(|_| format!("{d:?}"));
                f.write_str(&v)
            }
            ToolInput::ToolCall(tc) => write!(f, "{}", tc.args),
        }
    }
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

#[derive(Debug, Clone)]
pub enum ToolOutput {
    String(String),
    Message(ToolMessage),
    ContentAndArtifact { content: Value, artifact: Value },
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

#[async_trait]
pub trait BaseTool: Send + Sync + Debug {
    fn name(&self) -> &str;

    fn description(&self) -> &str;

    fn args_schema(&self) -> Option<&ArgsSchema> {
        None
    }

    fn return_direct(&self) -> bool {
        false
    }

    fn verbose(&self) -> bool {
        false
    }

    fn tags(&self) -> Option<&[String]> {
        None
    }

    fn metadata(&self) -> Option<&HashMap<String, Value>> {
        None
    }

    fn handle_tool_error(&self) -> &ErrorHandler {
        &ErrorHandler::Ignore
    }

    fn handle_validation_error(&self) -> &ErrorHandler {
        &ErrorHandler::Ignore
    }

    fn response_format(&self) -> ResponseFormat {
        ResponseFormat::Content
    }

    fn callbacks(&self) -> Option<&Callbacks> {
        None
    }

    fn extras(&self) -> Option<&HashMap<String, Value>> {
        None
    }

    fn is_single_input(&self) -> bool {
        self.args().keys().filter(|k| *k != "kwargs").count() == 1
    }

    fn args(&self) -> HashMap<String, Value> {
        self.args_schema()
            .map(|s| s.properties())
            .unwrap_or_default()
    }

    fn tool_call_schema(&self) -> ArgsSchema {
        self.args_schema().cloned().unwrap_or_default()
    }

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

    fn parameters_schema(&self) -> Value {
        self.definition().parameters
    }

    async fn tool_run(
        &self,
        input: ToolInput,
        run_manager: Option<&CallbackManagerForToolRun>,
        config: &RunnableConfig,
    ) -> Result<ToolOutput>;

    async fn run(
        &self,
        input: ToolInput,
        config: Option<RunnableConfig>,
        tool_call_id: Option<String>,
    ) -> Result<ToolOutput> {
        let (run_manager, child_config, _) = self.setup_run(&input, config);

        let result = self
            .tool_run(input, Some(&run_manager), &child_config)
            .await;
        self.finalize_run(result, &run_manager, tool_call_id.as_deref())
    }

    async fn invoke(&self, input: ToolInput, config: Option<RunnableConfig>) -> Result<ToolOutput> {
        let (tool_input, tool_call_id, config) = prep_run_args(input, config);
        self.run(tool_input, Some(config), tool_call_id).await
    }

    async fn invoke_tool_call(&self, tool_call: ToolCall) -> AnyMessage {
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

    async fn invoke_args(&self, args: Value) -> Value {
        let tool_call = ToolCall::builder().name(self.name()).args(args).build();
        let result = self.invoke_tool_call(tool_call).await;
        Value::String(result.text())
    }
}

trait BaseToolExt: BaseTool {
    fn setup_run(
        &self,
        input: &ToolInput,
        config: Option<RunnableConfig>,
    ) -> (CallbackManagerForToolRun, RunnableConfig, String) {
        let config = ensure_config(config);

        let callback_manager = CallbackManager::configure()
            .maybe_inheritable_callbacks(config.callbacks.clone())
            .maybe_local_callbacks(self.callbacks().cloned())
            .verbose(self.verbose())
            .inheritable_tags(config.tags.clone())
            .maybe_local_tags(self.tags().map(|t| t.to_vec()))
            .inheritable_metadata(config.metadata.clone())
            .maybe_local_metadata(self.metadata().cloned())
            .call();

        let serialized = HashMap::from([
            ("name".to_string(), Value::String(self.name().to_string())),
            (
                "description".to_string(),
                Value::String(self.description().to_string()),
            ),
        ]);

        let input_str = input.to_string();

        let run_manager =
            callback_manager.on_tool_start(&serialized, &input_str, config.run_id, None);

        let child_config = patch_config()
            .config(config)
            .callbacks(run_manager.get_child(None))
            .call();

        (run_manager, child_config, input_str)
    }

    fn finalize_run(
        &self,
        result: Result<ToolOutput>,
        run_manager: &CallbackManagerForToolRun,
        tool_call_id: Option<&str>,
    ) -> Result<ToolOutput> {
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
                                "response_format='content_and_artifact' requires the tool \
                                 function to return a two-element JSON array [content, artifact]"
                                    .to_string(),
                            );
                            run_manager.on_tool_error(&err);
                            return Err(err);
                        }
                    },
                    ResponseFormat::Content => (output, None),
                };

                let formatted = format_output(
                    content,
                    artifact,
                    tool_call_id,
                    self.name(),
                    crate::messages::ToolStatus::Success,
                );
                run_manager.on_tool_end(&formatted.to_string_lossy());
                Ok(formatted)
            }
            Err(e) => {
                if let Some(tool_err_msg) = e.as_tool_exception()
                    && let Some(handled) = self
                        .handle_tool_error()
                        .handle(tool_err_msg, "Tool execution error")
                {
                    let formatted = format_output(
                        ToolOutput::String(handled.clone()),
                        None,
                        tool_call_id,
                        self.name(),
                        crate::messages::ToolStatus::Error,
                    );
                    run_manager.on_tool_end(&handled);
                    return Ok(formatted);
                }
                if let Some(validation_msg) = e.as_validation_error()
                    && let Some(handled) = self
                        .handle_validation_error()
                        .handle(validation_msg, "Tool input validation error")
                {
                    let formatted = format_output(
                        ToolOutput::String(handled.clone()),
                        None,
                        tool_call_id,
                        self.name(),
                        crate::messages::ToolStatus::Error,
                    );
                    run_manager.on_tool_end(&handled);
                    return Ok(formatted);
                }
                run_manager.on_tool_error(&e);
                Err(e)
            }
        }
    }
}

impl<T: BaseTool + ?Sized> BaseToolExt for T {}

impl ToolOutput {
    pub fn to_string_lossy(&self) -> String {
        match self {
            ToolOutput::String(s) => s.clone(),
            ToolOutput::Message(m) => m.content.to_string(),
            ToolOutput::Json(v) => stringify(v),
            ToolOutput::ContentAndArtifact { content, .. } => stringify(content),
        }
    }
}

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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    async fn invoke(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output> {
        let (tool_input, tool_call_id, config) = prep_run_args(input, config);
        self.tool.run(tool_input, Some(config), tool_call_id).await
    }
}

pub fn is_tool_call(input: &Value) -> bool {
    input.get("type").and_then(|t| t.as_str()) == Some("tool_call")
}

pub fn format_output(
    content: ToolOutput,
    artifact: Option<Value>,
    tool_call_id: Option<&str>,
    name: &str,
    status: crate::messages::ToolStatus,
) -> ToolOutput {
    let tool_call_id = match tool_call_id {
        Some(id) => id,
        None => return content,
    };

    if let ToolOutput::Message(_) = content {
        return content;
    }

    let msg = ToolMessage::builder()
        .content(content.to_string_lossy())
        .tool_call_id(tool_call_id)
        .status(status)
        .name(name.to_string())
        .maybe_artifact(artifact)
        .build();

    ToolOutput::Message(msg)
}

pub fn is_message_content_type(obj: &Value) -> bool {
    match obj {
        Value::String(_) => true,
        Value::Array(arr) => arr.iter().all(is_message_content_block),
        _ => false,
    }
}

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

pub fn stringify(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| other.to_string()),
    }
}

pub fn prep_run_args(
    input: ToolInput,
    config: Option<RunnableConfig>,
) -> (ToolInput, Option<String>, RunnableConfig) {
    let config = ensure_config(config);

    match input {
        ToolInput::ToolCall(tc) => {
            let tool_call_id = tc.id;
            let args = tc
                .args
                .as_object()
                .map(|obj| obj.clone().into_iter().collect())
                .unwrap_or_default();
            (ToolInput::Dict(args), tool_call_id, config)
        }
        other => (other, None, config),
    }
}

pub trait BaseToolkit: Send + Sync {
    fn get_tools(&self) -> Vec<Arc<dyn BaseTool>>;
}

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
    fn test_error_handler() {
        let result = ErrorHandler::Ignore.handle("test error", "default");
        assert!(result.is_none());

        let result = ErrorHandler::UseDefault.handle("test error", "default");
        assert_eq!(result, Some("test error".to_string()));

        let result = ErrorHandler::UseDefault.handle("", "default");
        assert_eq!(result, Some("default".to_string()));

        let result = ErrorHandler::Message("custom".to_string()).handle("test error", "default");
        assert_eq!(result, Some("custom".to_string()));
    }

    #[test]
    fn test_tool_input_display() {
        let input = ToolInput::String("hello".to_string());
        assert_eq!(input.to_string(), "hello");

        let mut dict = HashMap::new();
        dict.insert("key".to_string(), Value::String("value".to_string()));
        let input = ToolInput::Dict(dict);
        let display = input.to_string();
        assert!(display.contains("key"));
        assert!(display.contains("value"));
    }
}
