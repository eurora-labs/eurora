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
pub enum BaseMessage {
    Human(HumanMessage),
    System(SystemMessage),
    AI(AIMessage),
    Tool(ToolMessage),
    Chat(ChatMessage),
    Function(FunctionMessage),
    Remove(RemoveMessage),
}

impl Serialize for BaseMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            BaseMessage::Human(m) => m.serialize(serializer),
            BaseMessage::System(m) => m.serialize(serializer),
            BaseMessage::AI(m) => m.serialize(serializer),
            BaseMessage::Tool(m) => m.serialize(serializer),
            BaseMessage::Chat(m) => m.serialize(serializer),
            BaseMessage::Function(m) => m.serialize(serializer),
            BaseMessage::Remove(m) => m.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for BaseMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BaseMessageVisitor;

        impl<'de> Visitor<'de> for BaseMessageVisitor {
            type Value = BaseMessage;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a message object with a 'type' field")
            }

            fn visit_map<M>(self, mut map: M) -> Result<BaseMessage, M::Error>
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
                        Ok(BaseMessage::Human(msg))
                    }
                    "system" => {
                        let msg: SystemMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessage::System(msg))
                    }
                    "ai" => {
                        let msg: AIMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessage::AI(msg))
                    }
                    "tool" => {
                        let msg: ToolMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessage::Tool(msg))
                    }
                    "chat" => {
                        let msg: ChatMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessage::Chat(msg))
                    }
                    "function" => {
                        let msg: FunctionMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessage::Function(msg))
                    }
                    "remove" => {
                        let msg: RemoveMessage =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessage::Remove(msg))
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

impl BaseMessage {
    pub fn content(&self) -> &MessageContent {
        match self {
            BaseMessage::Human(m) => &m.content,
            BaseMessage::System(m) => &m.content,
            BaseMessage::AI(m) => &m.content,
            BaseMessage::Tool(m) => &m.content,
            BaseMessage::Chat(m) => &m.content,
            BaseMessage::Function(m) => &m.content,
            BaseMessage::Remove(_) => MessageContent::empty(),
        }
    }

    pub fn id(&self) -> Option<String> {
        match self {
            BaseMessage::Human(m) => m.id.clone(),
            BaseMessage::System(m) => m.id.clone(),
            BaseMessage::AI(m) => m.id.clone(),
            BaseMessage::Tool(m) => m.id.clone(),
            BaseMessage::Chat(m) => m.id.clone(),
            BaseMessage::Function(m) => m.id.clone(),
            BaseMessage::Remove(m) => Some(m.id.clone()),
        }
    }

    pub fn name(&self) -> Option<String> {
        match self {
            BaseMessage::Human(m) => m.name.clone(),
            BaseMessage::System(m) => m.name.clone(),
            BaseMessage::AI(m) => m.name.clone(),
            BaseMessage::Tool(m) => m.name.clone(),
            BaseMessage::Chat(m) => m.name.clone(),
            BaseMessage::Function(m) => Some(m.name.clone()),
            BaseMessage::Remove(m) => m.name.clone(),
        }
    }

    pub fn set_id(&mut self, id: String) {
        match self {
            BaseMessage::Human(m) => m.set_id(id),
            BaseMessage::System(m) => m.set_id(id),
            BaseMessage::AI(m) => m.set_id(id),
            BaseMessage::Tool(m) => m.set_id(id),
            BaseMessage::Chat(m) => m.set_id(id),
            BaseMessage::Function(m) => m.set_id(id),
            BaseMessage::Remove(m) => m.set_id(id),
        }
    }

    pub fn text(&self) -> String {
        match self {
            BaseMessage::Human(m) => m.content.as_text(),
            BaseMessage::System(m) => m.content.as_text(),
            BaseMessage::AI(m) => m.content.as_text(),
            BaseMessage::Tool(m) => m.content.as_text(),
            BaseMessage::Chat(m) => m.content.as_text(),
            BaseMessage::Function(m) => m.content.as_text(),
            BaseMessage::Remove(_) => String::new(),
        }
    }

    pub fn tool_calls(&self) -> &[ToolCall] {
        match self {
            BaseMessage::AI(m) => &m.tool_calls,
            _ => &[],
        }
    }

