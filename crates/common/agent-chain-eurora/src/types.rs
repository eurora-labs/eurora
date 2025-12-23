//! Type conversions between agent-chain messages and proto types.
//!
//! These conversions are straightforward since the proto types are designed
//! to mirror agent-chain's message structure directly.

use agent_chain::{
    AIMessage, BaseMessage, ContentPart, HumanMessage, ImageDetail, ImageSource, MessageContent,
    SystemMessage, ToolCall, ToolMessage,
};

use crate::proto::chat::{
    ProtoAiMessage, ProtoBase64Image, ProtoBaseMessage, ProtoContentPart, ProtoContentParts,
    ProtoHumanMessage, ProtoImagePart, ProtoImageSource, ProtoMessageContent, ProtoSystemMessage,
    ProtoTextPart, ProtoToolCall, ProtoToolMessage,
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
        }
    }
}

impl From<ProtoHumanMessage> for HumanMessage {
    fn from(msg: ProtoHumanMessage) -> Self {
        match msg.content {
            Some(content) => match content.content {
                Some(ProtoContentVariant::Text(text)) => HumanMessage::new(text),
                Some(ProtoContentVariant::Parts(parts)) => {
                    let content_parts: Vec<ContentPart> =
                        parts.parts.into_iter().map(Into::into).collect();
                    HumanMessage::with_content(content_parts)
                }
                None => HumanMessage::new(""),
            },
            None => HumanMessage::new(""),
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
        }
    }
}

impl From<ProtoSystemMessage> for SystemMessage {
    fn from(msg: ProtoSystemMessage) -> Self {
        SystemMessage::new(msg.content)
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
            tool_calls: msg.tool_calls().iter().map(Into::into).collect(),
        }
    }
}

impl From<ProtoAiMessage> for AIMessage {
    fn from(msg: ProtoAiMessage) -> Self {
        if msg.tool_calls.is_empty() {
            AIMessage::new(msg.content)
        } else {
            let tool_calls: Vec<ToolCall> = msg.tool_calls.into_iter().map(Into::into).collect();
            AIMessage::with_tool_calls(msg.content, tool_calls)
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
        }
    }
}

impl From<ProtoToolMessage> for ToolMessage {
    fn from(msg: ProtoToolMessage) -> Self {
        ToolMessage::new(msg.content, msg.tool_call_id)
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
                    ImageDetail::Low => "low".to_string(),
                    ImageDetail::High => "high".to_string(),
                    ImageDetail::Auto => "auto".to_string(),
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
                let detail = img.detail.and_then(|d| match d.as_str() {
                    "low" => Some(ImageDetail::Low),
                    "high" => Some(ImageDetail::High),
                    "auto" => Some(ImageDetail::Auto),
                    _ => None,
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
