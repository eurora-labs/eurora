//! Message utility types and functions.
//!
//! This module contains utility types like `AnyMessage` and helper functions
//! for working with messages. Mirrors `langchain_core.messages.utils`.

use super::ai::{AIMessage, AIMessageChunk};
use super::base::{BaseMessage, BaseMessageChunk};
use super::chat::{ChatMessage, ChatMessageChunk};
use super::function::{FunctionMessage, FunctionMessageChunk};
use super::human::{HumanMessage, HumanMessageChunk};
use super::modifier::RemoveMessage;
use super::system::{SystemMessage, SystemMessageChunk};
use super::tool::{ToolCall, ToolMessage, ToolMessageChunk};

/// Type alias for any message type, matching LangChain's AnyMessage.
/// This is equivalent to BaseMessage but provides naming consistency with Python.
pub type AnyMessage = BaseMessage;

/// A type representing the various ways a message can be represented.
///
/// This corresponds to `MessageLikeRepresentation` in LangChain Python.
pub type MessageLikeRepresentation = serde_json::Value;

// ============================================================================
// msg_to_chunk / chunk_to_msg
// ============================================================================

/// Convert a `BaseMessage` to the corresponding `BaseMessageChunk`.
///
/// This corresponds to `_msg_to_chunk` in LangChain Python.
pub(crate) fn msg_to_chunk(message: &BaseMessage) -> BaseMessageChunk {
    match message {
        BaseMessage::Human(m) => BaseMessageChunk::Human(
            HumanMessageChunk::builder()
                .content(m.content.clone())
                .maybe_id(m.id.clone())
                .maybe_name(m.name.clone())
                .additional_kwargs(m.additional_kwargs.clone())
                .response_metadata(m.response_metadata.clone())
                .build(),
        ),
        BaseMessage::AI(m) => {
            let mut chunk = AIMessageChunk::builder()
                .content(m.content.clone())
                .maybe_id(m.id.clone())
                .maybe_name(m.name.clone())
                .tool_calls(m.tool_calls.clone())
                .invalid_tool_calls(m.invalid_tool_calls.clone())
                .maybe_usage_metadata(m.usage_metadata.clone())
                .additional_kwargs(m.additional_kwargs.clone())
                .response_metadata(m.response_metadata.clone())
                .build();
            // Populate tool_call_chunks from tool_calls so they merge properly
            chunk.init_tool_calls();
            BaseMessageChunk::AI(chunk)
        }
        BaseMessage::System(m) => BaseMessageChunk::System(
            SystemMessageChunk::builder()
                .content(m.content.clone())
                .maybe_id(m.id.clone())
                .maybe_name(m.name.clone())
                .additional_kwargs(m.additional_kwargs.clone())
                .response_metadata(m.response_metadata.clone())
                .build(),
        ),
        BaseMessage::Tool(m) => BaseMessageChunk::Tool(
            ToolMessageChunk::builder()
                .content(m.content.clone())
                .tool_call_id(m.tool_call_id.clone())
                .maybe_id(m.id.clone())
                .maybe_name(m.name.clone())
                .status(m.status.clone())
                .maybe_artifact(m.artifact.clone())
                .additional_kwargs(m.additional_kwargs.clone())
                .response_metadata(m.response_metadata.clone())
                .build(),
        ),
        BaseMessage::Chat(m) => BaseMessageChunk::Chat(
            ChatMessageChunk::builder()
                .content(m.content.clone())
                .role(m.role.clone())
                .maybe_id(m.id.clone())
                .maybe_name(m.name.clone())
                .additional_kwargs(m.additional_kwargs.clone())
                .response_metadata(m.response_metadata.clone())
                .build(),
        ),
        BaseMessage::Function(m) => BaseMessageChunk::Function(
            FunctionMessageChunk::builder()
                .content(m.content.clone())
                .name(m.name.clone())
                .maybe_id(m.id.clone())
                .additional_kwargs(m.additional_kwargs.clone())
                .response_metadata(m.response_metadata.clone())
                .build(),
        ),
        BaseMessage::Remove(_) => {
            panic!("Cannot convert RemoveMessage to chunk")
        }
    }
}

/// Convert a `BaseMessageChunk` to the corresponding `BaseMessage`.
///
/// This corresponds to `_chunk_to_msg` in LangChain Python.
pub(crate) fn chunk_to_msg(chunk: &BaseMessageChunk) -> BaseMessage {
    chunk.to_message()
}

// ============================================================================
// get_buffer_string
// ============================================================================

