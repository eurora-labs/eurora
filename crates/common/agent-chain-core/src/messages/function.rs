//! Function message type.
//!
//! This module contains the `FunctionMessage` and `FunctionMessageChunk` types which represent
//! messages for passing the result of executing a function back to a model.
//! Mirrors `langchain_core.messages.function`.
//!
//! Note: FunctionMessage is an older version of ToolMessage and doesn't contain
//! the `tool_call_id` field. Consider using ToolMessage for new code.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(feature = "specta")]
use specta::Type;

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
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FunctionMessage {
    /// The message content (result of the function)
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

impl FunctionMessage {
    /// Create a new function message.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the function that was executed.
    /// * `content` - The result of the function execution.
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            name: name.into(),
            id: Some(Uuid::new_v4().to_string()),
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Create a new function message with an explicit ID.
    ///
    /// Use this when deserializing or reconstructing messages where the ID must be preserved.
    pub fn with_id(
        id: impl Into<String>,
        name: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            content: content.into(),
            name: name.into(),
            id: Some(id.into()),
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
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
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
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

/// Function message chunk (yielded when streaming).
///
/// This corresponds to `FunctionMessageChunk` in LangChain Python.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

impl FunctionMessageChunk {
    /// Create a new function message chunk.
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
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
        name: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            content: content.into(),
            name: name.into(),
            id: Some(id.into()),
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
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
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
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