//! Message modifier types.
//!
//! This module contains types for modifying message history,
//! such as `RemoveMessage`. Mirrors `langchain_core.messages.modifier`.

use serde::{Deserialize, Serialize};

#[cfg(feature = "specta")]
use specta::Type;

/// Message responsible for deleting other messages.
///
/// This is used to remove messages from a conversation history by their ID.
/// This corresponds to `RemoveMessage` in LangChain Python.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RemoveMessage {
    /// The ID of the message to remove
    id: String,
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
    pub fn id(&self) -> Option<&str> {
        Some(&self.id)
    }

    /// Get the target message ID.
    pub fn target_id(&self) -> &str {
        &self.id
    }
}