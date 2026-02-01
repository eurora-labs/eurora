//! Tool-related message types.
//!
//! This module contains types for tool calls and tool messages,
//! mirroring `langchain_core.messages.tool`.

use bon::bon;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;

/// Mixin trait for objects that tools can return directly.
///
/// If a custom Tool is invoked with a `ToolCall` and the output of custom code is
/// not an instance of `ToolOutputMixin`, the output will automatically be coerced to
/// a string and wrapped in a `ToolMessage`.
pub trait ToolOutputMixin {}

/// A tool call made by the AI model.
///
/// Represents an AI's request to call a tool. This corresponds to
/// `ToolCall` in LangChain Python.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    /// Unique identifier for this tool call
    pub id: Option<String>,
    /// Name of the tool to call
    pub name: String,
    /// Arguments for the tool call as a JSON object
    pub args: serde_json::Value,
}

#[bon]
impl ToolCall {
    /// Create a new tool call.
    #[builder]
    pub fn new(name: impl Into<String>, args: serde_json::Value, id: Option<String>) -> Self {
        Self {
            id,
            name: name.into(),
            args,
        }
    }
}

/// A tool call chunk (yielded when streaming).
///
/// When merging tool call chunks, all string attributes are concatenated.
/// Chunks are only merged if their values of `index` are equal and not None.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCallChunk {
    /// The name of the tool to be called (may be partial during streaming)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The arguments to the tool call (may be partial JSON string during streaming)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
    /// An identifier associated with the tool call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The index of the tool call in a sequence
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i32>,
}

#[bon]
impl ToolCallChunk {
    /// Create a new tool call chunk.
    #[builder]
    pub fn new(
        name: Option<String>,
        args: Option<String>,
        id: Option<String>,
        index: Option<i32>,
    ) -> Self {
        Self {
            name,
            args,
            id,
            index,
        }
    }
}

/// Represents an invalid tool call that failed parsing.
///
/// Here we add an `error` key to surface errors made during generation
/// (e.g., invalid JSON arguments.)

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvalidToolCall {
    /// The name of the tool to be called
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The arguments to the tool call (unparsed string)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
    /// An identifier associated with the tool call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// An error message associated with the tool call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[bon]
impl InvalidToolCall {
    /// Create a new invalid tool call.
    #[builder]
    pub fn new(
        name: Option<String>,
        args: Option<String>,
        id: Option<String>,
        error: Option<String>,
    ) -> Self {
        Self {
            name,
            args,
            id,
            error,
        }
    }
}

/// A tool message containing the result of a tool call.
///
/// `ToolMessage` objects contain the result of a tool invocation. Typically, the result
/// is encoded inside the `content` field.
///
/// This corresponds to `ToolMessage` in LangChain Python.

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ToolMessage {
    /// The tool result content
    pub content: String,
    /// The ID of the tool call this message is responding to
    pub tool_call_id: String,
    /// Optional unique identifier
    pub id: Option<String>,
    /// Optional name for the tool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Status of the tool invocation
    #[serde(default = "default_status")]
    pub status: ToolStatus,
    /// Artifact of the tool execution which is not meant to be sent to the model.
    ///
    /// Should only be specified if it is different from the message content, e.g. if only
    /// a subset of the full tool output is being passed as message content but the full
    /// output is needed in other parts of the code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact: Option<serde_json::Value>,
    /// Additional metadata
    #[serde(default)]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
    /// Response metadata
    #[serde(default)]
    pub response_metadata: HashMap<String, serde_json::Value>,
}

impl Serialize for ToolMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut field_count = 6;
        if self.name.is_some() {
            field_count += 1;
        }
        if self.artifact.is_some() {
            field_count += 1;
        }
        // Add 1 for additional type field
        field_count += 1;

        let mut map = serializer.serialize_map(Some(field_count))?;
        map.serialize_entry("type", "tool")?;
        map.serialize_entry("content", &self.content)?;
        map.serialize_entry("tool_call_id", &self.tool_call_id)?;
        map.serialize_entry("id", &self.id)?;
        if self.name.is_some() {
            map.serialize_entry("name", &self.name)?;
        }
        map.serialize_entry("status", &self.status)?;
        if self.artifact.is_some() {
            map.serialize_entry("artifact", &self.artifact)?;
        }
        map.serialize_entry("additional_kwargs", &self.additional_kwargs)?;
        map.serialize_entry("response_metadata", &self.response_metadata)?;

        map.end()
    }
}

fn default_status() -> ToolStatus {
    ToolStatus::Success
}

/// Status of a tool invocation.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ToolStatus {
    #[default]
    Success,
    Error,
}

impl PartialEq<str> for ToolStatus {
    fn eq(&self, other: &str) -> bool {
        matches!(
            (self, other),
            (ToolStatus::Success, "success") | (ToolStatus::Error, "error")
        )
    }
}

