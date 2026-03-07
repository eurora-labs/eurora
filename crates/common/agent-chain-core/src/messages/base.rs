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

#[derive(Debug, Clone, PartialEq)]
pub enum AnyMessage {
    Human(HumanMessage),
    System(SystemMessage),
    AI(AIMessage),
    Tool(ToolMessage),
    Chat(ChatMessage),
    Function(FunctionMessage),
    Remove(RemoveMessage),
}

impl Serialize for AnyMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            AnyMessage::Human(m) => m.serialize(serializer),
            AnyMessage::System(m) => m.serialize(serializer),
            AnyMessage::AI(m) => m.serialize(serializer),
            AnyMessage::Tool(m) => m.serialize(serializer),
            AnyMessage::Chat(m) => m.serialize(serializer),
            AnyMessage::Function(m) => m.serialize(serializer),
            AnyMessage::Remove(m) => m.serialize(serializer),
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
                        Ok(AnyMessage::Human(msg))
                    }
                    "system" => {
                        let msg: SystemMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessage::System(msg))
                    }
                    "ai" => {
                        let msg: AIMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessage::AI(msg))
                    }
                    "tool" => {
                        let msg: ToolMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessage::Tool(msg))
                    }
                    "chat" => {
                        let msg: ChatMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessage::Chat(msg))
                    }
                    "function" => {
                        let msg: FunctionMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessage::Function(msg))
                    }
                    "remove" => {
                        let msg: RemoveMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessage::Remove(msg))
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
    pub fn content(&self) -> &MessageContent {
        match self {
            AnyMessage::Human(m) => &m.content,
            AnyMessage::System(m) => &m.content,
            AnyMessage::AI(m) => &m.content,
            AnyMessage::Tool(m) => &m.content,
            AnyMessage::Chat(m) => &m.content,
            AnyMessage::Function(m) => &m.content,
            AnyMessage::Remove(_) => MessageContent::empty(),
        }
    }

    pub fn id(&self) -> Option<String> {
        match self {
            AnyMessage::Human(m) => m.id.clone(),
            AnyMessage::System(m) => m.id.clone(),
            AnyMessage::AI(m) => m.id.clone(),
            AnyMessage::Tool(m) => m.id.clone(),
            AnyMessage::Chat(m) => m.id.clone(),
            AnyMessage::Function(m) => m.id.clone(),
            AnyMessage::Remove(m) => Some(m.id.clone()),
        }
    }

    pub fn name(&self) -> Option<String> {
        match self {
            AnyMessage::Human(m) => m.name.clone(),
            AnyMessage::System(m) => m.name.clone(),
            AnyMessage::AI(m) => m.name.clone(),
            AnyMessage::Tool(m) => m.name.clone(),
            AnyMessage::Chat(m) => m.name.clone(),
            AnyMessage::Function(m) => Some(m.name.clone()),
            AnyMessage::Remove(m) => m.name.clone(),
        }
    }

    pub fn set_id(&mut self, id: String) {
        match self {
            AnyMessage::Human(m) => m.set_id(id),
            AnyMessage::System(m) => m.set_id(id),
            AnyMessage::AI(m) => m.set_id(id),
            AnyMessage::Tool(m) => m.set_id(id),
            AnyMessage::Chat(m) => m.set_id(id),
            AnyMessage::Function(m) => m.set_id(id),
            AnyMessage::Remove(m) => m.set_id(id),
        }
    }

    pub fn text(&self) -> String {
        match self {
            AnyMessage::Human(m) => m.content.as_text(),
            AnyMessage::System(m) => m.content.as_text(),
            AnyMessage::AI(m) => m.content.as_text(),
            AnyMessage::Tool(m) => m.content.as_text(),
            AnyMessage::Chat(m) => m.content.as_text(),
            AnyMessage::Function(m) => m.content.as_text(),
            AnyMessage::Remove(_) => String::new(),
        }
    }

    pub fn tool_calls(&self) -> &[ToolCall] {
        match self {
            AnyMessage::AI(m) => &m.tool_calls,
            _ => &[],
        }
    }

    pub fn message_type(&self) -> &'static str {
        match self {
            AnyMessage::Human(_) => "human",
            AnyMessage::System(_) => "system",
            AnyMessage::AI(_) => "ai",
            AnyMessage::Tool(_) => "tool",
            AnyMessage::Chat(_) => "chat",
            AnyMessage::Function(_) => "function",
            AnyMessage::Remove(_) => "remove",
        }
    }

    pub fn additional_kwargs(&self) -> Option<&HashMap<String, serde_json::Value>> {
        match self {
            AnyMessage::Human(m) => Some(&m.additional_kwargs),
            AnyMessage::System(m) => Some(&m.additional_kwargs),
            AnyMessage::AI(m) => Some(&m.additional_kwargs),
            AnyMessage::Tool(m) => Some(&m.additional_kwargs),
            AnyMessage::Chat(m) => Some(&m.additional_kwargs),
            AnyMessage::Function(m) => Some(&m.additional_kwargs),
            AnyMessage::Remove(m) => Some(&m.additional_kwargs),
        }
    }

    pub fn response_metadata(&self) -> Option<&HashMap<String, serde_json::Value>> {
        match self {
            AnyMessage::Human(m) => Some(&m.response_metadata),
            AnyMessage::System(m) => Some(&m.response_metadata),
            AnyMessage::AI(m) => Some(&m.response_metadata),
            AnyMessage::Tool(m) => Some(&m.response_metadata),
            AnyMessage::Chat(m) => Some(&m.response_metadata),
            AnyMessage::Function(m) => Some(&m.response_metadata),
            AnyMessage::Remove(m) => Some(&m.response_metadata),
        }
    }

    pub fn pretty_print(&self) {
        println!("{}", self.pretty_repr(is_interactive_env()));
    }

    pub fn pretty_repr(&self, html: bool) -> String {
        let msg_type = self.message_type();
        let title_cased = title_case(msg_type);
        let title = format!("{} Message", title_cased);
        let title = get_msg_title_repr(&title, html);

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
        self.id().clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum AnyMessageChunk {
    AI(AIMessageChunk),
    Human(HumanMessageChunk),
    System(SystemMessageChunk),
    Tool(ToolMessageChunk),
    Chat(ChatMessageChunk),
    Function(FunctionMessageChunk),
}

impl Serialize for AnyMessageChunk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            AnyMessageChunk::AI(m) => m.serialize(serializer),
            AnyMessageChunk::Human(m) => m.serialize(serializer),
            AnyMessageChunk::System(m) => m.serialize(serializer),
            AnyMessageChunk::Tool(m) => m.serialize(serializer),
            AnyMessageChunk::Chat(m) => m.serialize(serializer),
            AnyMessageChunk::Function(m) => m.serialize(serializer),
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
                        Ok(AnyMessageChunk::AI(msg))
                    }
                    "HumanMessageChunk" => {
                        let msg: HumanMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessageChunk::Human(msg))
                    }
                    "SystemMessageChunk" => {
                        let msg: SystemMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessageChunk::System(msg))
                    }
                    "ToolMessageChunk" => {
                        let msg: ToolMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessageChunk::Tool(msg))
                    }
                    "ChatMessageChunk" => {
                        let msg: ChatMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessageChunk::Chat(msg))
                    }
                    "FunctionMessageChunk" => {
                        let msg: FunctionMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(AnyMessageChunk::Function(msg))
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
            AnyMessageChunk::AI(m) => &m.content,
            AnyMessageChunk::Human(m) => &m.content,
            AnyMessageChunk::System(m) => &m.content,
            AnyMessageChunk::Tool(m) => &m.content,
            AnyMessageChunk::Chat(m) => &m.content,
            AnyMessageChunk::Function(m) => &m.content,
        }
    }

    pub fn id(&self) -> Option<String> {
        match self {
            AnyMessageChunk::AI(m) => m.id.clone(),
            AnyMessageChunk::Human(m) => m.id.clone(),
            AnyMessageChunk::System(m) => m.id.clone(),
            AnyMessageChunk::Tool(m) => m.id.clone(),
            AnyMessageChunk::Chat(m) => m.id.clone(),
            AnyMessageChunk::Function(m) => m.id.clone(),
        }
    }

    pub fn message_type(&self) -> &'static str {
        match self {
            AnyMessageChunk::AI(_) => "AIMessageChunk",
            AnyMessageChunk::Human(_) => "HumanMessageChunk",
            AnyMessageChunk::System(_) => "SystemMessageChunk",
            AnyMessageChunk::Tool(_) => "ToolMessageChunk",
            AnyMessageChunk::Chat(_) => "ChatMessageChunk",
            AnyMessageChunk::Function(_) => "FunctionMessageChunk",
        }
    }

    pub fn to_message(&self) -> AnyMessage {
        match self {
            AnyMessageChunk::AI(m) => AnyMessage::AI(m.to_message()),
            AnyMessageChunk::Human(m) => AnyMessage::Human(m.to_message()),
            AnyMessageChunk::System(m) => AnyMessage::System(m.to_message()),
            AnyMessageChunk::Tool(m) => AnyMessage::Tool(m.to_message()),
            AnyMessageChunk::Chat(m) => AnyMessage::Chat(m.to_message()),
            AnyMessageChunk::Function(m) => AnyMessage::Function(m.to_message()),
        }
    }
}

