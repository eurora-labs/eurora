//! System message type.
//!
//! This module contains the `SystemMessage` and `SystemMessageChunk` types which represent
//! system instructions for priming AI behavior. Mirrors `langchain_core.messages.system`.

use bon::bon;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;

use super::base::merge_content;
use super::content::{ContentBlock, ContentPart, MessageContent};

/// A system message in the conversation.
///
/// The system message is usually passed in as the first of a sequence
/// of input messages. It's used to prime AI behavior with instructions.
///
/// # Example
///
/// ```
/// use agent_chain_core::messages::SystemMessage;
///
/// // Simple text message
/// let msg = SystemMessage::builder()
///     .content("You are a helpful assistant.")
///     .build();
///
/// // Message with ID and name
/// let msg = SystemMessage::builder()
///     .content("You are a helpful assistant.")
///     .maybe_id(Some("msg-123".to_string()))
///     .maybe_name(Some("system".to_string()))
///     .build();
/// ```
///
/// This corresponds to `SystemMessage` in LangChain Python.

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct SystemMessage {
    /// The message content (text or multipart)
    pub content: MessageContent,
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

impl Serialize for SystemMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut field_count = 4;
        if self.name.is_some() {
            field_count += 1;
        }
        // Add 1 for additional type field
        field_count += 1;

        let mut map = serializer.serialize_map(Some(field_count))?;
        map.serialize_entry("type", "system")?;
        map.serialize_entry("content", &self.content)?;
        map.serialize_entry("id", &self.id)?;
        if let Some(ref name) = self.name {
            map.serialize_entry("name", name)?;
        }
        map.serialize_entry("additional_kwargs", &self.additional_kwargs)?;
        map.serialize_entry("response_metadata", &self.response_metadata)?;

        map.end()
    }
}

#[bon]
impl SystemMessage {
    /// Create a new system message with named parameters using the builder pattern.
    ///
    /// # Example
    ///
    /// ```
    /// use agent_chain_core::messages::{SystemMessage, MessageContent};
    ///
    /// // Simple message with just content
    /// let msg = SystemMessage::builder()
    ///     .content("You are a helpful assistant.")
    ///     .build();
    ///
    /// // Message with ID and name
    /// let msg = SystemMessage::builder()
    ///     .content("You are a helpful assistant.")
    ///     .maybe_id(Some("msg-123".to_string()))
    ///     .maybe_name(Some("system".to_string()))
    ///     .build();
    /// ```
    #[builder]
    pub fn new(
        content: impl Into<MessageContent>,
        id: Option<String>,
        name: Option<String>,
        #[builder(default)] additional_kwargs: HashMap<String, serde_json::Value>,
        #[builder(default)] response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            content: content.into(),
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
        "system"
    }

    /// Get the raw content as a list of JSON values.
    ///
    /// If the content is a Parts list, it serializes each part to JSON.
    /// If the content is a string, it returns a single text block.
    pub fn content_list(&self) -> Vec<serde_json::Value> {
        match &self.content {
            MessageContent::Text(s) => {
                vec![serde_json::json!({"type": "text", "text": s})]
            }
            MessageContent::Parts(parts) => parts
                .iter()
                .map(|p| serde_json::to_value(p).unwrap_or(serde_json::Value::Null))
                .collect(),
        }
    }

