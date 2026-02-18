//! Chat message type.
//!
//! This module contains the `ChatMessage` and `ChatMessageChunk` types which represent
//! messages with an arbitrary speaker role. Mirrors `langchain_core.messages.chat`.

use bon::bon;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;

use super::base::{get_msg_title_repr, is_interactive_env, merge_content};
use super::content::{ContentBlock, ContentPart, KNOWN_BLOCK_TYPES, MessageContent};
use super::human::HumanMessageChunk;
use crate::load::Serializable;
use crate::utils::merge::{merge_dicts, merge_lists};

/// A chat message that can be assigned an arbitrary speaker (role).
///
/// Use this when you need to specify a custom role that isn't covered
/// by the standard message types (Human, AI, System, Tool).
///
/// # Example
///
/// ```
/// use agent_chain_core::messages::ChatMessage;
///
/// // Simple message with content and role
/// let msg = ChatMessage::builder()
///     .content("Hello!")
///     .role("assistant")
///     .build();
///
/// // Message with ID and name
/// let msg = ChatMessage::builder()
///     .content("Hello!")
///     .role("assistant")
///     .maybe_id(Some("msg-123".to_string()))
///     .maybe_name(Some("bot".to_string()))
///     .build();
/// ```
///
/// This corresponds to `ChatMessage` in LangChain Python.

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ChatMessage {
    /// The message content (text or multipart)
    pub content: MessageContent,
    /// The speaker / role of the message
    pub role: String,
    /// Optional unique identifier
    pub id: Option<String>,
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

impl Serialize for ChatMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut field_count = 5;
        if self.name.is_some() {
            field_count += 1;
        }
        field_count += 1;

        let mut map = serializer.serialize_map(Some(field_count))?;
        map.serialize_entry("type", "chat")?;
        map.serialize_entry("content", &self.content)?;
        map.serialize_entry("role", &self.role)?;
        map.serialize_entry("id", &self.id)?;
        if let Some(ref name) = self.name {
            map.serialize_entry("name", name)?;
        }

        let additional_kwargs_with_type = self.additional_kwargs.clone();
        map.serialize_entry("additional_kwargs", &additional_kwargs_with_type)?;

        map.serialize_entry("response_metadata", &self.response_metadata)?;

        map.end()
    }
}

#[bon]
impl ChatMessage {
    /// Create a new chat message with named parameters using the builder pattern.
    ///
    /// # Example
    ///
    /// ```
    /// use agent_chain_core::messages::ChatMessage;
    ///
    /// // Simple message with content and role
    /// let msg = ChatMessage::builder()
    ///     .content("Hello!")
    ///     .role("assistant")
    ///     .build();
    ///
    /// // Message with ID and name
    /// let msg = ChatMessage::builder()
    ///     .content("Hello!")
    ///     .role("assistant")
    ///     .maybe_id(Some("msg-123".to_string()))
    ///     .maybe_name(Some("bot".to_string()))
    ///     .build();
    /// ```
    #[builder]
    pub fn new(
        content: impl Into<MessageContent>,
        /// Optional typed standard content blocks. When provided, these are
        /// serialized and used as the message content instead of `content`.
        /// Corresponds to the `content_blocks` parameter in Python's
        /// `ChatMessage.__init__`.
        content_blocks: Option<Vec<ContentBlock>>,
        role: impl Into<String>,
        id: Option<String>,
        name: Option<String>,
        #[builder(default)] additional_kwargs: HashMap<String, serde_json::Value>,
        #[builder(default)] response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        let resolved_content = if let Some(blocks) = content_blocks {
            let parts: Vec<ContentPart> = blocks
                .into_iter()
                .filter_map(|block| serde_json::to_value(&block).ok().map(ContentPart::Other))
                .collect();
            MessageContent::Parts(parts)
        } else {
            content.into()
        };

