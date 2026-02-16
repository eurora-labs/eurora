use crate::error::DbError;
use crate::types::{Conversation, Message, MessageType};
use chrono::DateTime;
use prost_types::Timestamp;
use proto_gen::agent_chain::{
    ProtoAiMessage, ProtoBaseMessage, ProtoContentPart, ProtoContentParts, ProtoHumanMessage,
    ProtoMessageContent, ProtoSystemMessage, ProtoTextPart, ProtoToolCall, ProtoToolMessage,
    ProtoToolStatus, proto_base_message, proto_message_content,
};
use uuid::Uuid;

impl TryFrom<proto_gen::conversation::Conversation> for Conversation {
    type Error = DbError;

    fn try_from(value: proto_gen::conversation::Conversation) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&value.id).map_err(|e| DbError::Internal(e.to_string()))?;
        let user_id =
            Uuid::parse_str(&value.user_id).map_err(|e| DbError::Internal(e.to_string()))?;
        let created_at = value
            .created_at
            .ok_or_else(|| DbError::Internal("Missing created_at".to_string()))?;
        let created_at = DateTime::from_timestamp(created_at.seconds, created_at.nanos as u32)
            .ok_or_else(|| DbError::Internal("Invalid timestamp".to_string()))?;

        let updated_at = value
            .updated_at
            .ok_or_else(|| DbError::Internal("Missing updated_at".to_string()))?;
        let updated_at = DateTime::from_timestamp(updated_at.seconds, updated_at.nanos as u32)
            .ok_or_else(|| DbError::Internal("Invalid timestamp".to_string()))?;

        Ok(Conversation {
            id,
            user_id,
            title: Some(value.title),
            created_at,
            updated_at,
        })
    }
}

impl TryInto<proto_gen::conversation::Conversation> for Conversation {
    type Error = DbError;

    fn try_into(self) -> Result<proto_gen::conversation::Conversation, Self::Error> {
        let id = self.id.to_string();
        let user_id = self.user_id.to_string();
        let title = self.title.unwrap_or_default();

        Ok(proto_gen::conversation::Conversation {
            id,
            user_id,
            title,
            created_at: Some(Timestamp {
                seconds: self.created_at.timestamp(),
                nanos: self.created_at.timestamp_subsec_nanos() as i32,
            }),
            updated_at: Some(Timestamp {
                seconds: self.updated_at.timestamp(),
                nanos: self.updated_at.timestamp_subsec_nanos() as i32,
            }),
        })
    }
}

// =============================================================================
// Message Conversions
// =============================================================================

impl From<Message> for ProtoBaseMessage {
    fn from(msg: Message) -> Self {
        let id = Some(msg.id.to_string());
        let additional_kwargs = serde_json::to_string(&msg.additional_kwargs).ok();

        match msg.message_type {
            MessageType::Human => {
                let content = json_to_proto_message_content(&msg.content);
                ProtoBaseMessage {
                    message: Some(proto_base_message::Message::Human(ProtoHumanMessage {
                        content: Some(content),
                        id,
                        name: None,
                        additional_kwargs,
                    })),
                }
            }
            MessageType::System => {
                let content = json_to_proto_message_content(&msg.content);
                ProtoBaseMessage {
                    message: Some(proto_base_message::Message::System(ProtoSystemMessage {
                        content: Some(content),
                        id,
                        name: None,
                        additional_kwargs,
                    })),
                }
            }
            MessageType::Ai => {
                let content = json_to_proto_message_content(&msg.content);
                let tool_calls = msg
                    .tool_calls
                    .as_ref()
                    .map(json_to_proto_tool_calls)
                    .unwrap_or_default();
                ProtoBaseMessage {
                    message: Some(proto_base_message::Message::Ai(ProtoAiMessage {
                        content: Some(content),
                        id,
                        name: None,
                        tool_calls,
                        invalid_tool_calls: vec![],
                        usage_metadata: None,
                        additional_kwargs,
                        response_metadata: None,
                    })),
                }
            }
            MessageType::Tool => {
                let content = json_to_proto_message_content(&msg.content);
                let tool_call_id = msg.tool_call_id.unwrap_or_default();
                ProtoBaseMessage {
                    message: Some(proto_base_message::Message::Tool(ProtoToolMessage {
                        content: Some(content),
                        tool_call_id,
                        id,
                        name: None,
                        status: ProtoToolStatus::ToolStatusSuccess.into(),
                        artifact: None,
                        additional_kwargs,
                        response_metadata: None,
                    })),
                }
            }
        }
    }
}

/// Convert JSON content to ProtoMessageContent
/// Handles both simple text and multipart content
fn json_to_proto_message_content(content: &serde_json::Value) -> ProtoMessageContent {
    // Try to parse as MessageContent format: {"Text": "..."} or {"Parts": [...]}
    if let Some(text) = content.get("Text").and_then(|v| v.as_str()) {
        return ProtoMessageContent {
            content: Some(proto_message_content::Content::Text(text.to_string())),
        };
    }

    if let Some(parts) = content.get("Parts").and_then(|v| v.as_array()) {
        let proto_parts: Vec<ProtoContentPart> = parts
            .iter()
            .filter_map(|part| {
                if let Some(text_obj) = part.get("Text")
                    && let Some(text) = text_obj.get("text").and_then(|t| t.as_str())
                {
                    return Some(ProtoContentPart {
                        part: Some(proto_gen::agent_chain::proto_content_part::Part::Text(
                            ProtoTextPart {
                                text: text.to_string(),
                            },
                        )),
                    });
                }
                None
            })
            .collect();

        return ProtoMessageContent {
            content: Some(proto_message_content::Content::Parts(ProtoContentParts {
                parts: proto_parts,
            })),
        };
    }

    // Fallback: treat as plain string or serialize to string
    let text = match content {
        serde_json::Value::String(s) => s.clone(),
        _ => content.to_string(),
    };
    ProtoMessageContent {
        content: Some(proto_message_content::Content::Text(text)),
    }
}

/// Convert JSON tool_calls to Vec<ProtoToolCall>
fn json_to_proto_tool_calls(tool_calls: &serde_json::Value) -> Vec<ProtoToolCall> {
    let Some(arr) = tool_calls.as_array() else {
        return vec![];
    };

    arr.iter()
        .filter_map(|tc| {
            let id = tc.get("id").and_then(|v| v.as_str())?.to_string();
            let name = tc.get("name").and_then(|v| v.as_str())?.to_string();
            let args = tc
                .get("args")
                .map(|v| {
                    if v.is_string() {
                        v.as_str().unwrap_or("{}").to_string()
                    } else {
                        v.to_string()
                    }
                })
                .unwrap_or_else(|| "{}".to_string());

            Some(ProtoToolCall { id, name, args })
        })
        .collect()
}