    /// Get the content blocks translated to the standard format.
    ///
    /// This method translates provider-specific content blocks to the
    /// standardized LangChain content block format.
    ///
    /// This corresponds to `content_blocks` property in LangChain Python.
    pub fn content_blocks(&self) -> Vec<ContentBlock> {
        use super::content::{
            AudioContentBlock, FileContentBlock, ImageContentBlock, InvalidToolCallBlock,
            NonStandardContentBlock, PlainTextContentBlock, ReasoningContentBlock,
            TextContentBlock, ToolCallBlock, ToolCallChunkBlock, VideoContentBlock,
        };

        let raw_content = self.content_list();

        // Deserialize JSON blocks into ContentBlock structs
        raw_content
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
                    "non_standard" => serde_json::from_value::<NonStandardContentBlock>(v.clone())
                        .map(ContentBlock::NonStandard),
                    _ => {
                        // Unknown type, wrap as non_standard
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
                    // Wrap the malformed block as NonStandardContentBlock with error info
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
}

/// System message chunk (yielded when streaming).
///
/// This corresponds to `SystemMessageChunk` in LangChain Python.

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct SystemMessageChunk {
    /// The message content (may be partial during streaming)
    pub content: MessageContent,
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

impl Serialize for SystemMessageChunk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut field_count = 4;
        if self.name.is_some() {
            field_count += 1;
        }
        // Add 1 for additional type field
        field_count += 1;

        let mut map = serializer.serialize_map(Some(field_count))?;
        map.serialize_entry("type", "SystemMessageChunk")?;
        map.serialize_entry("content", &self.content)?;
        map.serialize_entry("id", &self.id)?;
        if let Some(ref name) = self.name {
            map.serialize_entry("name", name)?;
        }
        map.serialize_entry("additional_kwargs", &self.additional_kwargs)?;
        map.serialize_entry("response_metadata", &self.response_metadata)?;

        map.end()
    }
}

#[bon]
impl SystemMessageChunk {
    /// Create a new system message chunk with named parameters using the builder pattern.
    ///
    /// # Example
    ///
    /// ```
    /// use agent_chain_core::messages::SystemMessageChunk;
    ///
    /// // Simple chunk with just content
    /// let chunk = SystemMessageChunk::builder()
    ///     .content("You are")
    ///     .build();
    ///
    /// // Chunk with ID
    /// let chunk = SystemMessageChunk::builder()
    ///     .content("You are")
    ///     .maybe_id(Some("chunk-123".to_string()))
    ///     .build();
    /// ```
    #[builder]
    pub fn new(
        content: impl Into<MessageContent>,
        id: Option<String>,
        name: Option<String>,
        #[builder(default)] additional_kwargs: HashMap<String, serde_json::Value>,
        #[builder(default)] response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            content: content.into(),
            id,
            name,
            additional_kwargs,
            response_metadata,
        }
    }

    /// Get the message type as a string.
    pub fn message_type(&self) -> &'static str {
        "SystemMessageChunk"
    }

    /// Concatenate this chunk with another chunk.
    pub fn concat(&self, other: &SystemMessageChunk) -> SystemMessageChunk {
        let content = match (&self.content, &other.content) {
            (MessageContent::Text(a), MessageContent::Text(b)) => {
                MessageContent::Text(merge_content(a, b))
            }
            (MessageContent::Parts(a), MessageContent::Parts(b)) => {
                let mut parts = a.clone();
                parts.extend(b.clone());
                MessageContent::Parts(parts)
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

        SystemMessageChunk {
            content,
            id: self.id.clone().or_else(|| other.id.clone()),
            name: self.name.clone().or_else(|| other.name.clone()),
            additional_kwargs,
            response_metadata,
        }
    }

    /// Convert this chunk to a complete SystemMessage.
    pub fn to_message(&self) -> SystemMessage {
        SystemMessage {
            content: self.content.clone(),
            id: self.id.clone(),
            name: self.name.clone(),
            additional_kwargs: self.additional_kwargs.clone(),
            response_metadata: self.response_metadata.clone(),
        }
    }
}

impl std::ops::Add for SystemMessageChunk {
    type Output = SystemMessageChunk;

    fn add(self, other: SystemMessageChunk) -> SystemMessageChunk {
        self.concat(&other)
    }
}

impl std::iter::Sum for SystemMessageChunk {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|a, b| a + b)
            .unwrap_or_else(|| SystemMessageChunk::builder().content("").build())
    }
}

impl From<SystemMessageChunk> for SystemMessage {
    fn from(chunk: SystemMessageChunk) -> Self {
        chunk.to_message()
    }
}
