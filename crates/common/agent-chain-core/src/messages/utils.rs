//! Message utility types and functions.
//!
//! This module contains utility types like `AnyMessage` and helper functions
//! for working with messages. Mirrors `langchain_core.messages.utils`.

use super::ai::AIMessage;
use super::base::{BaseMessage, BaseMessageChunk, BaseMessageTrait};
use super::chat::ChatMessage;
use super::function::FunctionMessage;
use super::human::HumanMessage;
use super::modifier::RemoveMessage;
use super::system::SystemMessage;
use super::tool::ToolMessage;

/// Type alias for any message type, matching LangChain's AnyMessage.
/// This is equivalent to BaseMessage but provides naming consistency with Python.
pub type AnyMessage = BaseMessage;

/// A type representing the various ways a message can be represented.
///
/// This corresponds to `MessageLikeRepresentation` in LangChain Python.
pub type MessageLikeRepresentation = serde_json::Value;

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
                BaseMessage::Chat(c) => c.role(),
                BaseMessage::Function(_) => "Function",
                BaseMessage::Remove(_) => "Remove",
            };
            format!("{}: {}", role, m.content())
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
    // Serialize the inner message directly to avoid the duplicate "type" field
    // that would occur from BaseMessage's #[serde(tag = "type")] attribute
    let data = match message {
        BaseMessage::Human(m) => serde_json::to_value(m).unwrap_or_default(),
        BaseMessage::System(m) => serde_json::to_value(m).unwrap_or_default(),
        BaseMessage::AI(m) => serde_json::to_value(m).unwrap_or_default(),
        BaseMessage::Tool(m) => serde_json::to_value(m).unwrap_or_default(),
        BaseMessage::Chat(m) => serde_json::to_value(m).unwrap_or_default(),
        BaseMessage::Function(m) => serde_json::to_value(m).unwrap_or_default(),
        BaseMessage::Remove(m) => serde_json::to_value(m).unwrap_or_default(),
    };
    serde_json::json!({
        "type": message.message_type(),
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

    let content = data.get("content").and_then(|c| c.as_str()).unwrap_or("");

    let id = data.get("id").and_then(|i| i.as_str());

    match msg_type {
        "human" => {
            let msg = match id {
                Some(id) => HumanMessage::with_id(id, content),
                None => HumanMessage::new(content),
            };
            Ok(BaseMessage::Human(msg))
        }
        "ai" => {
            let msg = match id {
                Some(id) => AIMessage::with_id(id, content),
                None => AIMessage::new(content),
            };
            Ok(BaseMessage::AI(msg))
        }
        "system" => {
            let msg = match id {
                Some(id) => SystemMessage::with_id(id, content),
                None => SystemMessage::new(content),
            };
            Ok(BaseMessage::System(msg))
        }
        "tool" => {
            let tool_call_id = data
                .get("tool_call_id")
                .and_then(|t| t.as_str())
                .unwrap_or("");
            let msg = match id {
                Some(id) => ToolMessage::with_id(id, content, tool_call_id),
                None => ToolMessage::new(content, tool_call_id),
            };
            Ok(BaseMessage::Tool(msg))
        }
        "chat" => {
            let role = data.get("role").and_then(|r| r.as_str()).unwrap_or("chat");
            let msg = match id {
                Some(id) => ChatMessage::with_id(id, role, content),
                None => ChatMessage::new(role, content),
            };
            Ok(BaseMessage::Chat(msg))
        }
        "function" => {
            let name = data.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let msg = match id {
                Some(id) => FunctionMessage::with_id(id, name, content),
                None => FunctionMessage::new(name, content),
            };
            Ok(BaseMessage::Function(msg))
        }
        "remove" => {
            let id = id.ok_or_else(|| "RemoveMessage requires an id".to_string())?;
            Ok(BaseMessage::Remove(RemoveMessage::new(id)))
        }
        _ => Err(format!("Unknown message type: {}", msg_type)),
    }
}

/// Convert a sequence of message dicts to messages.
///
/// This corresponds to `messages_from_dict` in LangChain Python.
pub fn messages_from_dict(messages: &[serde_json::Value]) -> Result<Vec<BaseMessage>, String> {
    messages.iter().map(message_from_dict).collect()
}

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
        // Already a message dict
        message_from_dict(message)
    } else if let Some(role) = message.get("role").and_then(|r| r.as_str()) {
        // OpenAI-style dict with "role" and "content"
        let content = message
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("");
        create_message_from_role(role, content)
    } else if let Some(s) = message.as_str() {
        // Plain string -> HumanMessage
        Ok(BaseMessage::Human(HumanMessage::new(s)))
    } else if let Some(arr) = message.as_array() {
        // 2-tuple: [role, content]
        if arr.len() == 2 {
            let role = arr[0].as_str().ok_or("First element must be role string")?;
            let content = arr[1]
                .as_str()
                .ok_or("Second element must be content string")?;
            create_message_from_role(role, content)
        } else {
            Err("Array message must have exactly 2 elements [role, content]".to_string())
        }
    } else {
        Err(format!("Cannot convert to message: {:?}", message))
    }
}

