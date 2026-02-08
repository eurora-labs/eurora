//! Chat message type.
//!
//! This module contains the `ChatMessage` and `ChatMessageChunk` types which represent
//! messages with an arbitrary speaker role. Mirrors `langchain_core.messages.chat`.

use bon::bon;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;

use super::base::merge_content;
use crate::utils::merge::merge_dicts;

/// A chat message that can be assigned an arbitrary speaker (role).
///
/// Use this when you need to specify a custom role that isn't covered
/// by the standard message types (Human, AI, System, Tool).
///
/// # Example
///
/// ```
/// use agent_chain_core::messages::ChatMessage;
///
/// // Simple message with content and role
/// let msg = ChatMessage::builder()
///     .content("Hello!")
///     .role("assistant")
///     .build();
///
/// // Message with ID and name
/// let msg = ChatMessage::builder()
///     .content("Hello!")
///     .role("assistant")
///     .maybe_id(Some("msg-123".to_string()))
///     .maybe_name(Some("bot".to_string()))
///     .build();
/// ```
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
        let mut field_count = 5;
        if self.name.is_some() {
            field_count += 1;
        }
        // Add 1 for additional type field
        field_count += 1;

        let mut map = serializer.serialize_map(Some(field_count))?;
        map.serialize_entry("type", "chat")?;
        map.serialize_entry("content", &self.content)?;
        map.serialize_entry("role", &self.role)?;
        map.serialize_entry("id", &self.id)?;
        if let Some(ref name) = self.name {
            map.serialize_entry("name", name)?;
        }

        let additional_kwargs_with_type = self.additional_kwargs.clone();
        map.serialize_entry("additional_kwargs", &additional_kwargs_with_type)?;

        map.serialize_entry("response_metadata", &self.response_metadata)?;

        map.end()
    }
}

#[bon]
impl ChatMessage {
    /// Create a new chat message with named parameters using the builder pattern.
    ///
    /// # Example
    ///
    /// ```
    /// use agent_chain_core::messages::ChatMessage;
    ///
    /// // Simple message with content and role
    /// let msg = ChatMessage::builder()
    ///     .content("Hello!")
    ///     .role("assistant")
    ///     .build();
    ///
    /// // Message with ID and name
    /// let msg = ChatMessage::builder()
    ///     .content("Hello!")
    ///     .role("assistant")
    ///     .maybe_id(Some("msg-123".to_string()))
    ///     .maybe_name(Some("bot".to_string()))
    ///     .build();
    /// ```
    #[builder]
    pub fn new(
        content: impl Into<String>,
        role: impl Into<String>,
        id: Option<String>,
        name: Option<String>,
        #[builder(default)] additional_kwargs: HashMap<String, serde_json::Value>,
        #[builder(default)] response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            content: content.into(),
            role: role.into(),
            id,
            name,
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
        "chat"
    }
}

/// Chat message chunk (yielded when streaming).
///
/// # Example
///
/// ```
/// use agent_chain_core::messages::ChatMessageChunk;
///
/// // Simple chunk with content and role
/// let chunk = ChatMessageChunk::builder()
///     .content("Hello")
///     .role("assistant")
///     .build();
///
/// // Chunk with ID
/// let chunk = ChatMessageChunk::builder()
///     .content("Hello")
///     .role("assistant")
///     .maybe_id(Some("chunk-123".to_string()))
///     .build();
/// ```
///
/// This corresponds to `ChatMessageChunk` in LangChain Python.

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ChatMessageChunk {
    /// The message content (may be partial during streaming)
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

impl Serialize for ChatMessageChunk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut field_count = 5;
        if self.name.is_some() {
            field_count += 1;
        }
        // Add 1 for additional type field
        field_count += 1;

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

#[bon]
impl ChatMessageChunk {
    /// Create a new chat message chunk with named parameters using the builder pattern.
    ///
    /// # Example
    ///
    /// ```
    /// use agent_chain_core::messages::ChatMessageChunk;
    ///
    /// // Simple chunk with content and role
    /// let chunk = ChatMessageChunk::builder()
    ///     .content("Hello")
    ///     .role("assistant")
    ///     .build();
    ///
    /// // Chunk with ID
    /// let chunk = ChatMessageChunk::builder()
    ///     .content("Hello")
    ///     .role("assistant")
    ///     .maybe_id(Some("chunk-123".to_string()))
    ///     .build();
    /// ```
    #[builder]
    pub fn new(
        content: impl Into<String>,
        role: impl Into<String>,
        id: Option<String>,
        name: Option<String>,
        #[builder(default)] additional_kwargs: HashMap<String, serde_json::Value>,
        #[builder(default)] response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            content: content.into(),
            role: role.into(),
            id,
            name,
            additional_kwargs,
            response_metadata,
        }
    }

    /// Get the message type as a string.
    pub fn message_type(&self) -> &'static str {
        "ChatMessageChunk"
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

        // Merge additional_kwargs using merge_dicts (recursive deep merge)
        let additional_kwargs = {
            let left_val = serde_json::to_value(&self.additional_kwargs).unwrap_or_default();
            let right_val = serde_json::to_value(&other.additional_kwargs).unwrap_or_default();
            match merge_dicts(left_val, vec![right_val]) {
                Ok(merged) => serde_json::from_value(merged).unwrap_or_default(),
                Err(_) => self.additional_kwargs.clone(),
            }
        };

        // Merge response_metadata using merge_dicts (recursive deep merge)
        let response_metadata = {
            let left_val = serde_json::to_value(&self.response_metadata).unwrap_or_default();
            let right_val = serde_json::to_value(&other.response_metadata).unwrap_or_default();
            match merge_dicts(left_val, vec![right_val]) {
                Ok(merged) => serde_json::from_value(merged).unwrap_or_default(),
                Err(_) => self.response_metadata.clone(),
            }
        };

        ChatMessageChunk {
            content,
            role: self.role.clone(),
            id: self.id.clone(),
            name: self.name.clone(),
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
