use agent_chain::{
    AIMessage, BaseMessage, HumanMessage, SystemMessage, ToolCall, ToolMessage,
    messages::MessageContent,
};
use be_remote_db::{Message, MessageType};

use crate::{ConversationServiceError, ConversationServiceResult};

/// Convert a database Message to an agent-chain BaseMessage.
///
/// The database stores content as JSONB with the following formats:
/// - Human: `MessageContent` enum (`{"Text": "..."}` or `{"Parts": [...]}`)
/// - System/AI/Tool: Simple string stored as JSON string (e.g., `"Hello"`)
pub fn convert_db_message_to_base_message(
    db_message: Message,
) -> ConversationServiceResult<BaseMessage> {
    let id = db_message.id.to_string();

    match db_message.message_type {
        MessageType::Human => {
            // Human messages use MessageContent format in the database
            let message_content: MessageContent =
                serde_json::from_value(db_message.content.clone()).map_err(|e| {
                    ConversationServiceError::Internal(format!(
                        "Failed to deserialize HumanMessage content: {}",
                        e
                    ))
                })?;

            let mut human_message = HumanMessage {
                content: message_content,
                id: Some(id),
                name: None,
                additional_kwargs: Default::default(),
            };

            // Parse additional_kwargs if present
            if let Some(kwargs) = db_message.additional_kwargs.as_object() {
                for (k, v) in kwargs {
                    human_message.additional_kwargs.insert(k.clone(), v.clone());
                }
            }

            Ok(BaseMessage::Human(human_message))
        }

        MessageType::System => {
            // System messages store content as a JSON string
            let content = extract_string_content(&db_message.content)?;

            let mut system_message = SystemMessage::with_id(id, content);

            // Parse additional_kwargs if present
            if let Some(kwargs) = db_message.additional_kwargs.as_object() {
                for (k, v) in kwargs {
                    system_message
                        .additional_kwargs
                        .insert(k.clone(), v.clone());
                }
            }

            Ok(BaseMessage::System(system_message))
        }

        MessageType::Ai => {
            // AI messages store content as a JSON string
            let content = extract_string_content(&db_message.content)?;

            // Parse tool_calls if present
            let tool_calls = if let Some(tc_value) = db_message.tool_calls {
                parse_tool_calls(&tc_value)?
            } else {
                Vec::new()
            };

            let mut ai_message = if tool_calls.is_empty() {
                AIMessage::with_id(&id, content)
            } else {
                AIMessage::with_id_and_tool_calls(&id, content, tool_calls)
            };

            // Parse additional_kwargs if present
            if let Some(kwargs) = db_message.additional_kwargs.as_object() {
                for (k, v) in kwargs {
                    ai_message.additional_kwargs.insert(k.clone(), v.clone());
                }
            }

            Ok(BaseMessage::AI(ai_message))
        }

        MessageType::Tool => {
            // Tool messages store content as a JSON string
            let content = extract_string_content(&db_message.content)?;

            let tool_call_id = db_message.tool_call_id.unwrap_or_default();

            let mut tool_message = ToolMessage::with_id(&id, content, tool_call_id);

            // Parse additional_kwargs if present
            if let Some(kwargs) = db_message.additional_kwargs.as_object() {
                for (k, v) in kwargs {
                    tool_message.additional_kwargs.insert(k.clone(), v.clone());
                }
            }

            Ok(BaseMessage::Tool(tool_message))
        }
    }
}

/// Extract string content from a JSON value.
///
/// The database stores simple string content as a JSON string (e.g., `"Hello"`),
/// so we need to extract the inner string value.
fn extract_string_content(content: &serde_json::Value) -> ConversationServiceResult<String> {
    match content {
        serde_json::Value::String(s) => Ok(s.clone()),
        // Handle case where content might be stored as an object with a "text" field
        serde_json::Value::Object(obj) => {
            if let Some(serde_json::Value::String(text)) = obj.get("text") {
                Ok(text.clone())
            } else if let Some(serde_json::Value::String(text)) = obj.get("Text") {
                Ok(text.clone())
            } else {
                // Fallback: serialize the entire object as a string
                Ok(serde_json::to_string(content).unwrap_or_default())
            }
        }
        // For any other type, convert to string representation
        _ => Ok(content.to_string()),
    }
}

/// Parse tool calls from a JSON value.
fn parse_tool_calls(value: &serde_json::Value) -> ConversationServiceResult<Vec<ToolCall>> {
    let tool_calls_array = value.as_array().ok_or_else(|| {
        ConversationServiceError::Internal("tool_calls is not an array".to_string())
    })?;

    let mut tool_calls = Vec::new();
    for tc in tool_calls_array {
        let name = tc
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        let args = tc.get("args").cloned().unwrap_or(serde_json::json!({}));
        let id = tc.get("id").and_then(|i| i.as_str()).map(String::from);

        let tool_call = match id {
            Some(id) => ToolCall::with_id(id, name, args),
            None => ToolCall::new(name, args),
        };
        tool_calls.push(tool_call);
    }

    Ok(tool_calls)
}
