//! Type conversions between agent-chain messages and proto types.
//!
//! These conversions are straightforward since the proto types are designed
//! to mirror agent-chain's message structure directly.

use agent_chain_core::{
    AIMessage, BaseMessage, ContentPart, HumanMessage, ImageDetail, ImageSource, MessageContent,
    SystemMessage, ToolCall, ToolMessage,
};
use serde_json;

use crate::proto::chat::{
    ProtoAiMessage, ProtoBase64Image, ProtoBaseMessage, ProtoContentPart, ProtoContentParts,
    ProtoHumanMessage, ProtoImageDetail, ProtoImagePart, ProtoImageSource, ProtoMessageContent,
    ProtoSystemMessage, ProtoTextPart, ProtoToolCall, ProtoToolMessage, ProtoToolStatus,
    proto_base_message::Message as ProtoMessageVariant,
    proto_content_part::Part as ProtoPartVariant,
    proto_image_source::Source as ProtoImageSourceVariant,
    proto_message_content::Content as ProtoContentVariant,
};

// ============================================================================
// BaseMessage conversions
// ============================================================================

impl From<&BaseMessage> for ProtoBaseMessage {
    fn from(msg: &BaseMessage) -> Self {
        let message = match msg {
            BaseMessage::Human(m) => ProtoMessageVariant::Human(m.into()),
            BaseMessage::System(m) => ProtoMessageVariant::System(m.into()),
            BaseMessage::AI(m) => ProtoMessageVariant::Ai(m.into()),
            BaseMessage::Tool(m) => ProtoMessageVariant::Tool(m.into()),
            BaseMessage::Remove(_) => {
                // Remove messages are operational (for message list manipulation)
                // and should not be serialized to proto format.
                // If this is reached, it indicates a logic error in the caller.
                unreachable!("Remove messages should be filtered before proto conversion")
            }
            BaseMessage::Chat(_) | BaseMessage::Function(_) => {
                // Chat and Function messages are deprecated/legacy types
                // and should not be serialized to proto format.
                unreachable!("Chat and Function messages are not supported in proto conversion")
            }
        };
        ProtoBaseMessage {
            message: Some(message),
        }
    }
}

impl From<ProtoBaseMessage> for BaseMessage {
    fn from(msg: ProtoBaseMessage) -> Self {
        match msg.message {
            Some(ProtoMessageVariant::Human(m)) => BaseMessage::Human(m.into()),
            Some(ProtoMessageVariant::System(m)) => BaseMessage::System(m.into()),
            Some(ProtoMessageVariant::Ai(m)) => BaseMessage::AI(m.into()),
            Some(ProtoMessageVariant::Tool(m)) => BaseMessage::Tool(m.into()),
            Some(ProtoMessageVariant::Chat(_))
            | Some(ProtoMessageVariant::Function(_))
            | Some(ProtoMessageVariant::Remove(_)) => {
                // These message types are not supported in agent-chain conversion.
                // Return a default human message as a fallback.
                BaseMessage::Human(HumanMessage::new(""))
            }
            None => BaseMessage::Human(HumanMessage::new("")),
        }
    }
}

// ============================================================================
// HumanMessage conversions
// ============================================================================

impl From<&HumanMessage> for ProtoHumanMessage {
    fn from(msg: &HumanMessage) -> Self {
        ProtoHumanMessage {
            content: Some(msg.message_content().into()),
            id: msg.id().map(String::from),
            name: None,
            additional_kwargs: None,
        }
    }
}

impl From<ProtoHumanMessage> for HumanMessage {
    fn from(msg: ProtoHumanMessage) -> Self {
        match (msg.id, msg.content) {
            (Some(id), Some(content)) => match content.content {
                Some(ProtoContentVariant::Text(text)) => HumanMessage::with_id(id, text),
                Some(ProtoContentVariant::Parts(parts)) => {
                    let content_parts: Vec<ContentPart> =
                        parts.parts.into_iter().map(Into::into).collect();
                    HumanMessage::with_id_and_content(id, content_parts)
                }
                None => HumanMessage::with_id(id, ""),
            },
            (Some(id), None) => HumanMessage::with_id(id, ""),
            (None, Some(content)) => match content.content {
                Some(ProtoContentVariant::Text(text)) => HumanMessage::new(text),
                Some(ProtoContentVariant::Parts(parts)) => {
                    let content_parts: Vec<ContentPart> =
                        parts.parts.into_iter().map(Into::into).collect();
                    HumanMessage::with_content(content_parts)
                }
                None => HumanMessage::new(""),
            },
            (None, None) => HumanMessage::new(""),
        }
    }
}

// ============================================================================
// SystemMessage conversions
// ============================================================================

impl From<&SystemMessage> for ProtoSystemMessage {
    fn from(msg: &SystemMessage) -> Self {
        ProtoSystemMessage {
            content: msg.content().to_string(),
            id: msg.id().map(String::from),
            name: None,
            additional_kwargs: None,
        }
    }
}

impl From<ProtoSystemMessage> for SystemMessage {
    fn from(msg: ProtoSystemMessage) -> Self {
        match msg.id {
            Some(id) => SystemMessage::with_id(id, msg.content),
            None => SystemMessage::new(msg.content),
        }
    }
}

// ============================================================================
// AIMessage conversions
// ============================================================================

impl From<&AIMessage> for ProtoAiMessage {
    fn from(msg: &AIMessage) -> Self {
        ProtoAiMessage {
            content: msg.content().to_string(),
            id: msg.id().map(String::from),
            name: None,
            tool_calls: msg.tool_calls().iter().map(Into::into).collect(),
            invalid_tool_calls: Vec::new(),
            usage_metadata: None,
            additional_kwargs: None,
            response_metadata: None,
        }
    }
}