    pub fn message_type(&self) -> &'static str {
        match self {
            BaseMessage::Human(_) => "human",
            BaseMessage::System(_) => "system",
            BaseMessage::AI(_) => "ai",
            BaseMessage::Tool(_) => "tool",
            BaseMessage::Chat(_) => "chat",
            BaseMessage::Function(_) => "function",
            BaseMessage::Remove(_) => "remove",
        }
    }

    pub fn additional_kwargs(&self) -> Option<&HashMap<String, serde_json::Value>> {
        match self {
            BaseMessage::Human(m) => Some(&m.additional_kwargs),
            BaseMessage::System(m) => Some(&m.additional_kwargs),
            BaseMessage::AI(m) => Some(&m.additional_kwargs),
            BaseMessage::Tool(m) => Some(&m.additional_kwargs),
            BaseMessage::Chat(m) => Some(&m.additional_kwargs),
            BaseMessage::Function(m) => Some(&m.additional_kwargs),
            BaseMessage::Remove(m) => Some(&m.additional_kwargs),
        }
    }

    pub fn response_metadata(&self) -> Option<&HashMap<String, serde_json::Value>> {
        match self {
            BaseMessage::Human(m) => Some(&m.response_metadata),
            BaseMessage::System(m) => Some(&m.response_metadata),
            BaseMessage::AI(m) => Some(&m.response_metadata),
            BaseMessage::Tool(m) => Some(&m.response_metadata),
            BaseMessage::Chat(m) => Some(&m.response_metadata),
            BaseMessage::Function(m) => Some(&m.response_metadata),
            BaseMessage::Remove(m) => Some(&m.response_metadata),
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

impl HasId for BaseMessage {
    fn get_id(&self) -> Option<String> {
        self.id().clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum BaseMessageChunk {
    AI(AIMessageChunk),
    Human(HumanMessageChunk),
    System(SystemMessageChunk),
    Tool(ToolMessageChunk),
    Chat(ChatMessageChunk),
    Function(FunctionMessageChunk),
}

impl Serialize for BaseMessageChunk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            BaseMessageChunk::AI(m) => m.serialize(serializer),
            BaseMessageChunk::Human(m) => m.serialize(serializer),
            BaseMessageChunk::System(m) => m.serialize(serializer),
            BaseMessageChunk::Tool(m) => m.serialize(serializer),
            BaseMessageChunk::Chat(m) => m.serialize(serializer),
            BaseMessageChunk::Function(m) => m.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for BaseMessageChunk {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BaseMessageChunkVisitor;

        impl<'de> Visitor<'de> for BaseMessageChunkVisitor {
            type Value = BaseMessageChunk;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a message chunk object with a 'type' field")
            }

            fn visit_map<M>(self, mut map: M) -> Result<BaseMessageChunk, M::Error>
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
                        Ok(BaseMessageChunk::AI(msg))
                    }
                    "HumanMessageChunk" => {
                        let msg: HumanMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessageChunk::Human(msg))
                    }
                    "SystemMessageChunk" => {
                        let msg: SystemMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessageChunk::System(msg))
                    }
                    "ToolMessageChunk" => {
                        let msg: ToolMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessageChunk::Tool(msg))
                    }
                    "ChatMessageChunk" => {
                        let msg: ChatMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessageChunk::Chat(msg))
                    }
                    "FunctionMessageChunk" => {
                        let msg: FunctionMessageChunk =
                            serde_json::from_value(json_value).map_err(de::Error::custom)?;
                        Ok(BaseMessageChunk::Function(msg))
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

        deserializer.deserialize_map(BaseMessageChunkVisitor)
    }
}

impl BaseMessageChunk {
    pub fn content(&self) -> &MessageContent {
        match self {
            BaseMessageChunk::AI(m) => &m.content,
            BaseMessageChunk::Human(m) => &m.content,
            BaseMessageChunk::System(m) => &m.content,
            BaseMessageChunk::Tool(m) => &m.content,
            BaseMessageChunk::Chat(m) => &m.content,
            BaseMessageChunk::Function(m) => &m.content,
        }
    }

    pub fn id(&self) -> Option<String> {
        match self {
            BaseMessageChunk::AI(m) => m.id.clone(),
            BaseMessageChunk::Human(m) => m.id.clone(),
            BaseMessageChunk::System(m) => m.id.clone(),
            BaseMessageChunk::Tool(m) => m.id.clone(),
            BaseMessageChunk::Chat(m) => m.id.clone(),
            BaseMessageChunk::Function(m) => m.id.clone(),
        }
    }