/// Create a message from a role string and content.
fn create_message_from_role(role: &str, content: &str) -> Result<BaseMessage, String> {
    match role {
        "human" | "user" => Ok(BaseMessage::Human(HumanMessage::new(content))),
        "ai" | "assistant" => Ok(BaseMessage::AI(AIMessage::new(content))),
        "system" | "developer" => Ok(BaseMessage::System(SystemMessage::new(content))),
        "function" => Err("Function messages require a name".to_string()),
        "tool" => Err("Tool messages require a tool_call_id".to_string()),
        _ => Ok(BaseMessage::Chat(ChatMessage::new(role, content))),
    }
}

/// Filter messages based on name, type, or ID.
///
/// This corresponds to `filter_messages` in LangChain Python.
pub fn filter_messages(
    messages: &[BaseMessage],
    include_names: Option<&[&str]>,
    exclude_names: Option<&[&str]>,
    include_types: Option<&[&str]>,
    exclude_types: Option<&[&str]>,
    include_ids: Option<&[&str]>,
    exclude_ids: Option<&[&str]>,
) -> Vec<BaseMessage> {
    messages
        .iter()
        .filter(|msg| {
            // Check exclusions first
            if let Some(exclude_names) = exclude_names
                && let Some(name) = msg.name()
                && exclude_names.contains(&name.as_str())
            {
                return false;
            }

            if let Some(exclude_types) = exclude_types
                && exclude_types.contains(&msg.message_type())
            {
                return false;
            }

            if let Some(exclude_ids) = exclude_ids
                && let Some(id) = msg.id()
                && exclude_ids.contains(&id.as_str())
            {
                return false;
            }

            // Check inclusions (default to including if no criteria given)
            // Match Python logic: include if no inclusion criteria are given,
            // OR if any specified inclusion criterion matches
            let no_include_criteria =
                include_names.is_none() && include_types.is_none() && include_ids.is_none();

            let matches_include_names = include_names.is_some_and(|names| {
                msg.name()
                    .is_some_and(|name| names.contains(&name.as_str()))
            });

            let matches_include_types =
                include_types.is_some_and(|types| types.contains(&msg.message_type()));

            let matches_include_ids = include_ids
                .is_some_and(|ids| msg.id().is_some_and(|id| ids.contains(&id.as_str())));

            no_include_criteria
                || matches_include_names
                || matches_include_types
                || matches_include_ids
        })
        .cloned()
        .collect()
}

