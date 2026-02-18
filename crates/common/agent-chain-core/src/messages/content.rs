//! Standard, multimodal content blocks for Large Language Model I/O.
//!
//! This module provides standardized data structures for representing inputs to and
//! outputs from LLMs. The core abstraction is the **Content Block**, a struct with
//! a `type` field for discrimination.
//!
//! Mirrors `langchain_core.messages.content` from Python.
//!
//! # Rationale
//!
//! Different LLM providers use distinct and incompatible API schemas. This module
//! provides a unified, provider-agnostic format to facilitate these interactions. A
//! message to or from a model is simply a list of content blocks, allowing for the natural
//! interleaving of text, images, and other content in a single ordered sequence.
//!
//! # Key Block Types
//!
//! - [`TextContentBlock`]: Standard text output.
//! - [`Citation`]: For annotations that link text output to a source document.
//! - [`ReasoningContentBlock`]: To capture a model's thought process.
//! - Multimodal data:
//!     - [`ImageContentBlock`]
//!     - [`AudioContentBlock`]
//!     - [`VideoContentBlock`]
//!     - [`PlainTextContentBlock`] (e.g. .txt or .md files)
//!     - [`FileContentBlock`] (e.g. PDFs, etc.)
//! - Tool calls:
//!     - [`ToolCallBlock`]
//!     - [`ToolCallChunkBlock`]
//!     - [`InvalidToolCallBlock`]
//!     - [`ServerToolCall`]
//!     - [`ServerToolCallChunk`]
//!     - [`ServerToolResult`]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::utils::base::ensure_id;


/// Image detail level for vision models.
///
/// This controls how the model processes the image:
/// - `Low`: Faster, lower token cost, suitable for simple images
/// - `High`: More detailed analysis, higher token cost
/// - `Auto`: Let the model decide based on image size

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImageDetail {
    Low,
    High,
    #[default]
    Auto,
}

/// Source of an image for multimodal messages.

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
    /// Image from a file ID (e.g., from a file storage system).
    #[serde(rename = "file")]
    FileId {
        /// The file ID
        file_id: String,
    },
}

/// A content part in a multimodal message.
///
/// Messages can contain multiple content parts, allowing for mixed text and images.
/// This corresponds to content blocks in LangChain Python's `langchain_core.messages.content`.

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
    /// Other/unknown content type (for provider-specific content).
    #[serde(untagged)]
    Other(serde_json::Value),
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text content.
    Text(String),
    /// Multiple content parts (for multimodal messages).
    Parts(Vec<ContentPart>),
}

static EMPTY_MESSAGE_CONTENT: LazyLock<MessageContent> =
    LazyLock::new(|| MessageContent::Text(String::new()));

