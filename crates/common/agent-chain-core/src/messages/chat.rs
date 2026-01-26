//! Chat message type.
//!
//! This module contains the `ChatMessage` and `ChatMessageChunk` types which represent
//! messages with an arbitrary speaker role. Mirrors `langchain_core.messages.chat`.

use crate::utils::uuid7;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;

use super::base::merge_content;

/// A chat message that can be assigned an arbitrary speaker (role).
///
/// Use this when you need to specify a custom role that isn't covered
/// by the standard message types (Human, AI, System, Tool).
///
/// This corresponds to `ChatMessage` in LangChain Python.

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ChatMessage {
    /// The message content
    pub content: String,
    /// The speaker / role of the message
    pub role: String,
    /// Optional unique identifier
    pub id: Option<String>,
    /// Optional name for the message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Additional metadata
    #[serde(default)]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
    /// Response metadata
    #[serde(default)]
    pub response_metadata: HashMap<String, serde_json::Value>,
}

impl Serialize for ChatMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Count the number of fields to serialize
        let mut field_count = 5; // content, role, id, additional_kwargs, response_metadata
        if self.name.is_some() {
            field_count += 1;
        }
        // Add 1 for the type field in additional_kwargs
        let mut map = serializer.serialize_map(Some(field_count))?;

        map.serialize_entry("type", "chat")?;
        map.serialize_entry("content", &self.content)?;
        map.serialize_entry("role", &self.role)?;
        map.serialize_entry("id", &self.id)?;
        if let Some(ref name) = self.name {
            map.serialize_entry("name", name)?;
        }

        // Merge the type into additional_kwargs during serialization
        let additional_kwargs_with_type = self.additional_kwargs.clone();
        map.serialize_entry("additional_kwargs", &additional_kwargs_with_type)?;

        map.serialize_entry("response_metadata", &self.response_metadata)?;

        map.end()
    }
}

impl ChatMessage {
    /// Create a new chat message with the given content and role.
    pub fn new(content: impl Into<String>, role: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            role: role.into(),
            id: Some(uuid7(None).to_string()),
            name: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Set the message ID.
    pub fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }

    /// Create a new chat message with an explicit ID.
    ///
    /// Use this when deserializing or reconstructing messages where the ID must be preserved.
    pub fn with_id(
        id: impl Into<String>,
        content: impl Into<String>,
        role: impl Into<String>,
    ) -> Self {
        Self {
            content: content.into(),
            role: role.into(),
            id: Some(id.into()),
            name: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Set the name for this message (builder pattern).
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
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

    /// Get the message role.
    pub fn role(&self) -> &str {
        &self.role
    }

    /// Get the message ID.
    pub fn id(&self) -> Option<String> {
        self.id.clone()
    }

    /// Get the message name.
    pub fn name(&self) -> Option<String> {
        self.name.clone()
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
        "chat"
    }

    /// Get the text content of the message.
    pub fn text(&self) -> &str {
        self.content()
    }
}

/// Chat message chunk (yielded when streaming).
///
/// This corresponds to `ChatMessageChunk` in LangChain Python.

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ChatMessageChunk {
    /// The message content (may be partial during streaming)
    content: String,
    /// The speaker / role of the message
    role: String,
    /// Optional unique identifier
    id: Option<String>,
    /// Optional name for the message
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// Additional metadata
    #[serde(default)]
    additional_kwargs: HashMap<String, serde_json::Value>,
    /// Response metadata
    #[serde(default)]
    response_metadata: HashMap<String, serde_json::Value>,
}

impl Serialize for ChatMessageChunk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Count the number of fields to serialize
        let mut field_count = 5; // content, role, id, additional_kwargs, response_metadata
        if self.name.is_some() {
            field_count += 1;
        }
        let mut map = serializer.serialize_map(Some(field_count))?;

        map.serialize_entry("type", "ChatMessageChunk")?;
        map.serialize_entry("content", &self.content)?;
        map.serialize_entry("role", &self.role)?;
        map.serialize_entry("id", &self.id)?;
        if let Some(ref name) = self.name {
            map.serialize_entry("name", name)?;
        }

        // Merge the type into additional_kwargs during serialization
        let additional_kwargs_with_type = self.additional_kwargs.clone();
        map.serialize_entry("additional_kwargs", &additional_kwargs_with_type)?;

        map.serialize_entry("response_metadata", &self.response_metadata)?;

        map.end()
    }
}

impl ChatMessageChunk {
    /// Create a new chat message chunk with the given content and role.
    pub fn new(content: impl Into<String>, role: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            role: role.into(),
            id: None,
            name: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Create a new chat message chunk with an ID.
    pub fn with_id(
        id: impl Into<String>,
        content: impl Into<String>,
        role: impl Into<String>,
    ) -> Self {
        Self {
            content: content.into(),
            role: role.into(),
            id: Some(id.into()),
            name: None,
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

    /// Get the message role.
    pub fn role(&self) -> &str {
        &self.role
    }

    /// Get the message ID.
    pub fn id(&self) -> Option<String> {
        self.id.clone()
    }

    /// Get the message name.
    pub fn name(&self) -> Option<String> {
        self.name.clone()
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
        "ChatMessageChunk"
    }

    /// Get the text content of the message.
    pub fn text(&self) -> &str {
        self.content()
    }

    /// Concatenate this chunk with another chunk.
    ///
    /// # Panics
    ///
    /// Panics if the roles are different.
    pub fn concat(&self, other: &ChatMessageChunk) -> ChatMessageChunk {
        if self.role != other.role {
            panic!("Cannot concatenate ChatMessageChunks with different roles");
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

        ChatMessageChunk {
            content,
            role: self.role.clone(),
            id: self.id.clone().or_else(|| other.id.clone()),
            name: self.name.clone().or_else(|| other.name.clone()),
            additional_kwargs,
            response_metadata,
        }
    }

    /// Convert this chunk to a complete ChatMessage.
    pub fn to_message(&self) -> ChatMessage {
        ChatMessage {
            content: self.content.clone(),
            role: self.role.clone(),
            id: self.id.clone(),
            name: self.name.clone(),
            additional_kwargs: self.additional_kwargs.clone(),
            response_metadata: self.response_metadata.clone(),
        }
    }
}

impl std::ops::Add for ChatMessageChunk {
    type Output = ChatMessageChunk;

    fn add(self, other: ChatMessageChunk) -> ChatMessageChunk {
        self.concat(&other)
    }
}

impl super::base::BaseMessageTrait for ChatMessage {
    fn content(&self) -> &str {
        ChatMessage::content(self)
    }

    fn id(&self) -> Option<String> {
        ChatMessage::id(self)
    }

    fn name(&self) -> Option<String> {
        ChatMessage::name(self)
    }

    fn set_id(&mut self, id: String) {
        ChatMessage::set_id(self, id)
    }

    fn additional_kwargs(&self) -> Option<&HashMap<String, serde_json::Value>> {
        Some(ChatMessage::additional_kwargs(self))
    }
}
