use bon::bon;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;

use super::base::{get_msg_title_repr, is_interactive_env, merge_content};
use super::content::MessageContent;
use crate::utils::merge::{merge_dicts, merge_obj};

fn deserialize_tool_call_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(s) => Ok(s),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        serde_json::Value::Null => Ok(String::new()),
        other => Ok(other.to_string()),
    }
}

pub trait ToolOutputMixin {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    pub id: Option<String>,
    pub name: String,
    pub args: serde_json::Value,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub call_type: Option<String>,
}

#[bon]
impl ToolCall {
    #[builder]
    pub fn new(name: impl Into<String>, args: serde_json::Value, id: Option<String>) -> Self {
        Self {
            id,
            name: name.into(),
            args,
            call_type: Some("tool_call".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCallChunk {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i32>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub chunk_type: Option<String>,
}

#[bon]
impl ToolCallChunk {
    #[builder]
    pub fn new(
        name: Option<String>,
        args: Option<String>,
        id: Option<String>,
        index: Option<i32>,
    ) -> Self {
        Self {
            name,
            args,
            id,
            index,
            chunk_type: Some("tool_call_chunk".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvalidToolCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub call_type: Option<String>,
}

#[bon]
impl InvalidToolCall {
    #[builder]
    pub fn new(
        name: Option<String>,
        args: Option<String>,
        id: Option<String>,
        error: Option<String>,
    ) -> Self {
        Self {
            name,
            args,
            id,
            error,
            call_type: Some("invalid_tool_call".to_string()),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ToolMessage {
    pub content: MessageContent,
    #[serde(deserialize_with = "deserialize_tool_call_id")]
    pub tool_call_id: String,
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default = "default_status")]
    pub status: ToolStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact: Option<serde_json::Value>,
    #[serde(default)]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub response_metadata: HashMap<String, serde_json::Value>,
}

impl Serialize for ToolMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut field_count = 6;
        if self.name.is_some() {
            field_count += 1;
        }
        if self.artifact.is_some() {
            field_count += 1;
        }
        field_count += 1;

        let mut map = serializer.serialize_map(Some(field_count))?;
        map.serialize_entry("type", "tool")?;
        map.serialize_entry("content", &self.content)?;
        map.serialize_entry("tool_call_id", &self.tool_call_id)?;
        map.serialize_entry("id", &self.id)?;
        if self.name.is_some() {
            map.serialize_entry("name", &self.name)?;
        }
        map.serialize_entry("status", &self.status)?;
        if self.artifact.is_some() {
            map.serialize_entry("artifact", &self.artifact)?;
        }
        map.serialize_entry("additional_kwargs", &self.additional_kwargs)?;
        map.serialize_entry("response_metadata", &self.response_metadata)?;

        map.end()
    }
}

fn default_status() -> ToolStatus {
    ToolStatus::Success
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ToolStatus {
    #[default]
    Success,
    Error,
}

impl PartialEq<str> for ToolStatus {
    fn eq(&self, other: &str) -> bool {
        matches!(
            (self, other),
            (ToolStatus::Success, "success") | (ToolStatus::Error, "error")
        )
    }
}

impl From<String> for ToolStatus {
    fn from(value: String) -> Self {
        match value.as_str() {
            "success" => ToolStatus::Success,
            "error" => ToolStatus::Error,
            _ => ToolStatus::default(),
        }
    }
}

impl From<ToolStatus> for String {
    fn from(value: ToolStatus) -> Self {
        match value {
            ToolStatus::Success => "success".to_string(),
            ToolStatus::Error => "error".to_string(),
        }
    }
}

impl PartialEq<&str> for ToolStatus {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

#[bon]
impl ToolMessage {
    #[builder]
    pub fn new(
        content: impl Into<MessageContent>,
        tool_call_id: impl Into<String>,
        id: Option<String>,
        name: Option<String>,
        #[builder(default)] status: ToolStatus,
        artifact: Option<serde_json::Value>,
        #[builder(default)] additional_kwargs: HashMap<String, serde_json::Value>,
        #[builder(default)] response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            content: content.into(),
            tool_call_id: tool_call_id.into(),
            id,
            name,
            status,
            artifact,
            additional_kwargs,
            response_metadata,
        }
    }

    pub fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }

    pub fn message_type(&self) -> &'static str {
        "tool"
    }

    pub fn text(&self) -> String {
        self.content.as_text()
    }

    pub fn pretty_repr(&self, html: bool) -> String {
        let title = get_msg_title_repr("Tool Message", html);
        let name_line = if let Some(name) = &self.name {
            format!("\nName: {}", name)
        } else {
            String::new()
        };
        format!("{}{}\n\n{}", title, name_line, self.content)
    }

    pub fn pretty_print(&self) {
        println!("{}", self.pretty_repr(is_interactive_env()));
    }
}

impl ToolOutputMixin for ToolMessage {}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ToolMessageChunk {
    pub content: MessageContent,
    #[serde(deserialize_with = "deserialize_tool_call_id")]
    pub tool_call_id: String,
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default = "default_status")]
    pub status: ToolStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact: Option<serde_json::Value>,
    #[serde(default)]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub response_metadata: HashMap<String, serde_json::Value>,
}

impl Serialize for ToolMessageChunk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut field_count = 6;
        if self.name.is_some() {
            field_count += 1;
        }
        if self.artifact.is_some() {
            field_count += 1;
        }
        field_count += 1;

        let mut map = serializer.serialize_map(Some(field_count))?;
        map.serialize_entry("type", "ToolMessageChunk")?;
        map.serialize_entry("content", &self.content)?;
        map.serialize_entry("tool_call_id", &self.tool_call_id)?;
        map.serialize_entry("id", &self.id)?;
        if self.name.is_some() {
            map.serialize_entry("name", &self.name)?;
        }
        map.serialize_entry("status", &self.status)?;
        if self.artifact.is_some() {
            map.serialize_entry("artifact", &self.artifact)?;
        }
        map.serialize_entry("additional_kwargs", &self.additional_kwargs)?;
        map.serialize_entry("response_metadata", &self.response_metadata)?;

        map.end()
    }
}

#[bon]
impl ToolMessageChunk {
    #[builder]
    pub fn new(
        content: impl Into<MessageContent>,
        tool_call_id: impl Into<String>,
        id: Option<String>,
        name: Option<String>,
        #[builder(default)] status: ToolStatus,
        artifact: Option<serde_json::Value>,
        #[builder(default)] additional_kwargs: HashMap<String, serde_json::Value>,
        #[builder(default)] response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            content: content.into(),
            tool_call_id: tool_call_id.into(),
            id,
            name,
            status,
            artifact,
            additional_kwargs,
            response_metadata,
        }
    }

