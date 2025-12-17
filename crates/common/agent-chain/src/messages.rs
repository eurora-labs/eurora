//! Message types for LLM interactions.
//!
//! This module provides message types for different roles (human, AI, system, tool)
//! as well as types for tool calls.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A tool call made by the AI model.
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

/// A human message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HumanMessage {
    /// The message content
    content: String,
    /// Optional unique identifier
    id: Option<String>,
    /// Additional metadata
    #[serde(default)]
    additional_kwargs: HashMap<String, serde_json::Value>,
}

impl HumanMessage {
    /// Create a new human message.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            id: Some(Uuid::new_v4().to_string()),
            additional_kwargs: HashMap::new(),
        }
    }

    /// Get the message content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the message ID.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }
}

/// A system message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemMessage {
    /// The message content
    content: String,
    /// Optional unique identifier
    id: Option<String>,
    /// Additional metadata
    #[serde(default)]
    additional_kwargs: HashMap<String, serde_json::Value>,
}

impl SystemMessage {
    /// Create a new system message.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            id: Some(Uuid::new_v4().to_string()),
            additional_kwargs: HashMap::new(),
        }
    }

    /// Get the message content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the message ID.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }
}

/// An AI message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AIMessage {
    /// The message content
    content: String,
    /// Optional unique identifier
    id: Option<String>,
    /// Tool calls made by the AI
    #[serde(default)]
    tool_calls: Vec<ToolCall>,
    /// Additional metadata
    #[serde(default)]
    additional_kwargs: HashMap<String, serde_json::Value>,
}

impl AIMessage {
    /// Create a new AI message.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            id: Some(Uuid::new_v4().to_string()),
            tool_calls: Vec::new(),
            additional_kwargs: HashMap::new(),
        }
    }

    /// Create a new AI message with tool calls.
    pub fn with_tool_calls(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            content: content.into(),
            id: Some(Uuid::new_v4().to_string()),
            tool_calls,
            additional_kwargs: HashMap::new(),
        }
    }

    /// Get the message content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the message ID.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Get the tool calls.
    pub fn tool_calls(&self) -> &[ToolCall] {
        &self.tool_calls
    }

    /// Add annotations to the message (e.g., citations from web search).
    /// Annotations are stored in additional_kwargs under the "annotations" key.
    pub fn with_annotations<T: Serialize>(mut self, annotations: Vec<T>) -> Self {
        if let Ok(value) = serde_json::to_value(&annotations) {
            self.additional_kwargs
                .insert("annotations".to_string(), value);
        }
        self
    }

    /// Get annotations from the message if present.
    pub fn annotations(&self) -> Option<&serde_json::Value> {
        self.additional_kwargs.get("annotations")
    }

    /// Get additional kwargs.
    pub fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value> {
        &self.additional_kwargs
    }
}

/// A tool message containing the result of a tool call.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolMessage {
    /// The tool result content
    content: String,
    /// The ID of the tool call this message is responding to
    tool_call_id: String,
    /// Optional unique identifier
    id: Option<String>,
    /// Additional metadata
    #[serde(default)]
    additional_kwargs: HashMap<String, serde_json::Value>,
}

impl ToolMessage {
    /// Create a new tool message.
    pub fn new(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            tool_call_id: tool_call_id.into(),
            id: Some(Uuid::new_v4().to_string()),
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
}

/// A unified message type that can represent any message role.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum BaseMessage {
    /// A human message
    Human(HumanMessage),
    /// A system message
    System(SystemMessage),
    /// An AI message
    AI(AIMessage),
    /// A tool result message
    Tool(ToolMessage),
}

impl BaseMessage {
    /// Get the message content.
    pub fn content(&self) -> &str {
        match self {
            BaseMessage::Human(m) => m.content(),
            BaseMessage::System(m) => m.content(),
            BaseMessage::AI(m) => m.content(),
            BaseMessage::Tool(m) => m.content(),
        }
    }

    /// Get the message ID.
    pub fn id(&self) -> Option<&str> {
        match self {
            BaseMessage::Human(m) => m.id(),
            BaseMessage::System(m) => m.id(),
            BaseMessage::AI(m) => m.id(),
            BaseMessage::Tool(m) => m.id(),
        }
    }

    /// Get tool calls if this is an AI message.
    pub fn tool_calls(&self) -> &[ToolCall] {
        match self {
            BaseMessage::AI(m) => m.tool_calls(),
            _ => &[],
        }
    }

    /// Get the message type as a string.
    pub fn message_type(&self) -> &'static str {
        match self {
            BaseMessage::Human(_) => "human",
            BaseMessage::System(_) => "system",
            BaseMessage::AI(_) => "ai",
            BaseMessage::Tool(_) => "tool",
        }
    }
}

impl From<HumanMessage> for BaseMessage {
    fn from(msg: HumanMessage) -> Self {
        BaseMessage::Human(msg)
    }
}

impl From<SystemMessage> for BaseMessage {
    fn from(msg: SystemMessage) -> Self {
        BaseMessage::System(msg)
    }
}

impl From<AIMessage> for BaseMessage {
    fn from(msg: AIMessage) -> Self {
        BaseMessage::AI(msg)
    }
}

impl From<ToolMessage> for BaseMessage {
    fn from(msg: ToolMessage) -> Self {
        BaseMessage::Tool(msg)
    }
}

/// Trait for types that have an optional ID.
/// Used for message merging operations.
pub trait HasId {
    /// Get the ID if present.
    fn get_id(&self) -> Option<&str>;
}

impl HasId for BaseMessage {
    fn get_id(&self) -> Option<&str> {
        self.id()
    }
}

/// Type alias for any message type, matching LangChain's AnyMessage.
/// This is equivalent to BaseMessage but provides naming consistency with Python.
pub type AnyMessage = BaseMessage;

impl BaseMessage {
    /// Pretty print the message to stdout.
    /// This mimics LangChain's pretty_print() method for messages.
    pub fn pretty_print(&self) {
        let (role, content) = match self {
            BaseMessage::Human(m) => ("Human", m.content()),
            BaseMessage::System(m) => ("System", m.content()),
            BaseMessage::AI(m) => {
                let tool_calls = m.tool_calls();
                if tool_calls.is_empty() {
                    ("AI", m.content())
                } else {
                    println!(
                        "================================== AI Message =================================="
                    );
                    if !m.content().is_empty() {
                        println!("{}", m.content());
                    }
                    for tc in tool_calls {
                        println!("Tool Call: {} ({})", tc.name(), tc.id());
                        println!("  Args: {}", tc.args());
                    }
                    return;
                }
            }
            BaseMessage::Tool(m) => {
                println!(
                    "================================= Tool Message ================================="
                );
                println!("[{}] {}", m.tool_call_id(), m.content());
                return;
            }
        };

        let header = format!("=== {} Message ===", role);
        let padding = (80 - header.len()) / 2;
        println!(
            "{:=>padding$}{}{:=>padding$}",
            "",
            header,
            "",
            padding = padding
        );
        println!("{}", content);
    }
}
