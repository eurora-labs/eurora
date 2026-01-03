//! AI message type.
//!
//! This module contains the `AIMessage` and `AIMessageChunk` types which represent
//! messages from an AI model. Mirrors `langchain_core.messages.ai`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "specta")]
use specta::Type;

use super::tool::{
    InvalidToolCall, ToolCall, ToolCallChunk, default_tool_chunk_parser, default_tool_parser,
    invalid_tool_call, tool_call,
};
use crate::utils::base::{LC_AUTO_PREFIX, LC_ID_PREFIX, ensure_id};
use crate::utils::json::parse_partial_json;
use crate::utils::merge::{merge_dicts, merge_lists};
use crate::utils::usage::{dict_int_add_json, dict_int_sub_floor_json};

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
            id: Some(ensure_id(None)),
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
            id: Some(ensure_id(None)),
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
            id: Some(ensure_id(None)),
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

    /// Set invalid tool calls for this message.
    pub fn with_invalid_tool_calls(mut self, invalid_tool_calls: Vec<InvalidToolCall>) -> Self {
        self.invalid_tool_calls = invalid_tool_calls;
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

    /// Get a pretty representation of the message.
    ///
    /// This corresponds to `pretty_repr` in LangChain Python.
    pub fn pretty_repr(&self, _html: bool) -> String {
        let title = "AI Message";
        let sep_len = (80 - title.len() - 2) / 2;
        let sep: String = "=".repeat(sep_len);
        let header = format!("{} {} {}", sep, title, sep);

        let mut lines = vec![header];

        if let Some(name) = &self.name {
            lines.push(format!("Name: {}", name));
        }

        lines.push(String::new());
        lines.push(self.content.clone());

        format_tool_calls_repr(&self.tool_calls, &self.invalid_tool_calls, &mut lines);

        lines.join("\n").trim().to_string()
    }
}

/// Helper function to format tool calls for pretty_repr.
fn format_tool_calls_repr(
    tool_calls: &[ToolCall],
    invalid_tool_calls: &[InvalidToolCall],
    lines: &mut Vec<String>,
) {
    if !tool_calls.is_empty() {
        lines.push("Tool Calls:".to_string());
        for tc in tool_calls {
            lines.push(format!("  {} ({})", tc.name(), tc.id()));
            lines.push(format!(" Call ID: {}", tc.id()));
            lines.push("  Args:".to_string());
            if let serde_json::Value::Object(args) = tc.args() {
                for (arg, value) in args {
                    lines.push(format!("    {}: {}", arg, value));
                }
            } else {
                lines.push(format!("    {}", tc.args()));
            }
        }
    }
    if !invalid_tool_calls.is_empty() {
        lines.push("Invalid Tool Calls:".to_string());
        for itc in invalid_tool_calls {
            let name = itc.name.as_deref().unwrap_or("Tool");
            let id = itc.id.as_deref().unwrap_or("unknown");
            lines.push(format!("  {} ({})", name, id));
            lines.push(format!(" Call ID: {}", id));
            if let Some(error) = &itc.error {
                lines.push(format!("  Error: {}", error));
            }
            lines.push("  Args:".to_string());
            if let Some(args) = &itc.args {
                lines.push(format!("    {}", args));
            }
        }
    }
}

