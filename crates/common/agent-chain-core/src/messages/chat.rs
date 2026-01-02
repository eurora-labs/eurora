//! Chat message type.
//!
//! This module contains the `ChatMessage` and `ChatMessageChunk` types which represent
//! messages with an arbitrary speaker role. Mirrors `langchain_core.messages.chat`.

use crate::utils::uuid7;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "specta")]
use specta::Type;

use super::base::merge_content;

/// A chat message that can be assigned an arbitrary speaker (role).
///
/// Use this when you need to specify a custom role that isn't covered
/// by the standard message types (Human, AI, System, Tool).
///
/// This corresponds to `ChatMessage` in LangChain Python.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    /// The message content
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

impl ChatMessage {
    /// Create a new chat message with the given role.
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            role: role.into(),
            id: Some(uuid7(None).to_string()),
            name: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Create a new chat message with an explicit ID.
    ///
    /// Use this when deserializing or reconstructing messages where the ID must be preserved.
    pub fn with_id(
        id: impl Into<String>,
        role: impl Into<String>,
        content: impl Into<String>,
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

    /// Set the name for this message.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
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
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Get the message name.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get additional kwargs.
    pub fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value> {
        &self.additional_kwargs
    }

    /// Get response metadata.
    pub fn response_metadata(&self) -> &HashMap<String, serde_json::Value> {
        &self.response_metadata
    }

    /// Set response metadata.
    pub fn with_response_metadata(
        mut self,
        response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        self.response_metadata = response_metadata;
        self
    }
}

/// Chat message chunk (yielded when streaming).
///
/// This corresponds to `ChatMessageChunk` in LangChain Python.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

impl ChatMessageChunk {
    /// Create a new chat message chunk with the given role.
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
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
        role: impl Into<String>,
        content: impl Into<String>,
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

    /// Get the message content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the message role.
    pub fn role(&self) -> &str {
        &self.role
    }

    /// Get the message ID.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Get the message name.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get additional kwargs.
    pub fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value> {
        &self.additional_kwargs
    }

    /// Get response metadata.
    pub fn response_metadata(&self) -> &HashMap<String, serde_json::Value> {
        &self.response_metadata
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
