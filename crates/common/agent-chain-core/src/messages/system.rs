//! System message type.
//!
//! This module contains the `SystemMessage` and `SystemMessageChunk` types which represent
//! system instructions for priming AI behavior. Mirrors `langchain_core.messages.system`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::base::merge_content;
use super::content::{ContentBlock, ContentPart, MessageContent};

/// A system message in the conversation.
///
/// The system message is usually passed in as the first of a sequence
/// of input messages. It's used to prime AI behavior with instructions.
///
/// This corresponds to `SystemMessage` in LangChain Python.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
}

impl SystemMessage {
    /// Create a new system message with simple text content.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: MessageContent::Text(content.into()),
            id: None,
            name: None,
            additional_kwargs: HashMap::new(),
        }
    }

    /// Set the message ID.
    pub fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }

    /// Create a new system message with simple text content and an explicit ID.
    ///
    /// Use this when deserializing or reconstructing messages where the ID must be preserved.
    pub fn with_id(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            content: MessageContent::Text(content.into()),
            id: Some(id.into()),
            name: None,
            additional_kwargs: HashMap::new(),
        }
    }

    /// Create a new system message with multipart content.
    pub fn with_content(parts: Vec<ContentPart>) -> Self {
        Self {
            content: MessageContent::Parts(parts),
            id: None,
            name: None,
            additional_kwargs: HashMap::new(),
        }
    }

    /// Create a new system message with multipart content and an explicit ID.
    ///
    /// Use this when deserializing or reconstructing messages where the ID must be preserved.
    pub fn with_id_and_content(id: impl Into<String>, parts: Vec<ContentPart>) -> Self {
        Self {
            content: MessageContent::Parts(parts),
            id: Some(id.into()),
            name: None,
            additional_kwargs: HashMap::new(),
        }
    }

    /// Set the name for this message.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Get the message content as text.
    ///
    /// For multipart messages, this returns an empty string.
    /// Use [`message_content()`](Self::message_content) to access the full content.
    pub fn content(&self) -> &str {
        match &self.content {
            MessageContent::Text(s) => s,
            MessageContent::Parts(_) => "",
        }
    }

    /// Get the full message content (text or multipart).
    pub fn message_content(&self) -> &MessageContent {
        &self.content
    }

    /// Get the message ID.
    pub fn id(&self) -> Option<String> {
        self.id.clone()
    }

    /// Get the message name.
    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }

    /// Get additional kwargs.
    pub fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value> {
        &self.additional_kwargs
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemMessageChunk {
    /// The message content (may be partial during streaming)
    content: MessageContent,
    /// Optional unique identifier
    id: Option<String>,
    /// Optional name for the message
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// Additional metadata
    #[serde(default)]
    additional_kwargs: HashMap<String, serde_json::Value>,
    /// Response metadata
    #[serde(default)]
    response_metadata: HashMap<String, serde_json::Value>,
}

impl SystemMessageChunk {
    /// Create a new system message chunk with text content.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: MessageContent::Text(content.into()),
            id: None,
            name: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Create a new system message chunk with an ID.
    pub fn with_id(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            content: MessageContent::Text(content.into()),
            id: Some(id.into()),
            name: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Get the message content as text.
    pub fn content(&self) -> &str {
        match &self.content {
            MessageContent::Text(s) => s,
            MessageContent::Parts(_) => "",
        }
    }

    /// Get the full message content.
    pub fn message_content(&self) -> &MessageContent {
        &self.content
    }

    /// Get the message ID.
    pub fn id(&self) -> Option<String> {
        self.id.clone()
    }

    /// Get the message name.
    pub fn name(&self) -> Option<String> {
        self.name.clone()
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
        }
    }
}

impl std::ops::Add for SystemMessageChunk {
    type Output = SystemMessageChunk;

    fn add(self, other: SystemMessageChunk) -> SystemMessageChunk {
        self.concat(&other)
    }
}

impl super::base::BaseMessageTrait for SystemMessage {
    fn content(&self) -> &str {
        SystemMessage::content(self)
    }

    fn id(&self) -> Option<String> {
        SystemMessage::id(self)
    }

    fn name(&self) -> Option<String> {
        SystemMessage::name(self)
    }

    fn set_id(&mut self, id: String) {
        SystemMessage::set_id(self, id)
    }

    fn additional_kwargs(&self) -> Option<&std::collections::HashMap<String, serde_json::Value>> {
        Some(SystemMessage::additional_kwargs(self))
    }
}