impl From<String> for ToolStatus {
    fn from(value: String) -> Self {
        match value.as_str() {
            "success" => ToolStatus::Success,
            "error" => ToolStatus::Error,
            _ => ToolStatus::default(),
        }
    }
}

impl From<ToolStatus> for String {
    fn from(value: ToolStatus) -> Self {
        match value {
            ToolStatus::Success => "success".to_string(),
            ToolStatus::Error => "error".to_string(),
        }
    }
}

impl PartialEq<&str> for ToolStatus {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

#[bon]
impl ToolMessage {
    /// Create a new tool message.
    #[builder]
    pub fn new(
        content: impl Into<String>,
        tool_call_id: impl Into<String>,
        id: Option<String>,
        name: Option<String>,
        #[builder(default)] status: ToolStatus,
        artifact: Option<serde_json::Value>,
        #[builder(default)] additional_kwargs: HashMap<String, serde_json::Value>,
        #[builder(default)] response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            content: content.into(),
            tool_call_id: tool_call_id.into(),
            id,
            name,
            status,
            artifact,
            additional_kwargs,
            response_metadata,
        }
    }

    /// Set the message ID.
    pub fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }

    /// Get the message type as a string.
    pub fn message_type(&self) -> &'static str {
        "tool"
    }

    /// Get the text content of the message.
    pub fn text(&self) -> &str {
        &self.content
    }
}

impl ToolOutputMixin for ToolMessage {}

/// Tool message chunk (yielded when streaming).
///
/// This corresponds to `ToolMessageChunk` in LangChain Python.

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ToolMessageChunk {
    /// The tool result content (may be partial during streaming)
    pub content: String,
    /// The ID of the tool call this message is responding to
    pub tool_call_id: String,
    /// Optional unique identifier
    pub id: Option<String>,
    /// Optional name for the tool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Status of the tool invocation
    #[serde(default = "default_status")]
    pub status: ToolStatus,
    /// Artifact of the tool execution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact: Option<serde_json::Value>,
    /// Additional metadata
    #[serde(default)]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
    /// Response metadata
    #[serde(default)]
    pub response_metadata: HashMap<String, serde_json::Value>,
}

impl Serialize for ToolMessageChunk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut field_count = 6;
        if self.name.is_some() {
            field_count += 1;
        }
        if self.artifact.is_some() {
            field_count += 1;
        }
        // Add 1 for additional type field
        field_count += 1;

        let mut map = serializer.serialize_map(Some(field_count))?;
        map.serialize_entry("type", "ToolMessageChunk")?;
        map.serialize_entry("content", &self.content)?;
        map.serialize_entry("tool_call_id", &self.tool_call_id)?;
        map.serialize_entry("id", &self.id)?;
        if self.name.is_some() {
            map.serialize_entry("name", &self.name)?;
        }
        map.serialize_entry("status", &self.status)?;
        if self.artifact.is_some() {
            map.serialize_entry("artifact", &self.artifact)?;
        }
        map.serialize_entry("additional_kwargs", &self.additional_kwargs)?;
        map.serialize_entry("response_metadata", &self.response_metadata)?;

        map.end()
    }
}

#[bon]
impl ToolMessageChunk {
    /// Create a new tool message chunk.
    #[builder]
    pub fn new(
        content: impl Into<String>,
        tool_call_id: impl Into<String>,
        id: Option<String>,
        name: Option<String>,
        #[builder(default)] status: ToolStatus,
        artifact: Option<serde_json::Value>,
        #[builder(default)] additional_kwargs: HashMap<String, serde_json::Value>,
        #[builder(default)] response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            content: content.into(),
            tool_call_id: tool_call_id.into(),
            id,
            name,
            status,
            artifact,
            additional_kwargs,
            response_metadata,
        }
    }

    /// Get the message type as a string.
    pub fn message_type(&self) -> &'static str {
        "ToolMessageChunk"
    }

    /// Concatenate this chunk with another chunk.
    ///
    /// Panics if the tool_call_ids don't match.
    pub fn concat(&self, other: &ToolMessageChunk) -> ToolMessageChunk {
        if self.tool_call_id != other.tool_call_id {
            panic!(
                "Cannot concatenate ToolMessageChunks with different tool_call_ids: '{}' vs '{}'",
                self.tool_call_id, other.tool_call_id
            );
        }

        let mut content = self.content.clone();
        content.push_str(&other.content);

        // Merge status (error takes precedence)
        let status = if self.status == ToolStatus::Error || other.status == ToolStatus::Error {
            ToolStatus::Error
        } else {
            ToolStatus::Success
        };

        // Merge response_metadata
        let mut response_metadata = self.response_metadata.clone();
        for (k, v) in &other.response_metadata {
            response_metadata
                .entry(k.clone())
                .or_insert_with(|| v.clone());
        }

        ToolMessageChunk {
            content,
            tool_call_id: self.tool_call_id.clone(),
            id: self.id.clone().or_else(|| other.id.clone()),
            name: self.name.clone().or_else(|| other.name.clone()),
            status,
            artifact: self.artifact.clone().or_else(|| other.artifact.clone()),
            additional_kwargs: self.additional_kwargs.clone(),
            response_metadata,
        }
    }

    /// Convert this chunk to a complete ToolMessage.
    pub fn to_message(&self) -> ToolMessage {
        ToolMessage {
            content: self.content.clone(),
            tool_call_id: self.tool_call_id.clone(),
            id: self.id.clone(),
            name: self.name.clone(),
            status: self.status.clone(),
            artifact: self.artifact.clone(),
            additional_kwargs: self.additional_kwargs.clone(),
            response_metadata: self.response_metadata.clone(),
        }
    }
}

