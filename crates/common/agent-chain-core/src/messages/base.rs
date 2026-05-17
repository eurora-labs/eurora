use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::ai::{AIMessage, AIMessageChunk};
use super::chat::{ChatMessage, ChatMessageChunk};
use super::content::{ContentBlocks, ReasoningContentBlock};
use super::human::{HumanMessage, HumanMessageChunk};
use super::modifier::RemoveMessage;
use super::system::{SystemMessage, SystemMessageChunk};
use super::tool::{ToolCall, ToolMessage, ToolMessageChunk};
use crate::utils::merge::merge_lists;

#[enum_dispatch]
pub trait BaseMessage {
    fn id(&self) -> Option<String>;
    fn content(&self) -> &ContentBlocks;
    fn name(&self) -> Option<String>;
    fn set_id(&mut self, id: String);
    fn message_type(&self) -> &'static str;
    fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value>;
    fn response_metadata(&self) -> &HashMap<String, serde_json::Value>;
}

/// Tagged union of every message variant. Wire format is internally tagged on
/// the `type` field — `{"type": "human", "content": ..., ...}` etc. The inner
/// per-variant structs (`HumanMessage`, `AIMessage`, ...) deliberately do NOT
/// carry their own `type` field; the discriminant is added at the union level
/// and only there. This keeps the Rust types and the specta-generated TS types
/// in lockstep with the actual JSON shape.
#[enum_dispatch(BaseMessage)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "type")]
pub enum AnyMessage {
    #[serde(rename = "human")]
    HumanMessage(HumanMessage),
    #[serde(rename = "system")]
    SystemMessage(SystemMessage),
    #[serde(rename = "ai")]
    AIMessage(AIMessage),
    #[serde(rename = "tool")]
    ToolMessage(ToolMessage),
    #[serde(rename = "chat")]
    ChatMessage(ChatMessage),
    #[serde(rename = "remove")]
    RemoveMessage(RemoveMessage),
}

impl AnyMessage {
    pub fn text(&self) -> String {
        self.content().to_string()
    }

    pub fn tool_calls(&self) -> &[ToolCall] {
        match self {
            AnyMessage::AIMessage(m) => &m.tool_calls,
            _ => &[],
        }
    }

    pub fn pretty_print(&self) {
        println!("{}", self.pretty_repr(is_interactive_env()));
    }

    pub fn pretty_repr(&self, html: bool) -> String {
        let title_cased = title_case(self.message_type());
        let title = get_msg_title_repr(&format!("{} Message", title_cased), html);

        let name_line = if let Some(name) = self.name() {
            format!("\nName: {}", name)
        } else {
            String::new()
        };

        format!("{}{}\n\n{}", title, name_line, self.text())
    }
}

pub trait HasId {
    fn get_id(&self) -> Option<String>;
}

impl HasId for AnyMessage {
    fn get_id(&self) -> Option<String> {
        self.id()
    }
}

#[enum_dispatch]
pub trait BaseMessageChunk {
    fn id(&self) -> Option<String>;
    fn content(&self) -> &ContentBlocks;
    fn name(&self) -> Option<String>;
    fn set_id(&mut self, id: String);
    fn message_type(&self) -> &'static str;
    fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value>;
    fn response_metadata(&self) -> &HashMap<String, serde_json::Value>;
    fn to_message(&self) -> AnyMessage;
}

/// Tagged union of every chunk variant. Variants carry the same chunk-style
/// `type` value the consumer sees on the wire (`"ai_chunk"`, `"human_chunk"`,
/// ...) — symmetric with [`AnyMessage`]'s `"ai"`, `"human"`, ... and a clean
/// snake_case scheme across the board.
#[enum_dispatch(BaseMessageChunk)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "type")]
pub enum AnyMessageChunk {
    #[serde(rename = "ai_chunk")]
    AIMessageChunk(AIMessageChunk),
    #[serde(rename = "human_chunk")]
    HumanMessageChunk(HumanMessageChunk),
    #[serde(rename = "system_chunk")]
    SystemMessageChunk(SystemMessageChunk),
    #[serde(rename = "tool_chunk")]
    ToolMessageChunk(ToolMessageChunk),
    #[serde(rename = "chat_chunk")]
    ChatMessageChunk(ChatMessageChunk),
}

impl AnyMessageChunk {
    pub fn text(&self) -> String {
        self.content().to_string()
    }

