//! Base message types.
//!
//! This module contains the core `BaseMessage` enum and related traits,
//! mirroring `langchain_core.messages.base`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "specta")]
use specta::Type;

use super::ai::{AIMessage, AIMessageChunk};
use super::chat::{ChatMessage, ChatMessageChunk};
use super::function::{FunctionMessage, FunctionMessageChunk};
use super::human::{HumanMessage, HumanMessageChunk};
use super::modifier::RemoveMessage;
use super::system::{SystemMessage, SystemMessageChunk};
use super::tool::{ToolCall, ToolMessage, ToolMessageChunk};

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
    /// A chat message with arbitrary role
    #[serde(rename = "chat")]
    Chat(ChatMessage),
    /// A function message (deprecated, use Tool)
    #[serde(rename = "function")]
    Function(FunctionMessage),
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
            BaseMessage::Chat(m) => m.content(),
            BaseMessage::Function(m) => m.content(),
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
            BaseMessage::Chat(m) => m.id(),
            BaseMessage::Function(m) => m.id(),
            BaseMessage::Remove(m) => m.id(),
        }
    }

    /// Get the message name if present.
    pub fn name(&self) -> Option<&str> {
        match self {
            BaseMessage::Human(m) => m.name(),
            BaseMessage::System(m) => m.name(),
            BaseMessage::AI(m) => m.name(),
            BaseMessage::Tool(m) => m.name(),
            BaseMessage::Chat(m) => m.name(),
            BaseMessage::Function(_) => None,
            BaseMessage::Remove(_) => None,
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
            BaseMessage::Chat(_) => "chat",
            BaseMessage::Function(_) => "function",
            BaseMessage::Remove(_) => "remove",
        }
    }

    /// Get additional kwargs if present.
    pub fn additional_kwargs(&self) -> Option<&HashMap<String, serde_json::Value>> {
        match self {
            BaseMessage::Human(m) => Some(m.additional_kwargs()),
            BaseMessage::System(m) => Some(m.additional_kwargs()),
            BaseMessage::AI(m) => Some(m.additional_kwargs()),
            BaseMessage::Tool(m) => Some(m.additional_kwargs()),
            BaseMessage::Chat(m) => Some(m.additional_kwargs()),
            BaseMessage::Function(m) => Some(m.additional_kwargs()),
            BaseMessage::Remove(_) => None,
        }
    }

    /// Get response metadata if present.
    pub fn response_metadata(&self) -> Option<&HashMap<String, serde_json::Value>> {
        match self {
            BaseMessage::AI(m) => Some(m.response_metadata()),
            BaseMessage::Chat(m) => Some(m.response_metadata()),
            BaseMessage::Function(m) => Some(m.response_metadata()),
            BaseMessage::Tool(m) => Some(m.response_metadata()),
            _ => None,
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
            BaseMessage::Chat(m) => (m.role(), m.content()),
            BaseMessage::Function(m) => {
                println!(
                    "=============================== Function Message ==============================="
                );
                println!("[{}] {}", m.name(), m.content());
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

    /// Get a pretty representation of the message.
    pub fn pretty_repr(&self, html: bool) -> String {
        let title = format!("{} Message", self.message_type().to_uppercase());
        let title = get_msg_title_repr(&title, html);

        let name_line = if let Some(name) = self.name() {
            format!("\nName: {}", name)
        } else {
            String::new()
        };

        format!("{}{}\n\n{}", title, name_line, self.content())
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

impl From<ChatMessage> for BaseMessage {
    fn from(msg: ChatMessage) -> Self {
        BaseMessage::Chat(msg)
    }
}

impl From<FunctionMessage> for BaseMessage {
    fn from(msg: FunctionMessage) -> Self {
        BaseMessage::Function(msg)
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

/// A message chunk enum that represents streaming message chunks.
///
/// This corresponds to `BaseMessageChunk` in LangChain Python.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum BaseMessageChunk {
    /// An AI message chunk
    #[serde(rename = "AIMessageChunk")]
    AI(AIMessageChunk),
    /// A human message chunk
    #[serde(rename = "HumanMessageChunk")]
    Human(HumanMessageChunk),
    /// A system message chunk
    #[serde(rename = "SystemMessageChunk")]
    System(SystemMessageChunk),
    /// A tool message chunk
    #[serde(rename = "ToolMessageChunk")]
    Tool(ToolMessageChunk),
    /// A chat message chunk
    #[serde(rename = "ChatMessageChunk")]
    Chat(ChatMessageChunk),
    /// A function message chunk
    #[serde(rename = "FunctionMessageChunk")]
    Function(FunctionMessageChunk),
}

impl BaseMessageChunk {
    /// Get the message content.
    pub fn content(&self) -> &str {
        match self {
            BaseMessageChunk::AI(m) => m.content(),
            BaseMessageChunk::Human(m) => m.content(),
            BaseMessageChunk::System(m) => m.content(),
            BaseMessageChunk::Tool(m) => m.content(),
            BaseMessageChunk::Chat(m) => m.content(),
            BaseMessageChunk::Function(m) => m.content(),
        }
    }

    /// Get the message ID.
    pub fn id(&self) -> Option<&str> {
        match self {
            BaseMessageChunk::AI(m) => m.id(),
            BaseMessageChunk::Human(m) => m.id(),
            BaseMessageChunk::System(m) => m.id(),
            BaseMessageChunk::Tool(m) => m.id(),
            BaseMessageChunk::Chat(m) => m.id(),
            BaseMessageChunk::Function(m) => m.id(),
        }
    }

    /// Get the message type as a string.
    pub fn message_type(&self) -> &'static str {
        match self {
            BaseMessageChunk::AI(_) => "AIMessageChunk",
            BaseMessageChunk::Human(_) => "HumanMessageChunk",
            BaseMessageChunk::System(_) => "SystemMessageChunk",
            BaseMessageChunk::Tool(_) => "ToolMessageChunk",
            BaseMessageChunk::Chat(_) => "ChatMessageChunk",
            BaseMessageChunk::Function(_) => "FunctionMessageChunk",
        }
    }

    /// Convert this chunk to a complete message.
    pub fn to_message(&self) -> BaseMessage {
        match self {
            BaseMessageChunk::AI(m) => BaseMessage::AI(m.to_message()),
            BaseMessageChunk::Human(m) => BaseMessage::Human(m.to_message()),
            BaseMessageChunk::System(m) => BaseMessage::System(m.to_message()),
            BaseMessageChunk::Tool(m) => BaseMessage::Tool(m.to_message()),
            BaseMessageChunk::Chat(m) => BaseMessage::Chat(m.to_message()),
            BaseMessageChunk::Function(m) => BaseMessage::Function(m.to_message()),
        }
    }
}

impl From<AIMessageChunk> for BaseMessageChunk {
    fn from(chunk: AIMessageChunk) -> Self {
        BaseMessageChunk::AI(chunk)
    }
}

impl From<HumanMessageChunk> for BaseMessageChunk {
    fn from(chunk: HumanMessageChunk) -> Self {
        BaseMessageChunk::Human(chunk)
    }
}

impl From<SystemMessageChunk> for BaseMessageChunk {
    fn from(chunk: SystemMessageChunk) -> Self {
        BaseMessageChunk::System(chunk)
    }
}

impl From<ToolMessageChunk> for BaseMessageChunk {
    fn from(chunk: ToolMessageChunk) -> Self {
        BaseMessageChunk::Tool(chunk)
    }
}

impl From<ChatMessageChunk> for BaseMessageChunk {
    fn from(chunk: ChatMessageChunk) -> Self {
        BaseMessageChunk::Chat(chunk)
    }
}

impl From<FunctionMessageChunk> for BaseMessageChunk {
    fn from(chunk: FunctionMessageChunk) -> Self {
        BaseMessageChunk::Function(chunk)
    }
}

/// Merge multiple message contents.
///
/// This function handles merging string contents and list contents together.
/// If both contents are strings, they are concatenated.
/// If one is a string and one is a list, the string is prepended/appended.
/// If both are lists, the lists are concatenated with smart merging.
///
/// This corresponds to `merge_content` in LangChain Python.
pub fn merge_content(first: &str, second: &str) -> String {
    let mut result = first.to_string();
    result.push_str(second);
    result
}

/// Merge content vectors (for multimodal content).
pub fn merge_content_vec(
    first: Vec<serde_json::Value>,
    second: Vec<serde_json::Value>,
) -> Vec<serde_json::Value> {
    let mut result = first;
    result.extend(second);
    result
}

/// Convert a Message to a dictionary.
///
/// This corresponds to `message_to_dict` in LangChain Python.
pub fn message_to_dict(message: &BaseMessage) -> serde_json::Value {
    serde_json::json!({
        "type": message.message_type(),
        "data": {
            "content": message.content(),
            "id": message.id(),
            "name": message.name(),
        }
    })
}

/// Convert a sequence of Messages to a list of dictionaries.
///
/// This corresponds to `messages_to_dict` in LangChain Python.
pub fn messages_to_dict(messages: &[BaseMessage]) -> Vec<serde_json::Value> {
    messages.iter().map(message_to_dict).collect()
}

/// Get a title representation for a message.
fn get_msg_title_repr(title: &str, bold: bool) -> String {
    let padded = format!(" {} ", title);
    let sep_len = (80 - padded.len()) / 2;
    let sep: String = "=".repeat(sep_len);
    let second_sep = if padded.len() % 2 == 0 {
        sep.clone()
    } else {
        format!("{}=", sep)
    };

    if bold {
        format!("{}\x1b[1m{}\x1b[0m{}", sep, padded, second_sep)
    } else {
        format!("{}{}{}", sep, padded, second_sep)
    }
}