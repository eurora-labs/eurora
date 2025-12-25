//! Message utility types and functions.
//!
//! This module contains utility types like `AnyMessage` and helper functions
//! for working with messages. Mirrors `langchain_core.messages.utils`.

use super::ai::AIMessage;
use super::base::{BaseMessage, BaseMessageChunk};
use super::chat::ChatMessage;
use super::function::FunctionMessage;
use super::human::HumanMessage;
use super::modifier::RemoveMessage;
use super::system::SystemMessage;
use super::tool::ToolMessage;

/// Type alias for any message type, matching LangChain's AnyMessage.
/// This is equivalent to BaseMessage but provides naming consistency with Python.
pub type AnyMessage = BaseMessage;

/// A type representing the various ways a message can be represented.
///
/// This corresponds to `MessageLikeRepresentation` in LangChain Python.
pub type MessageLikeRepresentation = serde_json::Value;

/// Convert a sequence of messages to a buffer string.
///
/// This concatenates messages with role prefixes for display.
///
/// # Arguments
///
/// * `messages` - The messages to convert.
/// * `human_prefix` - The prefix to prepend to human messages (default: "Human").
/// * `ai_prefix` - The prefix to prepend to AI messages (default: "AI").
///
/// # Returns
///
/// A single string concatenation of all input messages.
pub fn get_buffer_string(
    messages: &[BaseMessage],
    human_prefix: &str,
    ai_prefix: &str,
) -> String {
    messages
        .iter()
        .map(|m| {
            let role = match m {
                BaseMessage::Human(_) => human_prefix,
                BaseMessage::System(_) => "System",
                BaseMessage::AI(_) => ai_prefix,
                BaseMessage::Tool(_) => "Tool",
                BaseMessage::Chat(c) => c.role(),
                BaseMessage::Function(_) => "Function",
                BaseMessage::Remove(_) => "Remove",
            };
            format!("{}: {}", role, m.content())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Convert a message to a dictionary representation.
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

/// Convert a sequence of messages to a list of dictionaries.
///
/// This corresponds to `messages_to_dict` in LangChain Python.
pub fn messages_to_dict(messages: &[BaseMessage]) -> Vec<serde_json::Value> {
    messages.iter().map(message_to_dict).collect()
}

/// Convert a dictionary to a message.
///
/// This corresponds to `_message_from_dict` in LangChain Python.
pub fn message_from_dict(message: &serde_json::Value) -> Result<BaseMessage, String> {
    let msg_type = message
        .get("type")
        .and_then(|t| t.as_str())
        .ok_or_else(|| "Message dict must contain 'type' key".to_string())?;

    let data = message
        .get("data")
        .ok_or_else(|| "Message dict must contain 'data' key".to_string())?;

    let content = data
        .get("content")
        .and_then(|c| c.as_str())
        .unwrap_or("");

    let id = data.get("id").and_then(|i| i.as_str());

    match msg_type {
        "human" => {
            let msg = match id {
                Some(id) => HumanMessage::with_id(id, content),
                None => HumanMessage::new(content),
            };
            Ok(BaseMessage::Human(msg))
        }
        "ai" => {
            let msg = match id {
                Some(id) => AIMessage::with_id(id, content),
                None => AIMessage::new(content),
            };
            Ok(BaseMessage::AI(msg))
        }
        "system" => {
            let msg = match id {
                Some(id) => SystemMessage::with_id(id, content),
                None => SystemMessage::new(content),
            };
            Ok(BaseMessage::System(msg))
        }
        "tool" => {
            let tool_call_id = data
                .get("tool_call_id")
                .and_then(|t| t.as_str())
                .unwrap_or("");
            let msg = match id {
                Some(id) => ToolMessage::with_id(id, content, tool_call_id),
                None => ToolMessage::new(content, tool_call_id),
            };
            Ok(BaseMessage::Tool(msg))
        }
        "chat" => {
            let role = data
                .get("role")
                .and_then(|r| r.as_str())
                .unwrap_or("chat");
            let msg = match id {
                Some(id) => ChatMessage::with_id(id, role, content),
                None => ChatMessage::new(role, content),
            };
            Ok(BaseMessage::Chat(msg))
        }
        "function" => {
            let name = data.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let msg = match id {
                Some(id) => FunctionMessage::with_id(id, name, content),
                None => FunctionMessage::new(name, content),
            };
            Ok(BaseMessage::Function(msg))
        }
        "remove" => {
            let id = id.ok_or_else(|| "RemoveMessage requires an id".to_string())?;
            Ok(BaseMessage::Remove(RemoveMessage::new(id)))
        }
        _ => Err(format!("Unknown message type: {}", msg_type)),
    }
}

/// Convert a sequence of message dicts to messages.
///
/// This corresponds to `messages_from_dict` in LangChain Python.
pub fn messages_from_dict(messages: &[serde_json::Value]) -> Result<Vec<BaseMessage>, String> {
    messages.iter().map(message_from_dict).collect()
}

/// Convert message-like representations to messages.
///
/// This function can convert from:
/// - BaseMessage (returned as-is)
/// - 2-tuple of (role, content) as serde_json::Value
/// - dict with "role"/"type" and "content" keys
/// - string (converted to HumanMessage)
///
/// This corresponds to `convert_to_messages` in LangChain Python.
pub fn convert_to_messages(messages: &[serde_json::Value]) -> Result<Vec<BaseMessage>, String> {
    let mut result = Vec::new();

    for message in messages {
        if let Some(_msg_type) = message.get("type").and_then(|t| t.as_str()) {
            // Already a message dict
            result.push(message_from_dict(message)?);
        } else if let Some(role) = message.get("role").and_then(|r| r.as_str()) {
            // OpenAI-style dict with "role" and "content"
            let content = message
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("");
            let msg = create_message_from_role(role, content)?;
            result.push(msg);
        } else if let Some(s) = message.as_str() {
            // Plain string -> HumanMessage
            result.push(BaseMessage::Human(HumanMessage::new(s)));
        } else if let Some(arr) = message.as_array() {
            // 2-tuple: [role, content]
            if arr.len() == 2 {
                let role = arr[0].as_str().ok_or("First element must be role string")?;
                let content = arr[1].as_str().ok_or("Second element must be content string")?;
                let msg = create_message_from_role(role, content)?;
                result.push(msg);
            } else {
                return Err("Array message must have exactly 2 elements [role, content]".to_string());
            }
        } else {
            return Err(format!("Cannot convert to message: {:?}", message));
        }
    }

    Ok(result)
}

/// Create a message from a role string and content.
fn create_message_from_role(role: &str, content: &str) -> Result<BaseMessage, String> {
    match role {
        "human" | "user" => Ok(BaseMessage::Human(HumanMessage::new(content))),
        "ai" | "assistant" => Ok(BaseMessage::AI(AIMessage::new(content))),
        "system" | "developer" => Ok(BaseMessage::System(SystemMessage::new(content))),
        "function" => Err("Function messages require a name".to_string()),
        "tool" => Err("Tool messages require a tool_call_id".to_string()),
        _ => Ok(BaseMessage::Chat(ChatMessage::new(role, content))),
    }
}

/// Filter messages based on name, type, or ID.
///
/// This corresponds to `filter_messages` in LangChain Python.
pub fn filter_messages(
    messages: &[BaseMessage],
    include_names: Option<&[&str]>,
    exclude_names: Option<&[&str]>,
    include_types: Option<&[&str]>,
    exclude_types: Option<&[&str]>,
    include_ids: Option<&[&str]>,
    exclude_ids: Option<&[&str]>,
) -> Vec<BaseMessage> {
    messages
        .iter()
        .filter(|msg| {
            // Check exclusions first
            if let Some(exclude_names) = exclude_names {
                if let Some(name) = msg.name() {
                    if exclude_names.contains(&name) {
                        return false;
                    }
                }
            }

            if let Some(exclude_types) = exclude_types {
                if exclude_types.contains(&msg.message_type()) {
                    return false;
                }
            }

            if let Some(exclude_ids) = exclude_ids {
                if let Some(id) = msg.id() {
                    if exclude_ids.contains(&id) {
                        return false;
                    }
                }
            }

            // Check inclusions (default to including if no criteria given)
            let include_by_name = include_names.map_or(true, |names| {
                msg.name().map_or(false, |name| names.contains(&name))
            });

            let include_by_type = include_types
                .map_or(true, |types| types.contains(&msg.message_type()));

            let include_by_id = include_ids.map_or(true, |ids| {
                msg.id().map_or(false, |id| ids.contains(&id))
            });

            // If any inclusion criteria is specified, at least one must match
            let any_include_specified =
                include_names.is_some() || include_types.is_some() || include_ids.is_some();

            if any_include_specified {
                include_by_name || include_by_type || include_by_id
            } else {
                true
            }
        })
        .cloned()
        .collect()
}

/// Merge consecutive messages of the same type.
///
/// Note: ToolMessages are not merged, as each has a distinct tool call ID.
///
/// This corresponds to `merge_message_runs` in LangChain Python.
pub fn merge_message_runs(messages: &[BaseMessage], chunk_separator: &str) -> Vec<BaseMessage> {
    if messages.is_empty() {
        return Vec::new();
    }

    let mut merged: Vec<BaseMessage> = Vec::new();

    for msg in messages {
        if merged.is_empty() {
            merged.push(msg.clone());
            continue;
        }

        let last = merged.last().expect("merged is not empty");

        // Don't merge ToolMessages or messages of different types
        if matches!(msg, BaseMessage::Tool(_))
            || std::mem::discriminant(last) != std::mem::discriminant(msg)
        {
            merged.push(msg.clone());
        } else {
            // Same type, merge content
            let last = merged.pop().expect("merged is not empty");
            let merged_content = format!("{}{}{}", last.content(), chunk_separator, msg.content());

            let new_msg = match (last, msg) {
                (BaseMessage::Human(_), BaseMessage::Human(_)) => {
                    BaseMessage::Human(HumanMessage::new(&merged_content))
                }
                (BaseMessage::AI(_), BaseMessage::AI(_)) => {
                    BaseMessage::AI(AIMessage::new(&merged_content))
                }
                (BaseMessage::System(_), BaseMessage::System(_)) => {
                    BaseMessage::System(SystemMessage::new(&merged_content))
                }
                (BaseMessage::Chat(c), BaseMessage::Chat(_)) => {
                    BaseMessage::Chat(ChatMessage::new(c.role(), &merged_content))
                }
                (BaseMessage::Function(f), BaseMessage::Function(_)) => {
                    BaseMessage::Function(FunctionMessage::new(f.name(), &merged_content))
                }
                _ => {
                    // Shouldn't happen due to discriminant check, but handle gracefully
                    merged.push(msg.clone());
                    continue;
                }
            };

            merged.push(new_msg);
        }
    }

    merged
}

/// Convert a message chunk to a complete message.
///
/// This corresponds to `message_chunk_to_message` in LangChain Python.
pub fn message_chunk_to_message(chunk: &BaseMessageChunk) -> BaseMessage {
    chunk.to_message()
}