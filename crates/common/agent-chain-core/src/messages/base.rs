use enum_dispatch::enum_dispatch;
use serde::de::{self, MapAccess, Visitor};
use serde::ser::Serializer;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

use super::ai::{AIMessage, AIMessageChunk};
use super::chat::{ChatMessage, ChatMessageChunk};
use super::content::{MessageContent, ReasoningContentBlock};
use super::function::{FunctionMessage, FunctionMessageChunk};
use super::human::{HumanMessage, HumanMessageChunk};
use super::modifier::RemoveMessage;
use super::system::{SystemMessage, SystemMessageChunk};
use super::tool::{ToolCall, ToolMessage, ToolMessageChunk};
use crate::utils::merge::merge_lists;

#[enum_dispatch]
pub trait BaseMessage {
    fn id(&self) -> Option<String>;
    fn content(&self) -> &MessageContent;
    fn name(&self) -> Option<String>;
    fn set_id(&mut self, id: String);
    fn message_type(&self) -> &'static str;
    fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value>;
    fn response_metadata(&self) -> &HashMap<String, serde_json::Value>;
}

#[enum_dispatch(BaseMessage)]
#[derive(Debug, Clone, PartialEq)]
pub enum AnyMessage {
    HumanMessage,
    SystemMessage,
    AIMessage,
    ToolMessage,
    ChatMessage,
    FunctionMessage,
    RemoveMessage,
}

impl Serialize for AnyMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            AnyMessage::HumanMessage(m) => m.serialize(serializer),
            AnyMessage::SystemMessage(m) => m.serialize(serializer),
            AnyMessage::AIMessage(m) => m.serialize(serializer),
            AnyMessage::ToolMessage(m) => m.serialize(serializer),
            AnyMessage::ChatMessage(m) => m.serialize(serializer),
            AnyMessage::FunctionMessage(m) => m.serialize(serializer),
            AnyMessage::RemoveMessage(m) => m.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for AnyMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BaseMessageVisitor;

        impl<'de> Visitor<'de> for BaseMessageVisitor {
            type Value = AnyMessage;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a message object with a 'type' field")
            }

            fn visit_map<M>(self, mut map: M) -> Result<AnyMessage, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut message_type: Option<String> = None;
                let mut fields: serde_json::Map<String, Value> = serde_json::Map::new();

                while let Some(key) = map.next_key::<String>()? {
                    let value: Value = map.next_value()?;
                    if key == "type" {
                        message_type = value.as_str().map(|s| s.to_string());
                    }
                    fields.insert(key, value);
                }

                let message_type = message_type.ok_or_else(|| de::Error::missing_field("type"))?;

                let json_value = Value::Object(fields);

                match message_type.as_str() {
                    "human" => {
                        let msg: HumanMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessage::HumanMessage(msg))
                    }
                    "system" => {
                        let msg: SystemMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessage::SystemMessage(msg))
                    }
                    "ai" => {
                        let msg: AIMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessage::AIMessage(msg))
                    }
                    "tool" => {
                        let msg: ToolMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessage::ToolMessage(msg))
                    }
                    "chat" => {
                        let msg: ChatMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessage::ChatMessage(msg))
                    }
                    "function" => {
                        let msg: FunctionMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessage::FunctionMessage(msg))
                    }
                    "remove" => {
                        let msg: RemoveMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessage::RemoveMessage(msg))
                    }
                    _ => Err(de::Error::unknown_variant(
                        &message_type,
                        &[
                            "human", "system", "ai", "tool", "chat", "function", "remove",
                        ],
                    )),
                }
            }
        }

        deserializer.deserialize_map(BaseMessageVisitor)
    }
}

