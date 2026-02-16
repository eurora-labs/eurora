use crate::agent_chain::*;
use agent_chain_core::messages::{
    AIMessage, AIMessageChunk, Annotation, AudioContentBlock, BaseMessage, BaseMessageChunk,
    BlockIndex, ChatMessage, ChatMessageChunk, ChunkPosition, ContentBlock, ContentPart,
    FileContentBlock, FunctionMessage, FunctionMessageChunk, HumanMessage, HumanMessageChunk,
    ImageContentBlock, ImageDetail, ImageSource, InputTokenDetails, InvalidToolCall,
    InvalidToolCallBlock, MessageContent, NonStandardContentBlock, OutputTokenDetails,
    PlainTextContentBlock, ReasoningContentBlock, RemoveMessage, ServerToolCall,
    ServerToolCallChunk, ServerToolResult, ServerToolStatus, SystemMessage, SystemMessageChunk,
    TextContentBlock, ToolCall, ToolCallBlock, ToolCallChunk, ToolCallChunkBlock, ToolMessage,
    ToolMessageChunk, ToolStatus, UsageMetadata, VideoContentBlock,
};
use std::collections::HashMap;

fn hashmap_to_json_string(map: &HashMap<String, serde_json::Value>) -> Option<String> {
    if map.is_empty() {
        None
    } else {
        serde_json::to_string(map).ok()
    }
}

fn json_string_to_hashmap(s: &Option<String>) -> HashMap<String, serde_json::Value> {
    s.as_ref()
        .and_then(|json| serde_json::from_str(json).ok())
        .unwrap_or_default()
}

