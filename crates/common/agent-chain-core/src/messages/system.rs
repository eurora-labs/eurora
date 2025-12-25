//! System message type.
//!
//! This module contains the `SystemMessage` type which represents
//! system instructions for priming AI behavior. Mirrors `langchain_core.messages.system`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(feature = "specta")]
use specta::Type;

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
    content: String,
    /// Optional unique identifier
    id: Option<String>,
    /// Optional name for the message
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// Additional metadata
    #[serde(default)]
    additional_kwargs: HashMap<String, serde_json::Value>,
}

impl SystemMessage {
    /// Create a new system message.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            id: Some(Uuid::new_v4().to_string()),
            name: None,
            additional_kwargs: HashMap::new(),
        }
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