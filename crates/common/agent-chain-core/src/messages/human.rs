//! Human message type.
//!
//! This module contains the `HumanMessage` and `HumanMessageChunk` types which represent
//! messages from the user. Mirrors `langchain_core.messages.human`.

use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;

use super::base::merge_content;
use super::content::{ContentBlock, ContentPart, ImageSource, MessageContent};

/// A human message in the conversation.
///
/// Human messages support both simple text content and multimodal content
/// with images. Use [`HumanMessage::new`] for simple text messages and
/// [`HumanMessage::with_content`] for multimodal messages.
///
/// This corresponds to `HumanMessage` in LangChain Python.

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct HumanMessage {
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

impl Serialize for HumanMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut field_count = 5; // type, content, id, additional_kwargs, response_metadata
        if self.name.is_some() {
            field_count += 1;
        }
        let mut map = serializer.serialize_map(Some(field_count))?;

        map.serialize_entry("type", "human")?;
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

impl HumanMessage {
    /// Create a new human message with simple text content.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: MessageContent::Text(content.into()),
            id: None,
            name: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Create a new human message with simple text content and an explicit ID.
    ///
    /// Use this when deserializing or reconstructing messages where the ID must be preserved.
    pub fn with_id(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            content: MessageContent::Text(content.into()),
            id: Some(id.into()),
            name: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Create a new human message with multipart content.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use agent_chain_core::messages::{HumanMessage, ContentPart, ImageSource};
    ///
    /// let msg = HumanMessage::with_content(vec![
    ///     ContentPart::Text { text: "What's in this image?".into() },
    ///     ContentPart::Image {
    ///         source: ImageSource::Url {
    ///             url: "https://example.com/image.jpg".into(),
    ///         },
    ///         detail: None,
    ///     },
    /// ]);
    /// ```
    pub fn with_content(parts: Vec<ContentPart>) -> Self {
        Self {
            content: MessageContent::Parts(parts),
            id: None,
            name: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Create a new human message with multipart content and an explicit ID.
    ///
    /// Use this when deserializing or reconstructing messages where the ID must be preserved.
    pub fn with_id_and_content(id: impl Into<String>, parts: Vec<ContentPart>) -> Self {
        Self {
            content: MessageContent::Parts(parts),
            id: Some(id.into()),
            name: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Create a human message with text and a single image from a URL.
    pub fn with_image_url(text: impl Into<String>, url: impl Into<String>) -> Self {
        Self::with_content(vec![
            ContentPart::Text { text: text.into() },
            ContentPart::Image {
                source: ImageSource::Url { url: url.into() },
                detail: None,
            },
        ])
    }

    /// Create a human message with text and a single base64-encoded image.
    pub fn with_image_base64(
        text: impl Into<String>,
        media_type: impl Into<String>,
        data: impl Into<String>,
    ) -> Self {
        Self::with_content(vec![
            ContentPart::Text { text: text.into() },
            ContentPart::Image {
                source: ImageSource::Base64 {
                    media_type: media_type.into(),
                    data: data.into(),
                },
                detail: None,
            },
        ])
    }

    /// Create a new human message with a list of content blocks.
    ///
    /// This is used for multimodal content or provider-specific content blocks.
    /// The content is stored as `MessageContent::Parts` derived from the JSON blocks.
    pub fn with_content_list(content: Vec<serde_json::Value>) -> Self {
        // Convert JSON values to ContentParts
        let parts: Vec<ContentPart> = content
            .into_iter()
            .map(|v| {
                if let Some(obj) = v.as_object() {
                    let block_type = obj.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    match block_type {
                        "text" => {
                            let text = obj.get("text").and_then(|t| t.as_str()).unwrap_or("");
                            ContentPart::Text {
                                text: text.to_string(),
                            }
                        }
                        "image" => {
                            // Handle various image source formats
                            if let Some(url) = obj.get("url").and_then(|u| u.as_str()) {
                                ContentPart::Image {
                                    source: ImageSource::Url {
                                        url: url.to_string(),
                                    },
                                    detail: None,
                                }
                            } else if let Some(base64) = obj.get("base64").and_then(|b| b.as_str())
                            {
                                let mime_type = obj
                                    .get("mime_type")
                                    .and_then(|m| m.as_str())
                                    .unwrap_or("image/png");
                                ContentPart::Image {
                                    source: ImageSource::Base64 {
                                        media_type: mime_type.to_string(),
                                        data: base64.to_string(),
                                    },
                                    detail: None,
                                }
                            } else if let Some(source) =
                                obj.get("source").and_then(|s| s.as_object())
                            {
                                let source_type =
                                    source.get("type").and_then(|t| t.as_str()).unwrap_or("");
                                match source_type {
                                    "url" => {
                                        let url = source
                                            .get("url")
                                            .and_then(|u| u.as_str())
                                            .unwrap_or("");
                                        ContentPart::Image {
                                            source: ImageSource::Url {
                                                url: url.to_string(),
                                            },
                                            detail: None,
                                        }
                                    }
                                    "base64" => {
                                        let data = source
                                            .get("data")
                                            .and_then(|d| d.as_str())
                                            .unwrap_or("");
                                        let media_type = source
                                            .get("media_type")
                                            .and_then(|m| m.as_str())
                                            .unwrap_or("image/png");
                                        ContentPart::Image {
                                            source: ImageSource::Base64 {
                                                media_type: media_type.to_string(),
                                                data: data.to_string(),
                                            },
                                            detail: None,
                                        }
                                    }
                                    "file" => {
                                        let file_id = source
                                            .get("file_id")
                                            .and_then(|f| f.as_str())
                                            .unwrap_or("");
                                        ContentPart::Image {
                                            source: ImageSource::FileId {
                                                file_id: file_id.to_string(),
                                            },
                                            detail: None,
                                        }
                                    }
                                    _ => ContentPart::Other(v.clone()),
                                }
                            } else if let Some(id) = obj.get("id").and_then(|i| i.as_str()) {
                                ContentPart::Image {
                                    source: ImageSource::FileId {
                                        file_id: id.to_string(),
                                    },
                                    detail: None,
                                }
                            } else {
                                ContentPart::Other(v.clone())
                            }
                        }
                        _ => ContentPart::Other(v.clone()),
                    }
                } else if let Some(s) = v.as_str() {
                    ContentPart::Text {
                        text: s.to_string(),
                    }
                } else {
                    ContentPart::Other(v.clone())
                }
            })
            .collect();

        Self {
            content: MessageContent::Parts(parts),
            id: None,
            name: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Set the name for this message.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the additional kwargs for this message (builder pattern).
    pub fn with_additional_kwargs(
        mut self,
        additional_kwargs: HashMap<String, serde_json::Value>,
    ) -> Self {
        self.additional_kwargs = additional_kwargs;
        self
    }

    /// Set the response metadata for this message (builder pattern).
    pub fn with_response_metadata(
        mut self,
        response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        self.response_metadata = response_metadata;
        self
    }

    /// Get response metadata.
    pub fn response_metadata(&self) -> &HashMap<String, serde_json::Value> {
        &self.response_metadata
    }

    /// Get the message type as a string.
    pub fn message_type(&self) -> &'static str {
        "human"
    }

    /// Get the text content of the message.
    ///
    /// This is the same as `content()` for simple text messages.
    pub fn text(&self) -> &str {
        self.content()
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

    /// Check if this message contains images.
    pub fn has_images(&self) -> bool {
        self.content.has_images()
    }

    /// Get the message ID.
    pub fn id(&self) -> Option<String> {
        self.id.clone()
    }

    /// Set the message ID.
    pub fn set_id(&mut self, id: String) {
        self.id = Some(id);
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
    /// standardized LangChain content block format. For HumanMessage,
    /// this uses the Anthropic input translator since human messages
    /// often contain Anthropic-specific document/image formats.
    ///
    /// This corresponds to `content_blocks` property in LangChain Python.
    pub fn content_blocks(&self) -> Vec<ContentBlock> {
        use super::content::{
            AudioContentBlock, FileContentBlock, ImageContentBlock, InvalidToolCallBlock,
            NonStandardContentBlock, PlainTextContentBlock, ReasoningContentBlock, ServerToolCall,
            ServerToolCallChunk, ServerToolResult, TextContentBlock, ToolCallBlock,
            ToolCallChunkBlock, VideoContentBlock,
        };
        use crate::messages::block_translators::anthropic::convert_input_to_standard_blocks as anthropic_convert;
        use crate::messages::block_translators::openai::convert_to_v1_from_chat_completions_input;

        let raw_content = self.content_list();

        // Try to detect if this is OpenAI Chat Completions format
        // Check if any blocks are image_url, input_audio, or file type
        let is_openai_format = raw_content.iter().any(|block| {
            block
                .get("type")
                .and_then(|t| t.as_str())
                .map(|t| ["image_url", "input_audio", "file"].contains(&t))
                .unwrap_or(false)
        });

        let blocks_json = if is_openai_format {
            convert_to_v1_from_chat_completions_input(&raw_content)
        } else {
            anthropic_convert(&raw_content)
        };

        // Deserialize JSON blocks into ContentBlock structs
        // We can't use direct serde deserialization because the enum has #[serde(tag = "type")]
        // which expects externally tagged format, but our JSON has type as a field inside.
        // So we need to manually deserialize based on the type field.
        //
        // On deserialization failure, we log a warning and wrap the malformed block
        // as NonStandardContentBlock with error info, rather than panicking.
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
}

/// Human message chunk (yielded when streaming).
///
/// This corresponds to `HumanMessageChunk` in LangChain Python.

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct HumanMessageChunk {
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

impl Serialize for HumanMessageChunk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut field_count = 5; // type, content, id, additional_kwargs, response_metadata
        if self.name.is_some() {
            field_count += 1;
        }
        let mut map = serializer.serialize_map(Some(field_count))?;

        map.serialize_entry("type", "HumanMessageChunk")?;
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

impl HumanMessageChunk {
    /// Create a new human message chunk with text content.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: MessageContent::Text(content.into()),
            id: None,
            name: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Create a new human message chunk with an ID.
    pub fn with_id(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            content: MessageContent::Text(content.into()),
            id: Some(id.into()),
            name: None,
            additional_kwargs: HashMap::new(),
            response_metadata: HashMap::new(),
        }
    }

    /// Set the name for this chunk (builder pattern).
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the additional kwargs for this chunk (builder pattern).
    pub fn with_additional_kwargs(
        mut self,
        additional_kwargs: HashMap<String, serde_json::Value>,
    ) -> Self {
        self.additional_kwargs = additional_kwargs;
        self
    }

    /// Set the response metadata for this chunk (builder pattern).
    pub fn with_response_metadata(
        mut self,
        response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        self.response_metadata = response_metadata;
        self
    }

    /// Get the message type as a string.
    pub fn message_type(&self) -> &'static str {
        "HumanMessageChunk"
    }

    /// Get the text content of the chunk.
    pub fn text(&self) -> &str {
        self.content()
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
    pub fn concat(&self, other: &HumanMessageChunk) -> HumanMessageChunk {
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

        HumanMessageChunk {
            content,
            id: self.id.clone().or_else(|| other.id.clone()),
            name: self.name.clone().or_else(|| other.name.clone()),
            additional_kwargs,
            response_metadata,
        }
    }

    /// Convert this chunk to a complete HumanMessage.
    pub fn to_message(&self) -> HumanMessage {
        HumanMessage {
            content: self.content.clone(),
            id: self.id.clone(),
            name: self.name.clone(),
            additional_kwargs: self.additional_kwargs.clone(),
            response_metadata: self.response_metadata.clone(),
        }
    }
}

impl std::ops::Add for HumanMessageChunk {
    type Output = HumanMessageChunk;

    fn add(self, other: HumanMessageChunk) -> HumanMessageChunk {
        self.concat(&other)
    }
}

impl std::iter::Sum for HumanMessageChunk {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|a, b| a + b)
            .unwrap_or_else(|| HumanMessageChunk::new(""))
    }
}

impl From<HumanMessageChunk> for HumanMessage {
    fn from(chunk: HumanMessageChunk) -> Self {
        chunk.to_message()
    }
}

impl super::base::BaseMessageTrait for HumanMessage {
    fn content(&self) -> &str {
        HumanMessage::content(self)
    }

    fn id(&self) -> Option<String> {
        HumanMessage::id(self)
    }

    fn name(&self) -> Option<String> {
        HumanMessage::name(self)
    }

    fn set_id(&mut self, id: String) {
        HumanMessage::set_id(self, id)
    }

    fn additional_kwargs(&self) -> Option<&HashMap<String, serde_json::Value>> {
        Some(HumanMessage::additional_kwargs(self))
    }
}
