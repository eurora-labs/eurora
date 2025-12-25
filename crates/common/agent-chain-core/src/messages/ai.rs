//! AI message type.
//!
//! This module contains the `AIMessage` and `AIMessageChunk` types which represent
//! messages from an AI model. Mirrors `langchain_core.messages.ai`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(feature = "specta")]
use specta::Type;

use super::tool::{InvalidToolCall, ToolCall, ToolCallChunk};

/// Breakdown of input token counts.
///
/// Does *not* need to sum to full input token count. Does *not* need to have all keys.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct InputTokenDetails {
    /// Audio input tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<i64>,
    /// Input tokens that were cached and there was a cache miss.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation: Option<i64>,
    /// Input tokens that were cached and there was a cache hit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<i64>,
}

/// Breakdown of output token counts.
///
/// Does *not* need to sum to full output token count. Does *not* need to have all keys.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct OutputTokenDetails {
    /// Audio output tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<i64>,
    /// Reasoning output tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<i64>,
}

/// Usage metadata for a message, such as token counts.
///
/// This is a standard representation of token usage that is consistent across models.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct UsageMetadata {
    /// Count of input (or prompt) tokens. Sum of all input token types.
    pub input_tokens: i64,
    /// Count of output (or completion) tokens. Sum of all output token types.
    pub output_tokens: i64,
    /// Total token count. Sum of `input_tokens` + `output_tokens`.
    pub total_tokens: i64,
    /// Breakdown of input token counts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_token_details: Option<InputTokenDetails>,
    /// Breakdown of output token counts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_token_details: Option<OutputTokenDetails>,
}

impl UsageMetadata {
    /// Create a new usage metadata with the given token counts.
    pub fn new(input_tokens: i64, output_tokens: i64) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            input_token_details: None,
            output_token_details: None,
        }
    }

    /// Add another UsageMetadata to this one.
    pub fn add(&self, other: &UsageMetadata) -> Self {
        Self {
            input_tokens: self.input_tokens + other.input_tokens,
            output_tokens: self.output_tokens + other.output_tokens,
            total_tokens: self.total_tokens + other.total_tokens,
            input_token_details: match (&self.input_token_details, &other.input_token_details) {
                (Some(a), Some(b)) => Some(InputTokenDetails {
                    audio: match (a.audio, b.audio) {
                        (Some(x), Some(y)) => Some(x + y),
                        (Some(x), None) | (None, Some(x)) => Some(x),
                        (None, None) => None,
                    },
                    cache_creation: match (a.cache_creation, b.cache_creation) {
                        (Some(x), Some(y)) => Some(x + y),
                        (Some(x), None) | (None, Some(x)) => Some(x),
                        (None, None) => None,
                    },
                    cache_read: match (a.cache_read, b.cache_read) {
                        (Some(x), Some(y)) => Some(x + y),
                        (Some(x), None) | (None, Some(x)) => Some(x),
                        (None, None) => None,
                    },
                }),
                (Some(a), None) => Some(a.clone()),
                (None, Some(b)) => Some(b.clone()),
                (None, None) => None,
            },
            output_token_details: match (&self.output_token_details, &other.output_token_details) {
                (Some(a), Some(b)) => Some(OutputTokenDetails {
                    audio: match (a.audio, b.audio) {
                        (Some(x), Some(y)) => Some(x + y),
                        (Some(x), None) | (None, Some(x)) => Some(x),
                        (None, None) => None,
                    },
                    reasoning: match (a.reasoning, b.reasoning) {
                        (Some(x), Some(y)) => Some(x + y),
                        (Some(x), None) | (None, Some(x)) => Some(x),
                        (None, None) => None,
                    },
                }),
                (Some(a), None) => Some(a.clone()),
                (None, Some(b)) => Some(b.clone()),
                (None, None) => None,
            },
        }
    }
}

/// An AI message in the conversation.
///
/// An `AIMessage` is returned from a chat model as a response to a prompt.
/// This message represents the output of the model and consists of both
/// the raw output as returned by the model and standardized fields
/// (e.g., tool calls, usage metadata).
///
/// This corresponds to `AIMessage` in LangChain Python.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AIMessage {
    /// The message content
    content: String,
    /// Optional unique identifier
    id: Option<String>,
    /// Optional name for the message
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// Tool calls made by the AI
    #[serde(default)]
    tool_calls: Vec<ToolCall>,
    /// Tool calls with parsing errors associated with the message
    #[serde(default)]
    invalid_tool_calls: Vec<InvalidToolCall>,
    /// If present, usage metadata for a message, such as token counts.
    #[serde(skip_serializing_if = "Option::is_none")]
    usage_metadata: Option<UsageMetadata>,
    /// Additional metadata
    #[serde(default)]
    additional_kwargs: HashMap<String, serde_json::Value>,
    /// Response metadata (e.g., response headers, logprobs, token counts, model name)
    #[serde(default)]
    response_metadata: HashMap<String, serde_json::Value>,
}

