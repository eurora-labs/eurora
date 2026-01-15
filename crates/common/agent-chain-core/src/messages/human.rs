//! Human message type.
//!
//! This module contains the `HumanMessage` and `HumanMessageChunk` types which represent
//! messages from the user. Mirrors `langchain_core.messages.human`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "specta")]
use specta::Type;

use super::base::merge_content;
use super::content::{ContentPart, ImageSource, MessageContent};

/// A human message in the conversation.
///
/// Human messages support both simple text content and multimodal content
/// with images. Use [`HumanMessage::new`] for simple text messages and
/// [`HumanMessage::with_content`] for multimodal messages.
///
/// This corresponds to `HumanMessage` in LangChain Python.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
}

impl HumanMessage {
    /// Create a new human message with simple text content.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: MessageContent::Text(content.into()),
            id: None,
            name: None,
            additional_kwargs: HashMap::new(),
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

    /// Check if this message contains images.
    pub fn has_images(&self) -> bool {
        self.content.has_images()
    }

    /// Get the message ID.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Get the message name.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get additional kwargs.
    pub fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value> {
        &self.additional_kwargs
    }
}

/// Human message chunk (yielded when streaming).
///
/// This corresponds to `HumanMessageChunk` in LangChain Python.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Get the message name.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
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
        }
    }
}

impl std::ops::Add for HumanMessageChunk {
    type Output = HumanMessageChunk;

    fn add(self, other: HumanMessageChunk) -> HumanMessageChunk {
        self.concat(&other)
    }
}