impl MessageContent {
    /// A static reference to an empty MessageContent.
    pub fn empty() -> &'static MessageContent {
        &EMPTY_MESSAGE_CONTENT
    }
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

    /// Get the text content as a reference.
    ///
    /// For simple text content, returns a reference to the string.
    /// For multipart content, returns an empty string reference.
    pub fn as_text_ref(&self) -> &str {
        match self {
            MessageContent::Text(s) => s,
            MessageContent::Parts(_) => "",
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

    /// Check if the content is empty.
    pub fn len(&self) -> usize {
        match self {
            MessageContent::Text(s) => s.len(),
            MessageContent::Parts(parts) => parts.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            MessageContent::Text(s) => s.is_empty(),
            MessageContent::Parts(parts) => parts.is_empty(),
        }
    }

    /// Convert to a list of JSON values.
    ///
    /// For Text content, returns a single-element vec with a text block.
    /// For Parts content, serializes each part to a JSON value.
    pub fn as_json_values(&self) -> Vec<serde_json::Value> {
        match self {
            MessageContent::Text(s) => {
                if s.is_empty() {
                    vec![]
                } else {
                    vec![serde_json::json!({"type": "text", "text": s})]
                }
            }
            MessageContent::Parts(parts) => parts
                .iter()
                .filter_map(|p| serde_json::to_value(p).ok())
                .collect(),
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

impl From<&String> for MessageContent {
    fn from(s: &String) -> Self {
        MessageContent::Text(s.clone())
    }
}

impl From<Vec<ContentPart>> for MessageContent {
    fn from(parts: Vec<ContentPart>) -> Self {
        MessageContent::Parts(parts)
    }
}

impl From<Vec<serde_json::Value>> for MessageContent {
    fn from(values: Vec<serde_json::Value>) -> Self {
        let parts: Vec<ContentPart> = values.into_iter().map(ContentPart::Other).collect();
        MessageContent::Parts(parts)
    }
}
impl std::fmt::Display for MessageContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageContent::Text(s) => write!(f, "{}", s),
            MessageContent::Parts(parts) => {
                let texts: Vec<&str> = parts
                    .iter()
                    .filter_map(|p| match p {
                        ContentPart::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect();
                write!(f, "{}", texts.join(" "))
            }
        }
    }
}
impl PartialEq<str> for MessageContent {
    fn eq(&self, other: &str) -> bool {
        match self {
            MessageContent::Text(s) => s == other,
            MessageContent::Parts(_) => false,
        }
    }
}

impl PartialEq<&str> for MessageContent {
    fn eq(&self, other: &&str) -> bool {
        match self {
            MessageContent::Text(s) => s == *other,
            MessageContent::Parts(_) => false,
        }
    }
}

impl MessageContent {
    pub fn contains(&self, pattern: &str) -> bool {
        match self {
            MessageContent::Text(s) => s.contains(pattern),
            MessageContent::Parts(parts) => parts.iter().any(|p| match p {
                ContentPart::Text { text } => text.contains(pattern),
                _ => false,
            }),
        }
    }

    pub fn split(&self, pattern: &str) -> Vec<String> {
        match self {
            MessageContent::Text(s) => s.split(pattern).map(String::from).collect(),
            MessageContent::Parts(_) => vec![self.as_text()],
        }
    }
}


/// Index type that can be either an integer or string.
/// Used during streaming for block ordering.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum BlockIndex {
    Int(i64),
    Str(String),
}

impl From<i64> for BlockIndex {
    fn from(i: i64) -> Self {
        BlockIndex::Int(i)
    }
}

impl From<i32> for BlockIndex {
    fn from(i: i32) -> Self {
        BlockIndex::Int(i as i64)
    }
}

impl From<usize> for BlockIndex {
    fn from(i: usize) -> Self {
        BlockIndex::Int(i as i64)
    }
}

impl From<String> for BlockIndex {
    fn from(s: String) -> Self {
        BlockIndex::Str(s)
    }
}

impl From<&str> for BlockIndex {
    fn from(s: &str) -> Self {
        BlockIndex::Str(s.to_string())
    }
}

/// A union of all defined Annotation types.
///
/// In Python this is: `Annotation = Citation | NonStandardAnnotation`
/// where each variant is a TypedDict with a `type` literal field for discrimination.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Annotation {
    /// Citation annotation for citing data from a document.
    #[serde(rename = "citation")]
    Citation {
        /// Content block identifier.
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// URL of the document source.
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        /// Source document title.
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        /// Start index of the response text.
        #[serde(skip_serializing_if = "Option::is_none")]
        start_index: Option<i64>,
        /// End index of the response text.
        #[serde(skip_serializing_if = "Option::is_none")]
        end_index: Option<i64>,
        /// Excerpt of source text being cited.
        #[serde(skip_serializing_if = "Option::is_none")]
        cited_text: Option<String>,
        /// Provider-specific metadata.
        #[serde(skip_serializing_if = "Option::is_none")]
        extras: Option<HashMap<String, serde_json::Value>>,
    },
    /// Provider-specific annotation format.
    #[serde(rename = "non_standard_annotation")]
    NonStandardAnnotation {
        /// Content block identifier.
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Provider-specific annotation data.
        value: HashMap<String, serde_json::Value>,
    },
}

impl Annotation {
    /// Create a new Citation annotation.
    pub fn citation() -> Self {
        Self::Citation {
            id: None,
            url: None,
            title: None,
            start_index: None,
            end_index: None,
            cited_text: None,
            extras: None,
        }
    }

    /// Create a new NonStandardAnnotation.
    pub fn non_standard(value: HashMap<String, serde_json::Value>) -> Self {
        Self::NonStandardAnnotation { id: None, value }
    }
}

/// Type alias for Citation (matches Python's Citation TypedDict).
/// This is a convenience type that serializes with `type: "citation"`.
pub type Citation = Annotation;

/// Type alias for NonStandardAnnotation (matches Python's NonStandardAnnotation TypedDict).
/// This is a convenience type that serializes with `type: "non_standard_annotation"`.
pub type NonStandardAnnotation = Annotation;

/// Text output from a LLM.
///
/// This typically represents the main text content of a message.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextContentBlock {
    /// Type of the content block. Always "text".
    #[serde(rename = "type")]
    pub block_type: String,
    /// Content block identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Block text.
    pub text: String,
    /// Citations and other annotations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<Annotation>>,
    /// Index of block in aggregate response. Used during streaming.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    /// Provider-specific metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl TextContentBlock {
    /// Create a new TextContentBlock with the given text.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            block_type: "text".to_string(),
            id: None,
            text: text.into(),
            annotations: None,
            index: None,
            extras: None,
        }
    }
}

/// Represents an AI's request to call a tool (content block version).
///
/// This version includes a `type` field for discrimination and is used
/// as part of content blocks.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCallBlock {
    /// Type of the content block. Always "tool_call".
    #[serde(rename = "type")]
    pub block_type: String,
    /// An identifier associated with the tool call.
    pub id: Option<String>,
    /// The name of the tool to be called.
    pub name: String,
    /// The arguments to the tool call.
    pub args: HashMap<String, serde_json::Value>,
    /// Index of block in aggregate response. Used during streaming.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    /// Provider-specific metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl ToolCallBlock {
    /// Create a new ToolCallBlock.
    pub fn new(name: impl Into<String>, args: HashMap<String, serde_json::Value>) -> Self {
        Self {
            block_type: "tool_call".to_string(),
            id: None,
            name: name.into(),
            args,
            index: None,
            extras: None,
        }
    }
}

/// A chunk of a tool call (yielded when streaming, content block version).

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCallChunkBlock {
    /// Type of the content block. Always "tool_call_chunk".
    #[serde(rename = "type")]
    pub block_type: String,
    /// An identifier associated with the tool call.
    pub id: Option<String>,
    /// The name of the tool to be called.
    pub name: Option<String>,
    /// The arguments to the tool call (as a string, since it may be partial JSON).
    pub args: Option<String>,
    /// The index of the tool call in a sequence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    /// Provider-specific metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl ToolCallChunkBlock {
    /// Create a new ToolCallChunkBlock.
    pub fn new() -> Self {
        Self {
            block_type: "tool_call_chunk".to_string(),
            id: None,
            name: None,
            args: None,
            index: None,
            extras: None,
        }
    }
}