/// Position indicator for an aggregated AIMessageChunk.
///
/// If a chunk with `chunk_position="last"` is aggregated into a stream,
/// `tool_call_chunks` in message content will be parsed into `tool_calls`.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChunkPosition {
    /// This is the last chunk in the stream
    Last,
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
    /// Optional span represented by an aggregated AIMessageChunk.
    ///
    /// If a chunk with `chunk_position=Some(ChunkPosition::Last)` is aggregated into a stream,
    /// `tool_call_chunks` in message content will be parsed into `tool_calls`.
    #[serde(skip_serializing_if = "Option::is_none")]
    chunk_position: Option<ChunkPosition>,
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
            chunk_position: None,
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
            chunk_position: None,
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
            chunk_position: None,
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

    /// Get chunk position.
    pub fn chunk_position(&self) -> Option<&ChunkPosition> {
        self.chunk_position.as_ref()
    }

    /// Set chunk position.
    pub fn set_chunk_position(&mut self, position: Option<ChunkPosition>) {
        self.chunk_position = position;
    }

    /// Set tool calls.
    pub fn set_tool_calls(&mut self, tool_calls: Vec<ToolCall>) {
        self.tool_calls = tool_calls;
    }

    /// Set invalid tool calls.
    pub fn set_invalid_tool_calls(&mut self, invalid_tool_calls: Vec<InvalidToolCall>) {
        self.invalid_tool_calls = invalid_tool_calls;
    }

    /// Set tool call chunks.
    pub fn set_tool_call_chunks(&mut self, tool_call_chunks: Vec<ToolCallChunk>) {
        self.tool_call_chunks = tool_call_chunks;
    }

    /// Initialize tool calls from tool call chunks.
    ///
    /// This parses the tool_call_chunks and populates tool_calls and invalid_tool_calls.
    /// This corresponds to `init_tool_calls` model validator in Python.
    pub fn init_tool_calls(&mut self) {
        if self.tool_call_chunks.is_empty() {
            if !self.tool_calls.is_empty() {
                self.tool_call_chunks = self
                    .tool_calls
                    .iter()
                    .map(|tc| ToolCallChunk {
                        name: Some(tc.name().to_string()),
                        args: Some(tc.args().to_string()),
                        id: Some(tc.id().to_string()),
                        index: None,
                    })
                    .collect();
            }
            if !self.invalid_tool_calls.is_empty() {
                self.tool_call_chunks
                    .extend(self.invalid_tool_calls.iter().map(|tc| ToolCallChunk {
                        name: tc.name.clone(),
                        args: tc.args.clone(),
                        id: tc.id.clone(),
                        index: None,
                    }));
            }
            return;
        }

        let mut new_tool_calls = Vec::new();
        let mut new_invalid_tool_calls = Vec::new();

        for chunk in &self.tool_call_chunks {
            let args_result = if let Some(args_str) = &chunk.args {
                if args_str.is_empty() {
                    Ok(serde_json::Value::Object(serde_json::Map::new()))
                } else {
                    parse_partial_json(args_str, false)
                }
            } else {
                Ok(serde_json::Value::Object(serde_json::Map::new()))
            };

            match args_result {
                Ok(args) if args.is_object() => {
                    new_tool_calls.push(tool_call(
                        chunk.name.clone().unwrap_or_default(),
                        args,
                        chunk.id.clone(),
                    ));
                }
                _ => {
                    new_invalid_tool_calls.push(invalid_tool_call(
                        chunk.name.clone(),
                        chunk.args.clone(),
                        chunk.id.clone(),
                        None,
                    ));
                }
            }
        }

        self.tool_calls = new_tool_calls;
        self.invalid_tool_calls = new_invalid_tool_calls;
    }

    /// Concatenate this chunk with another chunk.
    ///
    /// This merges content, tool_call_chunks, and metadata.
    /// For more sophisticated merging of multiple chunks, use `add_ai_message_chunks`.
    pub fn concat(&self, other: &AIMessageChunk) -> AIMessageChunk {
        add_ai_message_chunks(self.clone(), vec![other.clone()])
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

    /// Get a pretty representation of the message.
    ///
    /// This corresponds to `pretty_repr` in LangChain Python.
    pub fn pretty_repr(&self, _html: bool) -> String {
        let title = "AIMessageChunk";
        let sep_len = (80 - title.len() - 2) / 2;
        let sep: String = "=".repeat(sep_len);
        let header = format!("{} {} {}", sep, title, sep);

        let mut lines = vec![header];

        if let Some(name) = &self.name {
            lines.push(format!("Name: {}", name));
        }

        lines.push(String::new());
        lines.push(self.content.clone());

        format_tool_calls_repr(&self.tool_calls, &self.invalid_tool_calls, &mut lines);

        lines.join("\n").trim().to_string()
    }
}