fn value_to_json_string(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn json_string_to_value(s: &Option<String>) -> Option<serde_json::Value> {
    s.as_ref().and_then(|json| serde_json::from_str(json).ok())
}

impl From<InputTokenDetails> for ProtoInputTokenDetails {
    fn from(details: InputTokenDetails) -> Self {
        ProtoInputTokenDetails {
            audio: details.audio,
            cache_creation: details.cache_creation,
            cache_read: details.cache_read,
        }
    }
}

impl From<ProtoInputTokenDetails> for InputTokenDetails {
    fn from(proto: ProtoInputTokenDetails) -> Self {
        InputTokenDetails {
            audio: proto.audio,
            cache_creation: proto.cache_creation,
            cache_read: proto.cache_read,
        }
    }
}

impl From<OutputTokenDetails> for ProtoOutputTokenDetails {
    fn from(details: OutputTokenDetails) -> Self {
        ProtoOutputTokenDetails {
            audio: details.audio,
            reasoning: details.reasoning,
        }
    }
}

impl From<ProtoOutputTokenDetails> for OutputTokenDetails {
    fn from(proto: ProtoOutputTokenDetails) -> Self {
        OutputTokenDetails {
            audio: proto.audio,
            reasoning: proto.reasoning,
        }
    }
}

impl From<UsageMetadata> for ProtoUsageMetadata {
    fn from(usage: UsageMetadata) -> Self {
        ProtoUsageMetadata {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            total_tokens: usage.total_tokens,
            input_token_details: usage.input_token_details.map(Into::into),
            output_token_details: usage.output_token_details.map(Into::into),
        }
    }
}

impl From<ProtoUsageMetadata> for UsageMetadata {
    fn from(proto: ProtoUsageMetadata) -> Self {
        UsageMetadata {
            input_tokens: proto.input_tokens,
            output_tokens: proto.output_tokens,
            total_tokens: proto.total_tokens,
            input_token_details: proto.input_token_details.map(Into::into),
            output_token_details: proto.output_token_details.map(Into::into),
        }
    }
}

impl From<ToolCall> for ProtoToolCall {
    fn from(tc: ToolCall) -> Self {
        ProtoToolCall {
            id: tc.id.clone().unwrap_or_default(),
            name: tc.name.clone(),
            args: value_to_json_string(&tc.args),
        }
    }
}

impl From<ProtoToolCall> for ToolCall {
    fn from(proto: ProtoToolCall) -> Self {
        let args: serde_json::Value = serde_json::from_str(&proto.args)
            .unwrap_or(serde_json::Value::Object(Default::default()));
        if proto.id.is_empty() {
            ToolCall::builder().name(proto.name).args(args).build()
        } else {
            ToolCall::builder()
                .name(proto.name)
                .args(args)
                .id(proto.id)
                .build()
        }
    }
}

impl From<ToolCallChunk> for ProtoToolCallChunk {
    fn from(chunk: ToolCallChunk) -> Self {
        ProtoToolCallChunk {
            name: chunk.name.clone(),
            args: chunk.args.clone(),
            id: chunk.id.clone(),
            index: chunk.index,
        }
    }
}

impl From<ProtoToolCallChunk> for ToolCallChunk {
    fn from(proto: ProtoToolCallChunk) -> Self {
        ToolCallChunk {
            name: proto.name,
            args: proto.args,
            id: proto.id,
            index: proto.index,
            chunk_type: None,
        }
    }
}

impl From<InvalidToolCall> for ProtoInvalidToolCall {
    fn from(itc: InvalidToolCall) -> Self {
        ProtoInvalidToolCall {
            name: itc.name,
            args: itc.args,
            id: itc.id,
            error: itc.error,
        }
    }
}

impl From<ProtoInvalidToolCall> for InvalidToolCall {
    fn from(proto: ProtoInvalidToolCall) -> Self {
        InvalidToolCall {
            name: proto.name,
            args: proto.args,
            id: proto.id,
            error: proto.error,
            call_type: None,
        }
    }
}

impl From<ToolStatus> for ProtoToolStatus {
    fn from(status: ToolStatus) -> Self {
        match status {
            ToolStatus::Success => ProtoToolStatus::ToolStatusSuccess,
            ToolStatus::Error => ProtoToolStatus::ToolStatusError,
        }
    }
}

impl From<ProtoToolStatus> for ToolStatus {
    fn from(proto: ProtoToolStatus) -> Self {
        match proto {
            ProtoToolStatus::ToolStatusUnspecified | ProtoToolStatus::ToolStatusSuccess => {
                ToolStatus::Success
            }
            ProtoToolStatus::ToolStatusError => ToolStatus::Error,
        }
    }
}

pub fn i32_to_tool_status(val: i32) -> ToolStatus {
    match ProtoToolStatus::try_from(val) {
        Ok(ProtoToolStatus::ToolStatusError) => ToolStatus::Error,
        _ => ToolStatus::Success,
    }
}

impl From<ChunkPosition> for ProtoChunkPosition {
    fn from(pos: ChunkPosition) -> Self {
        match pos {
            ChunkPosition::Last => ProtoChunkPosition::ChunkPositionLast,
        }
    }
}

impl From<ProtoChunkPosition> for Option<ChunkPosition> {
    fn from(proto: ProtoChunkPosition) -> Self {
        match proto {
            ProtoChunkPosition::ChunkPositionUnspecified => None,
            ProtoChunkPosition::ChunkPositionLast => Some(ChunkPosition::Last),
        }
    }
}

impl From<ImageDetail> for ProtoImageDetail {
    fn from(detail: ImageDetail) -> Self {
        match detail {
            ImageDetail::Low => ProtoImageDetail::ImageDetailLow,
            ImageDetail::High => ProtoImageDetail::ImageDetailHigh,
            ImageDetail::Auto => ProtoImageDetail::ImageDetailAuto,
        }
    }
}

impl From<ProtoImageDetail> for ImageDetail {
    fn from(proto: ProtoImageDetail) -> Self {
        match proto {
            ProtoImageDetail::ImageDetailUnspecified | ProtoImageDetail::ImageDetailAuto => {
                ImageDetail::Auto
            }
            ProtoImageDetail::ImageDetailLow => ImageDetail::Low,
            ProtoImageDetail::ImageDetailHigh => ImageDetail::High,
        }
    }
}

impl From<ImageSource> for ProtoImageSource {
    fn from(source: ImageSource) -> Self {
        match source {
            ImageSource::Url { url } => ProtoImageSource {
                source: Some(proto_image_source::Source::Url(url)),
            },
            ImageSource::Base64 { media_type, data } => ProtoImageSource {
                source: Some(proto_image_source::Source::Base64(ProtoBase64Image {
                    media_type,
                    data,
                })),
            },
            ImageSource::FileId { file_id } => ProtoImageSource {
                source: Some(proto_image_source::Source::Url(format!(
                    "file://{}",
                    file_id
                ))),
            },
        }
    }
}

impl From<ProtoImageSource> for ImageSource {
    fn from(proto: ProtoImageSource) -> Self {
        match proto.source {
            Some(proto_image_source::Source::Url(url)) => {
                if let Some(file_id) = url.strip_prefix("file://") {
                    ImageSource::FileId {
                        file_id: file_id.to_string(),
                    }
                } else {
                    ImageSource::Url { url }
                }
            }
            Some(proto_image_source::Source::Base64(b64)) => ImageSource::Base64 {
                media_type: b64.media_type,
                data: b64.data,
            },
            None => ImageSource::Url { url: String::new() },
        }
    }
}

impl From<ContentPart> for ProtoContentPart {
    fn from(part: ContentPart) -> Self {
        match part {
            ContentPart::Text { text } => ProtoContentPart {
                part: Some(proto_content_part::Part::Text(ProtoTextPart { text })),
            },
            ContentPart::Image { source, detail } => ProtoContentPart {
                part: Some(proto_content_part::Part::Image(ProtoImagePart {
                    source: Some(source.into()),
                    detail: detail.map(|d| i32::from(ProtoImageDetail::from(d))),
                })),
            },
            ContentPart::Other(value) => {
                let text = serde_json::to_string(&value).unwrap_or_default();
                ProtoContentPart {
                    part: Some(proto_content_part::Part::Text(ProtoTextPart { text })),
                }
            }
        }
    }
}

impl From<ProtoContentPart> for ContentPart {
    fn from(proto: ProtoContentPart) -> Self {
        match proto.part {
            Some(proto_content_part::Part::Text(text_part)) => ContentPart::Text {
                text: text_part.text,
            },
            Some(proto_content_part::Part::Image(image_part)) => ContentPart::Image {
                source: image_part
                    .source
                    .map(Into::into)
                    .unwrap_or(ImageSource::Url { url: String::new() }),
                detail: image_part.detail.map(|d| {
                    ProtoImageDetail::try_from(d)
                        .unwrap_or(ProtoImageDetail::ImageDetailAuto)
                        .into()
                }),
            },
            None => ContentPart::Text {
                text: String::new(),
            },
        }
    }
}

impl From<MessageContent> for ProtoMessageContent {
    fn from(content: MessageContent) -> Self {
        match content {
            MessageContent::Text(text) => ProtoMessageContent {
                content: Some(proto_message_content::Content::Text(text)),
            },
            MessageContent::Parts(parts) => ProtoMessageContent {
                content: Some(proto_message_content::Content::Parts(ProtoContentParts {
                    parts: parts.into_iter().map(Into::into).collect(),
                })),
            },
        }
    }
}

impl From<ProtoMessageContent> for MessageContent {
    fn from(proto: ProtoMessageContent) -> Self {
        match proto.content {
            Some(proto_message_content::Content::Text(text)) => MessageContent::Text(text),
            Some(proto_message_content::Content::Parts(parts)) => {
                MessageContent::Parts(parts.parts.into_iter().map(Into::into).collect())
            }
            None => MessageContent::Text(String::new()),
        }
    }
}

impl From<BlockIndex> for ProtoBlockIndex {
    fn from(index: BlockIndex) -> Self {
        match index {
            BlockIndex::Int(i) => ProtoBlockIndex {
                index: Some(proto_block_index::Index::IntIndex(i)),
            },
            BlockIndex::Str(s) => ProtoBlockIndex {
                index: Some(proto_block_index::Index::StrIndex(s)),
            },
        }
    }
}

impl From<ProtoBlockIndex> for BlockIndex {
    fn from(proto: ProtoBlockIndex) -> Self {
        match proto.index {
            Some(proto_block_index::Index::IntIndex(i)) => BlockIndex::Int(i),
            Some(proto_block_index::Index::StrIndex(s)) => BlockIndex::Str(s),
            None => BlockIndex::Int(0),
        }
    }
}

impl From<Annotation> for ProtoAnnotation {
    fn from(ann: Annotation) -> Self {
        match ann {
            Annotation::Citation {
                id,
                url,
                title,
                start_index,
                end_index,
                cited_text,
                extras,
            } => ProtoAnnotation {
                annotation: Some(proto_annotation::Annotation::Citation(ProtoCitation {
                    id,
                    url,
                    title,
                    start_index,
                    end_index,
                    cited_text,
                    extras: extras.as_ref().and_then(|e| serde_json::to_string(e).ok()),
                })),
            },
            Annotation::NonStandardAnnotation { id, value } => ProtoAnnotation {
                annotation: Some(proto_annotation::Annotation::NonStandard(
                    ProtoNonStandardAnnotation {
                        id,
                        value: serde_json::to_string(&value).unwrap_or_default(),
                    },
                )),
            },
        }
    }
}

impl From<ProtoAnnotation> for Annotation {
    fn from(proto: ProtoAnnotation) -> Self {
        match proto.annotation {
            Some(proto_annotation::Annotation::Citation(citation)) => Annotation::Citation {
                id: citation.id,
                url: citation.url,
                title: citation.title,
                start_index: citation.start_index,
                end_index: citation.end_index,
                cited_text: citation.cited_text,
                extras: citation
                    .extras
                    .as_ref()
                    .and_then(|s| serde_json::from_str(s).ok()),
            },
            Some(proto_annotation::Annotation::NonStandard(ns)) => {
                Annotation::NonStandardAnnotation {
                    id: ns.id,
                    value: serde_json::from_str(&ns.value).unwrap_or_default(),
                }
            }
            None => Annotation::Citation {
                id: None,
                url: None,
                title: None,
                start_index: None,
                end_index: None,
                cited_text: None,
                extras: None,
            },
        }
    }
}

impl From<HumanMessage> for ProtoHumanMessage {
    fn from(msg: HumanMessage) -> Self {
        ProtoHumanMessage {
            content: Some(msg.content.into()),
            id: msg.id,
            name: msg.name,
            additional_kwargs: hashmap_to_json_string(&msg.additional_kwargs),
        }
    }
}

impl From<ProtoHumanMessage> for HumanMessage {
    fn from(proto: ProtoHumanMessage) -> Self {
        HumanMessage::builder()
            .maybe_id(proto.id)
            .content(
                proto
                    .content
                    .map(Into::into)
                    .unwrap_or(MessageContent::Text(String::new())),
            )
            .maybe_name(proto.name)
            .additional_kwargs(json_string_to_hashmap(&proto.additional_kwargs))
            .build()
    }
}

impl From<HumanMessageChunk> for ProtoHumanMessageChunk {
    fn from(chunk: HumanMessageChunk) -> Self {
        ProtoHumanMessageChunk {
            content: Some(chunk.content.into()),
            id: chunk.id,
            name: chunk.name,
            additional_kwargs: hashmap_to_json_string(&chunk.additional_kwargs),
            response_metadata: hashmap_to_json_string(&chunk.response_metadata),
        }
    }
}

impl From<ProtoHumanMessageChunk> for HumanMessageChunk {
    fn from(proto: ProtoHumanMessageChunk) -> Self {
        let content: MessageContent = proto
            .content
            .map(Into::into)
            .unwrap_or(MessageContent::Text(String::new()));

        HumanMessageChunk::builder()
            .maybe_id(proto.id)
            .content(content)
            .maybe_name(proto.name)
            .additional_kwargs(json_string_to_hashmap(&proto.additional_kwargs))
            .response_metadata(json_string_to_hashmap(&proto.response_metadata))
            .build()
    }
}

impl From<SystemMessage> for ProtoSystemMessage {
    fn from(msg: SystemMessage) -> Self {
        ProtoSystemMessage {
            content: Some(msg.content.into()),
            id: msg.id,
            name: msg.name,
            additional_kwargs: hashmap_to_json_string(&msg.additional_kwargs),
        }
    }
}

impl From<ProtoSystemMessage> for SystemMessage {
    fn from(proto: ProtoSystemMessage) -> Self {
        let content: MessageContent = proto
            .content
            .map(Into::into)
            .unwrap_or(MessageContent::Text(String::new()));

        SystemMessage::builder()
            .content(content)
            .maybe_id(proto.id)
            .maybe_name(proto.name)
            .additional_kwargs(json_string_to_hashmap(&proto.additional_kwargs))
            .build()
    }
}

impl From<SystemMessageChunk> for ProtoSystemMessageChunk {
    fn from(chunk: SystemMessageChunk) -> Self {
        ProtoSystemMessageChunk {
            content: Some(chunk.content.clone().into()),
            id: chunk.id.clone(),
            name: chunk.name.clone(),
            additional_kwargs: hashmap_to_json_string(&chunk.additional_kwargs),
            response_metadata: hashmap_to_json_string(&chunk.response_metadata),
        }
    }
}

impl From<ProtoSystemMessageChunk> for SystemMessageChunk {
    fn from(proto: ProtoSystemMessageChunk) -> Self {
        let content: MessageContent = proto
            .content
            .map(Into::into)
            .unwrap_or(MessageContent::Text(String::new()));

        SystemMessageChunk::builder()
            .content(content)
            .maybe_id(proto.id)
            .maybe_name(proto.name)
            .additional_kwargs(json_string_to_hashmap(&proto.additional_kwargs))
            .response_metadata(json_string_to_hashmap(&proto.response_metadata))
            .build()
    }
}

impl From<AIMessage> for ProtoAiMessage {
    fn from(msg: AIMessage) -> Self {
        ProtoAiMessage {
            content: Some(msg.content.into()),
            id: msg.id,
            name: msg.name,
            tool_calls: msg.tool_calls.into_iter().map(Into::into).collect(),
            invalid_tool_calls: msg.invalid_tool_calls.into_iter().map(Into::into).collect(),
            usage_metadata: msg.usage_metadata.map(Into::into),
            additional_kwargs: hashmap_to_json_string(&msg.additional_kwargs),
            response_metadata: hashmap_to_json_string(&msg.response_metadata),
        }
    }
}

impl From<ProtoAiMessage> for AIMessage {
    fn from(proto: ProtoAiMessage) -> Self {
        let content: MessageContent = proto
            .content
            .map(Into::into)
            .unwrap_or(MessageContent::Text(String::new()));
        let tool_calls: Vec<ToolCall> = proto.tool_calls.into_iter().map(Into::into).collect();
        let invalid_tool_calls: Vec<InvalidToolCall> = proto
            .invalid_tool_calls
            .into_iter()
            .map(Into::into)
            .collect();

        let usage_metadata = proto.usage_metadata.map(Into::into);

        AIMessage::builder()
            .maybe_id(proto.id)
            .content(content)
            .maybe_name(proto.name)
            .maybe_usage_metadata(usage_metadata)
            .tool_calls(tool_calls)
            .invalid_tool_calls(invalid_tool_calls)
            .additional_kwargs(json_string_to_hashmap(&proto.additional_kwargs))
            .response_metadata(json_string_to_hashmap(&proto.response_metadata))
            .build()
    }
}

impl From<AIMessageChunk> for ProtoAiMessageChunk {
    fn from(chunk: AIMessageChunk) -> Self {
        let chunk_position = chunk
            .chunk_position()
            .map(|p| i32::from(ProtoChunkPosition::from(p.clone())));

        ProtoAiMessageChunk {
            content: Some(chunk.content.into()),
            id: chunk.id,
            name: chunk.name,
            tool_calls: chunk.tool_calls.into_iter().map(Into::into).collect(),
            invalid_tool_calls: chunk
                .invalid_tool_calls
                .into_iter()
                .map(Into::into)
                .collect(),
            tool_call_chunks: chunk.tool_call_chunks.into_iter().map(Into::into).collect(),
            usage_metadata: chunk.usage_metadata.map(Into::into),
            additional_kwargs: hashmap_to_json_string(&chunk.additional_kwargs),
            response_metadata: hashmap_to_json_string(&chunk.response_metadata),
            chunk_position,
        }
    }
}

impl From<ProtoAiMessageChunk> for AIMessageChunk {
    fn from(proto: ProtoAiMessageChunk) -> Self {
        let content: MessageContent = proto
            .content
            .map(Into::into)
            .unwrap_or(MessageContent::Text(String::new()));
        let tool_call_chunks: Vec<ToolCallChunk> =
            proto.tool_call_chunks.into_iter().map(Into::into).collect();

        let chunk_position: Option<ChunkPosition> = match proto.chunk_position {
            Some(pos) => ProtoChunkPosition::try_from(pos)
                .unwrap_or(ProtoChunkPosition::ChunkPositionUnspecified)
                .into(),
            None => None,
        };

        AIMessageChunk::builder()
            .maybe_id(proto.id)
            .maybe_name(proto.name)
            .tool_call_chunks(tool_call_chunks)
            .content(content)
            .maybe_usage_metadata(proto.usage_metadata.map(Into::into))
            .tool_calls(proto.tool_calls.into_iter().map(Into::into).collect())
            .invalid_tool_calls(
                proto
                    .invalid_tool_calls
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            )
            .additional_kwargs(json_string_to_hashmap(&proto.additional_kwargs))
            .response_metadata(json_string_to_hashmap(&proto.response_metadata))
            .maybe_chunk_position(chunk_position)
            .build()
    }
}

impl From<ToolMessage> for ProtoToolMessage {
    fn from(msg: ToolMessage) -> Self {
        ProtoToolMessage {
            content: Some(msg.content.into()),
            tool_call_id: msg.tool_call_id,
            id: msg.id,
            name: msg.name,
            status: i32::from(ProtoToolStatus::from(msg.status)),
            artifact: msg.artifact.map(|a| value_to_json_string(&a)),
            additional_kwargs: hashmap_to_json_string(&msg.additional_kwargs),
            response_metadata: hashmap_to_json_string(&msg.response_metadata),
        }
    }
}

impl From<ProtoToolMessage> for ToolMessage {
    fn from(proto: ProtoToolMessage) -> Self {
        let content: MessageContent = proto
            .content
            .map(Into::into)
            .unwrap_or(MessageContent::Text(String::new()));
        let status: ToolStatus = i32_to_tool_status(proto.status);

        ToolMessage::builder()
            .content(content)
            .tool_call_id(proto.tool_call_id)
            .maybe_id(proto.id)
            .maybe_name(proto.name)
            .maybe_artifact(json_string_to_value(&proto.artifact))
            .status(status)
            .additional_kwargs(json_string_to_hashmap(&proto.additional_kwargs))
            .response_metadata(json_string_to_hashmap(&proto.response_metadata))
            .build()
    }
}

impl From<ToolMessageChunk> for ProtoToolMessageChunk {
    fn from(chunk: ToolMessageChunk) -> Self {
        ProtoToolMessageChunk {
            content: Some(chunk.content.into()),
            tool_call_id: chunk.tool_call_id.clone(),
            id: chunk.id.clone(),
            name: chunk.name.clone(),
            status: i32::from(ProtoToolStatus::from(chunk.status.clone())),
            artifact: chunk.artifact.as_ref().map(|a| value_to_json_string(a)),
            additional_kwargs: hashmap_to_json_string(&chunk.additional_kwargs),
            response_metadata: hashmap_to_json_string(&chunk.response_metadata),
        }
    }
}

impl From<ProtoToolMessageChunk> for ToolMessageChunk {
    fn from(proto: ProtoToolMessageChunk) -> Self {
        let content: MessageContent = proto
            .content
            .map(Into::into)
            .unwrap_or(MessageContent::Text(String::new()));

        ToolMessageChunk::builder()
            .content(content)
            .tool_call_id(proto.tool_call_id)
            .maybe_id(proto.id)
            .maybe_name(proto.name)
            .status(i32_to_tool_status(proto.status))
            .maybe_artifact(json_string_to_value(&proto.artifact))
            .additional_kwargs(json_string_to_hashmap(&proto.additional_kwargs))
            .response_metadata(json_string_to_hashmap(&proto.response_metadata))
            .build()
    }
}

impl From<ChatMessage> for ProtoChatMessage {
    fn from(msg: ChatMessage) -> Self {
        ProtoChatMessage {
            content: Some(msg.content.into()),
            role: msg.role,
            id: msg.id,
            name: msg.name,
            additional_kwargs: hashmap_to_json_string(&msg.additional_kwargs),
            response_metadata: hashmap_to_json_string(&msg.response_metadata),
        }
    }
}

impl From<ProtoChatMessage> for ChatMessage {
    fn from(proto: ProtoChatMessage) -> Self {
        let content: MessageContent = proto
            .content
            .map(Into::into)
            .unwrap_or(MessageContent::Text(String::new()));

        ChatMessage::builder()
            .content(content)
            .role(proto.role)
            .maybe_id(proto.id)
            .maybe_name(proto.name)
            .additional_kwargs(json_string_to_hashmap(&proto.additional_kwargs))
            .response_metadata(json_string_to_hashmap(&proto.response_metadata))
            .build()
    }
}

impl From<ChatMessageChunk> for ProtoChatMessageChunk {
    fn from(chunk: ChatMessageChunk) -> Self {
        ProtoChatMessageChunk {
            content: Some(chunk.content.into()),
            role: chunk.role.clone(),
            id: chunk.id.clone(),
            name: chunk.name.clone(),
            additional_kwargs: hashmap_to_json_string(&chunk.additional_kwargs),
            response_metadata: hashmap_to_json_string(&chunk.response_metadata),
        }
    }
}

impl From<ProtoChatMessageChunk> for ChatMessageChunk {
    fn from(proto: ProtoChatMessageChunk) -> Self {
        let content: MessageContent = proto
            .content
            .map(Into::into)
            .unwrap_or(MessageContent::Text(String::new()));

        ChatMessageChunk::builder()
            .content(content)
            .role(proto.role)
            .maybe_id(proto.id)
            .maybe_name(proto.name)
            .additional_kwargs(json_string_to_hashmap(&proto.additional_kwargs))
            .response_metadata(json_string_to_hashmap(&proto.response_metadata))
            .build()
    }
}

impl From<FunctionMessage> for ProtoFunctionMessage {
    fn from(msg: FunctionMessage) -> Self {
        ProtoFunctionMessage {
            content: Some(msg.content.into()),
            name: msg.name,
            id: msg.id,
            additional_kwargs: hashmap_to_json_string(&msg.additional_kwargs),
            response_metadata: hashmap_to_json_string(&msg.response_metadata),
        }
    }
}

impl From<ProtoFunctionMessage> for FunctionMessage {
    fn from(proto: ProtoFunctionMessage) -> Self {
        let content: MessageContent = proto
            .content
            .map(Into::into)
            .unwrap_or(MessageContent::Text(String::new()));

        FunctionMessage::builder()
            .content(content)
            .name(proto.name)
            .maybe_id(proto.id)
            .additional_kwargs(json_string_to_hashmap(&proto.additional_kwargs))
            .response_metadata(json_string_to_hashmap(&proto.response_metadata))
            .build()
    }
}

impl From<FunctionMessageChunk> for ProtoFunctionMessageChunk {
    fn from(chunk: FunctionMessageChunk) -> Self {
        ProtoFunctionMessageChunk {
            content: Some(chunk.content.into()),
            name: chunk.name.clone(),
            id: chunk.id.clone(),
            additional_kwargs: hashmap_to_json_string(&chunk.additional_kwargs),
            response_metadata: hashmap_to_json_string(&chunk.response_metadata),
        }
    }
}

impl From<ProtoFunctionMessageChunk> for FunctionMessageChunk {
    fn from(proto: ProtoFunctionMessageChunk) -> Self {
        let content: MessageContent = proto
            .content
            .map(Into::into)
            .unwrap_or(MessageContent::Text(String::new()));

        FunctionMessageChunk::builder()
            .content(content)
            .name(proto.name)
            .maybe_id(proto.id)
            .additional_kwargs(json_string_to_hashmap(&proto.additional_kwargs))
            .response_metadata(json_string_to_hashmap(&proto.response_metadata))
            .build()
    }
}

impl From<RemoveMessage> for ProtoRemoveMessage {
    fn from(msg: RemoveMessage) -> Self {
        ProtoRemoveMessage { id: msg.id }
    }
}

impl From<ProtoRemoveMessage> for RemoveMessage {
    fn from(proto: ProtoRemoveMessage) -> Self {
        RemoveMessage::builder().id(proto.id).build()
    }
}

impl From<BaseMessage> for ProtoBaseMessage {
    fn from(msg: BaseMessage) -> Self {
        match msg {
            BaseMessage::Human(m) => ProtoBaseMessage {
                message: Some(proto_base_message::Message::Human(m.into())),
            },
            BaseMessage::System(m) => ProtoBaseMessage {
                message: Some(proto_base_message::Message::System(m.into())),
            },
            BaseMessage::AI(m) => ProtoBaseMessage {
                message: Some(proto_base_message::Message::Ai(m.into())),
            },
            BaseMessage::Tool(m) => ProtoBaseMessage {
                message: Some(proto_base_message::Message::Tool(m.into())),
            },
            BaseMessage::Chat(m) => ProtoBaseMessage {
                message: Some(proto_base_message::Message::Chat(m.into())),
            },
            BaseMessage::Function(m) => ProtoBaseMessage {
                message: Some(proto_base_message::Message::Function(m.into())),
            },
            BaseMessage::Remove(m) => ProtoBaseMessage {
                message: Some(proto_base_message::Message::Remove(m.into())),
            },
        }
    }
}

impl From<ProtoBaseMessage> for BaseMessage {
    fn from(proto: ProtoBaseMessage) -> Self {
        match proto.message {
            Some(proto_base_message::Message::Human(m)) => BaseMessage::Human(m.into()),
            Some(proto_base_message::Message::System(m)) => BaseMessage::System(m.into()),
            Some(proto_base_message::Message::Ai(m)) => BaseMessage::AI(m.into()),
            Some(proto_base_message::Message::Tool(m)) => BaseMessage::Tool(m.into()),
            Some(proto_base_message::Message::Chat(m)) => BaseMessage::Chat(m.into()),
            Some(proto_base_message::Message::Function(m)) => BaseMessage::Function(m.into()),
            Some(proto_base_message::Message::Remove(m)) => BaseMessage::Remove(m.into()),
            None => BaseMessage::Human(HumanMessage::builder().content("").build()),
        }
    }
}

impl From<BaseMessageChunk> for ProtoBaseMessageChunk {
    fn from(chunk: BaseMessageChunk) -> Self {
        match chunk {
            BaseMessageChunk::AI(c) => ProtoBaseMessageChunk {
                chunk: Some(proto_base_message_chunk::Chunk::Ai(c.into())),
            },
            BaseMessageChunk::Human(c) => ProtoBaseMessageChunk {
                chunk: Some(proto_base_message_chunk::Chunk::Human(c.into())),
            },
            BaseMessageChunk::System(c) => ProtoBaseMessageChunk {
                chunk: Some(proto_base_message_chunk::Chunk::System(c.into())),
            },
            BaseMessageChunk::Tool(c) => ProtoBaseMessageChunk {
                chunk: Some(proto_base_message_chunk::Chunk::Tool(c.into())),
            },
            BaseMessageChunk::Chat(c) => ProtoBaseMessageChunk {
                chunk: Some(proto_base_message_chunk::Chunk::Chat(c.into())),
            },
            BaseMessageChunk::Function(c) => ProtoBaseMessageChunk {
                chunk: Some(proto_base_message_chunk::Chunk::Function(c.into())),
            },
        }
    }
}

impl From<ProtoBaseMessageChunk> for BaseMessageChunk {
    fn from(proto: ProtoBaseMessageChunk) -> Self {
        match proto.chunk {
            Some(proto_base_message_chunk::Chunk::Ai(c)) => BaseMessageChunk::AI(c.into()),
            Some(proto_base_message_chunk::Chunk::Human(c)) => BaseMessageChunk::Human(c.into()),
            Some(proto_base_message_chunk::Chunk::System(c)) => BaseMessageChunk::System(c.into()),
            Some(proto_base_message_chunk::Chunk::Tool(c)) => BaseMessageChunk::Tool(c.into()),
            Some(proto_base_message_chunk::Chunk::Chat(c)) => BaseMessageChunk::Chat(c.into()),
            Some(proto_base_message_chunk::Chunk::Function(c)) => {
                BaseMessageChunk::Function(c.into())
            }
            None => BaseMessageChunk::AI(AIMessageChunk::builder().content("").build()),
        }
    }
}

impl From<TextContentBlock> for ProtoTextContentBlock {
    fn from(block: TextContentBlock) -> Self {
        ProtoTextContentBlock {
            id: block.id,
            text: block.text,
            annotations: block
                .annotations
                .map(|anns| anns.into_iter().map(Into::into).collect())
                .unwrap_or_default(),
            index: block.index.map(Into::into),
            extras: block
                .extras
                .as_ref()
                .and_then(|e| serde_json::to_string(e).ok()),
        }
    }
}

impl From<ProtoTextContentBlock> for TextContentBlock {
    fn from(proto: ProtoTextContentBlock) -> Self {
        TextContentBlock {
            block_type: "text".to_string(),
            id: proto.id,
            text: proto.text,
            annotations: if proto.annotations.is_empty() {
                None
            } else {
                Some(proto.annotations.into_iter().map(Into::into).collect())
            },
            index: proto.index.map(Into::into),
            extras: proto
                .extras
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok()),
        }
    }
}

