//! AI message type.
//!
//! This module contains the `AIMessage` and `AIMessageChunk` types which represent
//! messages from an AI model. Mirrors `langchain_core.messages.ai`.

use bon::bon;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;

use super::base::get_msg_title_repr;
use super::content::ContentBlock;
use super::tool::{
    InvalidToolCall, ToolCall, ToolCallChunk, default_tool_chunk_parser, default_tool_parser,
    invalid_tool_call, tool_call,
};
use crate::utils::base::{LC_AUTO_PREFIX, LC_ID_PREFIX};
use crate::utils::json::parse_partial_json;
use crate::utils::merge::{merge_dicts, merge_lists};
use crate::utils::usage::{dict_int_add_json, dict_int_sub_floor_json};

/// Breakdown of input token counts.
///
/// Does *not* need to sum to full input token count. Does *not* need to have all keys.

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

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AIMessage {
    /// The message content
    pub content: String,
    /// Optional unique identifier
    pub id: Option<String>,
    /// Optional name for the message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Tool calls made by the AI
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    /// Tool calls with parsing errors associated with the message
    #[serde(default)]
    pub invalid_tool_calls: Vec<InvalidToolCall>,
    /// If present, usage metadata for a message, such as token counts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<UsageMetadata>,
    /// Additional metadata
    #[serde(default)]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
    /// Response metadata (e.g., response headers, logprobs, token counts, model name)
    #[serde(default)]
    pub response_metadata: HashMap<String, serde_json::Value>,
}

impl Serialize for AIMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut field_count = 6;
        if self.name.is_some() {
            field_count += 1;
        }
        if self.usage_metadata.is_some() {
            field_count += 1;
        }
        // Add 1 for additional type field
        field_count += 1;

        let mut map = serializer.serialize_map(Some(field_count))?;

        map.serialize_entry("type", "ai")?;
        map.serialize_entry("content", &self.content)?;
        map.serialize_entry("id", &self.id)?;
        if self.name.is_some() {
            map.serialize_entry("name", &self.name)?;
        }
        map.serialize_entry("tool_calls", &self.tool_calls)?;
        map.serialize_entry("invalid_tool_calls", &self.invalid_tool_calls)?;
        if self.usage_metadata.is_some() {
            map.serialize_entry("usage_metadata", &self.usage_metadata)?;
        }
        map.serialize_entry("additional_kwargs", &self.additional_kwargs)?;
        map.serialize_entry("response_metadata", &self.response_metadata)?;

        map.end()
    }
}