impl AIMessage {
    /// Create a new AI message.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            id: Some(Uuid::new_v4().to_string()),
            name: None,
            tool_calls: Vec::new(),
            invalid_tool_calls: Vec::new(),
            usage_metadata: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Create a new AI message with an explicit ID.
    ///
    /// Use this when deserializing or reconstructing messages where the ID must be preserved.
    pub fn with_id(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            id: Some(id.into()),
            name: None,
            tool_calls: Vec::new(),
            invalid_tool_calls: Vec::new(),
            usage_metadata: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Create a new AI message with tool calls.
    pub fn with_tool_calls(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            content: content.into(),
            id: Some(Uuid::new_v4().to_string()),
            name: None,
            tool_calls,
            invalid_tool_calls: Vec::new(),
            usage_metadata: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Create a new AI message with tool calls and an explicit ID.
    ///
    /// Use this when deserializing or reconstructing messages where the ID must be preserved.
    pub fn with_id_and_tool_calls(
        id: impl Into<String>,
        content: impl Into<String>,
        tool_calls: Vec<ToolCall>,
    ) -> Self {
        Self {
            content: content.into(),
            id: Some(id.into()),
            name: None,
            tool_calls,
            invalid_tool_calls: Vec::new(),
            usage_metadata: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Create a new AI message with both valid and invalid tool calls.
    pub fn with_all_tool_calls(
        content: impl Into<String>,
        tool_calls: Vec<ToolCall>,
        invalid_tool_calls: Vec<InvalidToolCall>,
    ) -> Self {
        Self {
            content: content.into(),
            id: Some(Uuid::new_v4().to_string()),
            name: None,
            tool_calls,
            invalid_tool_calls,
            usage_metadata: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Set the name for this message.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set usage metadata for this message.
    pub fn with_usage_metadata(mut self, usage_metadata: UsageMetadata) -> Self {
        self.usage_metadata = Some(usage_metadata);
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

    /// Get the tool calls.
    pub fn tool_calls(&self) -> &[ToolCall] {
        &self.tool_calls
    }

    /// Get the invalid tool calls.
    pub fn invalid_tool_calls(&self) -> &[InvalidToolCall] {
        &self.invalid_tool_calls
    }

    /// Get usage metadata if present.
    pub fn usage_metadata(&self) -> Option<&UsageMetadata> {
        self.usage_metadata.as_ref()
    }

    /// Add annotations to the message (e.g., citations from web search).
    /// Annotations are stored in additional_kwargs under the "annotations" key.
    pub fn with_annotations<T: Serialize>(mut self, annotations: Vec<T>) -> Self {
        if let Ok(value) = serde_json::to_value(&annotations) {
            self.additional_kwargs
                .insert("annotations".to_string(), value);
        }
        self
    }

    /// Get annotations from the message if present.
    pub fn annotations(&self) -> Option<&serde_json::Value> {
        self.additional_kwargs.get("annotations")
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

    /// Set additional kwargs.
    pub fn with_additional_kwargs(
        mut self,
        additional_kwargs: HashMap<String, serde_json::Value>,
    ) -> Self {
        self.additional_kwargs = additional_kwargs;
        self
    }
}

/// AI message chunk (yielded when streaming).
///
/// This is returned from a chat model during streaming to incrementally
/// build up a complete AIMessage.
///
/// This corresponds to `AIMessageChunk` in LangChain Python.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AIMessageChunk {
    /// The message content (may be partial during streaming)
    content: String,
    /// Optional unique identifier
    id: Option<String>,
    /// Optional name for the message
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// Tool calls made by the AI
    #[serde(default)]
    tool_calls: Vec<ToolCall>,
    /// Tool calls with parsing errors
    #[serde(default)]
    invalid_tool_calls: Vec<InvalidToolCall>,
    /// Tool call chunks (for streaming tool calls)
    #[serde(default)]
    tool_call_chunks: Vec<ToolCallChunk>,
    /// If present, usage metadata for a message
    #[serde(skip_serializing_if = "Option::is_none")]
    usage_metadata: Option<UsageMetadata>,
    /// Additional metadata
    #[serde(default)]
    additional_kwargs: HashMap<String, serde_json::Value>,
    /// Response metadata
    #[serde(default)]
    response_metadata: HashMap<String, serde_json::Value>,
}

impl AIMessageChunk {
    /// Create a new AI message chunk.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            id: None,
            name: None,
            tool_calls: Vec::new(),
            invalid_tool_calls: Vec::new(),
            tool_call_chunks: Vec::new(),
            usage_metadata: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Create a new AI message chunk with an ID.
    pub fn with_id(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            id: Some(id.into()),
            name: None,
            tool_calls: Vec::new(),
            invalid_tool_calls: Vec::new(),
            tool_call_chunks: Vec::new(),
            usage_metadata: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Create a new AI message chunk with tool call chunks.
    pub fn with_tool_call_chunks(
        content: impl Into<String>,
        tool_call_chunks: Vec<ToolCallChunk>,
    ) -> Self {
        Self {
            content: content.into(),
            id: None,
            name: None,
            tool_calls: Vec::new(),
            invalid_tool_calls: Vec::new(),
            tool_call_chunks,
            usage_metadata: None,
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

    /// Get the tool calls.
    pub fn tool_calls(&self) -> &[ToolCall] {
        &self.tool_calls
    }

    /// Get the invalid tool calls.
    pub fn invalid_tool_calls(&self) -> &[InvalidToolCall] {
        &self.invalid_tool_calls
    }

    /// Get the tool call chunks.
    pub fn tool_call_chunks(&self) -> &[ToolCallChunk] {
        &self.tool_call_chunks
    }

    /// Get usage metadata if present.
    pub fn usage_metadata(&self) -> Option<&UsageMetadata> {
        self.usage_metadata.as_ref()
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
    /// This merges content, tool_call_chunks, and metadata.
    pub fn concat(&self, other: &AIMessageChunk) -> AIMessageChunk {
        let mut content = self.content.clone();
        content.push_str(&other.content);

        let mut tool_call_chunks = self.tool_call_chunks.clone();
        tool_call_chunks.extend(other.tool_call_chunks.clone());

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

        // Merge usage metadata
        let usage_metadata = match (&self.usage_metadata, &other.usage_metadata) {
            (Some(a), Some(b)) => Some(a.add(b)),
            (Some(a), None) => Some(a.clone()),
            (None, Some(b)) => Some(b.clone()),
            (None, None) => None,
        };

        AIMessageChunk {
            content,
            id: self.id.clone().or_else(|| other.id.clone()),
            name: self.name.clone().or_else(|| other.name.clone()),
            tool_calls: self.tool_calls.clone(),
            invalid_tool_calls: self.invalid_tool_calls.clone(),
            tool_call_chunks,
            usage_metadata,
            additional_kwargs,
            response_metadata,
        }
    }

    /// Convert this chunk to a complete AIMessage.
    pub fn to_message(&self) -> AIMessage {
        AIMessage {
            content: self.content.clone(),
            id: self.id.clone(),
            name: self.name.clone(),
            tool_calls: self.tool_calls.clone(),
            invalid_tool_calls: self.invalid_tool_calls.clone(),
            usage_metadata: self.usage_metadata.clone(),
            additional_kwargs: self.additional_kwargs.clone(),
            response_metadata: self.response_metadata.clone(),
        }
    }
}

impl std::ops::Add for AIMessageChunk {
    type Output = AIMessageChunk;

    fn add(self, other: AIMessageChunk) -> AIMessageChunk {
        self.concat(&other)
    }
}

/// Add two UsageMetadata objects together.
///
/// This function adds the token counts from both UsageMetadata objects.
pub fn add_usage(left: Option<&UsageMetadata>, right: Option<&UsageMetadata>) -> UsageMetadata {
    match (left, right) {
        (Some(l), Some(r)) => l.add(r),
        (Some(l), None) => l.clone(),
        (None, Some(r)) => r.clone(),
        (None, None) => UsageMetadata::default(),
    }
}

/// Subtract two UsageMetadata objects.
///
/// Token counts cannot be negative so the actual operation is `max(left - right, 0)`.
pub fn subtract_usage(
    left: Option<&UsageMetadata>,
    right: Option<&UsageMetadata>,
) -> UsageMetadata {
    match (left, right) {
        (Some(l), Some(r)) => UsageMetadata {
            input_tokens: (l.input_tokens - r.input_tokens).max(0),
            output_tokens: (l.output_tokens - r.output_tokens).max(0),
            total_tokens: (l.total_tokens - r.total_tokens).max(0),
            input_token_details: match (&l.input_token_details, &r.input_token_details) {
                (Some(a), Some(b)) => Some(InputTokenDetails {
                    audio: a.audio.map(|x| (x - b.audio.unwrap_or(0)).max(0)),
                    cache_creation: a
                        .cache_creation
                        .map(|x| (x - b.cache_creation.unwrap_or(0)).max(0)),
                    cache_read: a.cache_read.map(|x| (x - b.cache_read.unwrap_or(0)).max(0)),
                }),
                (Some(a), None) => Some(a.clone()),
                (None, Some(b)) => Some(InputTokenDetails {
                    audio: b.audio.map(|_| 0),
                    cache_creation: b.cache_creation.map(|_| 0),
                    cache_read: b.cache_read.map(|_| 0),
                }),
                (None, None) => None,
            },
            output_token_details: match (&l.output_token_details, &r.output_token_details) {
                (Some(a), Some(b)) => Some(OutputTokenDetails {
                    audio: a.audio.map(|x| (x - b.audio.unwrap_or(0)).max(0)),
                    reasoning: a.reasoning.map(|x| (x - b.reasoning.unwrap_or(0)).max(0)),
                }),
                (Some(a), None) => Some(a.clone()),
                (None, Some(b)) => Some(OutputTokenDetails {
                    audio: b.audio.map(|_| 0),
                    reasoning: b.reasoning.map(|_| 0),
                }),
                (None, None) => None,
            },
        },
        (Some(l), None) => l.clone(),
        (None, Some(_)) => UsageMetadata::default(),
        (None, None) => UsageMetadata::default(),
    }
}