/// Merge consecutive messages of the same type.
///
/// Note: ToolMessages are not merged, as each has a distinct tool call ID.
///
/// This corresponds to `merge_message_runs` in LangChain Python.
pub fn merge_message_runs(messages: &[BaseMessage], chunk_separator: &str) -> Vec<BaseMessage> {
    if messages.is_empty() {
        return Vec::new();
    }

    let mut merged: Vec<BaseMessage> = Vec::new();

    for msg in messages {
        if merged.is_empty() {
            merged.push(msg.clone());
            continue;
        }

        let last = merged.last().expect("merged is not empty");

        // Don't merge ToolMessages or messages of different types
        if matches!(msg, BaseMessage::Tool(_))
            || std::mem::discriminant(last) != std::mem::discriminant(msg)
        {
            merged.push(msg.clone());
        } else {
            // Same type, merge content
            let last = merged.pop().expect("merged is not empty");
            let merged_content = format!("{}{}{}", last.content(), chunk_separator, msg.content());

            let new_msg = match (last, msg) {
                (BaseMessage::Human(_), BaseMessage::Human(_)) => {
                    BaseMessage::Human(HumanMessage::new(&merged_content))
                }
                (BaseMessage::AI(_), BaseMessage::AI(_)) => {
                    BaseMessage::AI(AIMessage::new(&merged_content))
                }
                (BaseMessage::System(_), BaseMessage::System(_)) => {
                    BaseMessage::System(SystemMessage::new(&merged_content))
                }
                (BaseMessage::Chat(c), BaseMessage::Chat(_)) => {
                    BaseMessage::Chat(ChatMessage::new(c.role(), &merged_content))
                }
                (BaseMessage::Function(f), BaseMessage::Function(_)) => {
                    BaseMessage::Function(FunctionMessage::new(f.name(), &merged_content))
                }
                _ => {
                    // Shouldn't happen due to discriminant check, but handle gracefully
                    merged.push(msg.clone());
                    continue;
                }
            };

            merged.push(new_msg);
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
        message_chars += message.content().len();

        // For AI messages, also count tool calls if present
        if let BaseMessage::AI(ai_msg) = message
            && !ai_msg.tool_calls().is_empty()
        {
            let tool_calls_str = format!("{:?}", ai_msg.tool_calls());
            message_chars += tool_calls_str.len();
        }

        // For tool messages, also count the tool call ID
        if let BaseMessage::Tool(tool_msg) = message {
            message_chars += tool_msg.tool_call_id().len();
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
        BaseMessage::System(_) => "system",
        BaseMessage::Function(_) => "function",
        BaseMessage::Chat(c) => {
            // Return static strings for common roles, otherwise return a generic one
            match c.role() {
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
///
/// # Returns
///
/// A list of OpenAI message dicts as JSON Values.
///
/// This corresponds to `convert_to_openai_messages` in LangChain Python.
pub fn convert_to_openai_messages(
    messages: &[BaseMessage],
    text_format: TextFormat,
) -> Vec<serde_json::Value> {
    messages
        .iter()
        .map(|msg| convert_single_to_openai_message(msg, text_format))
        .collect()
}

/// Convert a single message to OpenAI format.
fn convert_single_to_openai_message(
    message: &BaseMessage,
    text_format: TextFormat,
) -> serde_json::Value {
    let role = get_message_openai_role(message);
    let mut oai_msg = serde_json::json!({ "role": role });

    // Add name if present
    if let Some(name) = message.name() {
        oai_msg["name"] = serde_json::json!(name);
    }

    // Add tool_call_id for tool messages
    if let BaseMessage::Tool(tool_msg) = message {
        oai_msg["tool_call_id"] = serde_json::json!(tool_msg.tool_call_id());
    }

    // Add tool_calls for AI messages
    if let BaseMessage::AI(ai_msg) = message
        && !ai_msg.tool_calls().is_empty()
    {
        let tool_calls: Vec<serde_json::Value> = ai_msg
            .tool_calls()
            .iter()
            .map(|tc| {
                serde_json::json!({
                    "type": "function",
                    "id": tc.id(),
                    "function": {
                        "name": tc.name(),
                        "arguments": serde_json::to_string(&tc.args()).unwrap_or_default(),
                    }
                })
            })
            .collect();
        oai_msg["tool_calls"] = serde_json::json!(tool_calls);
    }

    // Handle content based on text_format
    let content = message.content();
    match text_format {
        TextFormat::String => {
            oai_msg["content"] = serde_json::json!(content);
        }
        TextFormat::Block => {
            if content.is_empty() {
                oai_msg["content"] = serde_json::json!([]);
            } else {
                oai_msg["content"] = serde_json::json!([{ "type": "text", "text": content }]);
            }
        }
    }

    oai_msg
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
pub struct TrimMessagesConfig<F>
where
    F: Fn(&[BaseMessage]) -> usize,
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
pub fn trim_messages<F>(
    messages: &[BaseMessage],
    config: &TrimMessagesConfig<F>,
) -> Vec<BaseMessage>
where
    F: Fn(&[BaseMessage]) -> usize,
{
    if messages.is_empty() {
        return Vec::new();
    }

    match config.strategy {
        TrimStrategy::First => trim_messages_first(messages, config),
        TrimStrategy::Last => trim_messages_last(messages, config),
    }
}

/// Trim messages from the beginning.
fn trim_messages_first<F>(
    messages: &[BaseMessage],
    config: &TrimMessagesConfig<F>,
) -> Vec<BaseMessage>
where
    F: Fn(&[BaseMessage]) -> usize,
{
    let messages: Vec<BaseMessage> = messages.to_vec();

    // Check if all messages already fit
    if (config.token_counter)(&messages) <= config.max_tokens {
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

    let idx = left;

    // Handle partial messages if allowed
    if config.allow_partial && idx < messages.len() {
        // Try to include part of the next message's content
        if let Some(partial_msg) = try_partial_message_first(&messages, idx, config) {
            let mut result = messages[..idx].to_vec();
            result.push(partial_msg);
            return result;
        }
    }

    messages[..idx].to_vec()
}

/// Trim messages from the end.
fn trim_messages_last<F>(
    messages: &[BaseMessage],
    config: &TrimMessagesConfig<F>,
) -> Vec<BaseMessage>
where
    F: Fn(&[BaseMessage]) -> usize,
{
    let mut messages: Vec<BaseMessage> = messages.to_vec();

    if messages.is_empty() {
        return messages;
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

    // Reverse and use first strategy
    messages.reverse();

    // Create a temporary config with adjusted max_tokens
    let reverse_config = TrimMessagesConfig {
        max_tokens: remaining_tokens,
        token_counter: &config.token_counter,
        strategy: TrimStrategy::First,
        allow_partial: config.allow_partial,
        include_system: false,
    };

    let mut result = trim_messages_first(&messages, &reverse_config);

    // Reverse back
    result.reverse();

    // Add system message back
    if let Some(sys_msg) = system_message {
        result.insert(0, sys_msg);
    }

    result
}

/// Try to create a partial message from the first strategy.
fn try_partial_message_first<F>(
    messages: &[BaseMessage],
    idx: usize,
    config: &TrimMessagesConfig<F>,
) -> Option<BaseMessage>
where
    F: Fn(&[BaseMessage]) -> usize,
{
    if idx >= messages.len() {
        return None;
    }

    let excluded = &messages[idx];
    let content = excluded.content();

    if content.is_empty() {
        return None;
    }

    // Try to split on newlines (default text splitter)
    let splits: Vec<&str> = content.split('\n').collect();
    if splits.len() <= 1 {
        return None;
    }

    // Reassemble splits with the newline separator
    let splits_with_sep: Vec<String> = splits
        .iter()
        .enumerate()
        .map(|(i, s)| {
            if i < splits.len() - 1 {
                format!("{}\n", s)
            } else {
                s.to_string()
            }
        })
        .collect();

    let base_messages = &messages[..idx];

    // Binary search for max splits we can include
    let mut left = 0;
    let mut right = splits_with_sep.len();

    while left < right {
        let mid = (left + right + 1).div_ceil(2);
        let partial_content: String = splits_with_sep[..mid].concat();
        let partial_msg = create_message_with_content(excluded, &partial_content);

        let mut test_messages = base_messages.to_vec();
        test_messages.push(partial_msg);

        if (config.token_counter)(&test_messages) <= config.max_tokens {
            left = mid;
        } else {
            right = mid - 1;
        }
    }

    if left > 0 {
        let partial_content: String = splits_with_sep[..left].concat();
        Some(create_message_with_content(excluded, &partial_content))
    } else {
        None
    }
}

/// Create a message of the same type with different content.
fn create_message_with_content(original: &BaseMessage, content: &str) -> BaseMessage {
    match original {
        BaseMessage::Human(m) => {
            let mut new_msg = HumanMessage::new(content);
            if let Some(id) = m.id() {
                new_msg = HumanMessage::with_id(id, content);
            }
            BaseMessage::Human(new_msg)
        }
        BaseMessage::AI(m) => {
            let mut new_msg = AIMessage::new(content);
            if let Some(id) = m.id() {
                new_msg = AIMessage::with_id(id, content);
            }
            BaseMessage::AI(new_msg)
        }
        BaseMessage::System(m) => {
            let mut new_msg = SystemMessage::new(content);
            if let Some(id) = m.id() {
                new_msg = SystemMessage::with_id(id, content);
            }
            BaseMessage::System(new_msg)
        }
        BaseMessage::Tool(m) => {
            let mut new_msg = ToolMessage::new(content, m.tool_call_id());
            if let Some(id) = m.id() {
                new_msg = ToolMessage::with_id(id, content, m.tool_call_id());
            }
            BaseMessage::Tool(new_msg)
        }
        BaseMessage::Chat(m) => {
            let mut new_msg = ChatMessage::new(m.role(), content);
            if let Some(id) = m.id() {
                new_msg = ChatMessage::with_id(id, m.role(), content);
            }
            BaseMessage::Chat(new_msg)
        }
        BaseMessage::Function(m) => {
            let mut new_msg = FunctionMessage::new(m.name(), content);
            if let Some(id) = m.id() {
                new_msg = FunctionMessage::with_id(id, m.name(), content);
            }
            BaseMessage::Function(new_msg)
        }
        BaseMessage::Remove(m) => {
            // RemoveMessage preserves the same id (which is the target id to remove)
            BaseMessage::Remove(RemoveMessage::new(m.target_id()))
        }
    }
}
