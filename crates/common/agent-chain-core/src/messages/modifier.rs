//! Message modifier types.
//!
//! This module contains types for modifying message history,
//! such as `RemoveMessage`. Mirrors `langchain_core.messages.modifier`.

use bon::bon;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;

use super::base::{get_msg_title_repr, is_interactive_env};
use super::content::ContentBlock;

/// Message responsible for deleting other messages.
///
/// This is used to remove messages from a conversation history by their ID.
/// This corresponds to `RemoveMessage` in LangChain Python.
///
/// # Example
///
/// ```
/// use agent_chain_core::messages::RemoveMessage;
///
/// // Simple remove message with just id
/// let msg = RemoveMessage::builder()
///     .id("msg-123")
///     .build();
///
/// // Message with name
/// let msg = RemoveMessage::builder()
///     .id("msg-123")
///     .maybe_name(Some("user".to_string()))
///     .build();
/// ```

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
        let mut field_count = 4; // type, content, id, additional_kwargs, response_metadata
        if self.name.is_some() {
            field_count += 1;
        }
        field_count += 1; // response_metadata

        let mut map = serializer.serialize_map(Some(field_count))?;
        map.serialize_entry("type", "remove")?;
        map.serialize_entry("content", "")?;
        map.serialize_entry("id", &self.id)?;
        if self.name.is_some() {
            map.serialize_entry("name", &self.name)?;
        }
        map.serialize_entry("additional_kwargs", &self.additional_kwargs)?;
        map.serialize_entry("response_metadata", &self.response_metadata)?;

        map.end()
    }
}

#[bon]
impl RemoveMessage {
    /// Create a new RemoveMessage with named parameters using the builder pattern.
    ///
    /// # Example
    ///
    /// ```
    /// use agent_chain_core::messages::RemoveMessage;
    ///
    /// // Simple remove message with just id
    /// let msg = RemoveMessage::builder()
    ///     .id("msg-123")
    ///     .build();
    ///
    /// // Message with name
    /// let msg = RemoveMessage::builder()
    ///     .id("msg-123")
    ///     .maybe_name(Some("user".to_string()))
    ///     .build();
    /// ```
    #[builder]
    pub fn new(
        id: impl Into<String>,
        name: Option<String>,
        #[builder(default)] additional_kwargs: HashMap<String, serde_json::Value>,
        #[builder(default)] response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            id: id.into(),
            name,
            additional_kwargs,
            response_metadata,
        }
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

    /// Get the content blocks of the message.
    ///
    /// RemoveMessage always returns an empty list since it has no content.
    pub fn content_blocks(&self) -> Vec<ContentBlock> {
        vec![]
    }

    /// Get a pretty representation of the message.
    ///
    /// # Arguments
    ///
    /// * `html` - Whether to format the message with bold text (using ANSI codes).
    pub fn pretty_repr(&self, html: bool) -> String {
        let title = get_msg_title_repr("Remove Message", html);
        let name_line = if let Some(name) = &self.name {
            format!("\nName: {}", name)
        } else {
            String::new()
        };
        format!("{}{}\n\n{}", title, name_line, self.content())
    }

    /// Pretty print the message to stdout.
    pub fn pretty_print(&self) {
        println!("{}", self.pretty_repr(is_interactive_env()));
    }
}