#[bon]
impl AIMessage {
    /// Create a new AI message with named parameters using the builder pattern.
    ///
    /// # Example
    ///
    /// ```
    /// use agent_chain_core::messages::AIMessage;
    ///
    /// // Simple message with just content
    /// let msg = AIMessage::builder()
    ///     .content("Hello!")
    ///     .build();
    ///
    /// // Message with ID and tool calls
    /// let msg = AIMessage::builder()
    ///     .content("Calling tool...")
    ///     .maybe_id(Some("msg-123".to_string()))
    ///     .tool_calls(vec![])
    ///     .build();
    /// ```
    #[builder]
    pub fn new(
        content: impl Into<String>,
        id: Option<String>,
        name: Option<String>,
        #[builder(default)] tool_calls: Vec<ToolCall>,
        #[builder(default)] invalid_tool_calls: Vec<InvalidToolCall>,
        usage_metadata: Option<UsageMetadata>,
        #[builder(default)] additional_kwargs: HashMap<String, serde_json::Value>,
        #[builder(default)] response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            content: content.into(),
            id,
            name,
            tool_calls,
            invalid_tool_calls,
            usage_metadata,
            additional_kwargs,
            response_metadata,
        }
    }

    /// Create a new AIMessage with a list of content blocks.
    ///
    /// This is a convenience method for creating messages with structured content.
    pub fn with_content_list(content_list: Vec<serde_json::Value>) -> Self {
        let content = serde_json::to_string(&content_list).unwrap_or_default();
        Self::builder().content(content).build()
    }

    /// Set the message ID.
    pub fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }

    /// Get the message content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the raw content as a list of JSON values.
    ///
    /// If the content is a JSON array, it returns the parsed array.
    /// If the content is a string, it returns a single text block.
    pub fn content_list(&self) -> Vec<serde_json::Value> {
        if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&self.content) {
            arr
        } else {
            vec![serde_json::json!({"type": "text", "text": self.content})]
        }
    }

    /// Get the content blocks translated to the standard format.
    ///
    /// This method translates provider-specific content blocks to the
    /// standardized LangChain content block format. The translation is
    /// based on the `model_provider` field in `response_metadata`.
    ///
    /// This corresponds to `content_blocks` property in LangChain Python.
    pub fn content_blocks(&self) -> Vec<ContentBlock> {
        use crate::messages::block_translators::anthropic::convert_to_standard_blocks as anthropic_convert;
        use crate::messages::block_translators::openai::{
            OpenAiContext, convert_to_standard_blocks_with_context as openai_convert,
        };

        let provider = self
            .response_metadata
            .get("model_provider")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let raw_content = self.content_list();

        let blocks_json = match provider {
            "anthropic" => anthropic_convert(&raw_content, false),
            "openai" => {
                // Create context with tool_calls and other message data for OpenAI translation
                let context = OpenAiContext {
                    tool_calls: self
                        .tool_calls
                        .iter()
                        .filter_map(|tc| serde_json::to_value(tc).ok())
                        .collect(),
                    tool_call_chunks: Vec::new(),
                    invalid_tool_calls: self
                        .invalid_tool_calls
                        .iter()
                        .filter_map(|tc| serde_json::to_value(tc).ok())
                        .collect(),
                    additional_kwargs: serde_json::to_value(&self.additional_kwargs)
                        .unwrap_or_default(),
                    response_metadata: serde_json::to_value(&self.response_metadata)
                        .unwrap_or_default(),
                    message_id: self.id.clone(),
                    chunk_position: None,
                };
                openai_convert(&raw_content, false, Some(&context))
            }
            _ => {
                // Default: return content as-is or wrap string in text block
                raw_content
            }
        };

        // Deserialize JSON blocks into ContentBlock structs
        // We can't use direct serde deserialization because the enum has #[serde(tag = "type")]
        // which expects externally tagged format, but our JSON has type as a field inside.
        // So we need to manually deserialize based on the type field.
        use super::content::{
            AudioContentBlock, FileContentBlock, ImageContentBlock, InvalidToolCallBlock,
            NonStandardContentBlock, PlainTextContentBlock, ReasoningContentBlock, ServerToolCall,
            ServerToolCallChunk, ServerToolResult, TextContentBlock, ToolCallBlock,
            ToolCallChunkBlock, VideoContentBlock,
        };

        blocks_json
            .into_iter()
            .map(|v| {
                let block_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
                let result = match block_type {
                    "text" => serde_json::from_value::<TextContentBlock>(v.clone())
                        .map(ContentBlock::Text),
                    "reasoning" => serde_json::from_value::<ReasoningContentBlock>(v.clone())
                        .map(ContentBlock::Reasoning),
                    "tool_call" => serde_json::from_value::<ToolCallBlock>(v.clone())
                        .map(ContentBlock::ToolCall),
                    "invalid_tool_call" => {
                        serde_json::from_value::<InvalidToolCallBlock>(v.clone())
                            .map(ContentBlock::InvalidToolCall)
                    }
                    "tool_call_chunk" => serde_json::from_value::<ToolCallChunkBlock>(v.clone())
                        .map(ContentBlock::ToolCallChunk),
                    "image" => serde_json::from_value::<ImageContentBlock>(v.clone())
                        .map(ContentBlock::Image),
                    "audio" => serde_json::from_value::<AudioContentBlock>(v.clone())
                        .map(ContentBlock::Audio),
                    "video" => serde_json::from_value::<VideoContentBlock>(v.clone())
                        .map(ContentBlock::Video),
                    "file" => serde_json::from_value::<FileContentBlock>(v.clone())
                        .map(ContentBlock::File),
                    "text-plain" => serde_json::from_value::<PlainTextContentBlock>(v.clone())
                        .map(ContentBlock::PlainText),
                    "server_tool_call" => serde_json::from_value::<ServerToolCall>(v.clone())
                        .map(ContentBlock::ServerToolCall),
                    "server_tool_call_chunk" => {
                        serde_json::from_value::<ServerToolCallChunk>(v.clone())
                            .map(ContentBlock::ServerToolCallChunk)
                    }
                    "server_tool_result" => serde_json::from_value::<ServerToolResult>(v.clone())
                        .map(ContentBlock::ServerToolResult),
                    "non_standard" => serde_json::from_value::<NonStandardContentBlock>(v.clone())
                        .map(ContentBlock::NonStandard),
                    _ => {
                        // Unknown type, wrap as non_standard
                        tracing::warn!(
                            block_type = %block_type,
                            json = %v,
                            "Unknown block type in AIMessage::content_blocks, treating as non_standard"
                        );
                        serde_json::from_value::<NonStandardContentBlock>(v.clone())
                            .map(ContentBlock::NonStandard)
                    }
                };

                result.unwrap_or_else(|e| {
                    tracing::warn!(
                        block_type = %block_type,
                        error = %e,
                        json = %v,
                        "Failed to deserialize ContentBlock in AIMessage::content_blocks, wrapping as non_standard"
                    );
                    // Wrap the malformed block as NonStandardContentBlock with error info
                    let mut error_value = std::collections::HashMap::new();
                    error_value.insert(
                        "original_json".to_string(),
                        v.clone(),
                    );
                    error_value.insert(
                        "deserialization_error".to_string(),
                        serde_json::Value::String(e.to_string()),
                    );
                    error_value.insert(
                        "original_type".to_string(),
                        serde_json::Value::String(block_type.to_string()),
                    );
                    ContentBlock::NonStandard(NonStandardContentBlock {
                        block_type: "non_standard".to_string(),
                        id: None,
                        value: error_value,
                        index: v.get("index").and_then(|i| {
                            serde_json::from_value(i.clone()).ok()
                        }),
                    })
                })
            })
            .collect()
    }

    /// Get a pretty representation of the message.
    ///
    /// This corresponds to `pretty_repr` in LangChain Python.
    /// Calls the base message `pretty_repr` logic, then appends tool call info.
    pub fn pretty_repr(&self, html: bool) -> String {
        // Build the base representation (matches BaseMessage.pretty_repr in Python)
        let title = get_msg_title_repr("Ai Message", html);
        let name_line = if let Some(name) = &self.name {
            format!("\nName: {}", name)
        } else {
            String::new()
        };
        let base = format!("{}{}\n\n{}", title, name_line, self.content);

        // Append tool call formatting
        let mut lines = Vec::new();
        format_tool_calls_repr(&self.tool_calls, &self.invalid_tool_calls, &mut lines);

        if lines.is_empty() {
            base.trim().to_string()
        } else {
            format!("{}\n{}", base.trim(), lines.join("\n"))
                .trim()
                .to_string()
        }
    }

    /// Get the message type as a string.
    pub fn message_type(&self) -> &'static str {
        "ai"
    }
}

