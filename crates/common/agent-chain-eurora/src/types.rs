//! Type conversions between agent-chain messages and proto types.

use std::collections::HashMap;

use agent_chain::{
    AIMessage, BaseMessage, ContentPart, ImageDetail, ImageSource, MessageContent, ToolCall,
};
use chrono::{DateTime, Utc};

use crate::proto::chat::{
    ProtoChatRequest, ProtoChatResponse, ProtoContentPart, ProtoFinishReason, ProtoFunctionCall,
    ProtoImagePart, ProtoImageSource, ProtoMessage, ProtoMessageContent, ProtoMetadata,
    ProtoMultimodalContent, ProtoParameters, ProtoRole, ProtoTextPart, ProtoToolCall,
    ProtoToolContent, proto_content_part::ProtoPartType, proto_image_source::ProtoSourceType,
    proto_message_content::ProtoContentType,
};

/// Convert agent-chain BaseMessage to ProtoMessage
impl From<&BaseMessage> for ProtoMessage {
    fn from(msg: &BaseMessage) -> Self {
        let (role, content) = match msg {
            BaseMessage::Human(m) => {
                let proto_content = match m.message_content() {
                    MessageContent::Text(text) => ProtoMessageContent {
                        proto_content_type: Some(ProtoContentType::Text(text.clone())),
                    },
                    MessageContent::Parts(parts) => {
                        let proto_parts: Vec<ProtoContentPart> =
                            parts.iter().map(|p| p.into()).collect();
                        ProtoMessageContent {
                            proto_content_type: Some(ProtoContentType::Multimodal(
                                ProtoMultimodalContent { parts: proto_parts },
                            )),
                        }
                    }
                };
                (ProtoRole::RoleUser, proto_content)
            }
            BaseMessage::System(m) => (
                ProtoRole::RoleSystem,
                ProtoMessageContent {
                    proto_content_type: Some(ProtoContentType::Text(m.content().to_string())),
                },
            ),
            BaseMessage::AI(m) => {
                let tool_calls = m.tool_calls();
                if tool_calls.is_empty() {
                    (
                        ProtoRole::RoleAssistant,
                        ProtoMessageContent {
                            proto_content_type: Some(ProtoContentType::Text(
                                m.content().to_string(),
                            )),
                        },
                    )
                } else {
                    let proto_tool_calls: Vec<ProtoToolCall> =
                        tool_calls.iter().map(|tc| tc.into()).collect();
                    (
                        ProtoRole::RoleAssistant,
                        ProtoMessageContent {
                            proto_content_type: Some(ProtoContentType::Tool(ProtoToolContent {
                                tool_calls: proto_tool_calls,
                                tool_call_id: None,
                                text: if m.content().is_empty() {
                                    None
                                } else {
                                    Some(m.content().to_string())
                                },
                            })),
                        },
                    )
                }
            }
            BaseMessage::Tool(m) => (
                ProtoRole::RoleTool,
                ProtoMessageContent {
                    proto_content_type: Some(ProtoContentType::Tool(ProtoToolContent {
                        tool_calls: vec![],
                        tool_call_id: Some(m.tool_call_id().to_string()),
                        text: Some(m.content().to_string()),
                    })),
                },
            ),
        };

        ProtoMessage {
            role: role.into(),
            content: Some(content),
        }
    }
}

/// Convert ProtoMessage to agent-chain BaseMessage
impl From<ProtoMessage> for BaseMessage {
    fn from(msg: ProtoMessage) -> Self {
        let role = ProtoRole::try_from(msg.role).unwrap_or(ProtoRole::RoleUser);
        let content = msg.content.and_then(|c| c.proto_content_type);

        match role {
            ProtoRole::RoleUser => {
                let text = extract_text_from_proto_content(content);
                agent_chain::HumanMessage::new(text).into()
            }
            ProtoRole::RoleSystem => {
                let text = extract_text_from_proto_content(content);
                agent_chain::SystemMessage::new(text).into()
            }
            ProtoRole::RoleAssistant => {
                let text = extract_text_from_proto_content(content.clone());
                // Check for tool calls
                if let Some(ProtoContentType::Tool(tool_content)) = content
                    && !tool_content.tool_calls.is_empty()
                {
                    let tool_calls: Vec<ToolCall> = tool_content
                        .tool_calls
                        .into_iter()
                        .map(Into::into)
                        .collect();
                    return AIMessage::with_tool_calls(text, tool_calls).into();
                }
                AIMessage::new(text).into()
            }
            ProtoRole::RoleTool => {
                // Extract tool call ID and content
                if let Some(ProtoContentType::Tool(tool_content)) = content {
                    let tool_call_id = tool_content.tool_call_id.unwrap_or_default();
                    let text = tool_content.text.unwrap_or_default();
                    agent_chain::ToolMessage::new(tool_call_id, text).into()
                } else {
                    // Fallback: treat as human message
                    agent_chain::HumanMessage::new("").into()
                }
            }
            ProtoRole::RoleUnspecified => {
                // Default to human message
                let text = extract_text_from_proto_content(content);
                agent_chain::HumanMessage::new(text).into()
            }
        }
    }
}