        Self {
            content: resolved_content,
            role: role.into(),
            id,
            name,
            additional_kwargs,
            response_metadata,
        }
    }

    /// Set the message ID.
    pub fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }

    /// Get the message type as a string.
    pub fn message_type(&self) -> &'static str {
        "chat"
    }

    /// Get the text content of the message as a string.
    ///
    /// This extracts text from both simple string content and list content
    /// (filtering for text blocks). Corresponds to the `text` property
    /// on `BaseMessage` in LangChain Python.
    pub fn text(&self) -> String {
        self.content.as_text()
    }

    /// Get a pretty representation of the message.
    ///
    /// Corresponds to `BaseMessage.pretty_repr` in LangChain Python.
    pub fn pretty_repr(&self, html: bool) -> String {
        let title = get_msg_title_repr("Chat Message", html);
        let name_line = if let Some(name) = &self.name {
            format!("\nName: {}", name)
        } else {
            String::new()
        };
        format!("{}{}\n\n{}", title, name_line, self.content.as_text_ref())
    }

    /// Pretty print the message to stdout.
    ///
    /// Corresponds to `BaseMessage.pretty_print` in LangChain Python.
    pub fn pretty_print(&self) {
        println!("{}", self.pretty_repr(is_interactive_env()));
    }

    /// Get the content blocks translated to the standard format.
    ///
    /// Translates provider-specific content blocks to the standardized
    /// LangChain content block format. Corresponds to the `content_blocks`
    /// property on `BaseMessage` in LangChain Python.
    pub fn content_blocks(&self) -> Vec<ContentBlock> {
        use super::content::{
            AudioContentBlock, FileContentBlock, ImageContentBlock, InvalidToolCallBlock,
            NonStandardContentBlock, PlainTextContentBlock, ReasoningContentBlock, ServerToolCall,
            ServerToolCallChunk, ServerToolResult, TextContentBlock, ToolCallBlock,
            ToolCallChunkBlock, VideoContentBlock,
        };
        use crate::messages::block_translators::anthropic::convert_input_to_standard_blocks as anthropic_convert;
        use crate::messages::block_translators::openai::convert_to_v1_from_chat_completions_input;

        let mut blocks: Vec<serde_json::Value> = Vec::new();

        let items: Vec<serde_json::Value> = match &self.content {
            MessageContent::Text(s) => {
                if s.is_empty() {
                    vec![]
                } else {
                    vec![serde_json::Value::String(s.clone())]
                }
            }
            MessageContent::Parts(parts) => parts
                .iter()
                .filter_map(|p| serde_json::to_value(p).ok())
                .collect(),
        };

        for item in items {
            if let Some(s) = item.as_str() {
                blocks.push(serde_json::json!({"type": "text", "text": s}));
            } else if item.is_object() {
                let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");

                if !KNOWN_BLOCK_TYPES.contains(&item_type) || item.get("source_type").is_some() {
                    blocks.push(serde_json::json!({"type": "non_standard", "value": item}));
                } else {
                    blocks.push(item);
                }
            }
        }

        blocks = convert_to_v1_from_chat_completions_input(&blocks);
        blocks = anthropic_convert(&blocks);

        blocks
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
                        tracing::warn!(
                            block_type = %block_type,
                            json = %v,
                            "Unknown block type in content_blocks, treating as non_standard"
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
                        "Failed to deserialize ContentBlock in content_blocks, wrapping as non_standard"
                    );
                    let mut error_value = std::collections::HashMap::new();
                    error_value.insert("original_json".to_string(), v.clone());
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
                        index: v.get("index").and_then(|i| serde_json::from_value(i.clone()).ok()),
                    })
                })
            })
            .collect()
    }
}

/// Chat message chunk (yielded when streaming).
///
/// # Example
///
/// ```
/// use agent_chain_core::messages::ChatMessageChunk;
///
/// // Simple chunk with content and role
/// let chunk = ChatMessageChunk::builder()
///     .content("Hello")
///     .role("assistant")
///     .build();
///
/// // Chunk with ID
/// let chunk = ChatMessageChunk::builder()
///     .content("Hello")
///     .role("assistant")
///     .maybe_id(Some("chunk-123".to_string()))
///     .build();
/// ```
///
/// This corresponds to `ChatMessageChunk` in LangChain Python.

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ChatMessageChunk {
    /// The message content (may be partial during streaming)
    pub content: MessageContent,
    /// The speaker / role of the message
    pub role: String,
    /// Optional unique identifier
    pub id: Option<String>,
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

impl Serialize for ChatMessageChunk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut field_count = 5;
        if self.name.is_some() {
            field_count += 1;
        }
        field_count += 1;

        let mut map = serializer.serialize_map(Some(field_count))?;
        map.serialize_entry("type", "ChatMessageChunk")?;
        map.serialize_entry("content", &self.content)?;
        map.serialize_entry("role", &self.role)?;
        map.serialize_entry("id", &self.id)?;
        if let Some(ref name) = self.name {
            map.serialize_entry("name", name)?;
        }

        let additional_kwargs_with_type = self.additional_kwargs.clone();
        map.serialize_entry("additional_kwargs", &additional_kwargs_with_type)?;

        map.serialize_entry("response_metadata", &self.response_metadata)?;

        map.end()
    }
}

