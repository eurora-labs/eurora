//! Function message type.
//!
//! This module contains the `FunctionMessage` and `FunctionMessageChunk` types which represent
//! messages for passing the result of executing a function back to a model.
//! Mirrors `langchain_core.messages.function`.
//!
//! Note: FunctionMessage is an older version of ToolMessage and doesn't contain
//! the `tool_call_id` field. Consider using ToolMessage for new code.

use bon::bon;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;

use super::base::merge_content;
use super::content::MessageContent;

/// A function message in the thread.
///
/// `FunctionMessage` objects are an older version of the `ToolMessage` schema, and
/// do not contain the `tool_call_id` field.
///
/// The `tool_call_id` field is used to associate the tool call request with the
/// tool call response. Useful in situations where a chat model is able
/// to request multiple tool calls in parallel.
///
/// This corresponds to `FunctionMessage` in LangChain Python.

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct FunctionMessage {
    /// The message content (result of the function)
    pub content: MessageContent,
    /// The name of the function that was executed
    pub name: String,
    /// Optional unique identifier
    pub id: Option<String>,
    /// Additional metadata
    #[serde(default)]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
    /// Response metadata
    #[serde(default)]
    pub response_metadata: HashMap<String, serde_json::Value>,
}

impl Serialize for FunctionMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(6))?;

        map.serialize_entry("type", "function")?;
        map.serialize_entry("content", &self.content)?;
        map.serialize_entry("name", &self.name)?;
        map.serialize_entry("id", &self.id)?;
        map.serialize_entry("additional_kwargs", &self.additional_kwargs)?;
        map.serialize_entry("response_metadata", &self.response_metadata)?;

        map.end()
    }
}

#[bon]
impl FunctionMessage {
    /// Create a new function message.
    #[builder]
    pub fn new(
        content: impl Into<MessageContent>,
        name: impl Into<String>,
        id: Option<String>,
        #[builder(default)] additional_kwargs: HashMap<String, serde_json::Value>,
        #[builder(default)] response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            content: content.into(),
            name: name.into(),
            id,
            additional_kwargs,
            response_metadata,
        }
    }

    /// Set the message ID.
    pub fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }

    /// Get the message type as a string.
    pub fn message_type(&self) -> &'static str {
        "function"
    }

    /// Get the text content of the message.
    pub fn text(&self) -> String {
        self.content.as_text()
    }
}

/// Function message chunk (yielded when streaming).
///
/// This corresponds to `FunctionMessageChunk` in LangChain Python.

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct FunctionMessageChunk {
    /// The message content (may be partial during streaming)
    pub content: MessageContent,
    /// The name of the function that was executed
    pub name: String,
    /// Optional unique identifier
    pub id: Option<String>,
    /// Additional metadata
    #[serde(default)]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
    /// Response metadata
    #[serde(default)]
    pub response_metadata: HashMap<String, serde_json::Value>,
}

impl Serialize for FunctionMessageChunk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(6))?;

        map.serialize_entry("type", "FunctionMessageChunk")?;
        map.serialize_entry("content", &self.content)?;
        map.serialize_entry("name", &self.name)?;
        map.serialize_entry("id", &self.id)?;
        map.serialize_entry("additional_kwargs", &self.additional_kwargs)?;
        map.serialize_entry("response_metadata", &self.response_metadata)?;

        map.end()
    }
}

#[bon]
impl FunctionMessageChunk {
    /// Create a new function message chunk.
    #[builder]
    pub fn new(
        content: impl Into<MessageContent>,
        name: impl Into<String>,
        id: Option<String>,
        #[builder(default)] additional_kwargs: HashMap<String, serde_json::Value>,
        #[builder(default)] response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            content: content.into(),
            name: name.into(),
            id,
            additional_kwargs,
            response_metadata,
        }
    }

    /// Get the message type as a string.
    pub fn message_type(&self) -> &'static str {
        "FunctionMessageChunk"
    }

    /// Get the text content of the message.
    pub fn text(&self) -> String {
        self.content.as_text()
    }

    /// Concatenate this chunk with another chunk.
    ///
    /// # Panics
    ///
    /// Panics if the function names are different.
    pub fn concat(&self, other: &FunctionMessageChunk) -> FunctionMessageChunk {
        if self.name != other.name {
            panic!("Cannot concatenate FunctionMessageChunks with different names");
        }

        let content: MessageContent =
            merge_content(self.content.as_text_ref(), other.content.as_text_ref()).into();

        let mut additional_kwargs = self.additional_kwargs.clone();
        for (k, v) in &other.additional_kwargs {
            additional_kwargs.insert(k.clone(), v.clone());
        }

        let mut response_metadata = self.response_metadata.clone();
        for (k, v) in &other.response_metadata {
            response_metadata.insert(k.clone(), v.clone());
        }

        FunctionMessageChunk {
            content,
            name: self.name.clone(),
            id: self.id.clone().or_else(|| other.id.clone()),
            additional_kwargs,
            response_metadata,
        }
    }

    /// Convert this chunk to a complete FunctionMessage.
    pub fn to_message(&self) -> FunctionMessage {
        FunctionMessage {
            content: self.content.clone(),
            name: self.name.clone(),
            id: self.id.clone(),
            additional_kwargs: self.additional_kwargs.clone(),
            response_metadata: self.response_metadata.clone(),
        }
    }
}

impl std::ops::Add for FunctionMessageChunk {
    type Output = FunctionMessageChunk;

    fn add(self, other: FunctionMessageChunk) -> FunctionMessageChunk {
        self.concat(&other)
    }
}