impl AnyMessage {
    pub fn text(&self) -> String {
        self.content().as_text()
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

        format!("{}{}\n\n{}", title, name_line, self.content())
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

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum AnyMessageChunk {
    AIMessageChunk(AIMessageChunk),
    HumanMessageChunk(HumanMessageChunk),
    SystemMessageChunk(SystemMessageChunk),
    ToolMessageChunk(ToolMessageChunk),
    ChatMessageChunk(ChatMessageChunk),
    FunctionMessageChunk(FunctionMessageChunk),
}

impl Serialize for AnyMessageChunk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            AnyMessageChunk::AIMessageChunk(m) => m.serialize(serializer),
            AnyMessageChunk::HumanMessageChunk(m) => m.serialize(serializer),
            AnyMessageChunk::SystemMessageChunk(m) => m.serialize(serializer),
            AnyMessageChunk::ToolMessageChunk(m) => m.serialize(serializer),
            AnyMessageChunk::ChatMessageChunk(m) => m.serialize(serializer),
            AnyMessageChunk::FunctionMessageChunk(m) => m.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for AnyMessageChunk {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct AnyMessageChunkVisitor;

        impl<'de> Visitor<'de> for AnyMessageChunkVisitor {
            type Value = AnyMessageChunk;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a message chunk object with a 'type' field")
            }

            fn visit_map<M>(self, mut map: M) -> Result<AnyMessageChunk, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut message_type: Option<String> = None;
                let mut fields: serde_json::Map<String, Value> = serde_json::Map::new();

                while let Some(key) = map.next_key::<String>()? {
                    let value: Value = map.next_value()?;
                    if key == "type" {
                        message_type = value.as_str().map(|s| s.to_string());
                    }
                    fields.insert(key, value);
                }

                let message_type = message_type.ok_or_else(|| de::Error::missing_field("type"))?;

                let json_value = Value::Object(fields);

                match message_type.as_str() {
                    "AIMessageChunk" => {
                        let msg: AIMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessageChunk::AIMessageChunk(msg))
                    }
                    "HumanMessageChunk" => {
                        let msg: HumanMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessageChunk::HumanMessageChunk(msg))
                    }
                    "SystemMessageChunk" => {
                        let msg: SystemMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessageChunk::SystemMessageChunk(msg))
                    }
                    "ToolMessageChunk" => {
                        let msg: ToolMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessageChunk::ToolMessageChunk(msg))
                    }
                    "ChatMessageChunk" => {
                        let msg: ChatMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessageChunk::ChatMessageChunk(msg))
                    }
                    "FunctionMessageChunk" => {
                        let msg: FunctionMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessageChunk::FunctionMessageChunk(msg))
                    }
                    _ => Err(de::Error::unknown_variant(
                        &message_type,
                        &[
                            "AIMessageChunk",
                            "HumanMessageChunk",
                            "SystemMessageChunk",
                            "ToolMessageChunk",
                            "ChatMessageChunk",
                            "FunctionMessageChunk",
                        ],
                    )),
                }
            }
        }

        deserializer.deserialize_map(AnyMessageChunkVisitor)
    }
}

impl AnyMessageChunk {
    pub fn content(&self) -> &MessageContent {
        match self {
            AnyMessageChunk::AIMessageChunk(m) => &m.content,
            AnyMessageChunk::HumanMessageChunk(m) => &m.content,
            AnyMessageChunk::SystemMessageChunk(m) => &m.content,
            AnyMessageChunk::ToolMessageChunk(m) => &m.content,
            AnyMessageChunk::ChatMessageChunk(m) => &m.content,
            AnyMessageChunk::FunctionMessageChunk(m) => &m.content,
        }
    }

    pub fn id(&self) -> Option<String> {
        match self {
            AnyMessageChunk::AIMessageChunk(m) => m.id.clone(),
            AnyMessageChunk::HumanMessageChunk(m) => m.id.clone(),
            AnyMessageChunk::SystemMessageChunk(m) => m.id.clone(),
            AnyMessageChunk::ToolMessageChunk(m) => m.id.clone(),
            AnyMessageChunk::ChatMessageChunk(m) => m.id.clone(),
            AnyMessageChunk::FunctionMessageChunk(m) => m.id.clone(),
        }
    }

