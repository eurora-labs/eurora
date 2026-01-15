//! System message type.
//!
//! This module contains the `SystemMessage` and `SystemMessageChunk` types which represent
//! system instructions for priming AI behavior. Mirrors `langchain_core.messages.system`.

use crate::utils::uuid7;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "specta")]
use specta::Type;

use super::base::merge_content;

/// A system message in the conversation.
///
/// The system message is usually passed in as the first of a sequence
/// of input messages. It's used to prime AI behavior with instructions.
///
/// This corresponds to `SystemMessage` in LangChain Python.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemMessage {
    /// The message content
    pub content: String,
    /// Optional unique identifier
    pub id: Option<String>,
    /// Optional name for the message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Additional metadata
    #[serde(default)]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
}

impl SystemMessage {
    /// Create a new system message.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            id: Some(uuid7(None).to_string()),
            name: None,
            additional_kwargs: HashMap::new(),
        }
    }

    /// Set the message ID.
    pub fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }

    /// Create a new system message with an explicit ID.
    ///
    /// Use this when deserializing or reconstructing messages where the ID must be preserved.
    pub fn with_id(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            id: Some(id.into()),
            name: None,
            additional_kwargs: HashMap::new(),
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
}

/// System message chunk (yielded when streaming).
///
/// This corresponds to `SystemMessageChunk` in LangChain Python.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemMessageChunk {
    /// The message content (may be partial during streaming)
    content: String,
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

impl SystemMessageChunk {
    /// Create a new system message chunk.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            id: None,
            name: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Create a new system message chunk with an ID.
    pub fn with_id(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
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
    pub fn concat(&self, other: &SystemMessageChunk) -> SystemMessageChunk {
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

        SystemMessageChunk {
            content,
            id: self.id.clone().or_else(|| other.id.clone()),
            name: self.name.clone().or_else(|| other.name.clone()),
            additional_kwargs,
            response_metadata,
        }
    }

    /// Convert this chunk to a complete SystemMessage.
    pub fn to_message(&self) -> SystemMessage {
        SystemMessage {
            content: self.content.clone(),
            id: self.id.clone(),
            name: self.name.clone(),
            additional_kwargs: self.additional_kwargs.clone(),
        }
    }
}

impl std::ops::Add for SystemMessageChunk {
    type Output = SystemMessageChunk;

    fn add(self, other: SystemMessageChunk) -> SystemMessageChunk {
        self.concat(&other)
    }
}