/// Add multiple AIMessageChunks together.
///
/// This corresponds to `add_ai_message_chunks` in LangChain Python.
///
/// # Arguments
///
/// * `left` - The first AIMessageChunk.
/// * `others` - Other AIMessageChunks to add.
///
/// # Returns
///
/// The resulting AIMessageChunk.
pub fn add_ai_message_chunks(left: AIMessageChunk, others: Vec<AIMessageChunk>) -> AIMessageChunk {
    // Merge content (simple string concatenation for now)
    let mut content = left.content.clone();
    for other in &others {
        content.push_str(&other.content);
    }

    // Merge additional_kwargs using merge_dicts
    let additional_kwargs = {
        let left_val = serde_json::to_value(&left.additional_kwargs).unwrap_or_default();
        let other_vals: Vec<serde_json::Value> = others
            .iter()
            .map(|o| serde_json::to_value(&o.additional_kwargs).unwrap_or_default())
            .collect();
        match merge_dicts(left_val, other_vals) {
            Ok(merged) => serde_json::from_value(merged).unwrap_or_default(),
            Err(_) => left.additional_kwargs.clone(),
        }
    };

    // Merge response_metadata using merge_dicts
    let response_metadata = {
        let left_val = serde_json::to_value(&left.response_metadata).unwrap_or_default();
        let other_vals: Vec<serde_json::Value> = others
            .iter()
            .map(|o| serde_json::to_value(&o.response_metadata).unwrap_or_default())
            .collect();
        match merge_dicts(left_val, other_vals) {
            Ok(merged) => serde_json::from_value(merged).unwrap_or_default(),
            Err(_) => left.response_metadata.clone(),
        }
    };

    // Merge tool_call_chunks using merge_lists
    let tool_call_chunks = {
        let left_chunks: Vec<serde_json::Value> = left
            .tool_call_chunks
            .iter()
            .filter_map(|tc| serde_json::to_value(tc).ok())
            .collect();
        let other_chunks: Vec<Option<Vec<serde_json::Value>>> = others
            .iter()
            .map(|o| {
                Some(
                    o.tool_call_chunks
                        .iter()
                        .filter_map(|tc| serde_json::to_value(tc).ok())
                        .collect(),
                )
            })
            .collect();

        match merge_lists(Some(left_chunks), other_chunks) {
            Ok(Some(merged)) => merged
                .into_iter()
                .map(|v| {
                    let name = v.get("name").and_then(|n| n.as_str()).map(String::from);
                    let args = v.get("args").and_then(|a| a.as_str()).map(String::from);
                    let id = v.get("id").and_then(|i| i.as_str()).map(String::from);
                    let index = v.get("index").and_then(|i| i.as_i64()).map(|i| i as i32);
                    ToolCallChunk {
                        name,
                        args,
                        id,
                        index,
                    }
                })
                .collect(),
            _ => {
                let mut chunks = left.tool_call_chunks.clone();
                for other in &others {
                    chunks.extend(other.tool_call_chunks.clone());
                }
                chunks
            }
        }
    };

    // Merge usage metadata
    let usage_metadata =
        if left.usage_metadata.is_some() || others.iter().any(|o| o.usage_metadata.is_some()) {
            let mut result = left.usage_metadata.clone();
            for other in &others {
                result = Some(add_usage(result.as_ref(), other.usage_metadata.as_ref()));
            }
            result
        } else {
            None
        };

    // Select ID with priority: provider-assigned > lc_run-* > lc_*
    let chunk_id = {
        let mut candidates = vec![left.id.as_deref()];
        candidates.extend(others.iter().map(|o| o.id.as_deref()));

        // First pass: pick the first provider-assigned id (non-run-* and non-lc_*)
        let mut selected_id: Option<&str> = None;
        for id_str in candidates.iter().flatten() {
            if !id_str.starts_with(LC_ID_PREFIX) && !id_str.starts_with(LC_AUTO_PREFIX) {
                selected_id = Some(id_str);
                break;
            }
        }

        // Second pass: prefer lc_run-* IDs over lc_* IDs
        if selected_id.is_none() {
            for id_str in candidates.iter().flatten() {
                if id_str.starts_with(LC_ID_PREFIX) {
                    selected_id = Some(id_str);
                    break;
                }
            }
        }

        // Third pass: take any remaining ID (auto-generated lc_* IDs)
        if selected_id.is_none()
            && let Some(id_str) = candidates.iter().flatten().next()
        {
            selected_id = Some(id_str);
        }

        selected_id.map(String::from)
    };

    // Determine chunk_position: if any chunk has "last", result is "last"
    let chunk_position = if left.chunk_position == Some(ChunkPosition::Last)
        || others
            .iter()
            .any(|o| o.chunk_position == Some(ChunkPosition::Last))
    {
        Some(ChunkPosition::Last)
    } else {
        None
    };

    let mut result = AIMessageChunk {
        content,
        id: chunk_id,
        name: left
            .name
            .clone()
            .or_else(|| others.iter().find_map(|o| o.name.clone())),
        tool_calls: left.tool_calls.clone(),
        invalid_tool_calls: left.invalid_tool_calls.clone(),
        tool_call_chunks,
        usage_metadata,
        additional_kwargs,
        response_metadata,
        chunk_position,
    };

    // Initialize tool calls from chunks if this is the last chunk
    if result.chunk_position == Some(ChunkPosition::Last) {
        result.init_tool_calls();
    }

    result
}