/// Helper function to format tool calls for pretty_repr.
///
/// This matches Python's `_format_tool_args` function which is shared
/// between `tool_calls` and `invalid_tool_calls` formatting.
fn format_tool_args(
    name: &str,
    id: Option<&str>,
    error: Option<&str>,
    args: &str,
    args_is_dict: bool,
    args_dict: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Vec<String> {
    let id_str = id.unwrap_or("None");
    let mut lines = vec![
        format!("  {} ({})", name, id_str),
        format!(" Call ID: {}", id_str),
    ];
    if let Some(err) = error {
        lines.push(format!("  Error: {}", err));
    }
    lines.push("  Args:".to_string());
    if args_is_dict {
        if let Some(dict) = args_dict {
            for (arg, value) in dict {
                lines.push(format!("    {}: {}", arg, value));
            }
        }
    } else {
        lines.push(format!("    {}", args));
    }
    lines
}

fn format_tool_calls_repr(
    tool_calls: &[ToolCall],
    invalid_tool_calls: &[InvalidToolCall],
    lines: &mut Vec<String>,
) {
    if !tool_calls.is_empty() {
        lines.push("Tool Calls:".to_string());
        for tc in tool_calls {
            let (args_is_dict, args_dict, args_str) =
                if let serde_json::Value::Object(ref map) = tc.args {
                    (true, Some(map), String::new())
                } else {
                    (false, None, tc.args.to_string())
                };
            lines.extend(format_tool_args(
                &tc.name,
                tc.id.as_deref(),
                None,
                &args_str,
                args_is_dict,
                args_dict,
            ));
        }
    }
    if !invalid_tool_calls.is_empty() {
        lines.push("Invalid Tool Calls:".to_string());
        for itc in invalid_tool_calls {
            let name = itc.name.as_deref().unwrap_or("Tool");
            let id = itc.id.as_deref();
            let args_str = itc.args.as_deref().unwrap_or("");
            lines.extend(format_tool_args(
                name,
                id,
                itc.error.as_deref(),
                args_str,
                false,
                None,
            ));
        }
    }
}

/// Position indicator for an aggregated AIMessageChunk.
///
/// If a chunk with `chunk_position="last"` is aggregated into a stream,
/// `tool_call_chunks` in message content will be parsed into `tool_calls`.

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

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AIMessageChunk {
    /// The message content (may be partial during streaming)
    pub content: String,
    /// Optional unique identifier
    pub id: Option<String>,
    /// Optional name for the message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Tool calls made by the AI
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    /// Tool calls with parsing errors
    #[serde(default)]
    pub invalid_tool_calls: Vec<InvalidToolCall>,
    /// Tool call chunks (for streaming tool calls)
    #[serde(default)]
    pub tool_call_chunks: Vec<ToolCallChunk>,
    /// If present, usage metadata for a message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<UsageMetadata>,
    /// Additional metadata
    #[serde(default)]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
    /// Response metadata
    #[serde(default)]
    pub response_metadata: HashMap<String, serde_json::Value>,
    /// Optional span represented by an aggregated AIMessageChunk.
    ///
    /// If a chunk with `chunk_position=Some(ChunkPosition::Last)` is aggregated into a stream,
    /// `tool_call_chunks` in message content will be parsed into `tool_calls`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_position: Option<ChunkPosition>,
}

impl Serialize for AIMessageChunk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut field_count = 7;
        if self.name.is_some() {
            field_count += 1;
        }
        if self.usage_metadata.is_some() {
            field_count += 1;
        }
        if self.chunk_position.is_some() {
            field_count += 1;
        }
        // Add 1 for additional type field
        field_count += 1;

        let mut map = serializer.serialize_map(Some(field_count))?;

        map.serialize_entry("type", "AIMessageChunk")?;
        map.serialize_entry("content", &self.content)?;
        map.serialize_entry("id", &self.id)?;
        if self.name.is_some() {
            map.serialize_entry("name", &self.name)?;
        }
        map.serialize_entry("tool_calls", &self.tool_calls)?;
        map.serialize_entry("invalid_tool_calls", &self.invalid_tool_calls)?;
        map.serialize_entry("tool_call_chunks", &self.tool_call_chunks)?;
        if self.usage_metadata.is_some() {
            map.serialize_entry("usage_metadata", &self.usage_metadata)?;
        }
        map.serialize_entry("additional_kwargs", &self.additional_kwargs)?;
        map.serialize_entry("response_metadata", &self.response_metadata)?;
        if self.chunk_position.is_some() {
            map.serialize_entry("chunk_position", &self.chunk_position)?;
        }

        map.end()
    }
}