impl From<ReasoningContentBlock> for ProtoReasoningContentBlock {
    fn from(block: ReasoningContentBlock) -> Self {
        ProtoReasoningContentBlock {
            id: block.id,
            reasoning: block.reasoning,
            index: block.index.map(Into::into),
            extras: block
                .extras
                .as_ref()
                .and_then(|e| serde_json::to_string(e).ok()),
        }
    }
}

impl From<ProtoReasoningContentBlock> for ReasoningContentBlock {
    fn from(proto: ProtoReasoningContentBlock) -> Self {
        ReasoningContentBlock {
            block_type: "reasoning".to_string(),
            id: proto.id,
            reasoning: proto.reasoning,
            index: proto.index.map(Into::into),
            extras: proto
                .extras
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok()),
        }
    }
}

impl From<ImageContentBlock> for ProtoImageContentBlock {
    fn from(block: ImageContentBlock) -> Self {
        ProtoImageContentBlock {
            id: block.id,
            file_id: block.file_id,
            mime_type: block.mime_type,
            index: block.index.map(Into::into),
            url: block.url,
            base64: block.base64,
            extras: block
                .extras
                .as_ref()
                .and_then(|e| serde_json::to_string(e).ok()),
        }
    }
}

impl From<ProtoImageContentBlock> for ImageContentBlock {
    fn from(proto: ProtoImageContentBlock) -> Self {
        ImageContentBlock {
            block_type: "image".to_string(),
            id: proto.id,
            file_id: proto.file_id,
            mime_type: proto.mime_type,
            index: proto.index.map(Into::into),
            url: proto.url,
            base64: proto.base64,
            extras: proto
                .extras
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok()),
        }
    }
}

