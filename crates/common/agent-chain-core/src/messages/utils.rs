//! Message utility types and functions.
//!
//! This module contains utility types like `AnyMessage` and helper functions
//! for working with messages. Mirrors `langchain_core.messages.utils`.

use super::base::BaseMessage;

/// Type alias for any message type, matching LangChain's AnyMessage.
/// This is equivalent to BaseMessage but provides naming consistency with Python.
pub type AnyMessage = BaseMessage;

/// Convert a sequence of messages to a buffer string.
///
/// This concatenates messages with role prefixes for display.
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
                BaseMessage::Remove(_) => "Remove",
            };
            format!("{}: {}", role, m.content())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Convert a message to a dictionary representation.
pub fn message_to_dict(message: &BaseMessage) -> serde_json::Value {
    serde_json::json!({
        "type": message.message_type(),
        "content": message.content(),
        "id": message.id(),
    })
}

/// Convert a sequence of messages to a list of dictionaries.
pub fn messages_to_dict(messages: &[BaseMessage]) -> Vec<serde_json::Value> {
    messages.iter().map(message_to_dict).collect()
}