    /// Concatenate two chunks of the same variant. Returns
    /// [`MergeError::MismatchedVariant`] across variants and propagates per-variant
    /// merge failures (e.g. mismatched chat role / tool_call_id).
    pub fn try_concat(self, other: Self) -> Result<Self, MergeError> {
        match (self, other) {
            (AnyMessageChunk::AIMessageChunk(a), AnyMessageChunk::AIMessageChunk(b)) => {
                Ok(AnyMessageChunk::AIMessageChunk(a + b))
            }
            (AnyMessageChunk::HumanMessageChunk(a), AnyMessageChunk::HumanMessageChunk(b)) => {
                Ok(AnyMessageChunk::HumanMessageChunk(a + b))
            }
            (AnyMessageChunk::SystemMessageChunk(a), AnyMessageChunk::SystemMessageChunk(b)) => {
                Ok(AnyMessageChunk::SystemMessageChunk(a + b))
            }
            (AnyMessageChunk::ToolMessageChunk(a), AnyMessageChunk::ToolMessageChunk(b)) => {
                a.try_concat(&b).map(AnyMessageChunk::ToolMessageChunk)
            }
            (AnyMessageChunk::ChatMessageChunk(a), AnyMessageChunk::ChatMessageChunk(b)) => {
                a.try_concat(&b).map(AnyMessageChunk::ChatMessageChunk)
            }
            (left, right) => Err(MergeError::MismatchedVariant {
                left: left.message_type(),
                right: right.message_type(),
            }),
        }
    }
}

/// Errors returned when merging chunks. Used by the chunk types whose `concat`
/// is not total (`ChatMessageChunk` requires identical `role`,
/// `ToolMessageChunk` requires identical `tool_call_id`) and by
/// [`AnyMessageChunk::try_concat`] for cross-variant pairs.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MergeError {
    #[error("cannot merge chunks of different variants: {left} vs {right}")]
    MismatchedVariant {
        left: &'static str,
        right: &'static str,
    },
    #[error("cannot merge ChatMessageChunks with different roles: {left:?} vs {right:?}")]
    MismatchedRole { left: String, right: String },
    #[error("cannot merge ToolMessageChunks with different tool_call_id: {left:?} vs {right:?}")]
    MismatchedToolCallId { left: String, right: String },
}

impl From<&str> for AnyMessage {
    fn from(text: &str) -> Self {
        AnyMessage::HumanMessage(HumanMessage::builder().content(text).build())
    }
}

impl From<String> for AnyMessage {
    fn from(text: String) -> Self {
        AnyMessage::HumanMessage(HumanMessage::builder().content(text).build())
    }
}