impl From<VideoContentBlock> for ProtoVideoContentBlock {
    fn from(block: VideoContentBlock) -> Self {
        ProtoVideoContentBlock {
            id: block.id,
            file_id: block.file_id,
            mime_type: block.mime_type,
            index: block.index.map(Into::into),
            url: block.url,
            base64: block.base64,
            extras: block
                .extras
                .as_ref()
                .and_then(|e| serde_json::to_string(e).ok()),
        }
    }
}

impl From<ProtoVideoContentBlock> for VideoContentBlock {
    fn from(proto: ProtoVideoContentBlock) -> Self {
        VideoContentBlock {
            block_type: "video".to_string(),
            id: proto.id,
            file_id: proto.file_id,
            mime_type: proto.mime_type,
            index: proto.index.map(Into::into),
            url: proto.url,
            base64: proto.base64,
            extras: proto
                .extras
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok()),
        }
    }
}

impl From<AudioContentBlock> for ProtoAudioContentBlock {
    fn from(block: AudioContentBlock) -> Self {
        ProtoAudioContentBlock {
            id: block.id,
            file_id: block.file_id,
            mime_type: block.mime_type,
            index: block.index.map(Into::into),
            url: block.url,
            base64: block.base64,
            extras: block
                .extras
                .as_ref()
                .and_then(|e| serde_json::to_string(e).ok()),
        }
    }
}

