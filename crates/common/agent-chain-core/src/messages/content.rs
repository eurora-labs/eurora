use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::utils::base::ensure_id;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImageDetail {
    Low,
    High,
    #[default]
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    Url {
        url: String,
    },
    Base64 {
        media_type: String,
        data: String,
    },
    #[serde(rename = "file")]
    FileId {
        file_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text {
        text: String,
    },
    Image {
        source: ImageSource,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<ImageDetail>,
    },
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

static EMPTY_MESSAGE_CONTENT: LazyLock<MessageContent> =
    LazyLock::new(|| MessageContent::Text(String::new()));

impl MessageContent {
    pub fn empty() -> &'static MessageContent {
        &EMPTY_MESSAGE_CONTENT
    }
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

    pub fn as_text_ref(&self) -> &str {
        match self {
            MessageContent::Text(s) => s,
            MessageContent::Parts(_) => "",
        }
    }

    pub fn has_images(&self) -> bool {
        match self {
            MessageContent::Text(_) => false,
            MessageContent::Parts(parts) => {
                parts.iter().any(|p| matches!(p, ContentPart::Image { .. }))
            }
        }
    }

    pub fn parts(&self) -> Vec<ContentPart> {
        match self {
            MessageContent::Text(s) => vec![ContentPart::Text { text: s.clone() }],
            MessageContent::Parts(parts) => parts.clone(),
        }
    }

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Annotation {
    #[serde(rename = "citation")]
    Citation {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        start_index: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        end_index: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cited_text: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        extras: Option<HashMap<String, serde_json::Value>>,
    },
    #[serde(rename = "non_standard_annotation")]
    NonStandardAnnotation {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        value: HashMap<String, serde_json::Value>,
    },
}

impl Annotation {
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

    pub fn non_standard(value: HashMap<String, serde_json::Value>) -> Self {
        Self::NonStandardAnnotation { id: None, value }
    }
}

pub type Citation = Annotation;

pub type NonStandardAnnotation = Annotation;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<Annotation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl TextContentBlock {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCallBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub id: Option<String>,
    pub name: String,
    pub args: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl ToolCallBlock {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCallChunkBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub id: Option<String>,
    pub name: Option<String>,
    pub args: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl ToolCallChunkBlock {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvalidToolCallBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub id: Option<String>,
    pub name: Option<String>,
    pub args: Option<String>,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl InvalidToolCallBlock {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerToolCall {
    #[serde(rename = "type")]
    pub block_type: String,
    pub id: String,
    pub name: String,
    pub args: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl ServerToolCall {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerToolCallChunk {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl ServerToolCallChunk {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServerToolStatus {
    Success,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerToolResult {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub tool_call_id: String,
    pub status: ServerToolStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl ServerToolResult {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReasoningContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl ReasoningContentBlock {
    pub fn new(reasoning: impl Into<String>) -> Self {
        Self {
            block_type: "reasoning".to_string(),
            id: None,
            reasoning: Some(reasoning.into()),
            index: None,
            extras: None,
        }
    }

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl ImageContentBlock {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl VideoContentBlock {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl AudioContentBlock {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlainTextContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl PlainTextContentBlock {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

impl FileContentBlock {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NonStandardContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub value: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
}

impl NonStandardContentBlock {
    pub fn new(value: HashMap<String, serde_json::Value>) -> Self {
        Self {
            block_type: "non_standard".to_string(),
            id: None,
            value,
            index: None,
        }
    }
}

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

const DATA_CONTENT_BLOCK_TYPES: &[&str] = &["image", "video", "audio", "text-plain", "file"];

pub fn get_data_content_block_types() -> &'static [&'static str] {
    DATA_CONTENT_BLOCK_TYPES
}

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

#[derive(Debug, Clone, Default)]
pub struct PlainTextBlockConfig {
    pub text: Option<String>,
    pub url: Option<String>,
    pub base64: Option<String>,
    pub file_id: Option<String>,
    pub title: Option<String>,
    pub context: Option<String>,
    pub id: Option<String>,
    pub index: Option<BlockIndex>,
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

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
