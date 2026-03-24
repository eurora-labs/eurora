use agent_chain::messages::ContentBlocks;
use agent_chain::{AIMessage, AnyMessage, HumanMessage, SystemMessage, ToolCall, ToolMessage};
use be_remote_db::{Message, MessageType};

use crate::{ThreadServiceError, ThreadServiceResult};

fn parse_content_blocks(value: serde_json::Value) -> ContentBlocks {
    serde_json::from_value(value).unwrap_or_default()
}

pub fn convert_db_message_to_base_message(db_message: Message) -> ThreadServiceResult<AnyMessage> {
    let id = db_message.id.to_string();
    let content = parse_content_blocks(db_message.content);

    match db_message.message_type {
        MessageType::Human => {
            let message = HumanMessage::builder().id(id).content(content).build();
            Ok(AnyMessage::HumanMessage(message))
        }
        MessageType::System => {
            let message = SystemMessage::builder().id(id).content(content).build();
            Ok(AnyMessage::SystemMessage(message))
        }
        MessageType::Ai => {
            let tool_calls = parse_tool_calls(&db_message.tool_calls)?;
            let message = AIMessage::builder()
                .id(id)
                .content(content)
                .tool_calls(tool_calls)
                .build();
            Ok(AnyMessage::AIMessage(message))
        }
        MessageType::Tool => {
            let tool_call_id = db_message.tool_call_id.ok_or_else(|| {
                ThreadServiceError::Internal("Tool message missing tool_call_id".to_string())
            })?;
            let message = ToolMessage::builder()
                .id(id)
                .content(content)
                .tool_call_id(tool_call_id)
                .build();
            Ok(AnyMessage::ToolMessage(message))
        }
    }
}

fn parse_tool_calls(tool_calls: &Option<serde_json::Value>) -> ThreadServiceResult<Vec<ToolCall>> {
    match tool_calls {
        None => Ok(Vec::new()),
        Some(serde_json::Value::Null) => Ok(Vec::new()),
        Some(value) => serde_json::from_value(value.clone()).map_err(|e| {
            ThreadServiceError::Internal(format!("Failed to parse tool calls: {}", e))
        }),
    }
}