impl From<ProtoAudioContentBlock> for AudioContentBlock {
    fn from(proto: ProtoAudioContentBlock) -> Self {
        AudioContentBlock {
            block_type: "audio".to_string(),
            id: proto.id,
            file_id: proto.file_id,
            mime_type: proto.mime_type,
            index: proto.index.map(Into::into),
            url: proto.url,
            base64: proto.base64,
            extras: proto
                .extras
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok()),
        }
    }
}

impl From<PlainTextContentBlock> for ProtoPlainTextContentBlock {
    fn from(block: PlainTextContentBlock) -> Self {
        ProtoPlainTextContentBlock {
            id: block.id,
            file_id: block.file_id,
            mime_type: block.mime_type,
            index: block.index.map(Into::into),
            url: block.url,
            base64: block.base64,
            text: block.text,
            title: block.title,
            context: block.context,
            extras: block
                .extras
                .as_ref()
                .and_then(|e| serde_json::to_string(e).ok()),
        }
    }
}

impl From<ProtoPlainTextContentBlock> for PlainTextContentBlock {
    fn from(proto: ProtoPlainTextContentBlock) -> Self {
        PlainTextContentBlock {
            block_type: "text-plain".to_string(),
            id: proto.id,
            file_id: proto.file_id,
            mime_type: proto.mime_type,
            index: proto.index.map(Into::into),
            url: proto.url,
            base64: proto.base64,
            text: proto.text,
            title: proto.title,
            context: proto.context,
            extras: proto
                .extras
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok()),
        }
    }
}