    pub fn message_type(&self) -> &'static str {
        "ToolMessageChunk"
    }

    pub fn concat(&self, other: &ToolMessageChunk) -> ToolMessageChunk {
        if self.tool_call_id != other.tool_call_id {
            panic!("Cannot concatenate ToolMessageChunks with different names.");
        }

        let content = merge_content(self.content.as_text_ref(), other.content.as_text_ref()).into();

        let artifact = match (&self.artifact, &other.artifact) {
            (Some(left), Some(right)) => merge_obj(left.clone(), right.clone()).ok(),
            (Some(left), None) => Some(left.clone()),
            (None, Some(right)) => Some(right.clone()),
            (None, None) => None,
        };

        let additional_kwargs = {
            let left_val = serde_json::to_value(&self.additional_kwargs).unwrap_or_default();
            let right_val = serde_json::to_value(&other.additional_kwargs).unwrap_or_default();
            match merge_dicts(left_val, vec![right_val]) {
                Ok(merged) => serde_json::from_value(merged).unwrap_or_default(),
                Err(_) => self.additional_kwargs.clone(),
            }
        };

        let response_metadata = {
            let left_val = serde_json::to_value(&self.response_metadata).unwrap_or_default();
            let right_val = serde_json::to_value(&other.response_metadata).unwrap_or_default();
            match merge_dicts(left_val, vec![right_val]) {
                Ok(merged) => serde_json::from_value(merged).unwrap_or_default(),
                Err(_) => self.response_metadata.clone(),
            }
        };

        ToolMessageChunk {
            content,
            tool_call_id: self.tool_call_id.clone(),
            id: self.id.clone(),
            name: self.name.clone().or_else(|| other.name.clone()),
            status: merge_status(&self.status, &other.status),
            artifact,
            additional_kwargs,
            response_metadata,
        }
    }

    pub fn to_message(&self) -> ToolMessage {
        ToolMessage {
            content: self.content.clone(),
            tool_call_id: self.tool_call_id.clone(),
            id: self.id.clone(),
            name: self.name.clone(),
            status: self.status.clone(),
            artifact: self.artifact.clone(),
            additional_kwargs: self.additional_kwargs.clone(),
            response_metadata: self.response_metadata.clone(),
        }
    }
}

