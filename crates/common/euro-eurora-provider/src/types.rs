//! Response types for gRPC providers.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use ferrous_llm_core::types::*;

use crate::proto::chat::{
    ProtoAudioPart, ProtoChatRequest, ProtoChatResponse, ProtoContentPart, ProtoFinishReason,
    ProtoFunctionCall, ProtoImagePart, ProtoImageSource, ProtoMessage, ProtoMessageContent,
    ProtoMetadata, ProtoMultimodalContent, ProtoParameters, ProtoRole, ProtoTextPart,
    ProtoToolCall, ProtoToolContent, ProtoUsage, proto_content_part::ProtoPartType,
    proto_image_source::ProtoSourceType, proto_message_content::ProtoContentType,
};

impl From<Metadata> for ProtoMetadata {
    fn from(metadata: Metadata) -> Self {
        ProtoMetadata {
            extensions: Some(hashmap_to_proto_struct(&metadata.extensions)),
            request_id: metadata.request_id,
            user_id: metadata.user_id,
            created_at: Some(datetime_to_proto_timestamp(&metadata.created_at)),
        }
    }
}

impl From<ProtoMetadata> for Metadata {
    fn from(metadata: ProtoMetadata) -> Self {
        Metadata {
            extensions: proto_struct_to_hashmap(metadata.extensions),
            request_id: metadata.request_id,
            user_id: metadata.user_id,
            created_at: proto_timestamp_to_datetime(metadata.created_at),
        }
    }
}

impl From<Parameters> for ProtoParameters {
    fn from(params: Parameters) -> Self {
        ProtoParameters {
            temperature: params.temperature,
            max_tokens: params.max_tokens,
            top_p: params.top_p,
            top_k: params.top_k,
            stop_sequences: params.stop_sequences,
            presence_penalty: params.presence_penalty,
            frequency_penalty: params.frequency_penalty,
        }
    }
}

impl From<FunctionCall> for ProtoFunctionCall {
    fn from(call: FunctionCall) -> Self {
        ProtoFunctionCall {
            name: call.name,
            arguments: call.arguments,
        }
    }
}

impl From<ToolCall> for ProtoToolCall {
    fn from(call: ToolCall) -> Self {
        ProtoToolCall {
            id: call.id,
            call_type: call.call_type,
            function: Some(call.function.into()),
        }
    }
}

impl From<ProtoToolCall> for ToolCall {
    fn from(call: ProtoToolCall) -> Self {
        ToolCall {
            id: call.id,
            call_type: call.call_type,
            function: call
                .function
                .map(Into::into)
                .unwrap_or_else(|| FunctionCall {
                    name: String::new(),
                    arguments: String::new(),
                }),
        }
    }
}

impl From<ProtoFunctionCall> for FunctionCall {
    fn from(call: ProtoFunctionCall) -> Self {
        FunctionCall {
            name: call.name,
            arguments: call.arguments,
        }
    }
}

impl From<ContentPart> for ProtoContentPart {
    fn from(part: ContentPart) -> Self {
        let part_type = match part {
            ferrous_llm_core::types::ContentPart::Text { text } => {
                ProtoPartType::Text(ProtoTextPart { text })
            }
            ferrous_llm_core::types::ContentPart::Image {
                image_source,
                detail,
            } => {
                let source = image_source.into();

                ProtoPartType::Image(ProtoImagePart {
                    image_source: Some(source),
                    detail,
                })
            }
            ferrous_llm_core::types::ContentPart::Audio { audio_url, format } => {
                ProtoPartType::Audio(ProtoAudioPart { audio_url, format })
            }
        };

        ProtoContentPart {
            proto_part_type: Some(part_type),
        }
    }
}

impl From<ProtoContentPart> for ContentPart {
    fn from(part: ProtoContentPart) -> Self {
        match part.proto_part_type {
            Some(ProtoPartType::Text(text)) => ContentPart::Text { text: text.text },
            Some(ProtoPartType::Image(image)) => ContentPart::Image {
                image_source: image.image_source.expect("Image source is required").into(),
                detail: image.detail,
            },
            Some(ProtoPartType::Audio(audio)) => ContentPart::Audio {
                audio_url: audio.audio_url,
                format: audio.format,
            },
            None => ContentPart::Text {
                text: String::new(),
            },
        }
    }
}

impl From<ProtoMessageContent> for MessageContent {
    fn from(content: ProtoMessageContent) -> Self {
        match content.proto_content_type {
            Some(ProtoContentType::Text(text)) => MessageContent::Text(text),
            Some(ProtoContentType::Multimodal(parts)) => {
                let parts = parts
                    .parts
                    .into_iter()
                    .map(|part| part.into())
                    .collect::<Vec<_>>();

                MessageContent::Multimodal(parts)
            }
            Some(ProtoContentType::Tool(tool_content)) => {
                let tool_calls = tool_content
                    .tool_calls
                    .into_iter()
                    .map(|call| call.into())
                    .collect::<Vec<_>>();

                MessageContent::Tool(ToolContent {
                    tool_calls: Some(tool_calls),
                    tool_call_id: tool_content.tool_call_id,
                    text: tool_content.text,
                })
            }
            None => MessageContent::Text(String::new()),
        }
    }
}