impl From<FileContentBlock> for ProtoFileContentBlock {
    fn from(block: FileContentBlock) -> Self {
        ProtoFileContentBlock {
            id: block.id,
            file_id: block.file_id,
            mime_type: block.mime_type,
            index: block.index.map(Into::into),
            url: block.url,
            base64: block.base64,
            extras: block
                .extras
                .as_ref()
                .and_then(|e| serde_json::to_string(e).ok()),
        }
    }
}

impl From<ProtoFileContentBlock> for FileContentBlock {
    fn from(proto: ProtoFileContentBlock) -> Self {
        FileContentBlock {
            block_type: "file".to_string(),
            id: proto.id,
            file_id: proto.file_id,
            mime_type: proto.mime_type,
            index: proto.index.map(Into::into),
            url: proto.url,
            base64: proto.base64,
            extras: proto
                .extras
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok()),
        }
    }
}

impl From<NonStandardContentBlock> for ProtoNonStandardContentBlock {
    fn from(block: NonStandardContentBlock) -> Self {
        ProtoNonStandardContentBlock {
            id: block.id,
            value: serde_json::to_string(&block.value).unwrap_or_default(),
            index: block.index.map(Into::into),
        }
    }
}

impl From<ProtoNonStandardContentBlock> for NonStandardContentBlock {
    fn from(proto: ProtoNonStandardContentBlock) -> Self {
        NonStandardContentBlock {
            block_type: "non_standard".to_string(),
            id: proto.id,
            value: serde_json::from_str(&proto.value).unwrap_or_default(),
            index: proto.index.map(Into::into),
        }
    }
}