impl Default for ToolCallChunkBlock {
    fn default() -> Self {
        Self::new()
    }
}

/// Allowance for errors made by LLM (content block version).
///
/// Here we add an `error` key to surface errors made during generation.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvalidToolCallBlock {
    /// Type of the content block. Always "invalid_tool_call".
    #[serde(rename = "type")]
    pub block_type: String,
    /// An identifier associated with the tool call.
    pub id: Option<String>,
    /// The name of the tool to be called.
    pub name: Option<String>,
    /// The arguments to the tool call.
    pub args: Option<String>,
    /// An error message associated with the tool call.
    pub error: Option<String>,
    /// Index of block in aggregate response. Used during streaming.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    /// Provider-specific metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl InvalidToolCallBlock {
    /// Create a new InvalidToolCallBlock.
    pub fn new() -> Self {
        Self {
            block_type: "invalid_tool_call".to_string(),
            id: None,
            name: None,
            args: None,
            error: None,
            index: None,
            extras: None,
        }
    }
}

impl Default for InvalidToolCallBlock {
    fn default() -> Self {
        Self::new()
    }
}

/// Tool call that is executed server-side.
///
/// For example: code execution, web search, etc.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerToolCall {
    /// Type of the content block. Always "server_tool_call".
    #[serde(rename = "type")]
    pub block_type: String,
    /// An identifier associated with the tool call.
    pub id: String,
    /// The name of the tool to be called.
    pub name: String,
    /// The arguments to the tool call.
    pub args: HashMap<String, serde_json::Value>,
    /// Index of block in aggregate response. Used during streaming.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    /// Provider-specific metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl ServerToolCall {
    /// Create a new ServerToolCall.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        args: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            block_type: "server_tool_call".to_string(),
            id: id.into(),
            name: name.into(),
            args,
            index: None,
            extras: None,
        }
    }
}

/// A chunk of a server-side tool call (yielded when streaming).

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerToolCallChunk {
    /// Type of the content block. Always "server_tool_call_chunk".
    #[serde(rename = "type")]
    pub block_type: String,
    /// The name of the tool to be called.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// JSON substring of the arguments to the tool call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
    /// An identifier associated with the tool call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Index of block in aggregate response. Used during streaming.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    /// Provider-specific metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl ServerToolCallChunk {
    /// Create a new ServerToolCallChunk.
    pub fn new() -> Self {
        Self {
            block_type: "server_tool_call_chunk".to_string(),
            name: None,
            args: None,
            id: None,
            index: None,
            extras: None,
        }
    }
}

impl Default for ServerToolCallChunk {
    fn default() -> Self {
        Self::new()
    }
}

/// Execution status of the server-side tool.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServerToolStatus {
    Success,
    Error,
}

/// Result of a server-side tool call.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerToolResult {
    /// Type of the content block. Always "server_tool_result".
    #[serde(rename = "type")]
    pub block_type: String,
    /// An identifier associated with the server tool result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// ID of the corresponding server tool call.
    pub tool_call_id: String,
    /// Execution status of the server-side tool.
    pub status: ServerToolStatus,
    /// Output of the executed tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    /// Index of block in aggregate response. Used during streaming.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    /// Provider-specific metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl ServerToolResult {
    /// Create a new successful ServerToolResult.
    pub fn success(tool_call_id: impl Into<String>) -> Self {
        Self {
            block_type: "server_tool_result".to_string(),
            id: None,
            tool_call_id: tool_call_id.into(),
            status: ServerToolStatus::Success,
            output: None,
            index: None,
            extras: None,
        }
    }

    /// Create a new error ServerToolResult.
    pub fn error(tool_call_id: impl Into<String>) -> Self {
        Self {
            block_type: "server_tool_result".to_string(),
            id: None,
            tool_call_id: tool_call_id.into(),
            status: ServerToolStatus::Error,
            output: None,
            index: None,
            extras: None,
        }
    }
}

/// Reasoning output from a LLM.
///
/// Used to represent reasoning/thinking content from AI models that support
/// chain-of-thought reasoning (e.g., DeepSeek, Ollama, XAI, Groq).

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReasoningContentBlock {
    /// Type of the content block. Always "reasoning".
    #[serde(rename = "type")]
    pub block_type: String,
    /// Content block identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Reasoning text. Either the thought summary or the raw reasoning text itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    /// Index of block in aggregate response. Used during streaming.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    /// Provider-specific metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl ReasoningContentBlock {
    /// Create a new reasoning content block.
    pub fn new(reasoning: impl Into<String>) -> Self {
        Self {
            block_type: "reasoning".to_string(),
            id: None,
            reasoning: Some(reasoning.into()),
            index: None,
            extras: None,
        }
    }

    /// Get the reasoning content.
    pub fn reasoning(&self) -> Option<&str> {
        self.reasoning.as_deref()
    }
}

impl Default for ReasoningContentBlock {
    fn default() -> Self {
        Self {
            block_type: "reasoning".to_string(),
            id: None,
            reasoning: None,
            index: None,
            extras: None,
        }
    }
}