impl std::ops::Add for AIMessageChunk {
    type Output = AIMessageChunk;

    fn add(self, other: AIMessageChunk) -> AIMessageChunk {
        add_ai_message_chunks(self, vec![other])
    }
}

impl std::iter::Sum for AIMessageChunk {
    fn sum<I: Iterator<Item = AIMessageChunk>>(iter: I) -> AIMessageChunk {
        let chunks: Vec<AIMessageChunk> = iter.collect();
        if chunks.is_empty() {
            AIMessageChunk::new("")
        } else {
            let first = chunks[0].clone();
            let rest = chunks[1..].to_vec();
            add_ai_message_chunks(first, rest)
        }
    }
}

/// Add two UsageMetadata objects together.
///
/// This function recursively adds the token counts from both UsageMetadata objects.
/// Uses the generic `_dict_int_op` pattern from Python.
///
/// # Example
///
/// ```
/// use agent_chain_core::messages::{add_usage, UsageMetadata, InputTokenDetails};
///
/// let left = UsageMetadata {
///     input_tokens: 5,
///     output_tokens: 0,
///     total_tokens: 5,
///     input_token_details: Some(InputTokenDetails {
///         audio: None,
///         cache_creation: None,
///         cache_read: Some(3),
///     }),
///     output_token_details: None,
/// };
/// let right = UsageMetadata {
///     input_tokens: 0,
///     output_tokens: 10,
///     total_tokens: 10,
///     input_token_details: None,
///     output_token_details: None,
/// };
///
/// let result = add_usage(Some(&left), Some(&right));
/// assert_eq!(result.input_tokens, 5);
/// assert_eq!(result.output_tokens, 10);
/// assert_eq!(result.total_tokens, 15);
/// ```
pub fn add_usage(left: Option<&UsageMetadata>, right: Option<&UsageMetadata>) -> UsageMetadata {
    match (left, right) {
        (None, None) => UsageMetadata::default(),
        (Some(l), None) => l.clone(),
        (None, Some(r)) => r.clone(),
        (Some(l), Some(r)) => {
            let left_json = serde_json::to_value(l).unwrap_or_default();
            let right_json = serde_json::to_value(r).unwrap_or_default();

            match dict_int_add_json(&left_json, &right_json) {
                Ok(merged) => serde_json::from_value(merged).unwrap_or_else(|_| l.add(r)),
                Err(_) => l.add(r),
            }
        }
    }
}