#[bon]
impl AIMessageChunk {
    /// Create a new AI message chunk with named parameters.
    ///
    /// # Example
    ///
    /// ```
    /// use agent_chain_core::messages::AIMessageChunk;
    ///
    /// // Simple chunk with just content
    /// let chunk = AIMessageChunk::builder()
    ///     .content("Hello")
    ///     .build();
    ///
    /// // Chunk with ID and tool call chunks
    /// let chunk = AIMessageChunk::builder()
    ///     .content("")
    ///     .maybe_id(Some("chunk-123".to_string()))
    ///     .tool_call_chunks(vec![])
    ///     .build();
    /// ```
    #[builder]
    pub fn new(
        content: impl Into<String>,
        id: Option<String>,
        name: Option<String>,
        #[builder(default)] tool_calls: Vec<ToolCall>,
        #[builder(default)] invalid_tool_calls: Vec<InvalidToolCall>,
        #[builder(default)] tool_call_chunks: Vec<ToolCallChunk>,
        usage_metadata: Option<UsageMetadata>,
        #[builder(default)] additional_kwargs: HashMap<String, serde_json::Value>,
        #[builder(default)] response_metadata: HashMap<String, serde_json::Value>,
        chunk_position: Option<ChunkPosition>,
    ) -> Self {
        Self {
            content: content.into(),
            id,
            name,
            tool_calls,
            invalid_tool_calls,
            tool_call_chunks,
            usage_metadata,
            additional_kwargs,
            response_metadata,
            chunk_position,
        }
    }

    /// Create a new AIMessageChunk with a list of content blocks.
    ///
    /// This is a convenience method for creating chunks with structured content.
    pub fn with_content_list(content_list: Vec<serde_json::Value>) -> Self {
        let content = serde_json::to_string(&content_list).unwrap_or_default();
        Self::builder().content(content).build()
    }

