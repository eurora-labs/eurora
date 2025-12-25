//! Tool-related message types.
//!
//! This module contains types for tool calls and tool messages,
//! mirroring `langchain_core.messages.tool`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(feature = "specta")]
use specta::Type;

/// A tool call made by the AI model.
///
/// Represents an AI's request to call a tool. This corresponds to
/// `ToolCall` in LangChain Python.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    /// Unique identifier for this tool call
    id: String,
    /// Name of the tool to call
    name: String,
    /// Arguments for the tool call as a JSON object
    args: serde_json::Value,
}

impl ToolCall {
    /// Create a new tool call.
    pub fn new(name: impl Into<String>, args: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            args,
        }
    }

    /// Create a new tool call with a specific ID.
    pub fn with_id(
        id: impl Into<String>,
        name: impl Into<String>,
        args: serde_json::Value,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            args,
        }
    }

    /// Get the tool call ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the tool name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the tool arguments.
    pub fn args(&self) -> &serde_json::Value {
        &self.args
    }
}

/// A tool call chunk (yielded when streaming).
///
/// When merging tool call chunks, all string attributes are concatenated.
/// Chunks are only merged if their values of `index` are equal and not None.
#[cfg_attr(feature = "specta", derive(Type))]
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

impl ToolCallChunk {
    /// Create a new tool call chunk.
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
#[cfg_attr(feature = "specta", derive(Type))]
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

impl InvalidToolCall {
    /// Create a new invalid tool call.
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
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolMessage {
    /// The tool result content
    content: String,
    /// The ID of the tool call this message is responding to
    tool_call_id: String,
    /// Optional unique identifier
    id: Option<String>,
    /// Status of the tool invocation
    #[serde(default = "default_status")]
    status: ToolStatus,
    /// Additional metadata
    #[serde(default)]
    additional_kwargs: HashMap<String, serde_json::Value>,
}

fn default_status() -> ToolStatus {
    ToolStatus::Success
}

/// Status of a tool invocation.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ToolStatus {
    #[default]
    Success,
    Error,
}

impl ToolMessage {
    /// Create a new tool message.
    pub fn new(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            tool_call_id: tool_call_id.into(),
            id: Some(Uuid::new_v4().to_string()),
            status: ToolStatus::Success,
            additional_kwargs: HashMap::new(),
        }
    }

    /// Create a new tool message with an explicit ID.
    ///
    /// Use this when deserializing or reconstructing messages where the ID must be preserved.
    pub fn with_id(
        id: impl Into<String>,
        content: impl Into<String>,
        tool_call_id: impl Into<String>,
    ) -> Self {
        Self {
            content: content.into(),
            tool_call_id: tool_call_id.into(),
            id: Some(id.into()),
            status: ToolStatus::Success,
            additional_kwargs: HashMap::new(),
        }
    }

    /// Create a new tool message with error status.
    pub fn error(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            tool_call_id: tool_call_id.into(),
            id: Some(Uuid::new_v4().to_string()),
            status: ToolStatus::Error,
            additional_kwargs: HashMap::new(),
        }
    }

    /// Get the message content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the tool call ID this message responds to.
    pub fn tool_call_id(&self) -> &str {
        &self.tool_call_id
    }

    /// Get the message ID.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Get the status of the tool invocation.
    pub fn status(&self) -> &ToolStatus {
        &self.status
    }

    /// Get additional kwargs.
    pub fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value> {
        &self.additional_kwargs
    }
}