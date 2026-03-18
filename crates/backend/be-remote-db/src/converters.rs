use crate::error::DbError;
use crate::types::{Message, MessageType, Thread};
use chrono::DateTime;
use prost_types::Timestamp;
use proto_gen::agent_chain::{
    ProtoAiMessage, ProtoBaseMessage, ProtoHumanMessage, ProtoMessageContent, ProtoSystemMessage,
    ProtoToolCall, ProtoToolMessage, ProtoToolStatus, proto_base_message, proto_message_content,
};
use uuid::Uuid;

impl TryFrom<proto_gen::thread::Thread> for Thread {
    type Error = DbError;

    fn try_from(value: proto_gen::thread::Thread) -> Result<Self, Self::Error> {
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

        let active_leaf_id = value
            .active_leaf_id
            .map(|s| Uuid::parse_str(&s))
            .transpose()
            .map_err(|e| DbError::Internal(e.to_string()))?;

        Ok(Thread {
            id,
            user_id,
            title: Some(value.title),
            active_leaf_id,
            created_at,
            updated_at,
        })
    }
}

impl TryInto<proto_gen::thread::Thread> for Thread {
    type Error = DbError;

    fn try_into(self) -> Result<proto_gen::thread::Thread, Self::Error> {
        let id = self.id.to_string();
        let user_id = self.user_id.to_string();
        let title = self.title.unwrap_or_default();

        Ok(proto_gen::thread::Thread {
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
            active_leaf_id: self.active_leaf_id.map(|id| id.to_string()),
        })
    }
}

impl From<Message> for ProtoBaseMessage {
    fn from(msg: Message) -> Self {
        let id = Some(msg.id.to_string());

        let additional_kwargs = {
            let mut kwargs = msg.additional_kwargs.clone();
            if let Some(reasoning) = &msg.reasoning_blocks {
                kwargs["reasoning_blocks"] = reasoning.clone();
            }
            serde_json::to_string(&kwargs).ok()
        };

        match msg.message_type {
            MessageType::Human => {
                let content = text_to_proto_message_content(&msg.content);
                ProtoBaseMessage {
                    message: Some(proto_base_message::Message::Human(ProtoHumanMessage {
                        content: Some(content),
                        id,
                        name: None,
                        additional_kwargs,
                        response_metadata: None,
                    })),
                }
            }
            MessageType::System => {
                let content = text_to_proto_message_content(&msg.content);
                ProtoBaseMessage {
                    message: Some(proto_base_message::Message::System(ProtoSystemMessage {
                        content: Some(content),
                        id,
                        name: None,
                        additional_kwargs,
                        response_metadata: None,
                    })),
                }
            }
            MessageType::Ai => {
                let content = text_to_proto_message_content(&msg.content);
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
                let content = text_to_proto_message_content(&msg.content);
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

fn text_to_proto_message_content(content: &str) -> ProtoMessageContent {
    ProtoMessageContent {
        content: Some(proto_message_content::Content::Text(content.to_string())),
    }
}

fn json_to_proto_tool_calls(tool_calls: &serde_json::Value) -> Vec<ProtoToolCall> {
    let Some(arr) = tool_calls.as_array() else {
        return vec![];
    };

    arr.iter()
        .filter_map(|tc| {
            let id = tc.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
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
            let call_type = tc
                .get("type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            Some(ProtoToolCall {
                id,
                name,
                args,
                call_type,
            })
        })
        .collect()
}