    /// Get the raw content as a list of JSON values.
    ///
    /// If the content is a JSON array, it returns the parsed array.
    /// If the content is a string, it returns a single text block.
    pub fn content_list(&self) -> Vec<serde_json::Value> {
        if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&self.content) {
            arr
        } else {
            vec![serde_json::json!({"type": "text", "text": self.content})]
        }
    }

    /// Get the content blocks translated to the standard format.
    ///
    /// This method translates provider-specific content blocks to the
    /// standardized LangChain content block format. The translation is
    /// based on the `model_provider` field in `response_metadata`.
    ///
    /// For chunks, this uses the chunk-specific translation which handles
    /// streaming content like `tool_call_chunk` and `input_json_delta`.
    ///
    /// This corresponds to `content_blocks` property in LangChain Python.
    pub fn content_blocks(&self) -> Vec<ContentBlock> {
        use crate::messages::block_translators::anthropic::{
            ChunkContext as AnthropicChunkContext,
            convert_to_standard_blocks_with_context as anthropic_convert,
        };
        use crate::messages::block_translators::openai::{
            OpenAiContext, convert_to_standard_blocks_with_context as openai_convert,
        };

        let provider = self
            .response_metadata
            .get("model_provider")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let raw_content = self.content_list();
        let is_last = self.chunk_position == Some(ChunkPosition::Last);

        let blocks_json = match provider {
            "anthropic" => {
                // Create context with tool_call_chunks for proper translation
                let context = AnthropicChunkContext {
                    tool_call_chunks: self
                        .tool_call_chunks
                        .iter()
                        .filter_map(|tc| serde_json::to_value(tc).ok())
                        .collect(),
                };
                anthropic_convert(&raw_content, !is_last, Some(&context))
            }
            "openai" => {
                // Create context with tool_call_chunks and other message data for OpenAI translation
                let chunk_position = if is_last {
                    Some("last".to_string())
                } else {
                    None
                };
                let context = OpenAiContext {
                    tool_calls: self
                        .tool_calls
                        .iter()
                        .filter_map(|tc| serde_json::to_value(tc).ok())
                        .collect(),
                    tool_call_chunks: self
                        .tool_call_chunks
                        .iter()
                        .filter_map(|tc| serde_json::to_value(tc).ok())
                        .collect(),
                    invalid_tool_calls: self
                        .invalid_tool_calls
                        .iter()
                        .filter_map(|tc| serde_json::to_value(tc).ok())
                        .collect(),
                    additional_kwargs: serde_json::to_value(&self.additional_kwargs)
                        .unwrap_or_default(),
                    response_metadata: serde_json::to_value(&self.response_metadata)
                        .unwrap_or_default(),
                    message_id: self.id.clone(),
                    chunk_position,
                };
                openai_convert(&raw_content, !is_last, Some(&context))
            }
            _ => {
                // Default: return content as-is or wrap string in text block
                // Also include tool_call_chunks if present
                let mut blocks = raw_content;

                // Add tool_call_chunks to content_blocks for default case
                for tc in &self.tool_call_chunks {
                    if let Ok(mut chunk_value) = serde_json::to_value(tc) {
                        chunk_value["type"] =
                            serde_json::Value::String("tool_call_chunk".to_string());
                        blocks.push(chunk_value);
                    }
                }

                blocks
            }
        };

        // Deserialize JSON blocks into ContentBlock structs
        // We can't use direct serde deserialization because the enum has #[serde(tag = "type")]
        // which expects externally tagged format, but our JSON has type as a field inside.
        // So we need to manually deserialize based on the type field.
        use super::content::{
            AudioContentBlock, FileContentBlock, ImageContentBlock, InvalidToolCallBlock,
            NonStandardContentBlock, PlainTextContentBlock, ReasoningContentBlock, ServerToolCall,
            ServerToolCallChunk, ServerToolResult, TextContentBlock, ToolCallBlock,
            ToolCallChunkBlock, VideoContentBlock,
        };

        blocks_json
            .into_iter()
            .map(|v| {
                let block_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
                let result = match block_type {
                    "text" => serde_json::from_value::<TextContentBlock>(v.clone())
                        .map(ContentBlock::Text),
                    "reasoning" => serde_json::from_value::<ReasoningContentBlock>(v.clone())
                        .map(ContentBlock::Reasoning),
                    "tool_call" => serde_json::from_value::<ToolCallBlock>(v.clone())
                        .map(ContentBlock::ToolCall),
                    "invalid_tool_call" => {
                        serde_json::from_value::<InvalidToolCallBlock>(v.clone())
                            .map(ContentBlock::InvalidToolCall)
                    }
                    "tool_call_chunk" => serde_json::from_value::<ToolCallChunkBlock>(v.clone())
                        .map(ContentBlock::ToolCallChunk),
                    "image" => serde_json::from_value::<ImageContentBlock>(v.clone())
                        .map(ContentBlock::Image),
                    "audio" => serde_json::from_value::<AudioContentBlock>(v.clone())
                        .map(ContentBlock::Audio),
                    "video" => serde_json::from_value::<VideoContentBlock>(v.clone())
                        .map(ContentBlock::Video),
                    "file" => serde_json::from_value::<FileContentBlock>(v.clone())
                        .map(ContentBlock::File),
                    "text-plain" => serde_json::from_value::<PlainTextContentBlock>(v.clone())
                        .map(ContentBlock::PlainText),
                    "server_tool_call" => serde_json::from_value::<ServerToolCall>(v.clone())
                        .map(ContentBlock::ServerToolCall),
                    "server_tool_call_chunk" => {
                        serde_json::from_value::<ServerToolCallChunk>(v.clone())
                            .map(ContentBlock::ServerToolCallChunk)
                    }
                    "server_tool_result" => serde_json::from_value::<ServerToolResult>(v.clone())
                        .map(ContentBlock::ServerToolResult),
                    "non_standard" => serde_json::from_value::<NonStandardContentBlock>(v.clone())
                        .map(ContentBlock::NonStandard),
                    _ => {
                        // Unknown type, wrap as non_standard
                        tracing::warn!(
                            block_type = %block_type,
                            json = %v,
                            "Unknown block type in AIMessageChunk::content_blocks, treating as non_standard"
                        );
                        serde_json::from_value::<NonStandardContentBlock>(v.clone())
                            .map(ContentBlock::NonStandard)
                    }
                };

                result.unwrap_or_else(|e| {
                    tracing::warn!(
                        block_type = %block_type,
                        error = %e,
                        json = %v,
                        "Failed to deserialize ContentBlock in AIMessageChunk::content_blocks, wrapping as non_standard"
                    );
                    // Wrap the malformed block as NonStandardContentBlock with error info
                    let mut error_value = std::collections::HashMap::new();
                    error_value.insert(
                        "original_json".to_string(),
                        v.clone(),
                    );
                    error_value.insert(
                        "deserialization_error".to_string(),
                        serde_json::Value::String(e.to_string()),
                    );
                    error_value.insert(
                        "original_type".to_string(),
                        serde_json::Value::String(block_type.to_string()),
                    );
                    ContentBlock::NonStandard(NonStandardContentBlock {
                        block_type: "non_standard".to_string(),
                        id: None,
                        value: error_value,
                        index: v.get("index").and_then(|i| {
                            serde_json::from_value(i.clone()).ok()
                        }),
                    })
                })
            })
            .collect()
    }

    /// Get chunk position.
    pub fn chunk_position(&self) -> Option<&ChunkPosition> {
        self.chunk_position.as_ref()
    }

    /// Set chunk position.
    pub fn set_chunk_position(&mut self, position: Option<ChunkPosition>) {
        self.chunk_position = position;
    }

    /// Set usage metadata.
    pub fn set_usage_metadata(&mut self, usage_metadata: Option<UsageMetadata>) {
        self.usage_metadata = usage_metadata;
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
                        name: Some(tc.name.clone()),
                        args: Some(tc.args.to_string()),
                        id: tc.id.clone(),
                        index: None,
                        chunk_type: Some("tool_call_chunk".to_string()),
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
                        chunk_type: Some("tool_call_chunk".to_string()),
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

        // When chunk_position is "last" and output_version is "v1" and content is a list,
        // replace tool_call_chunk blocks in content with full tool_call blocks.
        // This corresponds to the end of Python's init_tool_calls validator.
        if self.chunk_position == Some(ChunkPosition::Last)
            && !self.tool_call_chunks.is_empty()
            && self
                .response_metadata
                .get("output_version")
                .and_then(|v| v.as_str())
                == Some("v1")
            && let Ok(mut content_list) =
                serde_json::from_str::<Vec<serde_json::Value>>(&self.content)
        {
            // Build a map of id -> tool_call for replacement
            let id_to_tc: HashMap<String, serde_json::Value> = self
                .tool_calls
                .iter()
                .filter_map(|tc| {
                    tc.id.as_ref().map(|id| {
                        let mut tc_val = serde_json::json!({
                            "type": "tool_call",
                            "name": tc.name,
                            "args": tc.args,
                            "id": id,
                        });
                        tc_val
                            .as_object_mut()
                            .map(|m| (id.clone(), serde_json::Value::Object(m.clone())))
                    })
                })
                .flatten()
                .collect();

            let mut changed = false;
            for block in &mut content_list {
                if let Some(block_type) = block.get("type").and_then(|t| t.as_str())
                    && block_type == "tool_call_chunk"
                    && let Some(call_id) = block.get("id").and_then(|i| i.as_str())
                    && let Some(tc) = id_to_tc.get(call_id)
                {
                    let mut replacement = tc.clone();
                    // Preserve "extras" from the original block
                    if let Some(extras) = block.get("extras") {
                        replacement["extras"] = extras.clone();
                    }
                    *block = replacement;
                    changed = true;
                }
            }

            if changed {
                self.content = serde_json::to_string(&content_list).unwrap_or(self.content.clone());
            }
        }
    }

    /// Parse server tool call chunks when aggregation is complete.
    ///
    /// When `chunk_position` is "last" and `output_version` is "v1",
    /// this parses `server_tool_call_chunk` blocks that have string args
    /// into proper `server_tool_call` blocks with parsed JSON args.
    ///
    /// This corresponds to `init_server_tool_calls` model validator in Python.
    pub fn init_server_tool_calls(&mut self) {
        if self.chunk_position != Some(ChunkPosition::Last) {
            return;
        }

        if self
            .response_metadata
            .get("output_version")
            .and_then(|v| v.as_str())
            != Some("v1")
        {
            return;
        }

        if let Ok(mut content_list) = serde_json::from_str::<Vec<serde_json::Value>>(&self.content)
        {
            let mut changed = false;
            for block in &mut content_list {
                if let Some(block_type) = block.get("type").and_then(|t| t.as_str())
                    && (block_type == "server_tool_call" || block_type == "server_tool_call_chunk")
                    && let Some(args_str) = block.get("args").and_then(|a| a.as_str())
                    && let Ok(args) = serde_json::from_str::<serde_json::Value>(args_str)
                    && args.is_object()
                {
                    block["type"] = serde_json::Value::String("server_tool_call".to_string());
                    block["args"] = args;
                    changed = true;
                }
            }

            if changed {
                self.content = serde_json::to_string(&content_list).unwrap_or(self.content.clone());
            }
        }
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
    /// Calls the base message `pretty_repr` logic, then appends tool call info.
    pub fn pretty_repr(&self, html: bool) -> String {
        // Build the base representation (matches BaseMessage.pretty_repr in Python)
        let title = get_msg_title_repr("Aimessagechunk Message", html);
        let name_line = if let Some(name) = &self.name {
            format!("\nName: {}", name)
        } else {
            String::new()
        };
        let base = format!("{}{}\n\n{}", title, name_line, self.content);

        // Append tool call formatting
        let mut lines = Vec::new();
        format_tool_calls_repr(&self.tool_calls, &self.invalid_tool_calls, &mut lines);

        if lines.is_empty() {
            base.trim().to_string()
        } else {
            format!("{}\n{}", base.trim(), lines.join("\n"))
                .trim()
                .to_string()
        }
    }
}