impl std::ops::Add for ToolMessageChunk {
    type Output = ToolMessageChunk;

    fn add(self, other: ToolMessageChunk) -> ToolMessageChunk {
        self.concat(&other)
    }
}

impl std::iter::Sum for ToolMessageChunk {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|a, b| a + b).unwrap_or_else(|| {
            ToolMessageChunk::builder()
                .content("")
                .tool_call_id("")
                .build()
        })
    }
}

impl From<ToolMessageChunk> for ToolMessage {
    fn from(chunk: ToolMessageChunk) -> Self {
        chunk.to_message()
    }
}

fn merge_status(left: &ToolStatus, right: &ToolStatus) -> ToolStatus {
    if *left == ToolStatus::Error || *right == ToolStatus::Error {
        ToolStatus::Error
    } else {
        ToolStatus::Success
    }
}

pub fn tool_call(name: impl Into<String>, args: serde_json::Value, id: Option<String>) -> ToolCall {
    ToolCall::builder()
        .name(name)
        .args(args)
        .maybe_id(id)
        .build()
}

pub fn tool_call_chunk(
    name: Option<String>,
    args: Option<String>,
    id: Option<String>,
    index: Option<i32>,
) -> ToolCallChunk {
    ToolCallChunk::builder()
        .maybe_name(name)
        .maybe_args(args)
        .maybe_id(id)
        .maybe_index(index)
        .build()
}

pub fn invalid_tool_call(
    name: Option<String>,
    args: Option<String>,
    id: Option<String>,
    error: Option<String>,
) -> InvalidToolCall {
    InvalidToolCall::builder()
        .maybe_name(name)
        .maybe_args(args)
        .maybe_id(id)
        .maybe_error(error)
        .build()
}

pub fn default_tool_parser(
    raw_tool_calls: &[serde_json::Value],
) -> (Vec<ToolCall>, Vec<InvalidToolCall>) {
    let mut tool_calls = Vec::new();
    let mut invalid_tool_calls = Vec::new();

    for raw_tool_call in raw_tool_calls {
        let function = match raw_tool_call.get("function") {
            Some(f) => f,
            None => continue,
        };

        let function_name = function
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();

        let arguments_str = function
            .get("arguments")
            .and_then(|a| a.as_str())
            .unwrap_or("{}");

        let id = raw_tool_call
            .get("id")
            .and_then(|i| i.as_str())
            .map(|s| s.to_string());

        match serde_json::from_str::<serde_json::Value>(arguments_str) {
            Ok(args) => {
                let args = if args.is_object() {
                    args
                } else {
                    serde_json::Value::Object(serde_json::Map::new())
                };
                tool_calls.push(tool_call(function_name, args, id));
            }
            Err(_) => {
                invalid_tool_calls.push(invalid_tool_call(
                    Some(function_name),
                    Some(arguments_str.to_string()),
                    id,
                    None,
                ));
            }
        }
    }

    (tool_calls, invalid_tool_calls)
}

pub fn default_tool_chunk_parser(raw_tool_calls: &[serde_json::Value]) -> Vec<ToolCallChunk> {
    let mut chunks = Vec::new();

    for raw_tool_call in raw_tool_calls {
        let (function_name, function_args) = match raw_tool_call.get("function") {
            Some(f) => (
                f.get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string()),
                f.get("arguments")
                    .and_then(|a| a.as_str())
                    .map(|s| s.to_string()),
            ),
            None => (None, None),
        };

        let id = raw_tool_call
            .get("id")
            .and_then(|i| i.as_str())
            .map(|s| s.to_string());

        let index = raw_tool_call
            .get("index")
            .and_then(|i| i.as_i64())
            .map(|i| i as i32);

        chunks.push(tool_call_chunk(function_name, function_args, id, index));
    }

    chunks
}

use crate::load::Serializable;

impl Serializable for ToolMessage {
    fn is_lc_serializable() -> bool {
        true
    }

    fn get_lc_namespace() -> Vec<String> {
        vec![
            "langchain".to_string(),
            "schema".to_string(),
            "messages".to_string(),
        ]
    }
}