impl From<ToolCallBlock> for ProtoToolCallBlock {
    fn from(block: ToolCallBlock) -> Self {
        ProtoToolCallBlock {
            id: block.id,
            name: block.name,
            args: serde_json::to_string(&block.args).unwrap_or_default(),
            index: block.index.map(Into::into),
            extras: block
                .extras
                .as_ref()
                .and_then(|e| serde_json::to_string(e).ok()),
        }
    }
}

impl From<ProtoToolCallBlock> for ToolCallBlock {
    fn from(proto: ProtoToolCallBlock) -> Self {
        ToolCallBlock {
            block_type: "tool_call".to_string(),
            id: proto.id,
            name: proto.name,
            args: serde_json::from_str(&proto.args).unwrap_or_default(),
            index: proto.index.map(Into::into),
            extras: proto
                .extras
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok()),
        }
    }
}

impl From<ToolCallChunkBlock> for ProtoToolCallChunkBlock {
    fn from(block: ToolCallChunkBlock) -> Self {
        ProtoToolCallChunkBlock {
            id: block.id,
            name: block.name,
            args: block.args,
            index: block.index.map(Into::into),
            extras: block
                .extras
                .as_ref()
                .and_then(|e| serde_json::to_string(e).ok()),
        }
    }
}

impl From<ProtoToolCallChunkBlock> for ToolCallChunkBlock {
    fn from(proto: ProtoToolCallChunkBlock) -> Self {
        ToolCallChunkBlock {
            block_type: "tool_call_chunk".to_string(),
            id: proto.id,
            name: proto.name,
            args: proto.args,
            index: proto.index.map(Into::into),
            extras: proto
                .extras
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok()),
        }
    }
}

impl From<InvalidToolCallBlock> for ProtoInvalidToolCallBlock {
    fn from(block: InvalidToolCallBlock) -> Self {
        ProtoInvalidToolCallBlock {
            id: block.id,
            name: block.name,
            args: block.args,
            error: block.error,
            index: block.index.map(Into::into),
            extras: block
                .extras
                .as_ref()
                .and_then(|e| serde_json::to_string(e).ok()),
        }
    }
}

impl From<ProtoInvalidToolCallBlock> for InvalidToolCallBlock {
    fn from(proto: ProtoInvalidToolCallBlock) -> Self {
        InvalidToolCallBlock {
            block_type: "invalid_tool_call".to_string(),
            id: proto.id,
            name: proto.name,
            args: proto.args,
            error: proto.error,
            index: proto.index.map(Into::into),
            extras: proto
                .extras
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok()),
        }
    }
}

impl From<ServerToolCall> for ProtoServerToolCall {
    fn from(block: ServerToolCall) -> Self {
        ProtoServerToolCall {
            id: block.id,
            name: block.name,
            args: serde_json::to_string(&block.args).unwrap_or_default(),
            index: block.index.map(Into::into),
            extras: block
                .extras
                .as_ref()
                .and_then(|e| serde_json::to_string(e).ok()),
        }
    }
}

impl From<ProtoServerToolCall> for ServerToolCall {
    fn from(proto: ProtoServerToolCall) -> Self {
        ServerToolCall {
            block_type: "server_tool_call".to_string(),
            id: proto.id,
            name: proto.name,
            args: serde_json::from_str(&proto.args).unwrap_or_default(),
            index: proto.index.map(Into::into),
            extras: proto
                .extras
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok()),
        }
    }
}

impl From<ServerToolCallChunk> for ProtoServerToolCallChunk {
    fn from(block: ServerToolCallChunk) -> Self {
        ProtoServerToolCallChunk {
            name: block.name,
            args: block.args,
            id: block.id,
            index: block.index.map(Into::into),
            extras: block
                .extras
                .as_ref()
                .and_then(|e| serde_json::to_string(e).ok()),
        }
    }
}

