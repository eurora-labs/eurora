//! Message types for LLM interactions.
//!
//! This module provides message types for different roles (human, AI, system, tool)
//! as well as types for tool calls.
//!
//! # Multimodal Support
//!
//! The [`HumanMessage`] type supports multimodal content including text and images.
//! Images can be provided as URLs or base64-encoded data.
//!
//! ```ignore
//! use agent_chain_core::messages::{HumanMessage, ContentPart, ImageSource};
//!
//! // Simple text message
//! let msg = HumanMessage::new("Hello!");
//!
//! // Message with image from URL
//! let msg = HumanMessage::with_content(vec![
//!     ContentPart::Text { text: "What's in this image?".into() },
//!     ContentPart::Image {
//!         source: ImageSource::Url {
//!             url: "https://example.com/image.jpg".into(),
//!         },
//!         detail: None,
//!     },
//! ]);
//!
//! // Message with base64-encoded image
//! let msg = HumanMessage::with_content(vec![
//!     ContentPart::Text { text: "Describe this image".into() },
//!     ContentPart::Image {
//!         source: ImageSource::Base64 {
//!             media_type: "image/jpeg".into(),
//!             data: base64_image_data,
//!         },
//!         detail: Some(ImageDetail::High),
//!     },
//! ]);
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

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

/// A tool call made by the AI model.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    /// Unique identifier for this tool call
    id: String,
    /// Name of the tool to call
    name: String,
    /// Arguments for the tool call as a JSON object
    args: serde_json::Value,
}

impl ToolCall {
    /// Create a new tool call.
    pub fn new(name: impl Into<String>, args: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            args,
        }
    }

    /// Create a new tool call with a specific ID.
    pub fn with_id(
        id: impl Into<String>,
        name: impl Into<String>,
        args: serde_json::Value,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            args,
        }
    }

    /// Get the tool call ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the tool name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the tool arguments.
    pub fn args(&self) -> &serde_json::Value {
        &self.args
    }
}

/// Message content that can be either simple text or multipart.
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

/// A human message in the conversation.
///
/// Human messages support both simple text content and multimodal content
/// with images. Use [`HumanMessage::new`] for simple text messages and
/// [`HumanMessage::with_content`] for multimodal messages.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HumanMessage {
    /// The message content (text or multipart)
    content: MessageContent,
    /// Optional unique identifier
    id: Option<String>,
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

    /// Get the message content as text.
    ///
    /// For multipart messages, this concatenates all text parts.
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
}

/// A system message in the conversation.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemMessage {
    /// The message content
    content: String,
    /// Optional unique identifier
    id: Option<String>,
    /// Additional metadata
    #[serde(default)]
    additional_kwargs: HashMap<String, serde_json::Value>,
}

impl SystemMessage {
    /// Create a new system message.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            id: Some(Uuid::new_v4().to_string()),
            additional_kwargs: HashMap::new(),
        }
    }

    /// Create a new system message with an explicit ID.
    ///
    /// Use this when deserializing or reconstructing messages where the ID must be preserved.
    pub fn with_id(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            id: Some(id.into()),
            additional_kwargs: HashMap::new(),
        }
    }

    /// Get the message content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the message ID.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }
}

/// An AI message in the conversation.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AIMessage {
    /// The message content
    content: String,
    /// Optional unique identifier
    id: Option<String>,
    /// Tool calls made by the AI
    #[serde(default)]
    tool_calls: Vec<ToolCall>,
    /// Additional metadata
    #[serde(default)]
    additional_kwargs: HashMap<String, serde_json::Value>,
}

impl AIMessage {
    /// Create a new AI message.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            id: Some(Uuid::new_v4().to_string()),
            tool_calls: Vec::new(),
            additional_kwargs: HashMap::new(),
        }
    }

    /// Create a new AI message with an explicit ID.
    ///
    /// Use this when deserializing or reconstructing messages where the ID must be preserved.
    pub fn with_id(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            id: Some(id.into()),
            tool_calls: Vec::new(),
            additional_kwargs: HashMap::new(),
        }
    }

    /// Create a new AI message with tool calls.
    pub fn with_tool_calls(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            content: content.into(),
            id: Some(Uuid::new_v4().to_string()),
            tool_calls,
            additional_kwargs: HashMap::new(),
        }
    }

    /// Create a new AI message with tool calls and an explicit ID.
    ///
    /// Use this when deserializing or reconstructing messages where the ID must be preserved.
    pub fn with_id_and_tool_calls(
        id: impl Into<String>,
        content: impl Into<String>,
        tool_calls: Vec<ToolCall>,
    ) -> Self {
        Self {
            content: content.into(),
            id: Some(id.into()),
            tool_calls,
            additional_kwargs: HashMap::new(),
        }
    }

    /// Get the message content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the message ID.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Get the tool calls.
    pub fn tool_calls(&self) -> &[ToolCall] {
        &self.tool_calls
    }

    /// Add annotations to the message (e.g., citations from web search).
    /// Annotations are stored in additional_kwargs under the "annotations" key.
    pub fn with_annotations<T: Serialize>(mut self, annotations: Vec<T>) -> Self {
        if let Ok(value) = serde_json::to_value(&annotations) {
            self.additional_kwargs
                .insert("annotations".to_string(), value);
        }
        self
    }

    /// Get annotations from the message if present.
    pub fn annotations(&self) -> Option<&serde_json::Value> {
        self.additional_kwargs.get("annotations")
    }

    /// Get additional kwargs.
    pub fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value> {
        &self.additional_kwargs
    }
}

