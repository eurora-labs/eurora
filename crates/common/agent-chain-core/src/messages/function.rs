//! Function message type.
//!
//! This module contains the `FunctionMessage` and `FunctionMessageChunk` types which represent
//! messages for passing the result of executing a function back to a model.
//! Mirrors `langchain_core.messages.function`.
//!
//! Note: FunctionMessage is an older version of ToolMessage and doesn't contain
//! the `tool_call_id` field. Consider using ToolMessage for new code.

use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;

use super::base::merge_content;

/// A function message in the conversation.
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
    pub content: String,
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

impl FunctionMessage {
    /// Create a new function message.
    ///
    /// # Arguments
    ///
    /// * `content` - The result of the function execution.
    /// * `name` - The name of the function that was executed.
    pub fn new(content: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            name: name.into(),
            id: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Set the message ID.
    pub fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }

    /// Create a new function message with an explicit ID.
    ///
    /// Use this when deserializing or reconstructing messages where the ID must be preserved.
    pub fn with_id(
        id: impl Into<String>,
        content: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            content: content.into(),
            name: name.into(),
            id: Some(id.into()),
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Set additional kwargs (builder pattern).
    pub fn with_additional_kwargs(
        mut self,
        additional_kwargs: HashMap<String, serde_json::Value>,
    ) -> Self {
        self.additional_kwargs = additional_kwargs;
        self
    }

    /// Set response metadata (builder pattern).
    pub fn with_response_metadata(
        mut self,
        response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        self.response_metadata = response_metadata;
        self
    }

    /// Get the message content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the function name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the message ID.
    pub fn id(&self) -> Option<String> {
        self.id.clone()
    }

    /// Get additional kwargs.
    pub fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value> {
        &self.additional_kwargs
    }

    /// Get response metadata.
    pub fn response_metadata(&self) -> &HashMap<String, serde_json::Value> {
        &self.response_metadata
    }

    /// Get the message type as a string.
    pub fn message_type(&self) -> &'static str {
        "function"
    }

    /// Get the text content of the message.
    pub fn text(&self) -> &str {
        self.content()
    }
}

/// Function message chunk (yielded when streaming).
///
/// This corresponds to `FunctionMessageChunk` in LangChain Python.

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct FunctionMessageChunk {
    /// The message content (may be partial during streaming)
    content: String,
    /// The name of the function that was executed
    name: String,
    /// Optional unique identifier
    id: Option<String>,
    /// Additional metadata
    #[serde(default)]
    additional_kwargs: HashMap<String, serde_json::Value>,
    /// Response metadata
    #[serde(default)]
    response_metadata: HashMap<String, serde_json::Value>,
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

impl FunctionMessageChunk {
    /// Create a new function message chunk.
    pub fn new(content: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            name: name.into(),
            id: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Create a new function message chunk with an ID.
    pub fn with_id(
        id: impl Into<String>,
        content: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            content: content.into(),
            name: name.into(),
            id: Some(id.into()),
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Set additional kwargs (builder pattern).
    pub fn with_additional_kwargs(
        mut self,
        additional_kwargs: HashMap<String, serde_json::Value>,
    ) -> Self {
        self.additional_kwargs = additional_kwargs;
        self
    }

    /// Set response metadata (builder pattern).
    pub fn with_response_metadata(
        mut self,
        response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        self.response_metadata = response_metadata;
        self
    }

    /// Get the message content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the function name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the message ID.
    pub fn id(&self) -> Option<String> {
        self.id.clone()
    }

    /// Get additional kwargs.
    pub fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value> {
        &self.additional_kwargs
    }

    /// Get response metadata.
    pub fn response_metadata(&self) -> &HashMap<String, serde_json::Value> {
        &self.response_metadata
    }

    /// Get the message type as a string.
    pub fn message_type(&self) -> &'static str {
        "FunctionMessageChunk"
    }

    /// Get the text content of the message.
    pub fn text(&self) -> &str {
        self.content()
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

        let content = merge_content(&self.content, &other.content);

        // Merge additional_kwargs
        let mut additional_kwargs = self.additional_kwargs.clone();
        for (k, v) in &other.additional_kwargs {
            additional_kwargs.insert(k.clone(), v.clone());
        }

        // Merge response_metadata
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

impl super::base::BaseMessageTrait for FunctionMessage {
    fn content(&self) -> &str {
        FunctionMessage::content(self)
    }

    fn id(&self) -> Option<String> {
        FunctionMessage::id(self)
    }

    fn name(&self) -> Option<String> {
        Some(FunctionMessage::name(self).to_string())
    }

    fn set_id(&mut self, id: String) {
        FunctionMessage::set_id(self, id)
    }

    fn additional_kwargs(&self) -> Option<&HashMap<String, serde_json::Value>> {
        Some(FunctionMessage::additional_kwargs(self))
    }
}