/// Subtract two UsageMetadata objects.
///
/// Token counts cannot be negative so the actual operation is `max(left - right, 0)`.
/// Uses the generic `_dict_int_op` pattern from Python.
///
/// # Example
///
/// ```
/// use agent_chain_core::messages::{subtract_usage, UsageMetadata, InputTokenDetails};
///
/// let left = UsageMetadata {
///     input_tokens: 5,
///     output_tokens: 10,
///     total_tokens: 15,
///     input_token_details: Some(InputTokenDetails {
///         audio: None,
///         cache_creation: None,
///         cache_read: Some(4),
///     }),
///     output_token_details: None,
/// };
/// let right = UsageMetadata {
///     input_tokens: 3,
///     output_tokens: 8,
///     total_tokens: 11,
///     input_token_details: None,
///     output_token_details: None,
/// };
///
/// let result = subtract_usage(Some(&left), Some(&right));
/// assert_eq!(result.input_tokens, 2);
/// assert_eq!(result.output_tokens, 2);
/// assert_eq!(result.total_tokens, 4);
/// ```
pub fn subtract_usage(
    left: Option<&UsageMetadata>,
    right: Option<&UsageMetadata>,
) -> UsageMetadata {
    match (left, right) {
        (None, None) => UsageMetadata::default(),
        (Some(l), None) => l.clone(),
        (None, Some(_)) => UsageMetadata::default(),
        (Some(l), Some(r)) => {
            let left_json = serde_json::to_value(l).unwrap_or_default();
            let right_json = serde_json::to_value(r).unwrap_or_default();

            match dict_int_sub_floor_json(&left_json, &right_json) {
                Ok(subtracted) => {
                    serde_json::from_value(subtracted).unwrap_or_else(|_| subtract_manual(l, r))
                }
                Err(_) => subtract_manual(l, r),
            }
        }
    }
}

/// Manual subtraction fallback for UsageMetadata.
fn subtract_manual(l: &UsageMetadata, r: &UsageMetadata) -> UsageMetadata {
    UsageMetadata {
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
    }
}