#[bon]
impl ChatMessageChunk {
    /// Create a new chat message chunk with named parameters using the builder pattern.
    ///
    /// # Example
    ///
    /// ```
    /// use agent_chain_core::messages::ChatMessageChunk;
    ///
    /// // Simple chunk with content and role
    /// let chunk = ChatMessageChunk::builder()
    ///     .content("Hello")
    ///     .role("assistant")
    ///     .build();
    ///
    /// // Chunk with ID
    /// let chunk = ChatMessageChunk::builder()
    ///     .content("Hello")
    ///     .role("assistant")
    ///     .maybe_id(Some("chunk-123".to_string()))
    ///     .build();
    /// ```
    #[builder]
    pub fn new(
        content: impl Into<MessageContent>,
        role: impl Into<String>,
        id: Option<String>,
        name: Option<String>,
        #[builder(default)] additional_kwargs: HashMap<String, serde_json::Value>,
        #[builder(default)] response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            content: content.into(),
            role: role.into(),
            id,
            name,
            additional_kwargs,
            response_metadata,
        }
    }

    /// Get the message type as a string.
    pub fn message_type(&self) -> &'static str {
        "ChatMessageChunk"
    }

    /// Get the text content of the chunk as a string.
    ///
    /// Corresponds to the `text` property on `BaseMessage` in LangChain Python.
    pub fn text(&self) -> String {
        self.content.as_text()
    }

    /// Concatenate this chunk with another chunk.
    ///
    /// Handles both simple text and multipart content merging,
    /// matching the behavior of `ChatMessageChunk.__add__` in Python.
    ///
    /// # Panics
    ///
    /// Panics if the roles are different.
    pub fn concat(&self, other: &ChatMessageChunk) -> ChatMessageChunk {
        if self.role != other.role {
            panic!("Cannot concatenate ChatMessageChunks with different roles");
        }

        let content = match (&self.content, &other.content) {
            (MessageContent::Text(a), MessageContent::Text(b)) => {
                MessageContent::Text(merge_content(a, b))
            }
            (MessageContent::Parts(a), MessageContent::Parts(b)) => {
                let left: Vec<serde_json::Value> = a
                    .iter()
                    .filter_map(|p| serde_json::to_value(p).ok())
                    .collect();
                let right: Vec<serde_json::Value> = b
                    .iter()
                    .filter_map(|p| serde_json::to_value(p).ok())
                    .collect();
                match merge_lists(Some(left.clone()), vec![Some(right.clone())]) {
                    Ok(Some(merged)) => {
                        let parts: Vec<ContentPart> = merged
                            .into_iter()
                            .filter_map(|v| serde_json::from_value(v).ok())
                            .collect();
                        MessageContent::Parts(parts)
                    }
                    _ => {
                        let mut parts = a.clone();
                        parts.extend(b.clone());
                        MessageContent::Parts(parts)
                    }
                }
            }
            (MessageContent::Text(a), MessageContent::Parts(b)) => {
                let mut parts = vec![ContentPart::Text { text: a.clone() }];
                parts.extend(b.clone());
                MessageContent::Parts(parts)
            }
            (MessageContent::Parts(a), MessageContent::Text(b)) => {
                let mut parts = a.clone();
                parts.push(ContentPart::Text { text: b.clone() });
                MessageContent::Parts(parts)
            }
        };

        let additional_kwargs = {
            let left_val = serde_json::to_value(&self.additional_kwargs).unwrap_or_default();
            let right_val = serde_json::to_value(&other.additional_kwargs).unwrap_or_default();
            match merge_dicts(left_val, vec![right_val]) {
                Ok(merged) => serde_json::from_value(merged).unwrap_or_default(),
                Err(_) => self.additional_kwargs.clone(),
            }
        };

        let response_metadata = {
            let left_val = serde_json::to_value(&self.response_metadata).unwrap_or_default();
            let right_val = serde_json::to_value(&other.response_metadata).unwrap_or_default();
            match merge_dicts(left_val, vec![right_val]) {
                Ok(merged) => serde_json::from_value(merged).unwrap_or_default(),
                Err(_) => self.response_metadata.clone(),
            }
        };

        ChatMessageChunk {
            content,
            role: self.role.clone(),
            id: self.id.clone(),
            name: self.name.clone(),
            additional_kwargs,
            response_metadata,
        }
    }

    /// Get the content blocks translated to the standard format.
    ///
    /// Corresponds to the `content_blocks` property on `BaseMessage`
    /// in LangChain Python.
    pub fn content_blocks(&self) -> Vec<ContentBlock> {
        use super::content::{
            AudioContentBlock, FileContentBlock, ImageContentBlock, InvalidToolCallBlock,
            NonStandardContentBlock, PlainTextContentBlock, ReasoningContentBlock, ServerToolCall,
            ServerToolCallChunk, ServerToolResult, TextContentBlock, ToolCallBlock,
            ToolCallChunkBlock, VideoContentBlock,
        };
        use crate::messages::block_translators::anthropic::convert_input_to_standard_blocks as anthropic_convert;
        use crate::messages::block_translators::openai::convert_to_v1_from_chat_completions_input;

        let mut blocks: Vec<serde_json::Value> = Vec::new();

        let items: Vec<serde_json::Value> = match &self.content {
            MessageContent::Text(s) => {
                if s.is_empty() {
                    vec![]
                } else {
                    vec![serde_json::Value::String(s.clone())]
                }
            }
            MessageContent::Parts(parts) => parts
                .iter()
                .filter_map(|p| serde_json::to_value(p).ok())
                .collect(),
        };

        for item in items {
            if let Some(s) = item.as_str() {
                blocks.push(serde_json::json!({"type": "text", "text": s}));
            } else if item.is_object() {
                let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");

                if !KNOWN_BLOCK_TYPES.contains(&item_type) || item.get("source_type").is_some() {
                    blocks.push(serde_json::json!({"type": "non_standard", "value": item}));
                } else {
                    blocks.push(item);
                }
            }
        }

        blocks = convert_to_v1_from_chat_completions_input(&blocks);
        blocks = anthropic_convert(&blocks);

        blocks
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
                        tracing::warn!(
                            block_type = %block_type,
                            json = %v,
                            "Unknown block type in content_blocks, treating as non_standard"
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
                        "Failed to deserialize ContentBlock in content_blocks, wrapping as non_standard"
                    );
                    let mut error_value = std::collections::HashMap::new();
                    error_value.insert("original_json".to_string(), v.clone());
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
                        index: v
                            .get("index")
                            .and_then(|i| serde_json::from_value(i.clone()).ok()),
                    })
                })
            })
            .collect()
    }

    /// Convert this chunk to a complete ChatMessage.
    pub fn to_message(&self) -> ChatMessage {
        ChatMessage {
            content: self.content.clone(),
            role: self.role.clone(),
            id: self.id.clone(),
            name: self.name.clone(),
            additional_kwargs: self.additional_kwargs.clone(),
            response_metadata: self.response_metadata.clone(),
        }
    }
}

