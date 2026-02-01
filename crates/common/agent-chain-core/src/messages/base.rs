//! Base message types.
//!
//! This module contains the core `BaseMessage` enum and related traits,
//! mirroring `langchain_core.messages.base`.

use serde::de::{self, MapAccess, Visitor};
use serde::ser::Serializer;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

use super::ai::{AIMessage, AIMessageChunk};
use super::chat::{ChatMessage, ChatMessageChunk};
use super::content::ReasoningContentBlock;
use super::function::{FunctionMessage, FunctionMessageChunk};
use super::human::{HumanMessage, HumanMessageChunk};
use super::modifier::RemoveMessage;
use super::system::{SystemMessage, SystemMessageChunk};
use super::tool::{ToolCall, ToolMessage, ToolMessageChunk};
use crate::utils::merge::merge_lists;

/// A unified message type that can represent any message role.
///
/// This corresponds to `BaseMessage` in LangChain Python.
#[derive(Debug, Clone, PartialEq)]
pub enum BaseMessage {
    /// A human message
    Human(HumanMessage),
    /// A system message
    System(SystemMessage),
    /// An AI message
    AI(AIMessage),
    /// A tool result message
    Tool(ToolMessage),
    /// A chat message with arbitrary role
    Chat(ChatMessage),
    /// A function message (deprecated, use Tool)
    Function(FunctionMessage),
    /// A remove message (for message deletion)
    Remove(RemoveMessage),
}

impl Serialize for BaseMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            BaseMessage::Human(m) => m.serialize(serializer),
            BaseMessage::System(m) => m.serialize(serializer),
            BaseMessage::AI(m) => m.serialize(serializer),
            BaseMessage::Tool(m) => m.serialize(serializer),
            BaseMessage::Chat(m) => m.serialize(serializer),
            BaseMessage::Function(m) => m.serialize(serializer),
            BaseMessage::Remove(m) => m.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for BaseMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BaseMessageVisitor;

        impl<'de> Visitor<'de> for BaseMessageVisitor {
            type Value = BaseMessage;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a message object with a 'type' field")
            }

            fn visit_map<M>(self, mut map: M) -> Result<BaseMessage, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut message_type: Option<String> = None;
                let mut fields: serde_json::Map<String, Value> = serde_json::Map::new();

                while let Some(key) = map.next_key::<String>()? {
                    let value: Value = map.next_value()?;
                    if key == "type" {
                        message_type = value.as_str().map(|s| s.to_string());
                    }
                    fields.insert(key, value);
                }

                let message_type = message_type.ok_or_else(|| de::Error::missing_field("type"))?;

                let json_value = Value::Object(fields);

                match message_type.as_str() {
                    "human" => {
                        let msg: HumanMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessage::Human(msg))
                    }
                    "system" => {
                        let msg: SystemMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessage::System(msg))
                    }
                    "ai" => {
                        let msg: AIMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessage::AI(msg))
                    }
                    "tool" => {
                        let msg: ToolMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessage::Tool(msg))
                    }
                    "chat" => {
                        let msg: ChatMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessage::Chat(msg))
                    }
                    "function" => {
                        let msg: FunctionMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessage::Function(msg))
                    }
                    "remove" => {
                        let msg: RemoveMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessage::Remove(msg))
                    }
                    _ => Err(de::Error::unknown_variant(
                        &message_type,
                        &[
                            "human", "system", "ai", "tool", "chat", "function", "remove",
                        ],
                    )),
                }
            }
        }

        deserializer.deserialize_map(BaseMessageVisitor)
    }
}