impl std::ops::Add for ToolMessageChunk {
    type Output = ToolMessageChunk;

    fn add(self, other: ToolMessageChunk) -> ToolMessageChunk {
        self.concat(&other)
    }
}

/// Factory function to create a tool call.
///
/// This corresponds to the `tool_call` function in LangChain Python.
pub fn tool_call(name: impl Into<String>, args: serde_json::Value, id: Option<String>) -> ToolCall {
    ToolCall::builder()
        .name(name)
        .args(args)
        .maybe_id(id)
        .build()
}

/// Factory function to create a tool call chunk.
///
/// This corresponds to the `tool_call_chunk` function in LangChain Python.
pub fn tool_call_chunk(
    name: Option<String>,
    args: Option<String>,
    id: Option<String>,
    index: Option<i32>,
) -> ToolCallChunk {
    ToolCallChunk::builder()
        .maybe_name(name)
        .maybe_args(args)
        .maybe_id(id)
        .maybe_index(index)
        .build()
}

/// Factory function to create an invalid tool call.
///
/// This corresponds to the `invalid_tool_call` function in LangChain Python.
pub fn invalid_tool_call(
    name: Option<String>,
    args: Option<String>,
    id: Option<String>,
    error: Option<String>,
) -> InvalidToolCall {
    InvalidToolCall::builder()
        .maybe_name(name)
        .maybe_args(args)
        .maybe_id(id)
        .maybe_error(error)
        .build()
}

/// Best-effort parsing of tools from raw tool call dictionaries.
///
/// This corresponds to the `default_tool_parser` function in LangChain Python.
pub fn default_tool_parser(
    raw_tool_calls: &[serde_json::Value],
) -> (Vec<ToolCall>, Vec<InvalidToolCall>) {
    let mut tool_calls = Vec::new();
    let mut invalid_tool_calls = Vec::new();

    for raw_tool_call in raw_tool_calls {
        let function = match raw_tool_call.get("function") {
            Some(f) => f,
            None => continue,
        };

        let function_name = function
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();

        let arguments_str = function
            .get("arguments")
            .and_then(|a| a.as_str())
            .unwrap_or("{}");

        let id = raw_tool_call
            .get("id")
            .and_then(|i| i.as_str())
            .map(|s| s.to_string());

        match serde_json::from_str::<serde_json::Value>(arguments_str) {
            Ok(args) if args.is_object() => {
                tool_calls.push(tool_call(function_name, args, id));
            }
            _ => {
                invalid_tool_calls.push(invalid_tool_call(
                    Some(function_name),
                    Some(arguments_str.to_string()),
                    id,
                    None,
                ));
            }
        }
    }

    (tool_calls, invalid_tool_calls)
}

/// Best-effort parsing of tool call chunks from raw tool call dictionaries.
///
/// This corresponds to the `default_tool_chunk_parser` function in LangChain Python.
pub fn default_tool_chunk_parser(raw_tool_calls: &[serde_json::Value]) -> Vec<ToolCallChunk> {
    let mut chunks = Vec::new();

    for raw_tool_call in raw_tool_calls {
        let (function_name, function_args) = match raw_tool_call.get("function") {
            Some(f) => (
                f.get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string()),
                f.get("arguments")
                    .and_then(|a| a.as_str())
                    .map(|s| s.to_string()),
            ),
            None => (None, None),
        };

        let id = raw_tool_call
            .get("id")
            .and_then(|i| i.as_str())
            .map(|s| s.to_string());

        let index = raw_tool_call
            .get("index")
            .and_then(|i| i.as_i64())
            .map(|i| i as i32);

        chunks.push(tool_call_chunk(function_name, function_args, id, index));
    }

    chunks
}