    pub fn message_type(&self) -> &'static str {
        match self {
            AnyMessageChunk::AIMessageChunk(_) => "AIMessageChunk",
            AnyMessageChunk::HumanMessageChunk(_) => "HumanMessageChunk",
            AnyMessageChunk::SystemMessageChunk(_) => "SystemMessageChunk",
            AnyMessageChunk::ToolMessageChunk(_) => "ToolMessageChunk",
            AnyMessageChunk::ChatMessageChunk(_) => "ChatMessageChunk",
            AnyMessageChunk::FunctionMessageChunk(_) => "FunctionMessageChunk",
        }
    }

    pub fn to_message(&self) -> AnyMessage {
        match self {
            AnyMessageChunk::AIMessageChunk(m) => AnyMessage::AIMessage(m.to_message()),
            AnyMessageChunk::HumanMessageChunk(m) => AnyMessage::HumanMessage(m.to_message()),
            AnyMessageChunk::SystemMessageChunk(m) => AnyMessage::SystemMessage(m.to_message()),
            AnyMessageChunk::ToolMessageChunk(m) => AnyMessage::ToolMessage(m.to_message()),
            AnyMessageChunk::ChatMessageChunk(m) => AnyMessage::ChatMessage(m.to_message()),
            AnyMessageChunk::FunctionMessageChunk(m) => AnyMessage::FunctionMessage(m.to_message()),
        }
    }
}

impl From<AIMessageChunk> for AnyMessageChunk {
    fn from(chunk: AIMessageChunk) -> Self {
        AnyMessageChunk::AIMessageChunk(chunk)
    }
}

impl From<HumanMessageChunk> for AnyMessageChunk {
    fn from(chunk: HumanMessageChunk) -> Self {
        AnyMessageChunk::HumanMessageChunk(chunk)
    }
}

impl From<SystemMessageChunk> for AnyMessageChunk {
    fn from(chunk: SystemMessageChunk) -> Self {
        AnyMessageChunk::SystemMessageChunk(chunk)
    }
}

impl From<ToolMessageChunk> for AnyMessageChunk {
    fn from(chunk: ToolMessageChunk) -> Self {
        AnyMessageChunk::ToolMessageChunk(chunk)
    }
}

impl From<ChatMessageChunk> for AnyMessageChunk {
    fn from(chunk: ChatMessageChunk) -> Self {
        AnyMessageChunk::ChatMessageChunk(chunk)
    }
}

impl From<FunctionMessageChunk> for AnyMessageChunk {
    fn from(chunk: FunctionMessageChunk) -> Self {
        AnyMessageChunk::FunctionMessageChunk(chunk)
    }
}

impl std::ops::Add for AnyMessageChunk {
    type Output = AnyMessageChunk;

    fn add(self, other: AnyMessageChunk) -> AnyMessageChunk {
        match (self, other) {
            (AnyMessageChunk::AIMessageChunk(a), AnyMessageChunk::AIMessageChunk(b)) => {
                AnyMessageChunk::AIMessageChunk(a + b)
            }
            (AnyMessageChunk::HumanMessageChunk(a), AnyMessageChunk::HumanMessageChunk(b)) => {
                AnyMessageChunk::HumanMessageChunk(a + b)
            }
            (AnyMessageChunk::SystemMessageChunk(a), AnyMessageChunk::SystemMessageChunk(b)) => {
                AnyMessageChunk::SystemMessageChunk(a + b)
            }
            (AnyMessageChunk::ToolMessageChunk(a), AnyMessageChunk::ToolMessageChunk(b)) => {
                AnyMessageChunk::ToolMessageChunk(a + b)
            }
            (AnyMessageChunk::ChatMessageChunk(a), AnyMessageChunk::ChatMessageChunk(b)) => {
                AnyMessageChunk::ChatMessageChunk(a + b)
            }
            (
                AnyMessageChunk::FunctionMessageChunk(a),
                AnyMessageChunk::FunctionMessageChunk(b),
            ) => AnyMessageChunk::FunctionMessageChunk(a + b),
            (left, right) => {
                panic!(
                    "unsupported operand type(s) for +: \"{}\" and \"{}\"",
                    left.message_type(),
                    right.message_type()
                );
            }
        }
    }
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
        Some(ReasoningContentBlock::new(reasoning_content.clone()))
    } else {
        None
    }
}

pub use crate::utils::interactive_env::is_interactive_env;