/// Helper function to extract text content from proto content type
fn extract_text_from_proto_content(content: Option<ProtoContentType>) -> String {
    match content {
        Some(ProtoContentType::Text(text)) => text,
        Some(ProtoContentType::Multimodal(multimodal)) => {
            // Extract text parts and concatenate them
            multimodal
                .parts
                .into_iter()
                .filter_map(|part| {
                    if let Some(ProtoPartType::Text(text_part)) = part.proto_part_type {
                        Some(text_part.text)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        Some(ProtoContentType::Tool(tool_content)) => tool_content.text.unwrap_or_default(),
        None => String::new(),
    }
}

/// Convert agent-chain ContentPart to ProtoContentPart
impl From<&ContentPart> for ProtoContentPart {
    fn from(part: &ContentPart) -> Self {
        let part_type = match part {
            ContentPart::Text { text } => ProtoPartType::Text(ProtoTextPart { text: text.clone() }),
            ContentPart::Image { source, detail } => {
                let proto_source = match source {
                    ImageSource::Url { url } => ProtoImageSource {
                        proto_source_type: Some(ProtoSourceType::Url(url.clone())),
                    },
                    ImageSource::Base64 { media_type, data } => {
                        // Convert base64 data URL format to just the URL string
                        let url = format!("data:{};base64,{}", media_type, data);
                        ProtoImageSource {
                            proto_source_type: Some(ProtoSourceType::Url(url)),
                        }
                    }
                };
                let detail_str = detail.as_ref().map(|d| match d {
                    ImageDetail::Low => "low".to_string(),
                    ImageDetail::High => "high".to_string(),
                    ImageDetail::Auto => "auto".to_string(),
                });
                ProtoPartType::Image(ProtoImagePart {
                    image_source: Some(proto_source),
                    detail: detail_str,
                })
            }
        };

        ProtoContentPart {
            proto_part_type: Some(part_type),
        }
    }
}

/// Convert agent-chain ToolCall to ProtoToolCall
impl From<&ToolCall> for ProtoToolCall {
    fn from(tc: &ToolCall) -> Self {
        ProtoToolCall {
            id: tc.id().to_string(),
            call_type: "function".to_string(),
            function: Some(ProtoFunctionCall {
                name: tc.name().to_string(),
                arguments: tc.args().to_string(),
            }),
        }
    }
}

/// Convert ProtoToolCall to agent-chain ToolCall
impl From<ProtoToolCall> for ToolCall {
    fn from(tc: ProtoToolCall) -> Self {
        let args: serde_json::Value = tc
            .function
            .as_ref()
            .and_then(|f| serde_json::from_str(&f.arguments).ok())
            .unwrap_or_default();
        let name = tc
            .function
            .as_ref()
            .map(|f| f.name.clone())
            .unwrap_or_default();
        ToolCall::with_id(tc.id, name, args)
    }
}

/// Convert ProtoChatResponse to agent-chain AIMessage
impl From<&ProtoChatResponse> for AIMessage {
    fn from(resp: &ProtoChatResponse) -> Self {
        if resp.tool_calls.is_empty() {
            AIMessage::new(resp.content.clone())
        } else {
            let tool_calls: Vec<ToolCall> =
                resp.tool_calls.iter().cloned().map(Into::into).collect();
            AIMessage::with_tool_calls(resp.content.clone(), tool_calls)
        }
    }
}

/// Convert ProtoFinishReason to a string
pub fn finish_reason_to_string(reason: ProtoFinishReason) -> String {
    match reason {
        ProtoFinishReason::FinishReasonStop => "stop".to_string(),
        ProtoFinishReason::FinishReasonLength => "length".to_string(),
        ProtoFinishReason::FinishReasonContentFilter => "content_filter".to_string(),
        ProtoFinishReason::FinishReasonStopSequence => "stop_sequence".to_string(),
        ProtoFinishReason::FinishReasonToolCalls => "tool_calls".to_string(),
        ProtoFinishReason::FinishReasonError => "error".to_string(),
        ProtoFinishReason::FinishReasonUnspecified => "unspecified".to_string(),
    }
}

/// Build a ProtoChatRequest from agent-chain messages
#[allow(clippy::too_many_arguments)]
pub fn build_proto_request(
    messages: &[BaseMessage],
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    top_k: Option<u32>,
    stop_sequences: Vec<String>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
) -> ProtoChatRequest {
    let proto_messages: Vec<ProtoMessage> = messages.iter().map(Into::into).collect();

    let parameters = ProtoParameters {
        temperature,
        max_tokens,
        top_p,
        top_k,
        stop_sequences,
        frequency_penalty,
        presence_penalty,
    };

    let metadata = ProtoMetadata {
        extensions: None,
        request_id: None,
        user_id: None,
        created_at: Some(datetime_to_proto_timestamp(&Utc::now())),
    };

    ProtoChatRequest {
        messages: proto_messages,
        parameters: Some(parameters),
        metadata: Some(metadata),
    }
}

/// Convert proto Timestamp to DateTime<Utc>
pub fn proto_timestamp_to_datetime(timestamp: Option<prost_types::Timestamp>) -> DateTime<Utc> {
    timestamp
        .and_then(|ts| DateTime::from_timestamp(ts.seconds, ts.nanos as u32))
        .unwrap_or_else(Utc::now)
}

/// Convert DateTime<Utc> to proto Timestamp
pub fn datetime_to_proto_timestamp(datetime: &DateTime<Utc>) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: datetime.timestamp(),
        nanos: datetime.timestamp_subsec_nanos() as i32,
    }
}

/// Convert proto Struct to HashMap<String, serde_json::Value>
pub fn proto_struct_to_hashmap(
    proto_struct: Option<prost_types::Struct>,
) -> HashMap<String, serde_json::Value> {
    proto_struct
        .map(|s| {
            s.fields
                .into_iter()
                .filter_map(|(k, v)| proto_value_to_json_value(v).map(|json_val| (k, json_val)))
                .collect()
        })
        .unwrap_or_default()
}

/// Convert HashMap<String, serde_json::Value> to proto Struct
pub fn hashmap_to_proto_struct(map: &HashMap<String, serde_json::Value>) -> prost_types::Struct {
    let fields = map
        .iter()
        .filter_map(|(k, v)| json_value_to_proto_value(v).map(|proto_val| (k.clone(), proto_val)))
        .collect();

    prost_types::Struct { fields }
}

/// Convert proto Value to serde_json::Value
fn proto_value_to_json_value(value: prost_types::Value) -> Option<serde_json::Value> {
    use prost_types::value::Kind;

    match value.kind? {
        Kind::NullValue(_) => Some(serde_json::Value::Null),
        Kind::NumberValue(n) => Some(serde_json::Value::Number(serde_json::Number::from_f64(n)?)),
        Kind::StringValue(s) => Some(serde_json::Value::String(s)),
        Kind::BoolValue(b) => Some(serde_json::Value::Bool(b)),
        Kind::StructValue(s) => {
            let map = proto_struct_to_hashmap(Some(s));
            Some(serde_json::Value::Object(map.into_iter().collect()))
        }
        Kind::ListValue(l) => {
            let values: Option<Vec<_>> = l
                .values
                .into_iter()
                .map(proto_value_to_json_value)
                .collect();
            Some(serde_json::Value::Array(values?))
        }
    }
}

/// Convert serde_json::Value to proto Value
fn json_value_to_proto_value(value: &serde_json::Value) -> Option<prost_types::Value> {
    use prost_types::value::Kind;

    let kind = match value {
        serde_json::Value::Null => Kind::NullValue(0),
        serde_json::Value::Bool(b) => Kind::BoolValue(*b),
        serde_json::Value::Number(n) => Kind::NumberValue(n.as_f64()?),
        serde_json::Value::String(s) => Kind::StringValue(s.clone()),
        serde_json::Value::Array(arr) => {
            let values: Option<Vec<_>> = arr.iter().map(json_value_to_proto_value).collect();
            Kind::ListValue(prost_types::ListValue { values: values? })
        }
        serde_json::Value::Object(obj) => {
            let map: HashMap<String, serde_json::Value> =
                obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            Kind::StructValue(hashmap_to_proto_struct(&map))
        }
    };

    Some(prost_types::Value { kind: Some(kind) })
}
