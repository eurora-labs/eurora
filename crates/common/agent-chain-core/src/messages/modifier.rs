//! Message modifier types.
//!
//! This module contains types for modifying message history,
//! such as `RemoveMessage`. Mirrors `langchain_core.messages.modifier`.

use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;

/// Message responsible for deleting other messages.
///
/// This is used to remove messages from a conversation history by their ID.
/// This corresponds to `RemoveMessage` in LangChain Python.

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct RemoveMessage {
    /// The ID of the message to remove
    pub id: String,
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

impl Serialize for RemoveMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut field_count = 3;
        if self.name.is_some() {
            field_count += 1;
        }
        // Add 1 for additional type field
        field_count += 1;

        let mut map = serializer.serialize_map(Some(field_count))?;
        map.serialize_entry("type", "remove")?;
        map.serialize_entry("id", &self.id)?;
        if self.name.is_some() {
            map.serialize_entry("name", &self.name)?;
        }
        map.serialize_entry("additional_kwargs", &self.additional_kwargs)?;
        map.serialize_entry("response_metadata", &self.response_metadata)?;

        map.end()
    }
}

impl RemoveMessage {
    /// Create a new RemoveMessage.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the message to remove.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Get the ID of the message to be removed.
    pub fn id(&self) -> Option<String> {
        Some(self.id.clone())
    }

    /// Get the target message ID.
    pub fn target_id(&self) -> &str {
        &self.id
    }

    /// Set the message ID.
    pub fn set_id(&mut self, id: String) {
        self.id = id;
    }

    /// Get the message type as a string.
    pub fn message_type(&self) -> &'static str {
        "remove"
    }

    /// Get the text content of the message.
    ///
    /// RemoveMessage always returns an empty string.
    pub fn text(&self) -> &'static str {
        ""
    }

    /// Get the content of the message.
    ///
    /// RemoveMessage always returns an empty string.
    pub fn content(&self) -> &'static str {
        ""
    }

    /// Get the message name.
    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }

    /// Set the name for this message (builder pattern).
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Get additional kwargs.
    pub fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value> {
        &self.additional_kwargs
    }

    /// Set the additional kwargs for this message (builder pattern).
    pub fn with_additional_kwargs(
        mut self,
        additional_kwargs: HashMap<String, serde_json::Value>,
    ) -> Self {
        self.additional_kwargs = additional_kwargs;
        self
    }

    /// Get response metadata.
    pub fn response_metadata(&self) -> &HashMap<String, serde_json::Value> {
        &self.response_metadata
    }

    /// Set the response metadata for this message (builder pattern).
    pub fn with_response_metadata(
        mut self,
        response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        self.response_metadata = response_metadata;
        self
    }
}

impl super::base::BaseMessageTrait for RemoveMessage {
    fn content(&self) -> &str {
        RemoveMessage::content(self)
    }

    fn id(&self) -> Option<String> {
        RemoveMessage::id(self)
    }

    fn name(&self) -> Option<String> {
        RemoveMessage::name(self)
    }

    fn set_id(&mut self, id: String) {
        RemoveMessage::set_id(self, id)
    }

    fn additional_kwargs(&self) -> Option<&HashMap<String, serde_json::Value>> {
        Some(&self.additional_kwargs)
    }
}