/// Image data content block.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageContentBlock {
    /// Type of the content block. Always "image".
    #[serde(rename = "type")]
    pub block_type: String,
    /// Content block identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// ID of the image file, e.g., from a file storage system.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    /// MIME type of the image. Required for base64.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Index of block in aggregate response. Used during streaming.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    /// URL of the image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Data as a base64 string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base64: Option<String>,
    /// Provider-specific metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl ImageContentBlock {
    /// Create a new ImageContentBlock.
    pub fn new() -> Self {
        Self {
            block_type: "image".to_string(),
            id: None,
            file_id: None,
            mime_type: None,
            index: None,
            url: None,
            base64: None,
            extras: None,
        }
    }

    /// Create an ImageContentBlock from a URL.
    pub fn from_url(url: impl Into<String>) -> Self {
        Self {
            block_type: "image".to_string(),
            id: None,
            file_id: None,
            mime_type: None,
            index: None,
            url: Some(url.into()),
            base64: None,
            extras: None,
        }
    }

    /// Create an ImageContentBlock from base64 data.
    pub fn from_base64(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            block_type: "image".to_string(),
            id: None,
            file_id: None,
            mime_type: Some(mime_type.into()),
            index: None,
            url: None,
            base64: Some(data.into()),
            extras: None,
        }
    }
}

impl Default for ImageContentBlock {
    fn default() -> Self {
        Self::new()
    }
}

/// Video data content block.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoContentBlock {
    /// Type of the content block. Always "video".
    #[serde(rename = "type")]
    pub block_type: String,
    /// Content block identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// ID of the video file, e.g., from a file storage system.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    /// MIME type of the video. Required for base64.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Index of block in aggregate response. Used during streaming.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    /// URL of the video.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Data as a base64 string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base64: Option<String>,
    /// Provider-specific metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl VideoContentBlock {
    /// Create a new VideoContentBlock.
    pub fn new() -> Self {
        Self {
            block_type: "video".to_string(),
            id: None,
            file_id: None,
            mime_type: None,
            index: None,
            url: None,
            base64: None,
            extras: None,
        }
    }
}

impl Default for VideoContentBlock {
    fn default() -> Self {
        Self::new()
    }
}

/// Audio data content block.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioContentBlock {
    /// Type of the content block. Always "audio".
    #[serde(rename = "type")]
    pub block_type: String,
    /// Content block identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// ID of the audio file, e.g., from a file storage system.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    /// MIME type of the audio. Required for base64.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Index of block in aggregate response. Used during streaming.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    /// URL of the audio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Data as a base64 string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base64: Option<String>,
    /// Provider-specific metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl AudioContentBlock {
    /// Create a new AudioContentBlock.
    pub fn new() -> Self {
        Self {
            block_type: "audio".to_string(),
            id: None,
            file_id: None,
            mime_type: None,
            index: None,
            url: None,
            base64: None,
            extras: None,
        }
    }
}

impl Default for AudioContentBlock {
    fn default() -> Self {
        Self::new()
    }
}

/// Plaintext data content block (e.g., from a `.txt` or `.md` document).

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlainTextContentBlock {
    /// Type of the content block. Always "text-plain".
    #[serde(rename = "type")]
    pub block_type: String,
    /// Content block identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// ID of the plaintext file, e.g., from a file storage system.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    /// MIME type of the file. Always "text/plain".
    pub mime_type: String,
    /// Index of block in aggregate response. Used during streaming.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    /// URL of the plaintext.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Data as a base64 string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base64: Option<String>,
    /// Plaintext content. Optional if the data is provided as base64.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Title of the text data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Context for the text, e.g., a description or summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// Provider-specific metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl PlainTextContentBlock {
    /// Create a new PlainTextContentBlock.
    pub fn new() -> Self {
        Self {
            block_type: "text-plain".to_string(),
            id: None,
            file_id: None,
            mime_type: "text/plain".to_string(),
            index: None,
            url: None,
            base64: None,
            text: None,
            title: None,
            context: None,
            extras: None,
        }
    }
}

impl Default for PlainTextContentBlock {
    fn default() -> Self {
        Self::new()
    }
}

/// File data content block for files that don't fit other categories.
///
/// This block is intended for files that are not images, audio, or plaintext.
/// For example, it can be used for PDFs, Word documents, etc.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileContentBlock {
    /// Type of the content block. Always "file".
    #[serde(rename = "type")]
    pub block_type: String,
    /// Content block identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// ID of the file, e.g., from a file storage system.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    /// MIME type of the file. Required for base64.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Index of block in aggregate response. Used during streaming.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    /// URL of the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Data as a base64 string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base64: Option<String>,
    /// Provider-specific metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl FileContentBlock {
    /// Create a new FileContentBlock.
    pub fn new() -> Self {
        Self {
            block_type: "file".to_string(),
            id: None,
            file_id: None,
            mime_type: None,
            index: None,
            url: None,
            base64: None,
            extras: None,
        }
    }
}

impl Default for FileContentBlock {
    fn default() -> Self {
        Self::new()
    }
}

/// Provider-specific content data.
///
/// This block contains data for which there is not yet a standard type.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NonStandardContentBlock {
    /// Type of the content block. Always "non_standard".
    #[serde(rename = "type")]
    pub block_type: String,
    /// Content block identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Provider-specific content data.
    pub value: HashMap<String, serde_json::Value>,
    /// Index of block in aggregate response. Used during streaming.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
}

