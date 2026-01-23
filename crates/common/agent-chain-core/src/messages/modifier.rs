//! Message modifier types.
//!
//! This module contains types for modifying message history,
//! such as `RemoveMessage`. Mirrors `langchain_core.messages.modifier`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Message responsible for deleting other messages.
///
/// This is used to remove messages from a conversation history by their ID.
/// This corresponds to `RemoveMessage` in LangChain Python.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RemoveMessage {
    /// The ID of the message to remove
    pub id: String,
}

impl RemoveMessage {
    /// Create a new RemoveMessage.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the message to remove.
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
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
}

impl super::base::MessageLikeTrait for RemoveMessage {
    fn content(&self) -> &str {
        ""
    }

    fn id(&self) -> Option<String> {
        RemoveMessage::id(self)
    }

    fn name(&self) -> Option<String> {
        None
    }

    fn set_id(&mut self, id: String) {
        RemoveMessage::set_id(self, id)
    }

    fn additional_kwargs(&self) -> Option<&HashMap<String, serde_json::Value>> {
        None
    }
}