/// Merge message content from multiple chunks.
///
/// This corresponds to `merge_content` in LangChain Python.
/// Content can be either a plain string or a JSON array of content blocks.
///
/// The merge logic is:
/// - String + String → String concatenation
/// - String + List → Prepend string to list
/// - List + List → Use `merge_lists` (which merges by index)
/// - List + String → Append string to last element if it's a string, otherwise add as new element
///
/// # Arguments
///
/// * `first` - The first content string (may be plain text or JSON array)
/// * `others` - Other content strings to merge
///
/// # Returns
///
/// The merged content as a string (either plain text or JSON array)
fn merge_message_content(first: &str, others: &[&str]) -> String {
    // Try to parse first content as JSON array
    let mut merged: serde_json::Value = match serde_json::from_str::<Vec<serde_json::Value>>(first)
    {
        Ok(arr) => serde_json::Value::Array(arr),
        Err(_) => {
            // Not a JSON array, treat as plain string
            serde_json::Value::String(first.to_string())
        }
    };

    for content in others {
        // Try to parse other content as JSON array
        let other_val: serde_json::Value =
            match serde_json::from_str::<Vec<serde_json::Value>>(content) {
                Ok(arr) => serde_json::Value::Array(arr),
                Err(_) => {
                    // Not a JSON array, treat as plain string
                    serde_json::Value::String(content.to_string())
                }
            };

        merged = match (merged, other_val) {
            // String + String → String concatenation
            (serde_json::Value::String(s1), serde_json::Value::String(s2)) => {
                serde_json::Value::String(format!("{}{}", s1, s2))
            }
            // String + List → Prepend string to list
            (serde_json::Value::String(s), serde_json::Value::Array(mut arr)) => {
                if !s.is_empty() {
                    arr.insert(0, serde_json::json!({"type": "text", "text": s}));
                }
                serde_json::Value::Array(arr)
            }
            // List + List → Use merge_lists
            (serde_json::Value::Array(arr1), serde_json::Value::Array(arr2)) => {
                let other_chunks: Vec<Option<Vec<serde_json::Value>>> = vec![Some(arr2.clone())];
                match merge_lists(Some(arr1.clone()), other_chunks) {
                    Ok(Some(merged_arr)) => serde_json::Value::Array(merged_arr),
                    _ => {
                        // Fallback: just concatenate arrays
                        let mut result = arr1;
                        result.extend(arr2);
                        serde_json::Value::Array(result)
                    }
                }
            }
            // List + String → Append string to last element if it's a string, otherwise add as new element
            (serde_json::Value::Array(mut arr), serde_json::Value::String(s)) => {
                if s.is_empty() {
                    // Empty string is a no-op
                    serde_json::Value::Array(arr)
                } else if let Some(last) = arr.last_mut() {
                    // Check if last element is a text block or plain string
                    if let Some(text) = last.get_mut("text") {
                        if let Some(text_str) = text.as_str() {
                            *text = serde_json::Value::String(format!("{}{}", text_str, s));
                        }
                    } else if last.is_string() {
                        if let Some(last_str) = last.as_str() {
                            *last = serde_json::Value::String(format!("{}{}", last_str, s));
                        }
                    } else {
                        // Last element is not a string, add as new element
                        arr.push(serde_json::json!({"type": "text", "text": s}));
                    }
                    serde_json::Value::Array(arr)
                } else {
                    // Empty array, add as new element
                    arr.push(serde_json::json!({"type": "text", "text": s}));
                    serde_json::Value::Array(arr)
                }
            }
            // Fallback for other cases
            (m, _) => m,
        };
    }

    // Serialize back to string
    match merged {
        serde_json::Value::String(s) => s,
        serde_json::Value::Array(arr) => serde_json::to_string(&arr).unwrap_or_default(),
        _ => String::new(),
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
    // Merge content using merge_content logic from Python
    let content = merge_message_content(
        &left.content,
        &others
            .iter()
            .map(|o| o.content.as_str())
            .collect::<Vec<_>>(),
    );

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
                        chunk_type: Some("tool_call_chunk".to_string()),
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
        result.init_server_tool_calls();
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
            AIMessageChunk::builder().content("").build()
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
        (None, Some(r)) => r.clone(),
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

        // Left None, Right Some - should return right (matches Python: `left or right`)
        let result = subtract_usage(None, Some(&usage));
        assert_eq!(result.input_tokens, 10);
        assert_eq!(result.output_tokens, 20);
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
        assert_eq!(tool_calls[0].name, "get_weather");
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
        let chunk1 = AIMessageChunk::builder().content("Hello ").build();
        let chunk2 = AIMessageChunk::builder().content("world!").build();

        let result = chunk1 + chunk2;

        assert_eq!(result.content, "Hello world!");
    }

    #[test]
    fn test_ai_message_chunk_sum() {
        let chunks = vec![
            AIMessageChunk::builder().content("Hello ").build(),
            AIMessageChunk::builder().content("beautiful ").build(),
            AIMessageChunk::builder().content("world!").build(),
        ];

        let result: AIMessageChunk = chunks.into_iter().sum();

        assert_eq!(result.content, "Hello beautiful world!");
    }

    #[test]
    fn test_add_ai_message_chunks_with_usage() {
        let mut chunk1 = AIMessageChunk::builder().content("Hello ").build();
        chunk1.usage_metadata = Some(UsageMetadata::new(5, 0));

        let mut chunk2 = AIMessageChunk::builder().content("world!").build();
        chunk2.usage_metadata = Some(UsageMetadata::new(0, 10));

        let result = add_ai_message_chunks(chunk1, vec![chunk2]);

        assert_eq!(result.content, "Hello world!");
        assert!(result.usage_metadata.is_some());
        let usage = result.usage_metadata.as_ref().unwrap();
        assert_eq!(usage.input_tokens, 5);
        assert_eq!(usage.output_tokens, 10);
        assert_eq!(usage.total_tokens, 15);
    }

    #[test]
    fn test_add_ai_message_chunks_id_priority() {
        // Provider-assigned ID should take priority
        let chunk1 = AIMessageChunk::builder()
            .id("lc_auto123".to_string())
            .content("")
            .build();
        let chunk2 = AIMessageChunk::builder()
            .id("provider_id_456".to_string())
            .content("")
            .build();
        let chunk3 = AIMessageChunk::builder()
            .id("lc_run".to_string())
            .content("")
            .build();

        let result = add_ai_message_chunks(chunk1, vec![chunk2, chunk3]);

        // Provider ID should be selected (not lc_* or lc_run-*)
        assert_eq!(result.id, Some("provider_id_456".to_string()));
    }

    #[test]
    fn test_add_ai_message_chunks_lc_run_priority() {
        // lc_run-* should take priority over lc_*
        let chunk1 = AIMessageChunk::builder()
            .id("lc_auto123".to_string())
            .content("")
            .build();
        let chunk2 = AIMessageChunk::builder()
            .id("lc_run-789".to_string())
            .content("")
            .build();

        let result = add_ai_message_chunks(chunk1, vec![chunk2]);

        assert_eq!(result.id, Some("lc_run-789".to_string()));
    }
}