impl NonStandardContentBlock {
    /// Create a new NonStandardContentBlock.
    pub fn new(value: HashMap<String, serde_json::Value>) -> Self {
        Self {
            block_type: "non_standard".to_string(),
            id: None,
            value,
            index: None,
        }
    }
}


/// A union of all defined multimodal data ContentBlock types.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum DataContentBlock {
    #[serde(rename = "image")]
    Image(ImageContentBlock),
    #[serde(rename = "video")]
    Video(VideoContentBlock),
    #[serde(rename = "audio")]
    Audio(AudioContentBlock),
    #[serde(rename = "text-plain")]
    PlainText(PlainTextContentBlock),
    #[serde(rename = "file")]
    File(FileContentBlock),
}

/// A union of all tool-related ContentBlock types.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ToolContentBlock {
    #[serde(rename = "tool_call")]
    ToolCall(ToolCallBlock),
    #[serde(rename = "tool_call_chunk")]
    ToolCallChunk(ToolCallChunkBlock),
    #[serde(rename = "server_tool_call")]
    ServerToolCall(ServerToolCall),
    #[serde(rename = "server_tool_call_chunk")]
    ServerToolCallChunk(ServerToolCallChunk),
    #[serde(rename = "server_tool_result")]
    ServerToolResult(ServerToolResult),
}

/// A union of all defined ContentBlock types.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text(TextContentBlock),
    #[serde(rename = "invalid_tool_call")]
    InvalidToolCall(InvalidToolCallBlock),
    #[serde(rename = "reasoning")]
    Reasoning(ReasoningContentBlock),
    #[serde(rename = "non_standard")]
    NonStandard(NonStandardContentBlock),
    #[serde(rename = "image")]
    Image(ImageContentBlock),
    #[serde(rename = "video")]
    Video(VideoContentBlock),
    #[serde(rename = "audio")]
    Audio(AudioContentBlock),
    #[serde(rename = "text-plain")]
    PlainText(PlainTextContentBlock),
    #[serde(rename = "file")]
    File(FileContentBlock),
    #[serde(rename = "tool_call")]
    ToolCall(ToolCallBlock),
    #[serde(rename = "tool_call_chunk")]
    ToolCallChunk(ToolCallChunkBlock),
    #[serde(rename = "server_tool_call")]
    ServerToolCall(ServerToolCall),
    #[serde(rename = "server_tool_call_chunk")]
    ServerToolCallChunk(ServerToolCallChunk),
    #[serde(rename = "server_tool_result")]
    ServerToolResult(ServerToolResult),
}


/// These are block types known to langchain-core>=1.0.0.
///
/// If a block has a type not in this set, it is considered to be provider-specific.
pub const KNOWN_BLOCK_TYPES: &[&str] = &[
    "text",
    "reasoning",
    "tool_call",
    "invalid_tool_call",
    "tool_call_chunk",
    "image",
    "audio",
    "file",
    "text-plain",
    "video",
    "server_tool_call",
    "server_tool_call_chunk",
    "server_tool_result",
    "non_standard",
];

/// Data content block type literals.
const DATA_CONTENT_BLOCK_TYPES: &[&str] = &["image", "video", "audio", "text-plain", "file"];

/// Returns the tuple of data content block type literals.
///
/// Mirrors Python's `_get_data_content_block_types()`.
pub fn get_data_content_block_types() -> &'static [&'static str] {
    DATA_CONTENT_BLOCK_TYPES
}


/// Check if the provided content block is a data content block.
///
/// Returns true for both v0 (old-style) and v1 (new-style) multimodal data blocks.
pub fn is_data_content_block(block: &serde_json::Value) -> bool {
    let block_type = match block.get("type").and_then(|t| t.as_str()) {
        Some(t) => t,
        None => return false,
    };

    if !DATA_CONTENT_BLOCK_TYPES.contains(&block_type) {
        return false;
    }

    if block.get("url").is_some()
        || block.get("base64").is_some()
        || block.get("file_id").is_some()
        || block.get("text").is_some()
    {
        if block_type == "text" && block.get("source_type").is_none() {
            return false;
        }
        return true;
    }

    if let Some(source_type) = block.get("source_type").and_then(|s| s.as_str()) {
        if (source_type == "url" && block.get("url").is_some())
            || (source_type == "base64" && block.get("data").is_some())
        {
            return true;
        }
        if (source_type == "id" && block.get("id").is_some())
            || (source_type == "text" && block.get("url").is_some())
        {
            return true;
        }
    }

    false
}


/// Create a `TextContentBlock`.
///
/// # Arguments
///
/// * `text` - The text content of the block.
/// * `id` - Content block identifier. Generated automatically if not provided.
/// * `annotations` - Citations and other annotations for the text.
/// * `index` - Index of block in aggregate response.
/// * `extras` - Provider-specific metadata.
pub fn create_text_block(
    text: impl Into<String>,
    id: Option<String>,
    annotations: Option<Vec<Annotation>>,
    index: Option<BlockIndex>,
    extras: Option<HashMap<String, serde_json::Value>>,
) -> TextContentBlock {
    TextContentBlock {
        block_type: "text".to_string(),
        text: text.into(),
        id: Some(ensure_id(id)),
        annotations,
        index,
        extras,
    }
}