impl BaseMessage {
    /// Get the message content as a string reference.
    ///
    /// For messages with multimodal content, this returns the first text content
    /// or an empty string.
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
    pub fn id(&self) -> Option<String> {
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
    pub fn name(&self) -> Option<String> {
        match self {
            BaseMessage::Human(m) => m.name(),
            BaseMessage::System(m) => m.name(),
            BaseMessage::AI(m) => m.name(),
            BaseMessage::Tool(m) => m.name(),
            BaseMessage::Chat(m) => m.name(),
            BaseMessage::Function(m) => Some(m.name().to_string()),
            BaseMessage::Remove(_) => None,
        }
    }

    /// Set id of the message.
    pub fn set_id(&mut self, id: String) {
        match self {
            BaseMessage::Human(m) => m.set_id(id),
            BaseMessage::System(m) => m.set_id(id),
            BaseMessage::AI(m) => m.set_id(id),
            BaseMessage::Tool(m) => m.set_id(id),
            BaseMessage::Chat(m) => m.set_id(id),
            BaseMessage::Function(m) => m.set_id(id),
            BaseMessage::Remove(m) => m.set_id(id),
        }
    }

    /// Get the text content of the message as a string.
    ///
    /// This extracts text from both simple string content and list content
    /// (filtering for text blocks). Corresponds to the `text` property in Python.
    pub fn text(&self) -> String {
        match self {
            BaseMessage::Human(m) => m.message_content().as_text(),
            BaseMessage::System(m) => m.content().to_string(),
            BaseMessage::AI(m) => m.content().to_string(),
            BaseMessage::Tool(m) => m.content().to_string(),
            BaseMessage::Chat(m) => m.content().to_string(),
            BaseMessage::Function(m) => m.content().to_string(),
            BaseMessage::Remove(_) => String::new(),
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
                        println!("Tool Call: {} ({:?})", tc.name(), tc.id());
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
    ///
    /// # Arguments
    ///
    /// * `html` - Whether to format the message with bold text (using ANSI codes).
    ///   Named `html` for Python compatibility but actually uses terminal codes.
    pub fn pretty_repr(&self, html: bool) -> String {
        let msg_type = self.message_type();
        let title_cased = title_case(msg_type);
        let title = format!("{} Message", title_cased);
        let title = get_msg_title_repr(&title, html);

        let name_line = if let Some(name) = self.name() {
            format!("\nName: {}", name)
        } else {
            String::new()
        };

        format!("{}{}\n\n{}", title, name_line, self.content())
    }
}

/// Trait for types that have an optional ID.
/// Used for message merging operations.
pub trait HasId {
    /// Get the ID if present.
    fn get_id(&self) -> Option<String>;
}

impl HasId for BaseMessage {
    fn get_id(&self) -> Option<String> {
        self.id().clone()
    }
}

/// A message chunk enum that represents streaming message chunks.
///
/// This corresponds to `BaseMessageChunk` in LangChain Python.

#[derive(Debug, Clone, PartialEq)]
pub enum BaseMessageChunk {
    /// An AI message chunk
    AI(AIMessageChunk),
    /// A human message chunk
    Human(HumanMessageChunk),
    /// A system message chunk
    System(SystemMessageChunk),
    /// A tool message chunk
    Tool(ToolMessageChunk),
    /// A chat message chunk
    Chat(ChatMessageChunk),
    /// A function message chunk
    Function(FunctionMessageChunk),
}

impl Serialize for BaseMessageChunk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            BaseMessageChunk::AI(m) => m.serialize(serializer),
            BaseMessageChunk::Human(m) => m.serialize(serializer),
            BaseMessageChunk::System(m) => m.serialize(serializer),
            BaseMessageChunk::Tool(m) => m.serialize(serializer),
            BaseMessageChunk::Chat(m) => m.serialize(serializer),
            BaseMessageChunk::Function(m) => m.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for BaseMessageChunk {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BaseMessageChunkVisitor;

        impl<'de> Visitor<'de> for BaseMessageChunkVisitor {
            type Value = BaseMessageChunk;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a message chunk object with a 'type' field")
            }

            fn visit_map<M>(self, mut map: M) -> Result<BaseMessageChunk, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut message_type: Option<String> = None;
                let mut fields: serde_json::Map<String, Value> = serde_json::Map::new();

                while let Some(key) = map.next_key::<String>()? {
                    let value: Value = map.next_value()?;
                    if key == "type" {
                        message_type = value.as_str().map(|s| s.to_string());
                    }
                    fields.insert(key, value);
                }

                let message_type = message_type.ok_or_else(|| de::Error::missing_field("type"))?;

                let json_value = Value::Object(fields);

                match message_type.as_str() {
                    "AIMessageChunk" => {
                        let msg: AIMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessageChunk::AI(msg))
                    }
                    "HumanMessageChunk" => {
                        let msg: HumanMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessageChunk::Human(msg))
                    }
                    "SystemMessageChunk" => {
                        let msg: SystemMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessageChunk::System(msg))
                    }
                    "ToolMessageChunk" => {
                        let msg: ToolMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessageChunk::Tool(msg))
                    }
                    "ChatMessageChunk" => {
                        let msg: ChatMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessageChunk::Chat(msg))
                    }
                    "FunctionMessageChunk" => {
                        let msg: FunctionMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessageChunk::Function(msg))
                    }
                    _ => Err(de::Error::unknown_variant(
                        &message_type,
                        &[
                            "AIMessageChunk",
                            "HumanMessageChunk",
                            "SystemMessageChunk",
                            "ToolMessageChunk",
                            "ChatMessageChunk",
                            "FunctionMessageChunk",
                        ],
                    )),
                }
            }
        }

        deserializer.deserialize_map(BaseMessageChunkVisitor)
    }
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
    pub fn id(&self) -> Option<String> {
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

impl From<AIMessage> for BaseMessage {
    fn from(message: AIMessage) -> Self {
        BaseMessage::AI(message)
    }
}

impl From<HumanMessage> for BaseMessage {
    fn from(message: HumanMessage) -> Self {
        BaseMessage::Human(message)
    }
}

impl From<SystemMessage> for BaseMessage {
    fn from(message: SystemMessage) -> Self {
        BaseMessage::System(message)
    }
}

impl From<ToolMessage> for BaseMessage {
    fn from(message: ToolMessage) -> Self {
        BaseMessage::Tool(message)
    }
}

impl From<ChatMessage> for BaseMessage {
    fn from(message: ChatMessage) -> Self {
        BaseMessage::Chat(message)
    }
}

impl From<FunctionMessage> for BaseMessage {
    fn from(message: FunctionMessage) -> Self {
        BaseMessage::Function(message)
    }
}

impl From<RemoveMessage> for BaseMessage {
    fn from(message: RemoveMessage) -> Self {
        BaseMessage::Remove(message)
    }
}

/// Content type for merge operations.
///
/// Represents message content that can be either a string or a list of values.
#[derive(Debug, Clone, PartialEq)]
pub enum MergeableContent {
    /// String content.
    Text(String),
    /// List content (strings or dicts).
    List(Vec<Value>),
}

impl From<String> for MergeableContent {
    fn from(s: String) -> Self {
        MergeableContent::Text(s)
    }
}

impl From<&str> for MergeableContent {
    fn from(s: &str) -> Self {
        MergeableContent::Text(s.to_string())
    }
}

impl From<Vec<Value>> for MergeableContent {
    fn from(v: Vec<Value>) -> Self {
        MergeableContent::List(v)
    }
}

/// Merge multiple message contents (simple string version).
///
/// Concatenates two strings together. This is the simple version
/// that corresponds to the basic case of `merge_content` in LangChain Python.
///
/// For more complex merging with lists, use `merge_content_complex`.
pub fn merge_content(first: &str, second: &str) -> String {
    let mut result = first.to_string();
    result.push_str(second);
    result
}

/// Merge multiple message contents with support for both strings and lists.
///
/// This function handles merging string contents and list contents together.
/// If both contents are strings, they are concatenated.
/// If one is a string and one is a list, the string is prepended/appended.
/// If both are lists, the lists are concatenated with smart merging.
///
/// This corresponds to the full `merge_content` function in LangChain Python.
///
/// # Arguments
///
/// * `first_content` - The first content to merge.
/// * `contents` - Additional contents to merge.
///
/// # Returns
///
/// The merged content.
pub fn merge_content_complex(
    first_content: Option<MergeableContent>,
    contents: Vec<MergeableContent>,
) -> MergeableContent {
    let mut merged = first_content.unwrap_or(MergeableContent::Text(String::new()));

    for content in contents {
        merged = match (merged, content) {
            (MergeableContent::Text(mut left), MergeableContent::Text(right)) => {
                left.push_str(&right);
                MergeableContent::Text(left)
            }
            (MergeableContent::Text(left), MergeableContent::List(right)) => {
                let mut new_list = vec![Value::String(left)];
                new_list.extend(right);
                MergeableContent::List(new_list)
            }
            (MergeableContent::List(mut left), MergeableContent::List(right)) => {
                if let Ok(Some(merged_list)) =
                    merge_lists(Some(left.clone()), vec![Some(right.clone())])
                {
                    MergeableContent::List(merged_list)
                } else {
                    left.extend(right);
                    MergeableContent::List(left)
                }
            }
            (MergeableContent::List(mut left), MergeableContent::Text(right)) => {
                if !right.is_empty() {
                    if let Some(Value::String(last)) = left.last_mut() {
                        last.push_str(&right);
                    } else if !left.is_empty() {
                        left.push(Value::String(right));
                    }
                }
                MergeableContent::List(left)
            }
        };
    }

    merged
}

/// Merge content vectors (for multimodal content).
pub fn merge_content_vec(first: Vec<Value>, second: Vec<Value>) -> Vec<Value> {
    let mut result = first;
    result.extend(second);
    result
}

/// Convert a Message to a dictionary.
///
/// This corresponds to `message_to_dict` in LangChain Python.
/// The dict will have a `type` key with the message type and a `data` key
/// with the message data as a dict (all fields serialized).
pub fn message_to_dict(message: &BaseMessage) -> Value {
    // Serialize the message using serde - this includes the "type" field
    let mut data = serde_json::to_value(message).unwrap_or_default();

    // Extract the type from the serialized data (it's included by the Serialize impl)
    let msg_type = message.message_type();

    // Remove the "type" field from data since we'll put it at the top level
    if let Some(obj) = data.as_object_mut() {
        obj.remove("type");
    }

    serde_json::json!({
        "type": msg_type,
        "data": data
    })
}

/// Convert a sequence of Messages to a list of dictionaries.
///
/// This corresponds to `messages_to_dict` in LangChain Python.
pub fn messages_to_dict(messages: &[BaseMessage]) -> Vec<serde_json::Value> {
    messages.iter().map(message_to_dict).collect()
}

/// Get a title representation for a message.
///
/// # Arguments
///
/// * `title` - The title to format.
/// * `bold` - Whether to bold the title using ANSI escape codes.
///
/// # Returns
///
/// The formatted title representation.
pub fn get_msg_title_repr(title: &str, bold: bool) -> String {
    let padded = format!(" {} ", title);
    let sep_len = (80 - padded.len()) / 2;
    let sep: String = "=".repeat(sep_len);
    let second_sep = if padded.len() % 2 == 0 {
        sep.clone()
    } else {
        format!("{}=", sep)
    };

    if bold {
        let bolded = get_bolded_text(&padded);
        format!("{}{}{}", sep, bolded, second_sep)
    } else {
        format!("{}{}{}", sep, padded, second_sep)
    }
}

/// Get bolded text using ANSI escape codes.
///
/// Corresponds to `get_bolded_text` in Python's `langchain_core.utils.input`.
pub fn get_bolded_text(text: &str) -> String {
    format!("\x1b[1m{}\x1b[0m", text)
}

/// Convert a string to title case (capitalize first letter of each word).
fn title_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    let upper = first.to_uppercase().to_string();
                    upper + &chars.as_str().to_lowercase()
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract `reasoning_content` from `additional_kwargs`.
///
/// Handles reasoning content stored in various formats:
/// - `additional_kwargs["reasoning_content"]` (string) - Ollama, DeepSeek, XAI, Groq
///
/// Corresponds to `_extract_reasoning_from_additional_kwargs` in Python.
///
/// # Arguments
///
/// * `additional_kwargs` - The additional_kwargs dictionary from a message.
///
/// # Returns
///
/// A `ReasoningContentBlock` if reasoning content is found, None otherwise.
pub fn extract_reasoning_from_additional_kwargs(
    additional_kwargs: &HashMap<String, Value>,
) -> Option<ReasoningContentBlock> {
    if let Some(Value::String(reasoning_content)) = additional_kwargs.get("reasoning_content") {
        Some(ReasoningContentBlock::new(reasoning_content.clone()))
    } else {
        None
    }
}

/// Check if running in an interactive environment.
///
/// In Rust, this always returns false as we don't have the same
/// IPython/Jupyter detection available. Applications can override
/// behavior based on their own environment detection.
pub fn is_interactive_env() -> bool {
    false
}
