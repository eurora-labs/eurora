use agent_chain::{
    AIMessage, BaseMessage, HumanMessage, MessageContent, SystemMessage, ToolCall, ToolMessage,
};
use be_remote_db::{Message, MessageType};

use crate::{ThreadServiceError, ThreadServiceResult};

/// Convert a database message to a BaseMessage.
///
/// The database stores content as JSONB which can be:
/// - A simple JSON string: `"hello"` -> `MessageContent::Text`
/// - A MessageContent object: `{"Text": "..."}` or `{"Parts": [...]}` -> `MessageContent`
///
/// This function handles all message types: Human, System, AI, and Tool.
pub fn convert_db_message_to_base_message(db_message: Message) -> ThreadServiceResult<BaseMessage> {
    let id = db_message.id.to_string();

    match db_message.message_type {
        MessageType::Human => {
            let content = parse_message_content(&db_message.content)?;
            let message = match content {
                MessageContent::Text(text) => HumanMessage::builder().id(id).content(text).build(),
                MessageContent::Parts(parts) => {
                    HumanMessage::builder().id(id).content(parts).build()
                }
            };
            Ok(BaseMessage::Human(message))
        }
        MessageType::System => {
            let content = parse_message_content(&db_message.content)?;
            let message = SystemMessage::builder().id(id).content(content).build();
            Ok(BaseMessage::System(message))
        }
        MessageType::Ai => {
            let content = parse_ai_content(&db_message.content)?;
            let tool_calls = parse_tool_calls(&db_message.tool_calls)?;
            let message = AIMessage::builder()
                .id(id)
                .content(content)
                .tool_calls(tool_calls)
                .build();
            Ok(BaseMessage::AI(message))
        }
        MessageType::Tool => {
            let content = parse_ai_content(&db_message.content)?;
            let tool_call_id = db_message.tool_call_id.ok_or_else(|| {
                ThreadServiceError::Internal("Tool message missing tool_call_id".to_string())
            })?;
            let message = ToolMessage::builder()
                .id(id)
                .content(content)
                .tool_call_id(tool_call_id)
                .build();
            Ok(BaseMessage::Tool(message))
        }
    }
}

/// Parse message content from JSON value for Human/System messages.
///
/// Handles two formats:
/// 1. A plain JSON string: `"hello"` -> `MessageContent::Text("hello")`
/// 2. A serialized MessageContent: `{"Text": "..."}` or `{"Parts": [...]}`
fn parse_message_content(content: &serde_json::Value) -> ThreadServiceResult<MessageContent> {
    if let Some(text) = content.as_str() {
        return Ok(MessageContent::Text(text.to_string()));
    }

    serde_json::from_value(content.clone()).map_err(|e| {
        ThreadServiceError::Internal(format!("Failed to parse message content: {}", e))
    })
}

/// Parse content from JSON value for AI/Tool messages.
///
/// AI and Tool messages store content as a simple string.
/// Handles two formats:
/// 1. A plain JSON string: `"hello"` -> `"hello"`
/// 2. Any other JSON value: serialized to string
fn parse_ai_content(content: &serde_json::Value) -> ThreadServiceResult<String> {
    if let Some(text) = content.as_str() {
        return Ok(text.to_string());
    }

    Ok(serde_json::to_string(content).unwrap_or_default())
}

/// Parse tool calls from an optional JSON value.
///
/// The database stores tool_calls as a JSON array of ToolCall objects.
fn parse_tool_calls(tool_calls: &Option<serde_json::Value>) -> ThreadServiceResult<Vec<ToolCall>> {
    match tool_calls {
        None => Ok(Vec::new()),
        Some(serde_json::Value::Null) => Ok(Vec::new()),
        Some(value) => serde_json::from_value(value.clone()).map_err(|e| {
            ThreadServiceError::Internal(format!("Failed to parse tool calls: {}", e))
        }),
    }
}