/// A tool message containing the result of a tool call.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolMessage {
    /// The tool result content
    content: String,
    /// The ID of the tool call this message is responding to
    tool_call_id: String,
    /// Optional unique identifier
    id: Option<String>,
    /// Additional metadata
    #[serde(default)]
    additional_kwargs: HashMap<String, serde_json::Value>,
}

impl ToolMessage {
    /// Create a new tool message.
    pub fn new(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            tool_call_id: tool_call_id.into(),
            id: Some(Uuid::new_v4().to_string()),
            additional_kwargs: HashMap::new(),
        }
    }

    /// Create a new tool message with an explicit ID.
    ///
    /// Use this when deserializing or reconstructing messages where the ID must be preserved.
    pub fn with_id(
        id: impl Into<String>,
        content: impl Into<String>,
        tool_call_id: impl Into<String>,
    ) -> Self {
        Self {
            content: content.into(),
            tool_call_id: tool_call_id.into(),
            id: Some(id.into()),
            additional_kwargs: HashMap::new(),
        }
    }

    /// Get the message content.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the tool call ID this message responds to.
    pub fn tool_call_id(&self) -> &str {
        &self.tool_call_id
    }

    /// Get the message ID.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }
}

/// A unified message type that can represent any message role.
#[cfg_attr(feature = "specta", derive(Type))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum BaseMessage {
    /// A human message
    Human(HumanMessage),
    /// A system message
    System(SystemMessage),
    /// An AI message
    AI(AIMessage),
    /// A tool result message
    Tool(ToolMessage),
}

impl BaseMessage {
    /// Get the message content.
    pub fn content(&self) -> &str {
        match self {
            BaseMessage::Human(m) => m.content(),
            BaseMessage::System(m) => m.content(),
            BaseMessage::AI(m) => m.content(),
            BaseMessage::Tool(m) => m.content(),
        }
    }

    /// Get the message ID.
    pub fn id(&self) -> Option<&str> {
        match self {
            BaseMessage::Human(m) => m.id(),
            BaseMessage::System(m) => m.id(),
            BaseMessage::AI(m) => m.id(),
            BaseMessage::Tool(m) => m.id(),
        }
    }

    /// Get tool calls if this is an AI message.
    pub fn tool_calls(&self) -> &[ToolCall] {
        match self {
            BaseMessage::AI(m) => m.tool_calls(),
            _ => &[],
        }
    }

    /// Get the message type as a string.
    pub fn message_type(&self) -> &'static str {
        match self {
            BaseMessage::Human(_) => "human",
            BaseMessage::System(_) => "system",
            BaseMessage::AI(_) => "ai",
            BaseMessage::Tool(_) => "tool",
        }
    }
}

impl From<HumanMessage> for BaseMessage {
    fn from(msg: HumanMessage) -> Self {
        BaseMessage::Human(msg)
    }
}

impl From<SystemMessage> for BaseMessage {
    fn from(msg: SystemMessage) -> Self {
        BaseMessage::System(msg)
    }
}

impl From<AIMessage> for BaseMessage {
    fn from(msg: AIMessage) -> Self {
        BaseMessage::AI(msg)
    }
}

impl From<ToolMessage> for BaseMessage {
    fn from(msg: ToolMessage) -> Self {
        BaseMessage::Tool(msg)
    }
}

/// Trait for types that have an optional ID.
/// Used for message merging operations.
pub trait HasId {
    /// Get the ID if present.
    fn get_id(&self) -> Option<&str>;
}

impl HasId for BaseMessage {
    fn get_id(&self) -> Option<&str> {
        self.id()
    }
}

/// Type alias for any message type, matching LangChain's AnyMessage.
/// This is equivalent to BaseMessage but provides naming consistency with Python.
pub type AnyMessage = BaseMessage;

impl BaseMessage {
    /// Pretty print the message to stdout.
    /// This mimics LangChain's pretty_print() method for messages.
    pub fn pretty_print(&self) {
        let (role, content) = match self {
            BaseMessage::Human(m) => ("Human", m.content()),
            BaseMessage::System(m) => ("System", m.content()),
            BaseMessage::AI(m) => {
                let tool_calls = m.tool_calls();
                if tool_calls.is_empty() {
                    ("AI", m.content())
                } else {
                    println!(
                        "================================== AI Message =================================="
                    );
                    if !m.content().is_empty() {
                        println!("{}", m.content());
                    }
                    for tc in tool_calls {
                        println!("Tool Call: {} ({})", tc.name(), tc.id());
                        println!("  Args: {}", tc.args());
                    }
                    return;
                }
            }
            BaseMessage::Tool(m) => {
                println!(
                    "================================= Tool Message ================================="
                );
                println!("[{}] {}", m.tool_call_id(), m.content());
                return;
            }
        };

        let header = format!("=== {} Message ===", role);
        let padding = (80 - header.len()) / 2;
        println!(
            "{:=>padding$}{}{:=>padding$}",
            "",
            header,
            "",
            padding = padding
        );
        println!("{}", content);
    }
}
