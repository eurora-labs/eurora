use crate::error::DbError;
use crate::types::{Message, MessageType, Thread};
use agent_chain_core::messages::ContentBlock;
use agent_chain_core::proto::{
    ProtoAiMessage, ProtoBaseMessage, ProtoContentBlock, ProtoHumanMessage, ProtoSystemMessage,
    ProtoToolCall, ProtoToolMessage, ProtoToolStatus, proto_base_message,
};
use chrono::DateTime;
use prost_types::Timestamp;

use uuid::Uuid;

impl TryFrom<proto_gen::thread::ProtoThread> for Thread {
    type Error = DbError;

    fn try_from(value: proto_gen::thread::ProtoThread) -> Result<Self, Self::Error> {
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

impl TryInto<proto_gen::thread::ProtoThread> for Thread {
    type Error = DbError;

    fn try_into(self) -> Result<proto_gen::thread::ProtoThread, Self::Error> {
        let id = self.id.to_string();
        let user_id = self.user_id.to_string();
        let title = self.title.unwrap_or_default();

        Ok(proto_gen::thread::ProtoThread {
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

        let additional_kwargs = serde_json::to_string(&msg.additional_kwargs).ok();

        let content: Vec<ProtoContentBlock> =
            serde_json::from_value::<Vec<ContentBlock>>(msg.content)
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect();

        match msg.message_type {
            MessageType::Human => ProtoBaseMessage {
                message: Some(proto_base_message::Message::Human(ProtoHumanMessage {
                    content,
                    id,
                    name: None,
                    additional_kwargs,
                    response_metadata: None,
                })),
            },
            MessageType::System => ProtoBaseMessage {
                message: Some(proto_base_message::Message::System(ProtoSystemMessage {
                    content,
                    id,
                    name: None,
                    additional_kwargs,
                    response_metadata: None,
                })),
            },
            MessageType::Ai => {
                let tool_calls = msg
                    .tool_calls
                    .as_ref()
                    .map(json_to_proto_tool_calls)
                    .unwrap_or_default();
                ProtoBaseMessage {
                    message: Some(proto_base_message::Message::Ai(ProtoAiMessage {
                        content,
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
                let tool_call_id = msg.tool_call_id.unwrap_or_default();
                ProtoBaseMessage {
                    message: Some(proto_base_message::Message::Tool(ProtoToolMessage {
                        content,
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
