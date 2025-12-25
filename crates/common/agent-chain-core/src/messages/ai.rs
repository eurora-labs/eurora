//! AI message type.
//!
//! This module contains the `AIMessage` type which represents
//! messages from an AI model. Mirrors `langchain_core.messages.ai`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(feature = "specta")]
use specta::Type;

use super::tool::ToolCall;

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
}