impl std::ops::Add for ChatMessageChunk {
    type Output = ChatMessageChunk;

    fn add(self, other: ChatMessageChunk) -> ChatMessageChunk {
        self.concat(&other)
    }
}

/// Adding a `HumanMessageChunk` to a `ChatMessageChunk` produces a
/// `ChatMessageChunk`, matching the Python behavior where
/// `ChatMessageChunk.__add__` accepts any `BaseMessageChunk`.
impl std::ops::Add<HumanMessageChunk> for ChatMessageChunk {
    type Output = ChatMessageChunk;

    fn add(self, other: HumanMessageChunk) -> ChatMessageChunk {
        let other_as_chat = ChatMessageChunk {
            content: other.content,
            role: self.role.clone(),
            id: other.id,
            name: other.name,
            additional_kwargs: other.additional_kwargs,
            response_metadata: other.response_metadata,
        };
        self.concat(&other_as_chat)
    }
}

impl Serializable for ChatMessage {
    fn is_lc_serializable() -> bool {
        true
    }

    fn get_lc_namespace() -> Vec<String> {
        vec![
            "langchain".to_string(),
            "schema".to_string(),
            "messages".to_string(),
        ]
    }
}

impl Serializable for ChatMessageChunk {
    fn is_lc_serializable() -> bool {
        true
    }

    fn get_lc_namespace() -> Vec<String> {
        vec![
            "langchain".to_string(),
            "schema".to_string(),
            "messages".to_string(),
        ]
    }
}