/// Create an `ImageContentBlock`.
///
/// # Arguments
///
/// * `url` - URL of the image.
/// * `base64` - Base64-encoded image data.
/// * `file_id` - ID of the image file from a file storage system.
/// * `mime_type` - MIME type of the image. Required for base64 data.
/// * `id` - Content block identifier. Generated automatically if not provided.
/// * `index` - Index of block in aggregate response.
/// * `extras` - Provider-specific metadata.
///
/// # Errors
///
/// Returns an error if no image source is provided.
pub fn create_image_block(
    url: Option<String>,
    base64: Option<String>,
    file_id: Option<String>,
    mime_type: Option<String>,
    id: Option<String>,
    index: Option<BlockIndex>,
    extras: Option<HashMap<String, serde_json::Value>>,
) -> Result<ImageContentBlock, &'static str> {
    if url.is_none() && base64.is_none() && file_id.is_none() {
        return Err("Must provide one of: url, base64, or file_id");
    }

    Ok(ImageContentBlock {
        block_type: "image".to_string(),
        id: Some(ensure_id(id)),
        url,
        base64,
        file_id,
        mime_type,
        index,
        extras,
    })
}

/// Create a `VideoContentBlock`.
///
/// # Arguments
///
/// * `url` - URL of the video.
/// * `base64` - Base64-encoded video data.
/// * `file_id` - ID of the video file from a file storage system.
/// * `mime_type` - MIME type of the video. Required for base64 data.
/// * `id` - Content block identifier. Generated automatically if not provided.
/// * `index` - Index of block in aggregate response.
/// * `extras` - Provider-specific metadata.
///
/// # Errors
///
/// Returns an error if no video source is provided or if base64 is used without mime_type.
pub fn create_video_block(
    url: Option<String>,
    base64: Option<String>,
    file_id: Option<String>,
    mime_type: Option<String>,
    id: Option<String>,
    index: Option<BlockIndex>,
    extras: Option<HashMap<String, serde_json::Value>>,
) -> Result<VideoContentBlock, &'static str> {
    if url.is_none() && base64.is_none() && file_id.is_none() {
        return Err("Must provide one of: url, base64, or file_id");
    }

    if base64.is_some() && mime_type.is_none() {
        return Err("mime_type is required when using base64 data");
    }

    Ok(VideoContentBlock {
        block_type: "video".to_string(),
        id: Some(ensure_id(id)),
        url,
        base64,
        file_id,
        mime_type,
        index,
        extras,
    })
}

/// Create an `AudioContentBlock`.
///
/// # Arguments
///
/// * `url` - URL of the audio.
/// * `base64` - Base64-encoded audio data.
/// * `file_id` - ID of the audio file from a file storage system.
/// * `mime_type` - MIME type of the audio. Required for base64 data.
/// * `id` - Content block identifier. Generated automatically if not provided.
/// * `index` - Index of block in aggregate response.
/// * `extras` - Provider-specific metadata.
///
/// # Errors
///
/// Returns an error if no audio source is provided or if base64 is used without mime_type.
pub fn create_audio_block(
    url: Option<String>,
    base64: Option<String>,
    file_id: Option<String>,
    mime_type: Option<String>,
    id: Option<String>,
    index: Option<BlockIndex>,
    extras: Option<HashMap<String, serde_json::Value>>,
) -> Result<AudioContentBlock, &'static str> {
    if url.is_none() && base64.is_none() && file_id.is_none() {
        return Err("Must provide one of: url, base64, or file_id");
    }

    if base64.is_some() && mime_type.is_none() {
        return Err("mime_type is required when using base64 data");
    }

    Ok(AudioContentBlock {
        block_type: "audio".to_string(),
        id: Some(ensure_id(id)),
        url,
        base64,
        file_id,
        mime_type,
        index,
        extras,
    })
}

/// Create a `FileContentBlock`.
///
/// # Arguments
///
/// * `url` - URL of the file.
/// * `base64` - Base64-encoded file data.
/// * `file_id` - ID of the file from a file storage system.
/// * `mime_type` - MIME type of the file. Required for base64 data.
/// * `id` - Content block identifier. Generated automatically if not provided.
/// * `index` - Index of block in aggregate response.
/// * `extras` - Provider-specific metadata.
///
/// # Errors
///
/// Returns an error if no file source is provided or if base64 is used without mime_type.
pub fn create_file_block(
    url: Option<String>,
    base64: Option<String>,
    file_id: Option<String>,
    mime_type: Option<String>,
    id: Option<String>,
    index: Option<BlockIndex>,
    extras: Option<HashMap<String, serde_json::Value>>,
) -> Result<FileContentBlock, &'static str> {
    if url.is_none() && base64.is_none() && file_id.is_none() {
        return Err("Must provide one of: url, base64, or file_id");
    }

    if base64.is_some() && mime_type.is_none() {
        return Err("mime_type is required when using base64 data");
    }

    Ok(FileContentBlock {
        block_type: "file".to_string(),
        id: Some(ensure_id(id)),
        url,
        base64,
        file_id,
        mime_type,
        index,
        extras,
    })
}

