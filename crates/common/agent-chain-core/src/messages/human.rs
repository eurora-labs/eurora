//! Human message type.
//!
//! This module contains the `HumanMessage` type which represents
//! messages from the user. Mirrors `langchain_core.messages.human`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(feature = "specta")]
use specta::Type;

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
    content: MessageContent,
    /// Optional unique identifier
    id: Option<String>,
    /// Optional name for the message
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    /// Additional metadata
    #[serde(default)]
    additional_kwargs: HashMap<String, serde_json::Value>,
}

impl HumanMessage {
    /// Create a new human message with simple text content.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: MessageContent::Text(content.into()),
            id: Some(Uuid::new_v4().to_string()),
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
            id: Some(Uuid::new_v4().to_string()),
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