impl From<ProtoAiMessage> for AIMessage {
    fn from(msg: ProtoAiMessage) -> Self {
        let tool_calls: Vec<ToolCall> = msg.tool_calls.into_iter().map(Into::into).collect();
        match (msg.id, tool_calls.is_empty()) {
            (Some(id), true) => AIMessage::with_id(id, msg.content),
            (Some(id), false) => AIMessage::with_id_and_tool_calls(id, msg.content, tool_calls),
            (None, true) => AIMessage::new(msg.content),
            (None, false) => AIMessage::with_tool_calls(msg.content, tool_calls),
        }
    }
}

// ============================================================================
// ToolMessage conversions
// ============================================================================

impl From<&ToolMessage> for ProtoToolMessage {
    fn from(msg: &ToolMessage) -> Self {
        ProtoToolMessage {
            content: msg.content().to_string(),
            tool_call_id: msg.tool_call_id().to_string(),
            id: msg.id().map(String::from),
            name: None,
            status: ProtoToolStatus::ToolStatusUnspecified as i32,
            artifact: None,
            additional_kwargs: None,
            response_metadata: None,
        }
    }
}

impl From<ProtoToolMessage> for ToolMessage {
    fn from(msg: ProtoToolMessage) -> Self {
        match msg.id {
            Some(id) => ToolMessage::with_id(id, msg.content, msg.tool_call_id),
            None => ToolMessage::new(msg.content, msg.tool_call_id),
        }
    }
}

// ============================================================================
// MessageContent conversions
// ============================================================================

impl From<&MessageContent> for ProtoMessageContent {
    fn from(content: &MessageContent) -> Self {
        let proto_content = match content {
            MessageContent::Text(text) => ProtoContentVariant::Text(text.clone()),
            MessageContent::Parts(parts) => ProtoContentVariant::Parts(ProtoContentParts {
                parts: parts.iter().map(Into::into).collect(),
            }),
        };
        ProtoMessageContent {
            content: Some(proto_content),
        }
    }
}

// ============================================================================
// ContentPart conversions
// ============================================================================

impl From<&ContentPart> for ProtoContentPart {
    fn from(part: &ContentPart) -> Self {
        let proto_part = match part {
            ContentPart::Text { text } => {
                ProtoPartVariant::Text(ProtoTextPart { text: text.clone() })
            }
            ContentPart::Image { source, detail } => ProtoPartVariant::Image(ProtoImagePart {
                source: Some(source.into()),
                detail: detail.as_ref().map(|d| match d {
                    ImageDetail::Low => ProtoImageDetail::ImageDetailLow as i32,
                    ImageDetail::High => ProtoImageDetail::ImageDetailHigh as i32,
                    ImageDetail::Auto => ProtoImageDetail::ImageDetailAuto as i32,
                }),
            }),
        };
        ProtoContentPart {
            part: Some(proto_part),
        }
    }
}

impl From<ProtoContentPart> for ContentPart {
    fn from(part: ProtoContentPart) -> Self {
        match part.part {
            Some(ProtoPartVariant::Text(t)) => ContentPart::Text { text: t.text },
            Some(ProtoPartVariant::Image(img)) => {
                let source = img
                    .source
                    .map(Into::into)
                    .unwrap_or(ImageSource::Url { url: String::new() });
                let detail = img
                    .detail
                    .and_then(|d| match ProtoImageDetail::try_from(d) {
                        Ok(ProtoImageDetail::ImageDetailLow) => Some(ImageDetail::Low),
                        Ok(ProtoImageDetail::ImageDetailHigh) => Some(ImageDetail::High),
                        Ok(ProtoImageDetail::ImageDetailAuto) => Some(ImageDetail::Auto),
                        Ok(ProtoImageDetail::ImageDetailUnspecified) | Err(_) => None,
                    });
                ContentPart::Image { source, detail }
            }
            None => ContentPart::Text {
                text: String::new(),
            },
        }
    }
}

// ============================================================================
// ImageSource conversions
// ============================================================================

impl From<&ImageSource> for ProtoImageSource {
    fn from(source: &ImageSource) -> Self {
        let proto_source = match source {
            ImageSource::Url { url } => ProtoImageSourceVariant::Url(url.clone()),
            ImageSource::Base64 { media_type, data } => {
                ProtoImageSourceVariant::Base64(ProtoBase64Image {
                    media_type: media_type.clone(),
                    data: data.clone(),
                })
            }
        };
        ProtoImageSource {
            source: Some(proto_source),
        }
    }
}

impl From<ProtoImageSource> for ImageSource {
    fn from(source: ProtoImageSource) -> Self {
        match source.source {
            Some(ProtoImageSourceVariant::Url(url)) => ImageSource::Url { url },
            Some(ProtoImageSourceVariant::Base64(b64)) => ImageSource::Base64 {
                media_type: b64.media_type,
                data: b64.data,
            },
            None => ImageSource::Url { url: String::new() },
        }
    }
}

// ============================================================================
// ToolCall conversions
// ============================================================================

impl From<&ToolCall> for ProtoToolCall {
    fn from(tc: &ToolCall) -> Self {
        ProtoToolCall {
            id: tc.id().to_string(),
            name: tc.name().to_string(),
            args: tc.args().to_string(),
        }
    }
}

impl From<ProtoToolCall> for ToolCall {
    fn from(tc: ProtoToolCall) -> Self {
        let args: serde_json::Value = serde_json::from_str(&tc.args).unwrap_or_default();
        ToolCall::with_id(tc.id, tc.name, args)
    }
}