impl From<AIMessageChunk> for AnyMessageChunk {
    fn from(chunk: AIMessageChunk) -> Self {
        AnyMessageChunk::AI(chunk)
    }
}

impl From<HumanMessageChunk> for AnyMessageChunk {
    fn from(chunk: HumanMessageChunk) -> Self {
        AnyMessageChunk::Human(chunk)
    }
}

impl From<SystemMessageChunk> for AnyMessageChunk {
    fn from(chunk: SystemMessageChunk) -> Self {
        AnyMessageChunk::System(chunk)
    }
}

impl From<ToolMessageChunk> for AnyMessageChunk {
    fn from(chunk: ToolMessageChunk) -> Self {
        AnyMessageChunk::Tool(chunk)
    }
}

impl From<ChatMessageChunk> for AnyMessageChunk {
    fn from(chunk: ChatMessageChunk) -> Self {
        AnyMessageChunk::Chat(chunk)
    }
}

impl From<FunctionMessageChunk> for AnyMessageChunk {
    fn from(chunk: FunctionMessageChunk) -> Self {
        AnyMessageChunk::Function(chunk)
    }
}

impl std::ops::Add for AnyMessageChunk {
    type Output = AnyMessageChunk;

    fn add(self, other: AnyMessageChunk) -> AnyMessageChunk {
        match (self, other) {
            (AnyMessageChunk::AI(a), AnyMessageChunk::AI(b)) => AnyMessageChunk::AI(a + b),
            (AnyMessageChunk::Human(a), AnyMessageChunk::Human(b)) => AnyMessageChunk::Human(a + b),
            (AnyMessageChunk::System(a), AnyMessageChunk::System(b)) => {
                AnyMessageChunk::System(a + b)
            }
            (AnyMessageChunk::Tool(a), AnyMessageChunk::Tool(b)) => AnyMessageChunk::Tool(a + b),
            (AnyMessageChunk::Chat(a), AnyMessageChunk::Chat(b)) => AnyMessageChunk::Chat(a + b),
            (AnyMessageChunk::Function(a), AnyMessageChunk::Function(b)) => {
                AnyMessageChunk::Function(a + b)
            }
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

impl From<AIMessage> for AnyMessage {
    fn from(message: AIMessage) -> Self {
        AnyMessage::AI(message)
    }
}

impl From<HumanMessage> for AnyMessage {
    fn from(message: HumanMessage) -> Self {
        AnyMessage::Human(message)
    }
}

impl From<SystemMessage> for AnyMessage {
    fn from(message: SystemMessage) -> Self {
        AnyMessage::System(message)
    }
}

impl From<ToolMessage> for AnyMessage {
    fn from(message: ToolMessage) -> Self {
        AnyMessage::Tool(message)
    }
}

impl From<ChatMessage> for AnyMessage {
    fn from(message: ChatMessage) -> Self {
        AnyMessage::Chat(message)
    }
}

impl From<FunctionMessage> for AnyMessage {
    fn from(message: FunctionMessage) -> Self {
        AnyMessage::Function(message)
    }
}

impl From<RemoveMessage> for AnyMessage {
    fn from(message: RemoveMessage) -> Self {
        AnyMessage::Remove(message)
    }
}

impl From<&str> for AnyMessage {
    fn from(text: &str) -> Self {
        AnyMessage::Human(HumanMessage::builder().content(text).build())
    }
}

impl From<String> for AnyMessage {
    fn from(text: String) -> Self {
        AnyMessage::Human(HumanMessage::builder().content(text).build())
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