/// Parse tool calls from additional_kwargs for backwards compatibility.
///
/// This corresponds to `_backwards_compat_tool_calls` in LangChain Python.
/// It checks `additional_kwargs["tool_calls"]` and parses them into
/// either `tool_calls`/`invalid_tool_calls` (for AIMessage) or
/// `tool_call_chunks` (for AIMessageChunk).
///
/// # Arguments
///
/// * `additional_kwargs` - The additional_kwargs HashMap to check
/// * `is_chunk` - Whether this is for an AIMessageChunk (uses chunk parser) or AIMessage
///
/// # Returns
///
/// A tuple of (tool_calls, invalid_tool_calls, tool_call_chunks) where only
/// the appropriate fields are populated based on `is_chunk`.
pub fn backwards_compat_tool_calls(
    additional_kwargs: &HashMap<String, serde_json::Value>,
    is_chunk: bool,
) -> (Vec<ToolCall>, Vec<InvalidToolCall>, Vec<ToolCallChunk>) {
    let mut tool_calls = Vec::new();
    let mut invalid_tool_calls = Vec::new();
    let mut tool_call_chunks = Vec::new();

    if let Some(raw_tool_calls) = additional_kwargs.get("tool_calls")
        && let Some(raw_array) = raw_tool_calls.as_array()
    {
        if is_chunk {
            tool_call_chunks = default_tool_chunk_parser(raw_array);
        } else {
            let (parsed_calls, parsed_invalid) = default_tool_parser(raw_array);
            tool_calls = parsed_calls;
            invalid_tool_calls = parsed_invalid;
        }
    }

    (tool_calls, invalid_tool_calls, tool_call_chunks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_add_usage_basic() {
        let left = UsageMetadata {
            input_tokens: 5,
            output_tokens: 0,
            total_tokens: 5,
            input_token_details: Some(InputTokenDetails {
                audio: None,
                cache_creation: None,
                cache_read: Some(3),
            }),
            output_token_details: None,
        };
        let right = UsageMetadata {
            input_tokens: 0,
            output_tokens: 10,
            total_tokens: 10,
            input_token_details: None,
            output_token_details: Some(OutputTokenDetails {
                audio: None,
                reasoning: Some(4),
            }),
        };

        let result = add_usage(Some(&left), Some(&right));

        assert_eq!(result.input_tokens, 5);
        assert_eq!(result.output_tokens, 10);
        assert_eq!(result.total_tokens, 15);
        assert!(result.input_token_details.is_some());
        assert_eq!(
            result.input_token_details.as_ref().unwrap().cache_read,
            Some(3)
        );
        assert!(result.output_token_details.is_some());
        assert_eq!(
            result.output_token_details.as_ref().unwrap().reasoning,
            Some(4)
        );
    }

    #[test]
    fn test_add_usage_none_cases() {
        let usage = UsageMetadata::new(10, 20);

        // Both None
        let result = add_usage(None, None);
        assert_eq!(result.input_tokens, 0);
        assert_eq!(result.output_tokens, 0);
        assert_eq!(result.total_tokens, 0);

        // Left Some, Right None
        let result = add_usage(Some(&usage), None);
        assert_eq!(result.input_tokens, 10);
        assert_eq!(result.output_tokens, 20);

        // Left None, Right Some
        let result = add_usage(None, Some(&usage));
        assert_eq!(result.input_tokens, 10);
        assert_eq!(result.output_tokens, 20);
    }

    #[test]
    fn test_subtract_usage_basic() {
        let left = UsageMetadata {
            input_tokens: 5,
            output_tokens: 10,
            total_tokens: 15,
            input_token_details: Some(InputTokenDetails {
                audio: None,
                cache_creation: None,
                cache_read: Some(4),
            }),
            output_token_details: None,
        };
        let right = UsageMetadata {
            input_tokens: 3,
            output_tokens: 8,
            total_tokens: 11,
            input_token_details: None,
            output_token_details: Some(OutputTokenDetails {
                audio: None,
                reasoning: Some(4),
            }),
        };

        let result = subtract_usage(Some(&left), Some(&right));

        assert_eq!(result.input_tokens, 2);
        assert_eq!(result.output_tokens, 2);
        assert_eq!(result.total_tokens, 4);
        // cache_read should remain 4 (4 - 0 = 4)
        assert!(result.input_token_details.is_some());
        assert_eq!(
            result.input_token_details.as_ref().unwrap().cache_read,
            Some(4)
        );
        // reasoning should be 0 (0 - 4 = -4, floored to 0)
        assert!(result.output_token_details.is_some());
        assert_eq!(
            result.output_token_details.as_ref().unwrap().reasoning,
            Some(0)
        );
    }

    #[test]
    fn test_subtract_usage_floor_at_zero() {
        let left = UsageMetadata::new(5, 5);
        let right = UsageMetadata::new(10, 10);

        let result = subtract_usage(Some(&left), Some(&right));

        // Should floor at 0, not go negative
        assert_eq!(result.input_tokens, 0);
        assert_eq!(result.output_tokens, 0);
        assert_eq!(result.total_tokens, 0);
    }

    #[test]
    fn test_subtract_usage_none_cases() {
        let usage = UsageMetadata::new(10, 20);

        // Both None
        let result = subtract_usage(None, None);
        assert_eq!(result.input_tokens, 0);

        // Left Some, Right None - should return left unchanged
        let result = subtract_usage(Some(&usage), None);
        assert_eq!(result.input_tokens, 10);
        assert_eq!(result.output_tokens, 20);

        // Left None, Right Some - should return default (zeroes)
        let result = subtract_usage(None, Some(&usage));
        assert_eq!(result.input_tokens, 0);
        assert_eq!(result.output_tokens, 0);
    }

    #[test]
    fn test_backwards_compat_tool_calls_for_message() {
        let mut additional_kwargs = HashMap::new();
        additional_kwargs.insert(
            "tool_calls".to_string(),
            json!([
                {
                    "id": "call_123",
                    "function": {
                        "name": "get_weather",
                        "arguments": "{\"city\": \"London\"}"
                    }
                }
            ]),
        );

        let (tool_calls, invalid_tool_calls, tool_call_chunks) =
            backwards_compat_tool_calls(&additional_kwargs, false);

        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].name(), "get_weather");
        assert!(invalid_tool_calls.is_empty());
        assert!(tool_call_chunks.is_empty());
    }

    #[test]
    fn test_backwards_compat_tool_calls_for_chunk() {
        let mut additional_kwargs = HashMap::new();
        additional_kwargs.insert(
            "tool_calls".to_string(),
            json!([
                {
                    "id": "call_123",
                    "index": 0,
                    "function": {
                        "name": "get_weather",
                        "arguments": "{\"city\":"
                    }
                }
            ]),
        );

        let (tool_calls, invalid_tool_calls, tool_call_chunks) =
            backwards_compat_tool_calls(&additional_kwargs, true);

        assert!(tool_calls.is_empty());
        assert!(invalid_tool_calls.is_empty());
        assert_eq!(tool_call_chunks.len(), 1);
        assert_eq!(tool_call_chunks[0].name, Some("get_weather".to_string()));
        assert_eq!(tool_call_chunks[0].index, Some(0));
    }

    #[test]
    fn test_backwards_compat_tool_calls_empty() {
        let additional_kwargs = HashMap::new();

        let (tool_calls, invalid_tool_calls, tool_call_chunks) =
            backwards_compat_tool_calls(&additional_kwargs, false);

        assert!(tool_calls.is_empty());
        assert!(invalid_tool_calls.is_empty());
        assert!(tool_call_chunks.is_empty());
    }

    #[test]
    fn test_backwards_compat_tool_calls_invalid_json() {
        let mut additional_kwargs = HashMap::new();
        additional_kwargs.insert(
            "tool_calls".to_string(),
            json!([
                {
                    "id": "call_123",
                    "function": {
                        "name": "get_weather",
                        "arguments": "invalid json {"
                    }
                }
            ]),
        );

        let (tool_calls, invalid_tool_calls, _tool_call_chunks) =
            backwards_compat_tool_calls(&additional_kwargs, false);

        // Should be invalid because the JSON is malformed
        assert!(tool_calls.is_empty());
        assert_eq!(invalid_tool_calls.len(), 1);
        assert_eq!(invalid_tool_calls[0].name, Some("get_weather".to_string()));
    }

    #[test]
    fn test_ai_message_chunk_add() {
        let chunk1 = AIMessageChunk::new("Hello ");
        let chunk2 = AIMessageChunk::new("world!");

        let result = chunk1 + chunk2;

        assert_eq!(result.content(), "Hello world!");
    }

    #[test]
    fn test_ai_message_chunk_sum() {
        let chunks = vec![
            AIMessageChunk::new("Hello "),
            AIMessageChunk::new("beautiful "),
            AIMessageChunk::new("world!"),
        ];

        let result: AIMessageChunk = chunks.into_iter().sum();

        assert_eq!(result.content(), "Hello beautiful world!");
    }

    #[test]
    fn test_add_ai_message_chunks_with_usage() {
        let mut chunk1 = AIMessageChunk::new("Hello ");
        chunk1.usage_metadata = Some(UsageMetadata::new(5, 0));

        let mut chunk2 = AIMessageChunk::new("world!");
        chunk2.usage_metadata = Some(UsageMetadata::new(0, 10));

        let result = add_ai_message_chunks(chunk1, vec![chunk2]);

        assert_eq!(result.content(), "Hello world!");
        assert!(result.usage_metadata.is_some());
        let usage = result.usage_metadata.as_ref().unwrap();
        assert_eq!(usage.input_tokens, 5);
        assert_eq!(usage.output_tokens, 10);
        assert_eq!(usage.total_tokens, 15);
    }

    #[test]
    fn test_add_ai_message_chunks_id_priority() {
        // Provider-assigned ID should take priority
        let chunk1 = AIMessageChunk::with_id("lc_auto123", "");
        let chunk2 = AIMessageChunk::with_id("provider_id_456", "");
        let chunk3 = AIMessageChunk::with_id("lc_run-789", "");

        let result = add_ai_message_chunks(chunk1, vec![chunk2, chunk3]);

        // Provider ID should be selected (not lc_* or lc_run-*)
        assert_eq!(result.id(), Some("provider_id_456"));
    }

    #[test]
    fn test_add_ai_message_chunks_lc_run_priority() {
        // lc_run-* should take priority over lc_*
        let chunk1 = AIMessageChunk::with_id("lc_auto123", "");
        let chunk2 = AIMessageChunk::with_id("lc_run-789", "");

        let result = add_ai_message_chunks(chunk1, vec![chunk2]);

        assert_eq!(result.id(), Some("lc_run-789"));
    }
}