impl From<MessageContent> for ProtoMessageContent {
    fn from(content: MessageContent) -> Self {
        let content_type = match content {
            ferrous_llm_core::types::MessageContent::Text(text) => ProtoContentType::Text(text),
            ferrous_llm_core::types::MessageContent::Multimodal(parts) => {
                let proto_parts = parts
                    .into_iter()
                    .map(|part| part.into())
                    .collect::<Vec<_>>();

                ProtoContentType::Multimodal(ProtoMultimodalContent { parts: proto_parts })
            }
            ferrous_llm_core::types::MessageContent::Tool(tool_content) => {
                let tool_calls = tool_content
                    .tool_calls
                    .unwrap_or_default()
                    .into_iter()
                    .map(|call| call.into())
                    .collect::<Vec<_>>();

                ProtoContentType::Tool(ProtoToolContent {
                    tool_calls,
                    tool_call_id: tool_content.tool_call_id,
                    text: tool_content.text,
                })
            }
        };

        ProtoMessageContent {
            proto_content_type: Some(content_type),
        }
    }
}

impl From<Message> for ProtoMessage {
    fn from(message: Message) -> Self {
        ProtoMessage {
            // Double into here because prost treats all defined enums as i32 when used as properties
            role: ProtoRole::from(message.role).into(),
            content: Some(message.content.into()),
        }
    }
}

impl From<ProtoMessage> for Message {
    fn from(message: ProtoMessage) -> Self {
        let proto_role: ProtoRole = ProtoRole::try_from(message.role).unwrap_or_default();
        Message {
            role: proto_role.into(),
            content: message.content.unwrap().into(),
        }
    }
}

impl From<Role> for ProtoRole {
    fn from(role: Role) -> Self {
        match role {
            Role::User => ProtoRole::RoleUser,
            Role::Assistant => ProtoRole::RoleAssistant,
            Role::System => ProtoRole::RoleSystem,
            Role::Tool => ProtoRole::RoleTool,
        }
    }
}

impl From<ProtoRole> for Role {
    fn from(role: ProtoRole) -> Self {
        match role {
            ProtoRole::RoleUser => Role::User,
            ProtoRole::RoleAssistant => Role::Assistant,
            ProtoRole::RoleSystem => Role::System,
            ProtoRole::RoleTool => Role::Tool,
            ProtoRole::RoleUnspecified => Role::User,
        }
    }
}

impl From<ChatRequest> for ProtoChatRequest {
    fn from(request: ChatRequest) -> Self {
        ProtoChatRequest {
            messages: request.messages.into_iter().map(Into::into).collect(),
            parameters: Some(request.parameters.into()),
            metadata: Some(request.metadata.into()),
        }
    }
}

impl From<ProtoUsage> for Usage {
    fn from(usage: ProtoUsage) -> Self {
        Usage {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
        }
    }
}

impl From<ProtoFinishReason> for FinishReason {
    fn from(reason: ProtoFinishReason) -> Self {
        match reason {
            ProtoFinishReason::FinishReasonStop => FinishReason::Stop,
            ProtoFinishReason::FinishReasonLength => FinishReason::Length,
            ProtoFinishReason::FinishReasonContentFilter => FinishReason::ContentFilter,
            ProtoFinishReason::FinishReasonStopSequence => FinishReason::StopSequence,
            ProtoFinishReason::FinishReasonToolCalls => FinishReason::ToolCalls,
            ProtoFinishReason::FinishReasonError => FinishReason::Error,

            ProtoFinishReason::FinishReasonUnspecified => FinishReason::Stop,
        }
    }
}

impl ChatResponse for ProtoChatResponse {
    fn content(&self) -> String {
        self.content.clone()
    }

    fn usage(&self) -> Option<Usage> {
        self.usage.map(Into::into)
    }

    fn finish_reason(&self) -> Option<FinishReason> {
        Some(FinishReason::from(self.finish_reason()))
    }

    fn metadata(&self) -> Metadata {
        self.metadata.clone().map(Into::into).unwrap_or_default()
    }

    fn tool_calls(&self) -> Option<Vec<ToolCall>> {
        Some(
            self.tool_calls
                .clone()
                .into_iter()
                .map(|tool_calls| tool_calls.into())
                .collect::<Vec<_>>(),
        )
    }
}

/// Convert proto Timestamp to DateTime<Utc>
pub fn proto_timestamp_to_datetime(timestamp: Option<prost_types::Timestamp>) -> DateTime<Utc> {
    timestamp
        .map(|ts| DateTime::from_timestamp(ts.seconds, ts.nanos as u32).unwrap_or(Utc::now()))
        .unwrap_or(Utc::now())
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

impl From<ImageSource> for ProtoImageSource {
    fn from(source: ImageSource) -> Self {
        match source {
            ImageSource::Url(url) => ProtoImageSource {
                proto_source_type: Some(ProtoSourceType::Url(url)),
            },
            ImageSource::DynamicImage(image) => {
                // Convert image to RGB
                let rgb_image = image.to_rgb8();
                // Encode the image as PNG bytes
                let mut buffer = std::io::Cursor::new(Vec::new());
                if let Err(e) = rgb_image.write_to(&mut buffer, image::ImageFormat::Png) {
                    tracing::error!("Failed to encode image as PNG: {}", e);
                    // Fallback to empty bytes if encoding fails
                    return ProtoImageSource {
                        proto_source_type: Some(ProtoSourceType::Data(Vec::new())),
                    };
                }

                ProtoImageSource {
                    proto_source_type: Some(ProtoSourceType::Data(buffer.into_inner())),
                }
            }
        }
    }
}

impl From<ProtoImageSource> for ImageSource {
    fn from(source: ProtoImageSource) -> Self {
        match source.proto_source_type {
            Some(ProtoSourceType::Url(url)) => ImageSource::Url(url),
            Some(ProtoSourceType::Data(data)) => ImageSource::DynamicImage(
                image::load_from_memory(&data).expect("Failed to load image"),
            ),
            None => ImageSource::Url(String::new()),
        }
    }
}