impl From<&AnyMessage> for AnyMessageChunk {
    fn from(message: &AnyMessage) -> Self {
        match message {
            AnyMessage::HumanMessage(m) => AnyMessageChunk::HumanMessageChunk(
                HumanMessageChunk::builder()
                    .content(m.content.clone())
                    .maybe_id(m.id.clone())
                    .maybe_name(m.name.clone())
                    .additional_kwargs(m.additional_kwargs.clone())
                    .response_metadata(m.response_metadata.clone())
                    .build(),
            ),
            AnyMessage::AIMessage(m) => {
                let mut chunk = AIMessageChunk::builder()
                    .content(m.content.clone())
                    .maybe_id(m.id.clone())
                    .maybe_name(m.name.clone())
                    .tool_calls(m.tool_calls.clone())
                    .invalid_tool_calls(m.invalid_tool_calls.clone())
                    .maybe_usage_metadata(m.usage_metadata.clone())
                    .additional_kwargs(m.additional_kwargs.clone())
                    .response_metadata(m.response_metadata.clone())
                    .build();
                chunk.init_tool_calls();
                AnyMessageChunk::AIMessageChunk(chunk)
            }
            AnyMessage::SystemMessage(m) => AnyMessageChunk::SystemMessageChunk(
                SystemMessageChunk::builder()
                    .content(m.content.clone())
                    .maybe_id(m.id.clone())
                    .maybe_name(m.name.clone())
                    .additional_kwargs(m.additional_kwargs.clone())
                    .response_metadata(m.response_metadata.clone())
                    .build(),
            ),
            AnyMessage::ToolMessage(m) => AnyMessageChunk::ToolMessageChunk(
                ToolMessageChunk::builder()
                    .content(m.content.clone())
                    .tool_call_id(m.tool_call_id.clone())
                    .maybe_id(m.id.clone())
                    .maybe_name(m.name.clone())
                    .status(m.status.clone())
                    .maybe_artifact(m.artifact.clone())
                    .additional_kwargs(m.additional_kwargs.clone())
                    .response_metadata(m.response_metadata.clone())
                    .build(),
            ),
            AnyMessage::ChatMessage(m) => AnyMessageChunk::ChatMessageChunk(
                ChatMessageChunk::builder()
                    .content(m.content.clone())
                    .role(m.role.clone())
                    .maybe_id(m.id.clone())
                    .maybe_name(m.name.clone())
                    .additional_kwargs(m.additional_kwargs.clone())
                    .response_metadata(m.response_metadata.clone())
                    .build(),
            ),
            AnyMessage::RemoveMessage(_) => {
                panic!("Cannot convert RemoveMessage to chunk")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MergeableContent {
    Text(String),
    List(Vec<Value>),
}

impl From<String> for MergeableContent {
    fn from(s: String) -> Self {
        MergeableContent::Text(s)
    }
}

impl From<&str> for MergeableContent {
    fn from(s: &str) -> Self {
        MergeableContent::Text(s.to_string())
    }
}

impl From<Vec<Value>> for MergeableContent {
    fn from(v: Vec<Value>) -> Self {
        MergeableContent::List(v)
    }
}

pub fn merge_content(first: &str, second: &str) -> String {
    let mut result = first.to_string();
    result.push_str(second);
    result
}

pub fn merge_content_complex(
    first_content: MergeableContent,
    contents: Vec<MergeableContent>,
) -> MergeableContent {
    let mut merged = first_content;

    for content in contents {
        merged = match (merged, content) {
            (MergeableContent::Text(mut left), MergeableContent::Text(right)) => {
                left.push_str(&right);
                MergeableContent::Text(left)
            }
            (MergeableContent::Text(left), MergeableContent::List(right)) => {
                let mut new_list = vec![Value::String(left)];
                new_list.extend(right);
                MergeableContent::List(new_list)
            }
            (MergeableContent::List(left), MergeableContent::List(right)) => {
                match merge_lists(Some(left.clone()), vec![Some(right.clone())]) {
                    Ok(Some(merged_list)) => MergeableContent::List(merged_list),
                    _ => {
                        let mut result = left;
                        result.extend(right);
                        MergeableContent::List(result)
                    }
                }
            }
            (MergeableContent::List(mut left), MergeableContent::Text(right)) => {
                if !left.is_empty() && left.last().is_some_and(|v| v.is_string()) {
                    if let Some(Value::String(s)) = left.last_mut() {
                        s.push_str(&right);
                    }
                } else if right.is_empty() {
                } else if !left.is_empty() {
                    left.push(Value::String(right));
                }
                MergeableContent::List(left)
            }
        };
    }

    merged
}

pub fn merge_content_vec(first: Vec<Value>, second: Vec<Value>) -> Vec<Value> {
    let mut result = first;
    result.extend(second);
    result
}

/// Wrap a message into the `{ "type": <kind>, "data": { ... } }` envelope used
/// by langchain-style dump/load. Internally `serde_json::to_value(msg)` already
/// emits the discriminant via the `AnyMessage` enum's internal tag, so we strip
/// it from the inner data to avoid duplication and let the wrapper carry it.
pub fn message_to_dict(message: &AnyMessage) -> Value {
    let mut data = serde_json::to_value(message).unwrap_or_default();

    let msg_type = message.message_type();

    if let Some(obj) = data.as_object_mut() {
        obj.remove("type");
    }

    serde_json::json!({
        "type": msg_type,
        "data": data
    })
}

pub fn messages_to_dict(messages: &[AnyMessage]) -> Vec<serde_json::Value> {
    messages.iter().map(message_to_dict).collect()
}

pub fn get_msg_title_repr(title: &str, bold: bool) -> String {
    let padded = format!(" {} ", title);
    let sep_len = 80usize.saturating_sub(padded.len()) / 2;
    let sep: String = "=".repeat(sep_len);
    let second_sep = if padded.len() % 2 == 0 {
        sep.clone()
    } else {
        format!("{}=", sep)
    };

    if bold {
        let bolded = get_bolded_text(&padded);
        format!("{}{}{}", sep, bolded, second_sep)
    } else {
        format!("{}{}{}", sep, padded, second_sep)
    }
}

pub fn get_bolded_text(text: &str) -> String {
    format!("\x1b[1m{}\x1b[0m", text)
}

fn title_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    let upper = first.to_uppercase().to_string();
                    upper + &chars.as_str().to_lowercase()
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn extract_reasoning_from_additional_kwargs(
    additional_kwargs: &HashMap<String, Value>,
) -> Option<ReasoningContentBlock> {
    if let Some(Value::String(reasoning_content)) = additional_kwargs.get("reasoning_content") {
        Some(
            ReasoningContentBlock::builder()
                .reasoning(reasoning_content.clone())
                .build(),
        )
    } else {
        None
    }
}

pub use crate::utils::interactive_env::is_interactive_env;
