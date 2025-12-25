//! Content types for multimodal messages.
//!
//! This module provides types for representing message content including
//! text, images, and other multimodal data. Mirrors `langchain_core.messages.content`.

use serde::{Deserialize, Serialize};

#[cfg(feature = "specta")]
use specta::Type;

/// Image detail level for vision models.
///
/// This controls how the model processes the image:
/// - `Low`: Faster, lower token cost, suitable for simple images
/// - `High`: More detailed analysis, higher token cost
/// - `Auto`: Let the model decide based on image size
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImageDetail {
    Low,
    High,
    #[default]
    Auto,
}

/// Source of an image for multimodal messages.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    /// Image from a URL.
    Url { url: String },
    /// Base64-encoded image data.
    Base64 {
        /// MIME type (e.g., "image/jpeg", "image/png", "image/gif", "image/webp")
        media_type: String,
        /// Base64-encoded image data (without the data URL prefix)
        data: String,
    },
}

/// A content part in a multimodal message.
///
/// Messages can contain multiple content parts, allowing for mixed text and images.
/// This corresponds to content blocks in LangChain Python's `langchain_core.messages.content`.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    /// Text content.
    Text { text: String },
    /// Image content.
    Image {
        source: ImageSource,
        /// Detail level for image processing (optional, defaults to Auto)
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<ImageDetail>,
    },
}

impl From<&str> for ContentPart {
    fn from(text: &str) -> Self {
        ContentPart::Text {
            text: text.to_string(),
        }
    }
}

impl From<String> for ContentPart {
    fn from(text: String) -> Self {
        ContentPart::Text { text }
    }
}

/// Message content that can be either simple text or multipart.
///
/// This represents the content field of messages and can be either
/// a simple string or a list of content parts for multimodal messages.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text content.
    Text(String),
    /// Multiple content parts (for multimodal messages).
    Parts(Vec<ContentPart>),
}

impl MessageContent {
    /// Get the text content, concatenating text parts if multipart.
    pub fn as_text(&self) -> String {
        match self {
            MessageContent::Text(s) => s.clone(),
            MessageContent::Parts(parts) => parts
                .iter()
                .filter_map(|p| match p {
                    ContentPart::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    /// Check if this content has any images.
    pub fn has_images(&self) -> bool {
        match self {
            MessageContent::Text(_) => false,
            MessageContent::Parts(parts) => {
                parts.iter().any(|p| matches!(p, ContentPart::Image { .. }))
            }
        }
    }

    /// Get content parts, converting simple text to a single text part if needed.
    pub fn parts(&self) -> Vec<ContentPart> {
        match self {
            MessageContent::Text(s) => vec![ContentPart::Text { text: s.clone() }],
            MessageContent::Parts(parts) => parts.clone(),
        }
    }
}

impl Default for MessageContent {
    fn default() -> Self {
        MessageContent::Text(String::new())
    }
}

impl From<String> for MessageContent {
    fn from(s: String) -> Self {
        MessageContent::Text(s)
    }
}

impl From<&str> for MessageContent {
    fn from(s: &str) -> Self {
        MessageContent::Text(s.to_string())
    }
}

impl From<Vec<ContentPart>> for MessageContent {
    fn from(parts: Vec<ContentPart>) -> Self {
        MessageContent::Parts(parts)
    }
}