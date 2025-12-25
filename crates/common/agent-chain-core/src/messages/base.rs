//! Base message types.
//!
//! This module contains the core `BaseMessage` enum and related traits,
//! mirroring `langchain_core.messages.base`.

use serde::{Deserialize, Serialize};

#[cfg(feature = "specta")]
use specta::Type;

use super::ai::AIMessage;
use super::human::HumanMessage;
use super::modifier::RemoveMessage;
use super::system::SystemMessage;
use super::tool::{ToolCall, ToolMessage};

/// A unified message type that can represent any message role.
///
/// This corresponds to `BaseMessage` in LangChain Python.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum BaseMessage {
    /// A human message
    #[serde(rename = "human")]
    Human(HumanMessage),
    /// A system message
    #[serde(rename = "system")]
    System(SystemMessage),
    /// An AI message
    #[serde(rename = "ai")]
    AI(AIMessage),
    /// A tool result message
    #[serde(rename = "tool")]
    Tool(ToolMessage),
    /// A remove message (for message deletion)
    #[serde(rename = "remove")]
    Remove(RemoveMessage),
}

impl BaseMessage {
    /// Get the message content.
    pub fn content(&self) -> &str {
        match self {
            BaseMessage::Human(m) => m.content(),
            BaseMessage::System(m) => m.content(),
            BaseMessage::AI(m) => m.content(),
            BaseMessage::Tool(m) => m.content(),
            BaseMessage::Remove(_) => "",
        }
    }

    /// Get the message ID.
    pub fn id(&self) -> Option<&str> {
        match self {
            BaseMessage::Human(m) => m.id(),
            BaseMessage::System(m) => m.id(),
            BaseMessage::AI(m) => m.id(),
            BaseMessage::Tool(m) => m.id(),
            BaseMessage::Remove(m) => m.id(),
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
            BaseMessage::Remove(_) => "remove",
        }
    }

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
            BaseMessage::Remove(m) => {
                println!(
                    "================================ Remove Message ================================"
                );
                if let Some(id) = m.id() {
                    println!("Remove message with id: {}", id);
                }
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

impl From<RemoveMessage> for BaseMessage {
    fn from(msg: RemoveMessage) -> Self {
        BaseMessage::Remove(msg)
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