/// Configuration for creating a `PlainTextContentBlock`.
#[derive(Debug, Clone, Default)]
pub struct PlainTextBlockConfig {
    /// The plaintext content.
    pub text: Option<String>,
    /// URL of the plaintext file.
    pub url: Option<String>,
    /// Base64-encoded plaintext data.
    pub base64: Option<String>,
    /// ID of the plaintext file from a file storage system.
    pub file_id: Option<String>,
    /// Title of the text data.
    pub title: Option<String>,
    /// Context or description of the text content.
    pub context: Option<String>,
    /// Content block identifier. Generated automatically if not provided.
    pub id: Option<String>,
    /// Index of block in aggregate response.
    pub index: Option<BlockIndex>,
    /// Provider-specific metadata.
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

/// Create a `PlainTextContentBlock`.
///
/// # Arguments
///
/// * `config` - Configuration for the plaintext block.
pub fn create_plaintext_block(config: PlainTextBlockConfig) -> PlainTextContentBlock {
    PlainTextContentBlock {
        block_type: "text-plain".to_string(),
        mime_type: "text/plain".to_string(),
        id: Some(ensure_id(config.id)),
        text: config.text,
        url: config.url,
        base64: config.base64,
        file_id: config.file_id,
        title: config.title,
        context: config.context,
        index: config.index,
        extras: config.extras,
    }
}

/// Create a `ToolCallBlock`.
///
/// # Arguments
///
/// * `name` - The name of the tool to be called.
/// * `args` - The arguments to the tool call.
/// * `id` - An identifier for the tool call. Generated automatically if not provided.
/// * `index` - Index of block in aggregate response.
/// * `extras` - Provider-specific metadata.
pub fn create_tool_call(
    name: impl Into<String>,
    args: HashMap<String, serde_json::Value>,
    id: Option<String>,
    index: Option<BlockIndex>,
    extras: Option<HashMap<String, serde_json::Value>>,
) -> ToolCallBlock {
    ToolCallBlock {
        block_type: "tool_call".to_string(),
        name: name.into(),
        args,
        id: Some(ensure_id(id)),
        index,
        extras,
    }
}

/// Create a `ReasoningContentBlock`.
///
/// # Arguments
///
/// * `reasoning` - The reasoning text or thought summary.
/// * `id` - Content block identifier. Generated automatically if not provided.
/// * `index` - Index of block in aggregate response.
/// * `extras` - Provider-specific metadata.
pub fn create_reasoning_block(
    reasoning: Option<String>,
    id: Option<String>,
    index: Option<BlockIndex>,
    extras: Option<HashMap<String, serde_json::Value>>,
) -> ReasoningContentBlock {
    ReasoningContentBlock {
        block_type: "reasoning".to_string(),
        reasoning: Some(reasoning.unwrap_or_default()),
        id: Some(ensure_id(id)),
        index,
        extras,
    }
}

/// Create a `Citation` annotation.
///
/// # Arguments
///
/// * `url` - URL of the document source.
/// * `title` - Source document title.
/// * `start_index` - Start index in the response text where citation applies.
/// * `end_index` - End index in the response text where citation applies.
/// * `cited_text` - Excerpt of source text being cited.
/// * `id` - Content block identifier. Generated automatically if not provided.
/// * `extras` - Provider-specific metadata.
pub fn create_citation(
    url: Option<String>,
    title: Option<String>,
    start_index: Option<i64>,
    end_index: Option<i64>,
    cited_text: Option<String>,
    id: Option<String>,
    extras: Option<HashMap<String, serde_json::Value>>,
) -> Annotation {
    Annotation::Citation {
        id: Some(ensure_id(id)),
        url,
        title,
        start_index,
        end_index,
        cited_text,
        extras,
    }
}

/// Create a `NonStandardContentBlock`.
///
/// # Arguments
///
/// * `value` - Provider-specific content data.
/// * `id` - Content block identifier. Generated automatically if not provided.
/// * `index` - Index of block in aggregate response.
pub fn create_non_standard_block(
    value: HashMap<String, serde_json::Value>,
    id: Option<String>,
    index: Option<BlockIndex>,
) -> NonStandardContentBlock {
    NonStandardContentBlock {
        block_type: "non_standard".to_string(),
        value,
        id: Some(ensure_id(id)),
        index,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_content_block_serialization() {
        let block = TextContentBlock::new("Hello, world!");
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"text\":\"Hello, world!\""));
    }

    #[test]
    fn test_create_text_block() {
        let block = create_text_block("Test", None, None, None, None);
        assert_eq!(block.text, "Test");
        assert!(block.id.unwrap().starts_with("lc_"));
    }

    #[test]
    fn test_create_image_block() {
        let block = create_image_block(
            Some("https://example.com/image.png".to_string()),
            None,
            None,
            Some("image/png".to_string()),
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(block.url.as_ref().unwrap(), "https://example.com/image.png");
        assert_eq!(block.mime_type.as_ref().unwrap(), "image/png");
    }

    #[test]
    fn test_create_image_block_error() {
        let result = create_image_block(None, None, None, None, None, None, None);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Must provide one of: url, base64, or file_id"
        );
    }

    #[test]
    fn test_reasoning_content_block() {
        let block = ReasoningContentBlock::new("Thinking...");
        assert_eq!(block.reasoning(), Some("Thinking..."));
        assert_eq!(block.block_type, "reasoning");
    }

    #[test]
    fn test_known_block_types() {
        assert!(KNOWN_BLOCK_TYPES.contains(&"text"));
        assert!(KNOWN_BLOCK_TYPES.contains(&"reasoning"));
        assert!(KNOWN_BLOCK_TYPES.contains(&"image"));
        assert!(KNOWN_BLOCK_TYPES.contains(&"tool_call"));
    }