impl From<ProtoServerToolCallChunk> for ServerToolCallChunk {
    fn from(proto: ProtoServerToolCallChunk) -> Self {
        ServerToolCallChunk {
            block_type: "server_tool_call_chunk".to_string(),
            name: proto.name,
            args: proto.args,
            id: proto.id,
            index: proto.index.map(Into::into),
            extras: proto
                .extras
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok()),
        }
    }
}

impl From<ServerToolStatus> for ProtoServerToolStatus {
    fn from(status: ServerToolStatus) -> Self {
        match status {
            ServerToolStatus::Success => ProtoServerToolStatus::ServerToolStatusSuccess,
            ServerToolStatus::Error => ProtoServerToolStatus::ServerToolStatusError,
        }
    }
}

impl From<ProtoServerToolStatus> for ServerToolStatus {
    fn from(proto: ProtoServerToolStatus) -> Self {
        match proto {
            ProtoServerToolStatus::ServerToolStatusUnspecified
            | ProtoServerToolStatus::ServerToolStatusSuccess => ServerToolStatus::Success,
            ProtoServerToolStatus::ServerToolStatusError => ServerToolStatus::Error,
        }
    }
}

impl From<ServerToolResult> for ProtoServerToolResult {
    fn from(block: ServerToolResult) -> Self {
        ProtoServerToolResult {
            id: block.id,
            tool_call_id: block.tool_call_id,
            status: i32::from(ProtoServerToolStatus::from(block.status)),
            output: block.output.map(|o| value_to_json_string(&o)),
            index: block.index.map(Into::into),
            extras: block
                .extras
                .as_ref()
                .and_then(|e| serde_json::to_string(e).ok()),
        }
    }
}

impl From<ProtoServerToolResult> for ServerToolResult {
    fn from(proto: ProtoServerToolResult) -> Self {
        ServerToolResult {
            block_type: "server_tool_result".to_string(),
            id: proto.id,
            tool_call_id: proto.tool_call_id,
            status: ProtoServerToolStatus::try_from(proto.status)
                .unwrap_or(ProtoServerToolStatus::ServerToolStatusSuccess)
                .into(),
            output: json_string_to_value(&proto.output),
            index: proto.index.map(Into::into),
            extras: proto
                .extras
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok()),
        }
    }
}

impl From<ContentBlock> for ProtoContentBlock {
    fn from(block: ContentBlock) -> Self {
        match block {
            ContentBlock::Text(b) => ProtoContentBlock {
                block: Some(proto_content_block::Block::Text(b.into())),
            },
            ContentBlock::InvalidToolCall(b) => ProtoContentBlock {
                block: Some(proto_content_block::Block::InvalidToolCall(b.into())),
            },
            ContentBlock::Reasoning(b) => ProtoContentBlock {
                block: Some(proto_content_block::Block::Reasoning(b.into())),
            },
            ContentBlock::NonStandard(b) => ProtoContentBlock {
                block: Some(proto_content_block::Block::NonStandard(b.into())),
            },
            ContentBlock::Image(b) => ProtoContentBlock {
                block: Some(proto_content_block::Block::Image(b.into())),
            },
            ContentBlock::Video(b) => ProtoContentBlock {
                block: Some(proto_content_block::Block::Video(b.into())),
            },
            ContentBlock::Audio(b) => ProtoContentBlock {
                block: Some(proto_content_block::Block::Audio(b.into())),
            },
            ContentBlock::PlainText(b) => ProtoContentBlock {
                block: Some(proto_content_block::Block::PlainText(b.into())),
            },
            ContentBlock::File(b) => ProtoContentBlock {
                block: Some(proto_content_block::Block::File(b.into())),
            },
            ContentBlock::ToolCall(b) => ProtoContentBlock {
                block: Some(proto_content_block::Block::ToolCall(b.into())),
            },
            ContentBlock::ToolCallChunk(b) => ProtoContentBlock {
                block: Some(proto_content_block::Block::ToolCallChunk(b.into())),
            },
            ContentBlock::ServerToolCall(b) => ProtoContentBlock {
                block: Some(proto_content_block::Block::ServerToolCall(b.into())),
            },
            ContentBlock::ServerToolCallChunk(b) => ProtoContentBlock {
                block: Some(proto_content_block::Block::ServerToolCallChunk(b.into())),
            },
            ContentBlock::ServerToolResult(b) => ProtoContentBlock {
                block: Some(proto_content_block::Block::ServerToolResult(b.into())),
            },
        }
    }
}

impl From<ProtoContentBlock> for ContentBlock {
    fn from(proto: ProtoContentBlock) -> Self {
        match proto.block {
            Some(proto_content_block::Block::Text(b)) => ContentBlock::Text(b.into()),
            Some(proto_content_block::Block::InvalidToolCall(b)) => {
                ContentBlock::InvalidToolCall(b.into())
            }
            Some(proto_content_block::Block::Reasoning(b)) => ContentBlock::Reasoning(b.into()),
            Some(proto_content_block::Block::NonStandard(b)) => ContentBlock::NonStandard(b.into()),
            Some(proto_content_block::Block::Image(b)) => ContentBlock::Image(b.into()),
            Some(proto_content_block::Block::Video(b)) => ContentBlock::Video(b.into()),
            Some(proto_content_block::Block::Audio(b)) => ContentBlock::Audio(b.into()),
            Some(proto_content_block::Block::PlainText(b)) => ContentBlock::PlainText(b.into()),
            Some(proto_content_block::Block::File(b)) => ContentBlock::File(b.into()),
            Some(proto_content_block::Block::ToolCall(b)) => ContentBlock::ToolCall(b.into()),
            Some(proto_content_block::Block::ToolCallChunk(b)) => {
                ContentBlock::ToolCallChunk(b.into())
            }
            Some(proto_content_block::Block::ServerToolCall(b)) => {
                ContentBlock::ServerToolCall(b.into())
            }
            Some(proto_content_block::Block::ServerToolCallChunk(b)) => {
                ContentBlock::ServerToolCallChunk(b.into())
            }
            Some(proto_content_block::Block::ServerToolResult(b)) => {
                ContentBlock::ServerToolResult(b.into())
            }
            None => ContentBlock::Text(TextContentBlock::new("")),
        }
    }
}