/// Convert a sequence of messages to a buffer string.
///
/// This concatenates messages with role prefixes for display.
///
/// # Arguments
///
/// * `messages` - The messages to convert.
/// * `human_prefix` - The prefix to prepend to human messages (default: "Human").
/// * `ai_prefix` - The prefix to prepend to AI messages (default: "AI").
///
/// # Returns
///
/// A single string concatenation of all input messages.
pub fn get_buffer_string(messages: &[BaseMessage], human_prefix: &str, ai_prefix: &str) -> String {
    messages
        .iter()
        .map(|m| {
            let role = match m {
                BaseMessage::Human(_) => human_prefix,
                BaseMessage::System(_) => "System",
                BaseMessage::AI(_) => ai_prefix,
                BaseMessage::Tool(_) => "Tool",
                BaseMessage::Chat(c) => &c.role,
                BaseMessage::Function(_) => "Function",
                BaseMessage::Remove(_) => "Remove",
            };
            let mut message = format!("{}: {}", role, m.text());
            if let BaseMessage::AI(ai_msg) = m
                && let Some(function_call) = ai_msg.additional_kwargs.get("function_call")
            {
                message.push_str(&function_call.to_string());
            }
            message
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Convert a message to a dictionary representation.
///
/// This corresponds to `message_to_dict` in LangChain Python.
/// The dict will have a `type` key with the message type and a `data` key
/// with the message data as a dict (all fields serialized).
pub fn message_to_dict(message: &BaseMessage) -> serde_json::Value {
    // Serialize the message using serde - this includes the "type" field
    let mut data = serde_json::to_value(message).unwrap_or_default();

    // Extract the type from the serialized data (it's included by the Serialize impl)
    let msg_type = message.message_type();

    // Remove the "type" field from data since we'll put it at the top level
    if let Some(obj) = data.as_object_mut() {
        obj.remove("type");
    }

    serde_json::json!({
        "type": msg_type,
        "data": data
    })
}

/// Convert a sequence of messages to a list of dictionaries.
///
/// This corresponds to `messages_to_dict` in LangChain Python.
pub fn messages_to_dict(messages: &[BaseMessage]) -> Vec<serde_json::Value> {
    messages.iter().map(message_to_dict).collect()
}

/// Convert a dictionary to a message.
///
/// This corresponds to `_message_from_dict` in LangChain Python.
pub fn message_from_dict(message: &serde_json::Value) -> Result<BaseMessage, String> {
    let msg_type = message
        .get("type")
        .and_then(|t| t.as_str())
        .ok_or_else(|| "Message dict must contain 'type' key".to_string())?;

    let data = message
        .get("data")
        .ok_or_else(|| "Message dict must contain 'data' key".to_string())?;

    // Merge the type field into the data for deserialization
    // The BaseMessage deserializer expects the type field to be present
    let mut merged_data = data.clone();
    if let Some(obj) = merged_data.as_object_mut() {
        obj.insert(
            "type".to_string(),
            serde_json::Value::String(msg_type.to_string()),
        );
    }

    // Use serde deserialization
    serde_json::from_value(merged_data).map_err(|e| {
        format!(
            "Failed to deserialize message of type '{}': {}",
            msg_type, e
        )
    })
}

/// Convert a sequence of message dicts to messages.
///
/// This corresponds to `messages_from_dict` in LangChain Python.
pub fn messages_from_dict(messages: &[serde_json::Value]) -> Result<Vec<BaseMessage>, String> {
    messages.iter().map(message_from_dict).collect()
}

// ============================================================================
// convert_to_messages / convert_to_message
// ============================================================================

/// Convert message-like representations to messages.
///
/// This function can convert from:
/// - BaseMessage (returned as-is)
/// - 2-tuple of (role, content) as serde_json::Value
/// - dict with "role"/"type" and "content" keys
/// - string (converted to HumanMessage)
///
/// This corresponds to `convert_to_messages` in LangChain Python.
pub fn convert_to_messages(messages: &[serde_json::Value]) -> Result<Vec<BaseMessage>, String> {
    let mut result = Vec::new();

    for message in messages {
        result.push(convert_to_message(message)?);
    }

    Ok(result)
}

pub fn convert_to_message(message: &serde_json::Value) -> Result<BaseMessage, String> {
    if let Some(_msg_type) = message.get("type").and_then(|t| t.as_str()) {
        // Check if it has a "data" key — if so it's a serialized message dict
        if message.get("data").is_some() {
            return message_from_dict(message);
        }
        // Otherwise treat it like a role-based dict (type acts like role)
        let msg_kwargs = message.as_object().ok_or("Expected object")?;
        let msg_type = msg_kwargs
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("");
        let content = msg_kwargs
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("");
        let name = msg_kwargs.get("name").and_then(|n| n.as_str());
        let tool_call_id = msg_kwargs.get("tool_call_id").and_then(|t| t.as_str());
        let tool_calls = msg_kwargs.get("tool_calls").and_then(|t| t.as_array());
        let id = msg_kwargs.get("id").and_then(|i| i.as_str());
        return create_message_from_role(msg_type, content, name, tool_call_id, tool_calls, id);
    }

    if let Some(obj) = message.as_object() {
        // Dict with "role" and "content" keys
        let msg_type = obj
            .get("role")
            .and_then(|r| r.as_str())
            .or_else(|| obj.get("type").and_then(|t| t.as_str()));

        if let Some(msg_type) = msg_type {
            let content = obj.get("content").and_then(|c| c.as_str()).unwrap_or("");
            let name = obj.get("name").and_then(|n| n.as_str());
            let tool_call_id = obj.get("tool_call_id").and_then(|t| t.as_str());
            let tool_calls = obj.get("tool_calls").and_then(|t| t.as_array());
            let id = obj.get("id").and_then(|i| i.as_str());
            return create_message_from_role(msg_type, content, name, tool_call_id, tool_calls, id);
        }
    }

    if let Some(s) = message.as_str() {
        // Plain string -> HumanMessage
        return Ok(BaseMessage::Human(
            HumanMessage::builder().content(s).build(),
        ));
    }

    if let Some(arr) = message.as_array() {
        // 2-tuple: [role, content]
        if arr.len() == 2 {
            let role = arr[0].as_str().ok_or("First element must be role string")?;
            let content = arr[1]
                .as_str()
                .ok_or("Second element must be content string")?;
            return create_message_from_role(role, content, None, None, None, None);
        } else {
            return Err("Array message must have exactly 2 elements [role, content]".to_string());
        }
    }

    Err(format!("Cannot convert to message: {:?}", message))
}

// ============================================================================
// create_message_from_role
// ============================================================================

/// Create a message from a message type string and content.
///
/// This corresponds to `_create_message_from_message_type` in LangChain Python.
fn create_message_from_role(
    role: &str,
    content: &str,
    name: Option<&str>,
    tool_call_id: Option<&str>,
    tool_calls: Option<&Vec<serde_json::Value>>,
    id: Option<&str>,
) -> Result<BaseMessage, String> {
    // Parse tool_calls from OpenAI format to ToolCall structs
    let parsed_tool_calls: Vec<ToolCall> = if let Some(tcs) = tool_calls {
        tcs.iter()
            .filter_map(|tc| {
                if let Some(function) = tc.get("function") {
                    let args_raw = function
                        .get("arguments")
                        .and_then(|a| a.as_str())
                        .unwrap_or("{}");
                    let args = serde_json::from_str(args_raw)
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                    Some(ToolCall {
                        name: function
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("")
                            .to_string(),
                        args,
                        id: tc.get("id").and_then(|i| i.as_str()).map(|s| s.to_string()),
                        call_type: Some("tool_call".to_string()),
                    })
                } else {
                    // Already in LangChain format
                    serde_json::from_value::<ToolCall>(tc.clone()).ok()
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    match role {
        "human" | "user" => Ok(BaseMessage::Human(
            HumanMessage::builder()
                .content(content)
                .maybe_name(name.map(|n| n.to_string()))
                .maybe_id(id.map(|i| i.to_string()))
                .build(),
        )),
        "ai" | "assistant" => Ok(BaseMessage::AI(
            AIMessage::builder()
                .content(content)
                .maybe_name(name.map(|n| n.to_string()))
                .maybe_id(id.map(|i| i.to_string()))
                .tool_calls(parsed_tool_calls)
                .build(),
        )),
        "system" => Ok(BaseMessage::System(
            SystemMessage::builder()
                .content(content)
                .maybe_name(name.map(|n| n.to_string()))
                .maybe_id(id.map(|i| i.to_string()))
                .build(),
        )),
        "developer" => {
            let mut msg = SystemMessage::builder()
                .content(content)
                .maybe_name(name.map(|n| n.to_string()))
                .maybe_id(id.map(|i| i.to_string()))
                .build();
            msg.additional_kwargs.insert(
                "__openai_role__".to_string(),
                serde_json::Value::String("developer".to_string()),
            );
            Ok(BaseMessage::System(msg))
        }
        "function" => {
            let fn_name = name.ok_or("Function messages require a name")?;
            Ok(BaseMessage::Function(
                FunctionMessage::builder()
                    .name(fn_name)
                    .content(content)
                    .maybe_id(id.map(|i| i.to_string()))
                    .build(),
            ))
        }
        "tool" => {
            let tc_id = tool_call_id.ok_or("Tool messages require a tool_call_id")?;
            Ok(BaseMessage::Tool(
                ToolMessage::builder()
                    .content(content)
                    .tool_call_id(tc_id)
                    .maybe_name(name.map(|n| n.to_string()))
                    .maybe_id(id.map(|i| i.to_string()))
                    .build(),
            ))
        }
        "remove" => {
            let msg_id = id.unwrap_or("");
            Ok(BaseMessage::Remove(
                RemoveMessage::builder().id(msg_id).build(),
            ))
        }
        _ => Ok(BaseMessage::Chat(
            ChatMessage::builder()
                .content(content)
                .role(role)
                .maybe_name(name.map(|n| n.to_string()))
                .maybe_id(id.map(|i| i.to_string()))
                .build(),
        )),
    }
}

// ============================================================================
// filter_messages
// ============================================================================

/// Options for excluding tool calls from filtered messages.
///
/// This corresponds to the `exclude_tool_calls` parameter in LangChain Python's
/// `filter_messages`.
#[derive(Debug, Clone)]
pub enum ExcludeToolCalls {
    /// Exclude all AIMessages with tool calls and all ToolMessages.
    All,
    /// Exclude ToolMessages with matching tool_call_ids and filter matching
    /// tool_calls from AIMessages (excluding the whole AIMessage if no
    /// tool_calls remain).
    Ids(Vec<String>),
}

/// Filter messages based on name, type, or ID.
///
/// This corresponds to `filter_messages` in LangChain Python.
#[allow(clippy::too_many_arguments)]
pub fn filter_messages(
    messages: &[BaseMessage],
    include_names: Option<&[&str]>,
    exclude_names: Option<&[&str]>,
    include_types: Option<&[&str]>,
    exclude_types: Option<&[&str]>,
    include_ids: Option<&[&str]>,
    exclude_ids: Option<&[&str]>,
    exclude_tool_calls: Option<&ExcludeToolCalls>,
) -> Vec<BaseMessage> {
    let mut filtered: Vec<BaseMessage> = Vec::new();

    for msg in messages {
        // Check exclusions first
        if let Some(exclude_names) = exclude_names
            && let Some(name) = msg.name()
            && exclude_names.contains(&name.as_str())
        {
            continue;
        }

        if let Some(exclude_types) = exclude_types
            && exclude_types.contains(&msg.message_type())
        {
            continue;
        }

        if let Some(exclude_ids) = exclude_ids
            && let Some(id) = msg.id()
            && exclude_ids.contains(&id.as_str())
        {
            continue;
        }

        // Handle exclude_tool_calls
        let mut msg = msg.clone();
        match exclude_tool_calls {
            Some(ExcludeToolCalls::All) => {
                if let BaseMessage::AI(ref ai_msg) = msg
                    && !ai_msg.tool_calls.is_empty()
                {
                    continue;
                }
                if matches!(msg, BaseMessage::Tool(_)) {
                    continue;
                }
            }
            Some(ExcludeToolCalls::Ids(ids)) => {
                if let BaseMessage::AI(ref ai_msg) = msg
                    && !ai_msg.tool_calls.is_empty()
                {
                    let remaining_tool_calls: Vec<ToolCall> = ai_msg
                        .tool_calls
                        .iter()
                        .filter(|tc| tc.id.as_ref().is_none_or(|id| !ids.contains(id)))
                        .cloned()
                        .collect();
                    if remaining_tool_calls.is_empty() {
                        continue;
                    }
                    if remaining_tool_calls.len() != ai_msg.tool_calls.len() {
                        msg = BaseMessage::AI(
                            AIMessage::builder()
                                .content(ai_msg.content.clone())
                                .maybe_id(ai_msg.id.clone())
                                .maybe_name(ai_msg.name.clone())
                                .tool_calls(remaining_tool_calls)
                                .invalid_tool_calls(ai_msg.invalid_tool_calls.clone())
                                .maybe_usage_metadata(ai_msg.usage_metadata.clone())
                                .additional_kwargs(ai_msg.additional_kwargs.clone())
                                .response_metadata(ai_msg.response_metadata.clone())
                                .build(),
                        );
                    }
                }
                if let BaseMessage::Tool(ref tool_msg) = msg
                    && ids.contains(&tool_msg.tool_call_id)
                {
                    continue;
                }
            }
            None => {}
        }

        // Check inclusions (default to including if no criteria given)
        let no_include_criteria =
            include_names.is_none() && include_types.is_none() && include_ids.is_none();

        let matches_include_names = include_names.is_some_and(|names| {
            msg.name()
                .is_some_and(|name| names.contains(&name.as_str()))
        });

        let matches_include_types =
            include_types.is_some_and(|types| types.contains(&msg.message_type()));

        let matches_include_ids =
            include_ids.is_some_and(|ids| msg.id().is_some_and(|id| ids.contains(&id.as_str())));

        if no_include_criteria
            || matches_include_names
            || matches_include_types
            || matches_include_ids
        {
            filtered.push(msg);
        }
    }

    filtered
}

// ============================================================================
// merge_message_runs
// ============================================================================

/// Merge consecutive messages of the same type.
///
/// Note: ToolMessages are not merged, as each has a distinct tool call ID.
///
/// This corresponds to `merge_message_runs` in LangChain Python.
/// Uses chunk-based merging to properly merge tool_calls, response_metadata,
/// additional_kwargs, and content blocks (not just string concatenation).
pub fn merge_message_runs(messages: &[BaseMessage], chunk_separator: &str) -> Vec<BaseMessage> {
    if messages.is_empty() {
        return Vec::new();
    }

    let mut merged: Vec<BaseMessage> = Vec::new();

    for msg in messages {
        let last = if merged.is_empty() {
            None
        } else {
            merged.pop()
        };

        let Some(last) = last else {
            merged.push(msg.clone());
            continue;
        };

        // Don't merge ToolMessages or messages of different types
        if matches!(msg, BaseMessage::Tool(_))
            || std::mem::discriminant(&last) != std::mem::discriminant(msg)
        {
            merged.push(last);
            merged.push(msg.clone());
        } else {
            // Same type — use chunk-based merging
            let last_chunk = msg_to_chunk(&last);
            let mut curr_chunk = msg_to_chunk(msg);

            // Clear response_metadata on the current chunk before merge
            // (matching Python behavior)
            match &mut curr_chunk {
                BaseMessageChunk::AI(c) => c.response_metadata.clear(),
                BaseMessageChunk::Human(c) => c.response_metadata.clear(),
                BaseMessageChunk::System(c) => c.response_metadata.clear(),
                BaseMessageChunk::Tool(c) => c.response_metadata.clear(),
                BaseMessageChunk::Chat(c) => c.response_metadata.clear(),
                BaseMessageChunk::Function(c) => c.response_metadata.clear(),
            }

            // Insert chunk_separator between string contents when both are non-empty
            if !chunk_separator.is_empty() {
                let last_content = last_chunk.content();
                let curr_content = curr_chunk.content();
                if !last_content.is_empty() && !curr_content.is_empty() {
                    let last_is_str =
                        matches!(last_content, super::content::MessageContent::Text(_));
                    let curr_is_str =
                        matches!(curr_content, super::content::MessageContent::Text(_));
                    if last_is_str && curr_is_str {
                        // Append separator to the last chunk's content before merge
                        match &mut curr_chunk {
                            BaseMessageChunk::AI(c) => {
                                if let super::content::MessageContent::Text(ref mut s) = c.content {
                                    *s = format!("{}{}", chunk_separator, s);
                                }
                            }
                            BaseMessageChunk::Human(c) => {
                                if let super::content::MessageContent::Text(ref mut s) = c.content {
                                    *s = format!("{}{}", chunk_separator, s);
                                }
                            }
                            BaseMessageChunk::System(c) => {
                                if let super::content::MessageContent::Text(ref mut s) = c.content {
                                    *s = format!("{}{}", chunk_separator, s);
                                }
                            }
                            BaseMessageChunk::Chat(c) => {
                                if let super::content::MessageContent::Text(ref mut s) = c.content {
                                    *s = format!("{}{}", chunk_separator, s);
                                }
                            }
                            BaseMessageChunk::Function(c) => {
                                if let super::content::MessageContent::Text(ref mut s) = c.content {
                                    *s = format!("{}{}", chunk_separator, s);
                                }
                            }
                            BaseMessageChunk::Tool(c) => {
                                if let super::content::MessageContent::Text(ref mut s) = c.content {
                                    *s = format!("{}{}", chunk_separator, s);
                                }
                            }
                        }
                    }
                }
            }

            let mut merged_chunk = last_chunk + curr_chunk;

            // For AI chunks, re-parse tool_call_chunks into tool_calls
            // (matching Python where init_tool_calls runs on every construction)
            if let BaseMessageChunk::AI(ref mut ai_chunk) = merged_chunk {
                ai_chunk.init_tool_calls();
            }

            merged.push(chunk_to_msg(&merged_chunk));
        }
    }

    merged
}

/// Convert a message chunk to a complete message.
///
/// This corresponds to `message_chunk_to_message` in LangChain Python.
pub fn message_chunk_to_message(chunk: &BaseMessageChunk) -> BaseMessage {
    chunk.to_message()
}

// ============================================================================
// count_tokens_approximately
// ============================================================================

/// Configuration for approximate token counting.
#[derive(Debug, Clone)]
pub struct CountTokensConfig {
    /// Number of characters per token to use for the approximation.
    /// One token corresponds to ~4 chars for common English text.
    pub chars_per_token: f64,
    /// Number of extra tokens to add per message, e.g. special tokens.
    pub extra_tokens_per_message: f64,
    /// Whether to include message names in the count.
    pub count_name: bool,
}

impl Default for CountTokensConfig {
    fn default() -> Self {
        Self {
            chars_per_token: 4.0,
            extra_tokens_per_message: 3.0,
            count_name: true,
        }
    }
}

/// Approximate the total number of tokens in messages.
///
/// The token count includes stringified message content, role, and (optionally) name.
/// - For AI messages, the token count also includes stringified tool calls.
/// - For tool messages, the token count also includes the tool call ID.
///
/// # Arguments
///
/// * `messages` - Slice of messages to count tokens for.
/// * `config` - Configuration for token counting (use `CountTokensConfig::default()` for defaults).
///
/// # Returns
///
/// Approximate number of tokens in the messages.
///
/// This corresponds to `count_tokens_approximately` in LangChain Python.
pub fn count_tokens_approximately(messages: &[BaseMessage], config: &CountTokensConfig) -> usize {
    let mut token_count: f64 = 0.0;

    for message in messages {
        let mut message_chars: usize = 0;

        // Count content characters
        message_chars += message.text().len();

        // For AI messages, also count tool calls if present
        if let BaseMessage::AI(ai_msg) = message
            && !ai_msg.tool_calls.is_empty()
        {
            let tool_calls_str = format!("{:?}", ai_msg.tool_calls);
            message_chars += tool_calls_str.len();
        }

        // For tool messages, also count the tool call ID
        if let BaseMessage::Tool(tool_msg) = message {
            message_chars += tool_msg.tool_call_id.len();
        }

        // Add role characters
        let role = get_message_openai_role(message);
        message_chars += role.len();

        // Add name if present and config says to count it
        if config.count_name
            && let Some(name) = message.name()
        {
            message_chars += name.len();
        }

        // Round up per message to ensure individual message token counts
        // add up to the total count for a list of messages
        token_count += (message_chars as f64 / config.chars_per_token).ceil();

        // Add extra tokens per message
        token_count += config.extra_tokens_per_message;
    }

    // Round up one more time in case extra_tokens_per_message is a float
    token_count.ceil() as usize
}

/// Get the OpenAI role string for a message.
fn get_message_openai_role(message: &BaseMessage) -> &'static str {
    match message {
        BaseMessage::AI(_) => "assistant",
        BaseMessage::Human(_) => "user",
        BaseMessage::Tool(_) => "tool",
        BaseMessage::System(msg) => {
            if msg
                .additional_kwargs
                .get("__openai_role__")
                .and_then(|v| v.as_str())
                == Some("developer")
            {
                "developer"
            } else {
                "system"
            }
        }
        BaseMessage::Function(_) => "function",
        BaseMessage::Chat(c) => {
            // Return static strings for common roles, otherwise return a generic one
            match c.role.as_str() {
                "user" => "user",
                "assistant" => "assistant",
                "system" => "system",
                "function" => "function",
                "tool" => "tool",
                _ => "user",
            }
        }
        BaseMessage::Remove(_) => "remove",
    }
}

// ============================================================================
// convert_to_openai_messages
// ============================================================================

/// Text format options for OpenAI message conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextFormat {
    /// If a message has a string content, this is left as a string.
    /// If a message has content blocks that are all of type 'text', these
    /// are joined with a newline to make a single string.
    #[default]
    String,
    /// If a message has a string content, this is turned into a list
    /// with a single content block of type 'text'.
    Block,
}

/// Convert LangChain messages into OpenAI message dicts.
///
/// # Arguments
///
/// * `messages` - Slice of messages to convert.
/// * `text_format` - How to format string or text block contents.
/// * `include_id` - Whether to include message IDs in the output.
///
/// # Returns
///
/// A list of OpenAI message dicts as JSON Values.
///
/// This corresponds to `convert_to_openai_messages` in LangChain Python.
pub fn convert_to_openai_messages(
    messages: &[BaseMessage],
    text_format: TextFormat,
    include_id: bool,
) -> Vec<serde_json::Value> {
    let mut oai_messages = Vec::new();
    for msg in messages {
        oai_messages.extend(convert_single_to_openai_message(
            msg,
            text_format,
            include_id,
        ));
    }
    oai_messages
}

/// Convert a single message to OpenAI format.
///
/// Returns a Vec because some content blocks (tool_result) can produce
/// additional ToolMessages that need to be appended.
fn convert_single_to_openai_message(
    message: &BaseMessage,
    text_format: TextFormat,
    include_id: bool,
) -> Vec<serde_json::Value> {
    let role = get_message_openai_role(message);
    let mut oai_msg = serde_json::json!({ "role": role });

    // Add name if present
    if let Some(name) = message.name() {
        oai_msg["name"] = serde_json::json!(name);
    }

    // Add tool_call_id for tool messages
    if let BaseMessage::Tool(tool_msg) = message {
        oai_msg["tool_call_id"] = serde_json::json!(tool_msg.tool_call_id);
    }

    // Add tool_calls for AI messages
    if let BaseMessage::AI(ai_msg) = message
        && !ai_msg.tool_calls.is_empty()
    {
        oai_msg["tool_calls"] = serde_json::json!(convert_to_openai_tool_calls(&ai_msg.tool_calls));
    }

    // Add refusal from additional_kwargs if present
    if let Some(additional_kwargs) = message.additional_kwargs()
        && let Some(refusal) = additional_kwargs.get("refusal")
    {
        oai_msg["refusal"] = refusal.clone();
    }

    // Add message ID if requested
    if include_id && let Some(id) = message.id() {
        oai_msg["id"] = serde_json::json!(id);
    }

    // Handle content
    // Try to get content as list (for multimodal messages)
    let raw_content = message.content();
    let content_list: Option<Vec<serde_json::Value>> = match raw_content {
        super::content::MessageContent::Parts(_) => Some(raw_content.as_json_values()),
        super::content::MessageContent::Text(s) => serde_json::from_str(s).ok(),
    };

    let mut tool_messages: Vec<serde_json::Value> = Vec::new();

    if let Some(blocks) = content_list {
        // Content is a list of blocks — process each block
        let mut content_blocks: Vec<serde_json::Value> = Vec::new();

        for block in &blocks {
            let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");

            match block_type {
                "text" => {
                    content_blocks.push(serde_json::json!({
                        "type": "text",
                        "text": block.get("text").and_then(|t| t.as_str()).unwrap_or(""),
                    }));
                }
                "image_url" => {
                    content_blocks.push(serde_json::json!({
                        "type": "image_url",
                        "image_url": block.get("image_url").cloned().unwrap_or_default(),
                    }));
                }
                "input_audio" => {
                    content_blocks.push(block.clone());
                }
                "tool_use" => {
                    // Anthropic tool_use block — convert to OpenAI tool_call if not
                    // already in tool_calls
                    if let BaseMessage::AI(ai_msg) = message {
                        let block_id = block.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        let already_in_tool_calls = ai_msg
                            .tool_calls
                            .iter()
                            .any(|tc| tc.id.as_deref() == Some(block_id));
                        if !already_in_tool_calls {
                            let tool_calls_arr = oai_msg
                                .get("tool_calls")
                                .and_then(|v| v.as_array())
                                .cloned()
                                .unwrap_or_default();
                            let mut new_tool_calls = tool_calls_arr;
                            new_tool_calls.push(serde_json::json!({
                                "type": "function",
                                "id": block_id,
                                "function": {
                                    "name": block.get("name").and_then(|n| n.as_str()).unwrap_or(""),
                                    "arguments": serde_json::to_string(
                                        block.get("input").unwrap_or(&serde_json::json!({}))
                                    ).unwrap_or_default(),
                                }
                            }));
                            oai_msg["tool_calls"] = serde_json::json!(new_tool_calls);
                        }
                    }
                }
                "tool_result" => {
                    // Anthropic tool_result — convert to ToolMessage
                    let tool_use_id = block
                        .get("tool_use_id")
                        .and_then(|t| t.as_str())
                        .unwrap_or("");
                    let tool_content = block.get("content").and_then(|c| c.as_str()).unwrap_or("");
                    let is_error = block
                        .get("is_error")
                        .and_then(|e| e.as_bool())
                        .unwrap_or(false);
                    let status = if is_error { "error" } else { "success" };
                    let tool_msg = ToolMessage::builder()
                        .content(tool_content)
                        .tool_call_id(tool_use_id)
                        .status(super::tool::ToolStatus::from(status.to_string()))
                        .build();
                    // Recursively convert tool message to OpenAI format
                    tool_messages.extend(convert_single_to_openai_message(
                        &BaseMessage::Tool(tool_msg),
                        text_format,
                        include_id,
                    ));
                }
                "image" | "source" => {
                    // Anthropic image format
                    if let Some(source) = block.get("source") {
                        let media_type = source
                            .get("media_type")
                            .and_then(|m| m.as_str())
                            .unwrap_or("");
                        let src_type = source.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        let data = source.get("data").and_then(|d| d.as_str()).unwrap_or("");
                        content_blocks.push(serde_json::json!({
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:{};{},{}", media_type, src_type, data),
                            }
                        }));
                    }
                }
                "thinking" | "reasoning" => {
                    content_blocks.push(block.clone());
                }
                _ => {
                    // Pass through unknown blocks
                    if let Some(s) = block.as_str() {
                        content_blocks.push(serde_json::json!({"type": "text", "text": s}));
                    } else {
                        content_blocks.push(block.clone());
                    }
                }
            }
        }

        // Apply text_format to the result
        match text_format {
            TextFormat::String => {
                if content_blocks
                    .iter()
                    .all(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
                {
                    // All text blocks — join into a single string
                    let joined: String = content_blocks
                        .iter()
                        .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                        .collect::<Vec<_>>()
                        .join("\n");
                    oai_msg["content"] = serde_json::json!(joined);
                } else {
                    oai_msg["content"] = serde_json::json!(content_blocks);
                }
            }
            TextFormat::Block => {
                oai_msg["content"] = serde_json::json!(content_blocks);
            }
        }
    } else {
        // Content is a plain string
        match text_format {
            TextFormat::String => {
                oai_msg["content"] = serde_json::json!(raw_content);
            }
            TextFormat::Block => {
                if raw_content.is_empty() {
                    oai_msg["content"] = serde_json::json!([]);
                } else {
                    oai_msg["content"] =
                        serde_json::json!([{ "type": "text", "text": raw_content }]);
                }
            }
        }
    }

    // If content is non-empty or there are no tool_messages, include the main message
    let has_content = oai_msg.get("content").is_some_and(|c| {
        if let Some(s) = c.as_str() {
            !s.is_empty()
        } else if let Some(arr) = c.as_array() {
            !arr.is_empty()
        } else {
            true
        }
    });

    if has_content || tool_messages.is_empty() {
        let mut result = vec![oai_msg];
        result.extend(tool_messages);
        result
    } else {
        tool_messages
    }
}

/// Convert ToolCall list to OpenAI tool_calls format.
fn convert_to_openai_tool_calls(tool_calls: &[ToolCall]) -> Vec<serde_json::Value> {
    tool_calls
        .iter()
        .map(|tc| {
            serde_json::json!({
                "type": "function",
                "id": tc.id,
                "function": {
                    "name": tc.name,
                    "arguments": serde_json::to_string(&tc.args).unwrap_or_default(),
                }
            })
        })
        .collect()
}

// ============================================================================
// _is_message_type / _default_text_splitter
// ============================================================================

/// Check if a message matches any of the given type strings.
///
/// This corresponds to `_is_message_type` in LangChain Python.
fn is_message_type(message: &BaseMessage, types: &[String]) -> bool {
    types.iter().any(|t| t == message.message_type())
}

/// Default text splitter that splits on newlines, keeping the separator.
///
/// This corresponds to `_default_text_splitter` in LangChain Python.
fn default_text_splitter(text: &str) -> Vec<String> {
    let splits: Vec<&str> = text.split('\n').collect();
    if splits.len() <= 1 {
        return vec![text.to_string()];
    }
    let mut result: Vec<String> = splits[..splits.len() - 1]
        .iter()
        .map(|s| format!("{}\n", s))
        .collect();
    result.push(splits.last().unwrap_or(&"").to_string());
    result
}

// ============================================================================
// trim_messages
// ============================================================================

/// Strategy for trimming messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrimStrategy {
    /// Keep the first `<= max_tokens` tokens of the messages.
    First,
    /// Keep the last `<= max_tokens` tokens of the messages.
    #[default]
    Last,
}

/// Configuration for trimming messages.
#[derive(Debug, Clone)]
pub struct TrimMessagesConfig<F, S = fn(&str) -> Vec<String>>
where
    F: Fn(&[BaseMessage]) -> usize,
    S: Fn(&str) -> Vec<String>,
{
    /// Maximum token count of trimmed messages.
    pub max_tokens: usize,
    /// Function for counting tokens in a list of messages.
    pub token_counter: F,
    /// Strategy for trimming.
    pub strategy: TrimStrategy,
    /// Whether to split a message if only part can be included.
    pub allow_partial: bool,
    /// Whether to keep the SystemMessage if there is one at index 0.
    /// Only valid with strategy="last".
    pub include_system: bool,
    /// The message type(s) to end on. If specified, every message after the last
    /// occurrence of this type is ignored. Can be specified as string names
    /// (e.g. "system", "human", "ai", ...).
    pub end_on: Option<Vec<String>>,
    /// The message type(s) to start on. Should only be specified if
    /// strategy="last". If specified, every message before the first occurrence
    /// of this type is ignored (after trimming to max_tokens).
    pub start_on: Option<Vec<String>>,
    /// Custom text splitter function for partial message splitting.
    /// When `allow_partial` is true, this function is used to split text content
    /// into chunks. Defaults to splitting on newlines.
    pub text_splitter: Option<S>,
}

impl<F> TrimMessagesConfig<F>
where
    F: Fn(&[BaseMessage]) -> usize,
{
    /// Create a new config with required parameters.
    pub fn new(max_tokens: usize, token_counter: F) -> Self {
        Self {
            max_tokens,
            token_counter,
            strategy: TrimStrategy::Last,
            allow_partial: false,
            include_system: false,
            end_on: None,
            start_on: None,
            text_splitter: None,
        }
    }

    /// Set the trimming strategy.
    pub fn with_strategy(mut self, strategy: TrimStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set whether to allow partial messages.
    pub fn with_allow_partial(mut self, allow_partial: bool) -> Self {
        self.allow_partial = allow_partial;
        self
    }

    /// Set whether to include the system message.
    pub fn with_include_system(mut self, include_system: bool) -> Self {
        self.include_system = include_system;
        self
    }

    /// Set the message type(s) to end on.
    pub fn with_end_on(mut self, end_on: Vec<String>) -> Self {
        self.end_on = Some(end_on);
        self
    }

    /// Set the message type(s) to start on.
    pub fn with_start_on(mut self, start_on: Vec<String>) -> Self {
        self.start_on = Some(start_on);
        self
    }

    /// Set a custom text splitter function for partial message splitting.
    pub fn with_text_splitter<S2: Fn(&str) -> Vec<String>>(
        self,
        text_splitter: S2,
    ) -> TrimMessagesConfig<F, S2> {
        TrimMessagesConfig {
            max_tokens: self.max_tokens,
            token_counter: self.token_counter,
            strategy: self.strategy,
            allow_partial: self.allow_partial,
            include_system: self.include_system,
            end_on: self.end_on,
            start_on: self.start_on,
            text_splitter: Some(text_splitter),
        }
    }
}

/// Trim messages to be below a token count.
///
/// # Arguments
///
/// * `messages` - Slice of messages to trim.
/// * `config` - Configuration for trimming.
///
/// # Returns
///
/// List of trimmed messages.
///
/// This corresponds to `trim_messages` in LangChain Python.
pub fn trim_messages<F, S>(
    messages: &[BaseMessage],
    config: &TrimMessagesConfig<F, S>,
) -> Vec<BaseMessage>
where
    F: Fn(&[BaseMessage]) -> usize,
    S: Fn(&str) -> Vec<String>,
{
    if messages.is_empty() {
        return Vec::new();
    }

    // Validate arguments
    if config.start_on.is_some() && config.strategy == TrimStrategy::First {
        panic!("start_on parameter is only valid with strategy='last'");
    }
    if config.include_system && config.strategy == TrimStrategy::First {
        panic!("include_system parameter is only valid with strategy='last'");
    }

    match config.strategy {
        TrimStrategy::First => trim_messages_first(messages, config, false),
        TrimStrategy::Last => trim_messages_last(messages, config),
    }
}

/// Trim messages from the beginning (strategy="first").
///
/// When `reverse_partial` is true, partial content is taken from the end
/// of the content (used when the overall strategy is "last" and messages
/// have been reversed).
fn trim_messages_first<F, S>(
    messages: &[BaseMessage],
    config: &TrimMessagesConfig<F, S>,
    reverse_partial: bool,
) -> Vec<BaseMessage>
where
    F: Fn(&[BaseMessage]) -> usize,
    S: Fn(&str) -> Vec<String>,
{
    let mut messages: Vec<BaseMessage> = messages.to_vec();

    if messages.is_empty() {
        return messages;
    }

    // Check if all messages already fit
    if (config.token_counter)(&messages) <= config.max_tokens {
        // When all messages fit, only apply end_on filtering if needed
        if let Some(ref end_on) = config.end_on {
            while !messages.is_empty()
                && !is_message_type(messages.last().expect("checked non-empty"), end_on)
            {
                messages.pop();
            }
        }
        return messages;
    }

    // Binary search to find maximum number of complete messages
    let mut left = 0;
    let mut right = messages.len();

    while left < right {
        let mid = (left + right).div_ceil(2);
        if (config.token_counter)(&messages[..mid]) <= config.max_tokens {
            left = mid;
        } else {
            right = mid - 1;
        }
    }

    let mut idx = left;

    // Handle partial messages if allowed
    if config.allow_partial && idx < messages.len() {
        let mut included_partial = false;

        // First try list content (multimodal blocks or JSON-encoded arrays in Text)
        let excluded_content = messages[idx].content();
        let content_blocks_opt: Option<Vec<serde_json::Value>> = match excluded_content {
            super::content::MessageContent::Parts(_) => Some(excluded_content.as_json_values()),
            super::content::MessageContent::Text(s) => serde_json::from_str(s).ok(),
        };
        if let Some(mut content_blocks) = content_blocks_opt
            && content_blocks.len() > 1
        {
            if reverse_partial {
                content_blocks.reverse();
            }
            let num_blocks = content_blocks.len();
            for remove_count in 1..num_blocks {
                let mut partial_blocks = content_blocks[..num_blocks - remove_count].to_vec();
                if reverse_partial {
                    partial_blocks.reverse();
                }
                let partial_content = serde_json::to_string(&partial_blocks).unwrap_or_default();
                let partial_msg = create_message_with_content(&messages[idx], &partial_content);
                let mut test = messages[..idx].to_vec();
                test.push(partial_msg);
                if (config.token_counter)(&test) <= config.max_tokens {
                    messages = test;
                    idx += 1;
                    included_partial = true;
                    break;
                }
            }
        }

        // Then try text splitting
        if !included_partial {
            let content_str = messages[idx].text();
            if !content_str.is_empty() {
                let mut split_texts = if let Some(ref splitter) = config.text_splitter {
                    splitter(&content_str)
                } else {
                    default_text_splitter(&content_str)
                };
                if split_texts.len() > 1 {
                    if reverse_partial {
                        split_texts.reverse();
                    }
                    // Binary search for max splits
                    let mut s_left = 0;
                    let mut s_right = split_texts.len();
                    while s_left < s_right {
                        let mid = (s_left + s_right).div_ceil(2);
                        let partial_content: String = split_texts[..mid].concat();
                        let partial_msg =
                            create_message_with_content(&messages[idx], &partial_content);
                        let mut test = messages[..idx].to_vec();
                        test.push(partial_msg);
                        if (config.token_counter)(&test) <= config.max_tokens {
                            s_left = mid;
                        } else {
                            s_right = mid - 1;
                        }
                    }
                    if s_left > 0 {
                        let mut content_splits = split_texts[..s_left].to_vec();
                        if reverse_partial {
                            content_splits.reverse();
                        }
                        let partial_content: String = content_splits.concat();
                        let partial_msg =
                            create_message_with_content(&messages[idx], &partial_content);
                        let end = idx;
                        messages = messages[..end].to_vec();
                        messages.push(partial_msg);
                        idx += 1;
                    }
                }
            }
        }
    }

    // Apply end_on filtering
    if let Some(ref end_on) = config.end_on {
        while idx > 0 && !is_message_type(&messages[idx - 1], end_on) {
            idx -= 1;
        }
    }

    messages[..idx].to_vec()
}

/// Trim messages from the end (strategy="last").
fn trim_messages_last<F, S>(
    messages: &[BaseMessage],
    config: &TrimMessagesConfig<F, S>,
) -> Vec<BaseMessage>
where
    F: Fn(&[BaseMessage]) -> usize,
    S: Fn(&str) -> Vec<String>,
{
    let mut messages: Vec<BaseMessage> = messages.to_vec();

    if messages.is_empty() {
        return messages;
    }

    // Apply end_on filtering first (for "last" strategy, done before trimming)
    if let Some(ref end_on) = config.end_on {
        while !messages.is_empty()
            && !is_message_type(messages.last().expect("checked non-empty"), end_on)
        {
            messages.pop();
        }
    }

    // Handle system message preservation
    let system_message = if config.include_system
        && !messages.is_empty()
        && matches!(messages.first(), Some(BaseMessage::System(_)))
    {
        Some(messages.remove(0))
    } else {
        None
    };

    // Calculate remaining tokens after system message
    let remaining_tokens = if let Some(ref sys_msg) = system_message {
        let sys_tokens = (config.token_counter)(std::slice::from_ref(sys_msg));
        config.max_tokens.saturating_sub(sys_tokens)
    } else {
        config.max_tokens
    };

    // Reverse and use first strategy logic
    messages.reverse();

    // Build a temporary config for first-strategy on reversed messages
    // Pass start_on as end_on (since we reversed)
    // Wrap the text_splitter reference in a closure so it fits the generic param
    #[allow(clippy::type_complexity)]
    let splitter_wrapper: Option<Box<dyn Fn(&str) -> Vec<String> + '_>> = config
        .text_splitter
        .as_ref()
        .map(|s| Box::new(move |text: &str| s(text)) as Box<dyn Fn(&str) -> Vec<String>>);
    let reverse_config = TrimMessagesConfig {
        max_tokens: remaining_tokens,
        token_counter: &config.token_counter,
        strategy: TrimStrategy::First,
        allow_partial: config.allow_partial,
        include_system: false,
        end_on: config.start_on.clone(),
        start_on: None,
        text_splitter: splitter_wrapper,
    };

    let mut result = trim_messages_first(&messages, &reverse_config, true);

    // Reverse back
    result.reverse();

    // Add system message back
    if let Some(sys_msg) = system_message {
        result.insert(0, sys_msg);
    }

    result
}

/// Create a message of the same type with different content.
fn create_message_with_content(original: &BaseMessage, content: &str) -> BaseMessage {
    match original {
        BaseMessage::Human(m) => BaseMessage::Human(
            HumanMessage::builder()
                .content(content)
                .maybe_id(m.id.clone())
                .build(),
        ),
        BaseMessage::AI(m) => BaseMessage::AI(
            AIMessage::builder()
                .content(content)
                .maybe_id(m.id.clone())
                .build(),
        ),
        BaseMessage::System(m) => BaseMessage::System(
            SystemMessage::builder()
                .content(content)
                .maybe_id(m.id.clone())
                .build(),
        ),
        BaseMessage::Tool(m) => {
            let new_msg = ToolMessage::builder()
                .content(content)
                .tool_call_id(&m.tool_call_id)
                .maybe_id(m.id.clone())
                .build();
            BaseMessage::Tool(new_msg)
        }
        BaseMessage::Chat(m) => BaseMessage::Chat(
            ChatMessage::builder()
                .content(content)
                .role(&m.role)
                .maybe_id(m.id.clone())
                .build(),
        ),
        BaseMessage::Function(m) => BaseMessage::Function(
            FunctionMessage::builder()
                .name(&m.name)
                .content(content)
                .maybe_id(m.id.clone())
                .build(),
        ),
        BaseMessage::Remove(m) => {
            // RemoveMessage preserves the same id (which is the target id to remove)
            BaseMessage::Remove(RemoveMessage::builder().id(&m.id).build())
        }
    }
}

// ============================================================================
// Runnable variants of message utility functions
// ============================================================================
// In Python, the @_runnable_support decorator makes filter_messages,
// merge_message_runs, and trim_messages return a RunnableLambda when called
// without messages. In Rust, we provide separate *_runnable() functions.

use crate::runnables::base::RunnableLambdaWithConfig;
use std::sync::Arc;

/// Create a [`RunnableLambdaWithConfig`] that filters messages.
///
/// This is the runnable counterpart to [`filter_messages`], matching Python's
/// `filter_messages()` called without messages (which returns a `RunnableLambda`
/// via the `@_runnable_support` decorator).
pub fn filter_messages_runnable(
    include_names: Option<Vec<String>>,
    exclude_names: Option<Vec<String>>,
    include_types: Option<Vec<String>>,
    exclude_types: Option<Vec<String>>,
    include_ids: Option<Vec<String>>,
    exclude_ids: Option<Vec<String>>,
    exclude_tool_calls: Option<ExcludeToolCalls>,
) -> RunnableLambdaWithConfig<Vec<BaseMessage>, Vec<BaseMessage>> {
    RunnableLambdaWithConfig::new(move |messages: Vec<BaseMessage>| {
        let include_names_refs: Option<Vec<&str>> = include_names
            .as_ref()
            .map(|v| v.iter().map(|s| s.as_str()).collect());
        let exclude_names_refs: Option<Vec<&str>> = exclude_names
            .as_ref()
            .map(|v| v.iter().map(|s| s.as_str()).collect());
        let include_types_refs: Option<Vec<&str>> = include_types
            .as_ref()
            .map(|v| v.iter().map(|s| s.as_str()).collect());
        let exclude_types_refs: Option<Vec<&str>> = exclude_types
            .as_ref()
            .map(|v| v.iter().map(|s| s.as_str()).collect());
        let include_ids_refs: Option<Vec<&str>> = include_ids
            .as_ref()
            .map(|v| v.iter().map(|s| s.as_str()).collect());
        let exclude_ids_refs: Option<Vec<&str>> = exclude_ids
            .as_ref()
            .map(|v| v.iter().map(|s| s.as_str()).collect());

        Ok(filter_messages(
            &messages,
            include_names_refs.as_deref(),
            exclude_names_refs.as_deref(),
            include_types_refs.as_deref(),
            exclude_types_refs.as_deref(),
            include_ids_refs.as_deref(),
            exclude_ids_refs.as_deref(),
            exclude_tool_calls.as_ref(),
        ))
    })
    .with_name("filter_messages")
}

/// Create a [`RunnableLambdaWithConfig`] that merges consecutive message runs.
///
/// This is the runnable counterpart to [`merge_message_runs`], matching Python's
/// `merge_message_runs()` called without messages.
pub fn merge_message_runs_runnable(
    chunk_separator: Option<String>,
) -> RunnableLambdaWithConfig<Vec<BaseMessage>, Vec<BaseMessage>> {
    let separator = chunk_separator.unwrap_or_else(|| "\n".to_string());
    RunnableLambdaWithConfig::new(move |messages: Vec<BaseMessage>| {
        Ok(merge_message_runs(&messages, &separator))
    })
    .with_name("merge_message_runs")
}

/// Create a [`RunnableLambdaWithConfig`] that trims messages to a token budget.
///
/// This is the runnable counterpart to [`trim_messages`], matching Python's
/// `trim_messages()` called without messages.
pub fn trim_messages_runnable<F, S>(
    config: TrimMessagesConfig<F, S>,
) -> RunnableLambdaWithConfig<Vec<BaseMessage>, Vec<BaseMessage>>
where
    F: Fn(&[BaseMessage]) -> usize + Send + Sync + 'static,
    S: Fn(&str) -> Vec<String> + Send + Sync + 'static,
{
    let config = Arc::new(config);
    RunnableLambdaWithConfig::new(move |messages: Vec<BaseMessage>| {
        Ok(trim_messages(&messages, &config))
    })
    .with_name("trim_messages")
}