    #[test]
    fn test_is_data_content_block() {
        let image_block = serde_json::json!({
            "type": "image",
            "url": "https://example.com/image.png"
        });
        assert!(is_data_content_block(&image_block));

        let text_block = serde_json::json!({
            "type": "text",
            "text": "Hello"
        });
        assert!(!is_data_content_block(&text_block));
    }

    #[test]
    fn test_content_block_enum_serialization() {
        let block = ContentBlock::Text(TextContentBlock::new("Hello"));
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"text\""));
    }

    #[test]
    fn test_legacy_message_content() {
        let content = MessageContent::Text("Hello".to_string());
        assert_eq!(content.as_text(), "Hello");

        let content = MessageContent::Parts(vec![
            ContentPart::Text {
                text: "Hello".to_string(),
            },
            ContentPart::Text {
                text: "World".to_string(),
            },
        ]);
        assert_eq!(content.as_text(), "Hello World");
    }

    #[test]
    fn test_annotation_citation_serialization() {
        let citation = Annotation::Citation {
            id: Some("test_id".to_string()),
            url: Some("https://example.com".to_string()),
            title: Some("Document Title".to_string()),
            start_index: Some(0),
            end_index: Some(10),
            cited_text: Some("The weather is sunny.".to_string()),
            extras: None,
        };

        let json = serde_json::to_value(&citation).unwrap();
        assert_eq!(json["type"], "citation");
        assert_eq!(json["id"], "test_id");
        assert_eq!(json["url"], "https://example.com");
        assert_eq!(json["title"], "Document Title");
        assert_eq!(json["start_index"], 0);
        assert_eq!(json["end_index"], 10);
        assert_eq!(json["cited_text"], "The weather is sunny.");
    }

    #[test]
    fn test_annotation_non_standard_serialization() {
        let mut value = HashMap::new();
        value.insert(
            "bar".to_string(),
            serde_json::Value::String("baz".to_string()),
        );

        let annotation = Annotation::NonStandardAnnotation {
            id: None,
            value: value.clone(),
        };

        let json = serde_json::to_value(&annotation).unwrap();
        assert_eq!(json["type"], "non_standard_annotation");
        assert_eq!(json["value"]["bar"], "baz");
    }

    #[test]
    fn test_annotation_deserialization() {
        let json_str = r#"{
            "type": "citation",
            "id": "lc_123",
            "title": "Document Title",
            "cited_text": "The weather is sunny.",
            "extras": {
                "source": "source_123"
            }
        }"#;

        let annotation: Annotation = serde_json::from_str(json_str).unwrap();
        match annotation {
            Annotation::Citation {
                id,
                title,
                cited_text,
                extras,
                ..
            } => {
                assert_eq!(id, Some("lc_123".to_string()));
                assert_eq!(title, Some("Document Title".to_string()));
                assert_eq!(cited_text, Some("The weather is sunny.".to_string()));
                assert!(extras.is_some());
                let extras = extras.unwrap();
                assert_eq!(
                    extras.get("source"),
                    Some(&serde_json::Value::String("source_123".to_string()))
                );
            }
            _ => panic!("Expected Citation variant"),
        }
    }

    #[test]
    fn test_text_block_with_annotations() {
        let mut extras = HashMap::new();
        extras.insert(
            "source".to_string(),
            serde_json::Value::String("source_123".to_string()),
        );
        extras.insert("search_result_index".to_string(), serde_json::json!(1));

        let citation = Annotation::Citation {
            id: None,
            url: None,
            title: Some("Document Title".to_string()),
            start_index: None,
            end_index: None,
            cited_text: Some("The weather is sunny.".to_string()),
            extras: Some(extras),
        };

        let mut non_std_value = HashMap::new();
        non_std_value.insert(
            "bar".to_string(),
            serde_json::Value::String("baz".to_string()),
        );
        let non_standard = Annotation::NonStandardAnnotation {
            id: None,
            value: non_std_value,
        };

        let text_block = TextContentBlock {
            block_type: "text".to_string(),
            id: None,
            text: "It's sunny.".to_string(),
            annotations: Some(vec![citation, non_standard]),
            index: None,
            extras: None,
        };

        let json = serde_json::to_value(&text_block).unwrap();
        assert_eq!(json["type"], "text");
        assert_eq!(json["text"], "It's sunny.");

        let annotations = json["annotations"].as_array().unwrap();
        assert_eq!(annotations.len(), 2);

        assert_eq!(annotations[0]["type"], "citation");
        assert_eq!(annotations[0]["title"], "Document Title");
        assert_eq!(annotations[0]["cited_text"], "The weather is sunny.");
        assert_eq!(annotations[0]["extras"]["source"], "source_123");

        assert_eq!(annotations[1]["type"], "non_standard_annotation");
        assert_eq!(annotations[1]["value"]["bar"], "baz");
    }

    #[test]
    fn test_create_citation_factory() {
        let citation = create_citation(
            Some("https://example.com".to_string()),
            Some("Title".to_string()),
            Some(0),
            Some(10),
            Some("Cited text".to_string()),
            None,
            None,
        );

        match citation {
            Annotation::Citation { id, url, title, .. } => {
                assert!(id.unwrap().starts_with("lc_"));
                assert_eq!(url, Some("https://example.com".to_string()));
                assert_eq!(title, Some("Title".to_string()));
            }
            _ => panic!("Expected Citation variant"),
        }
    }
}