    pub fn message_type(&self) -> &'static str {
        match self {
            BaseMessageChunk::AI(_) => "AIMessageChunk",
            BaseMessageChunk::Human(_) => "HumanMessageChunk",
            BaseMessageChunk::System(_) => "SystemMessageChunk",
            BaseMessageChunk::Tool(_) => "ToolMessageChunk",
            BaseMessageChunk::Chat(_) => "ChatMessageChunk",
            BaseMessageChunk::Function(_) => "FunctionMessageChunk",
        }
    }

    pub fn to_message(&self) -> BaseMessage {
        match self {
            BaseMessageChunk::AI(m) => BaseMessage::AI(m.to_message()),
            BaseMessageChunk::Human(m) => BaseMessage::Human(m.to_message()),
            BaseMessageChunk::System(m) => BaseMessage::System(m.to_message()),
            BaseMessageChunk::Tool(m) => BaseMessage::Tool(m.to_message()),
            BaseMessageChunk::Chat(m) => BaseMessage::Chat(m.to_message()),
            BaseMessageChunk::Function(m) => BaseMessage::Function(m.to_message()),
        }
    }
}

impl From<AIMessageChunk> for BaseMessageChunk {
    fn from(chunk: AIMessageChunk) -> Self {
        BaseMessageChunk::AI(chunk)
    }
}

impl From<HumanMessageChunk> for BaseMessageChunk {
    fn from(chunk: HumanMessageChunk) -> Self {
        BaseMessageChunk::Human(chunk)
    }
}

impl From<SystemMessageChunk> for BaseMessageChunk {
    fn from(chunk: SystemMessageChunk) -> Self {
        BaseMessageChunk::System(chunk)
    }
}

impl From<ToolMessageChunk> for BaseMessageChunk {
    fn from(chunk: ToolMessageChunk) -> Self {
        BaseMessageChunk::Tool(chunk)
    }
}

impl From<ChatMessageChunk> for BaseMessageChunk {
    fn from(chunk: ChatMessageChunk) -> Self {
        BaseMessageChunk::Chat(chunk)
    }
}

impl From<FunctionMessageChunk> for BaseMessageChunk {
    fn from(chunk: FunctionMessageChunk) -> Self {
        BaseMessageChunk::Function(chunk)
    }
}

impl std::ops::Add for BaseMessageChunk {
    type Output = BaseMessageChunk;

    fn add(self, other: BaseMessageChunk) -> BaseMessageChunk {
        match (self, other) {
            (BaseMessageChunk::AI(a), BaseMessageChunk::AI(b)) => BaseMessageChunk::AI(a + b),
            (BaseMessageChunk::Human(a), BaseMessageChunk::Human(b)) => {
                BaseMessageChunk::Human(a + b)
            }
            (BaseMessageChunk::System(a), BaseMessageChunk::System(b)) => {
                BaseMessageChunk::System(a + b)
            }
            (BaseMessageChunk::Tool(a), BaseMessageChunk::Tool(b)) => BaseMessageChunk::Tool(a + b),
            (BaseMessageChunk::Chat(a), BaseMessageChunk::Chat(b)) => BaseMessageChunk::Chat(a + b),
            (BaseMessageChunk::Function(a), BaseMessageChunk::Function(b)) => {
                BaseMessageChunk::Function(a + b)
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

impl From<AIMessage> for BaseMessage {
    fn from(message: AIMessage) -> Self {
        BaseMessage::AI(message)
    }
}

impl From<HumanMessage> for BaseMessage {
    fn from(message: HumanMessage) -> Self {
        BaseMessage::Human(message)
    }
}

impl From<SystemMessage> for BaseMessage {
    fn from(message: SystemMessage) -> Self {
        BaseMessage::System(message)
    }
}

impl From<ToolMessage> for BaseMessage {
    fn from(message: ToolMessage) -> Self {
        BaseMessage::Tool(message)
    }
}

impl From<ChatMessage> for BaseMessage {
    fn from(message: ChatMessage) -> Self {
        BaseMessage::Chat(message)
    }
}

impl From<FunctionMessage> for BaseMessage {
    fn from(message: FunctionMessage) -> Self {
        BaseMessage::Function(message)
    }
}

impl From<RemoveMessage> for BaseMessage {
    fn from(message: RemoveMessage) -> Self {
        BaseMessage::Remove(message)
    }
}

impl From<&str> for BaseMessage {
    fn from(text: &str) -> Self {
        BaseMessage::Human(HumanMessage::builder().content(text).build())
    }
}

impl From<String> for BaseMessage {
    fn from(text: String) -> Self {
        BaseMessage::Human(HumanMessage::builder().content(text).build())
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

pub fn message_to_dict(message: &BaseMessage) -> Value {
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

pub fn messages_to_dict(messages: &[BaseMessage]) -> Vec<serde_json::Value> {
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

pub fn is_interactive_env() -> bool {
    false
}
