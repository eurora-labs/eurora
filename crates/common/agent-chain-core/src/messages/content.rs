use bon::bon;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[bon]
impl Annotation {
    #[builder]
    pub fn citation(
        id: Option<String>,
        url: Option<String>,
        title: Option<String>,
        start_index: Option<i64>,
        end_index: Option<i64>,
        cited_text: Option<String>,
        extras: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self::Citation {
            id: Some(ensure_id(id)),
            url,
            title,
            start_index,
            end_index,
            cited_text,
            extras,
        }
    }

    #[builder]
    pub fn non_standard(value: HashMap<String, serde_json::Value>, id: Option<String>) -> Self {
        Self::NonStandardAnnotation {
            id: Some(ensure_id(id)),
            value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextContentBlock {
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

#[bon]
impl TextContentBlock {
    #[builder]
    pub fn new(
        text: impl Into<String>,
        id: Option<String>,
        annotations: Option<Vec<Annotation>>,
        index: Option<BlockIndex>,
        extras: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self {
            id: Some(ensure_id(id)),
            text: text.into(),
            annotations,
            index,
            extras,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCallBlock {
    pub id: Option<String>,
    pub name: String,
    pub args: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

#[bon]
impl ToolCallBlock {
    #[builder]
    pub fn new(
        name: impl Into<String>,
        args: HashMap<String, serde_json::Value>,
        id: Option<String>,
        index: Option<BlockIndex>,
        extras: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self {
            id: Some(ensure_id(id)),
            name: name.into(),
            args,
            index,
            extras,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCallChunkBlock {
    pub id: Option<String>,
    pub name: Option<String>,
    pub args: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

#[bon]
impl ToolCallChunkBlock {
    #[builder]
    pub fn new(
        id: Option<String>,
        name: Option<String>,
        args: Option<String>,
        index: Option<BlockIndex>,
        extras: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self {
            id,
            name,
            args,
            index,
            extras,
        }
    }
}

impl Default for ToolCallChunkBlock {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InvalidToolCallBlock {
    pub id: Option<String>,
    pub name: Option<String>,
    pub args: Option<String>,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

#[bon]
impl InvalidToolCallBlock {
    #[builder]
    pub fn new(
        id: Option<String>,
        name: Option<String>,
        args: Option<String>,
        error: Option<String>,
        index: Option<BlockIndex>,
        extras: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self {
            id,
            name,
            args,
            error,
            index,
            extras,
        }
    }
}

impl Default for InvalidToolCallBlock {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerToolCall {
    pub id: String,
    pub name: String,
    pub args: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

#[bon]
impl ServerToolCall {
    #[builder]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        args: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
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

#[bon]
impl ServerToolCallChunk {
    #[builder]
    pub fn new(
        name: Option<String>,
        args: Option<String>,
        id: Option<String>,
        index: Option<BlockIndex>,
        extras: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self {
            name,
            args,
            id,
            index,
            extras,
        }
    }
}

impl Default for ServerToolCallChunk {
    fn default() -> Self {
        Self::builder().build()
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

#[bon]
impl ServerToolResult {
    #[builder]
    pub fn success(tool_call_id: impl Into<String>) -> Self {
        Self {
            id: None,
            tool_call_id: tool_call_id.into(),
            status: ServerToolStatus::Success,
            output: None,
            index: None,
            extras: None,
        }
    }

    #[builder]
    pub fn error(tool_call_id: impl Into<String>) -> Self {
        Self {
            id: None,
            tool_call_id: tool_call_id.into(),
            status: ServerToolStatus::Error,
            output: None,
            index: None,
            extras: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ReasoningContentBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<HashMap<String, serde_json::Value>>,
}

#[bon]
impl ReasoningContentBlock {
    #[builder]
    pub fn new(
        reasoning: impl Into<String>,
        id: Option<String>,
        index: Option<BlockIndex>,
        extras: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self {
            id: Some(ensure_id(id)),
            reasoning: Some(reasoning.into()),
            index,
            extras,
        }
    }

    pub fn reasoning(&self) -> Option<&str> {
        self.reasoning.as_deref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageContentBlock {
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

#[bon]
impl ImageContentBlock {
    /// Create an image content block.
    ///
    /// Returns an error if none of `url`, `base64`, or `file_id` is provided.
    #[builder]
    pub fn new(
        url: Option<String>,
        base64: Option<String>,
        file_id: Option<String>,
        mime_type: Option<String>,
        id: Option<String>,
        index: Option<BlockIndex>,
        extras: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<Self, &'static str> {
        if url.is_none() && base64.is_none() && file_id.is_none() {
            return Err("Must provide one of: url, base64, or file_id");
        }

        Ok(Self {
            id: Some(ensure_id(id)),
            url,
            base64,
            file_id,
            mime_type,
            index,
            extras,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoContentBlock {
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

#[bon]
impl VideoContentBlock {
    /// Create a video content block.
    ///
    /// Returns an error if none of `url`, `base64`, or `file_id` is provided,
    /// or if `mime_type` is missing when using `base64`.
    #[builder]
    pub fn new(
        url: Option<String>,
        base64: Option<String>,
        file_id: Option<String>,
        mime_type: Option<String>,
        id: Option<String>,
        index: Option<BlockIndex>,
        extras: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<Self, &'static str> {
        if url.is_none() && base64.is_none() && file_id.is_none() {
            return Err("Must provide one of: url, base64, or file_id");
        }

        if base64.is_some() && mime_type.is_none() {
            return Err("mime_type is required when using base64 data");
        }

        Ok(Self {
            id: Some(ensure_id(id)),
            url,
            base64,
            file_id,
            mime_type,
            index,
            extras,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioContentBlock {
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

#[bon]
impl AudioContentBlock {
    /// Create an audio content block.
    ///
    /// Returns an error if none of `url`, `base64`, or `file_id` is provided,
    /// or if `mime_type` is missing when using `base64`.
    #[builder]
    pub fn new(
        url: Option<String>,
        base64: Option<String>,
        file_id: Option<String>,
        mime_type: Option<String>,
        id: Option<String>,
        index: Option<BlockIndex>,
        extras: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<Self, &'static str> {
        if url.is_none() && base64.is_none() && file_id.is_none() {
            return Err("Must provide one of: url, base64, or file_id");
        }

        if base64.is_some() && mime_type.is_none() {
            return Err("mime_type is required when using base64 data");
        }

        Ok(Self {
            id: Some(ensure_id(id)),
            url,
            base64,
            file_id,
            mime_type,
            index,
            extras,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlainTextContentBlock {
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

#[bon]
impl PlainTextContentBlock {
    #[builder]
    pub fn new(
        id: Option<String>,
        file_id: Option<String>,
        #[builder(default = "text/plain".to_string())] mime_type: String,
        index: Option<BlockIndex>,
        url: Option<String>,
        base64: Option<String>,
        text: Option<String>,
        title: Option<String>,
        context: Option<String>,
        extras: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        Self {
            id: Some(ensure_id(id)),
            file_id,
            mime_type,
            index,
            url,
            base64,
            text,
            title,
            context,
            extras,
        }
    }
}

impl Default for PlainTextContentBlock {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileContentBlock {
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

#[bon]
impl FileContentBlock {
    /// Create a file content block.
    ///
    /// Returns an error if none of `url`, `base64`, or `file_id` is provided,
    /// or if `mime_type` is missing when using `base64`.
    #[builder]
    pub fn new(
        url: Option<String>,
        base64: Option<String>,
        file_id: Option<String>,
        mime_type: Option<String>,
        id: Option<String>,
        index: Option<BlockIndex>,
        extras: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<Self, &'static str> {
        if url.is_none() && base64.is_none() && file_id.is_none() {
            return Err("Must provide one of: url, base64, or file_id");
        }

        if base64.is_some() && mime_type.is_none() {
            return Err("mime_type is required when using base64 data");
        }

        Ok(Self {
            id: Some(ensure_id(id)),
            url,
            base64,
            file_id,
            mime_type,
            index,
            extras,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NonStandardContentBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub value: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<BlockIndex>,
}

#[bon]
impl NonStandardContentBlock {
    #[builder]
    pub fn new(
        value: HashMap<String, serde_json::Value>,
        id: Option<String>,
        index: Option<BlockIndex>,
    ) -> Self {
        Self {
            id: Some(ensure_id(id)),
            value,
            index,
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

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct ContentBlocks(Vec<ContentBlock>);

impl<'de> serde::Deserialize<'de> for ContentBlocks {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, SeqAccess, Visitor};

        struct ContentBlocksVisitor;

        impl<'de> Visitor<'de> for ContentBlocksVisitor {
            type Value = ContentBlocks;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string or array of content blocks")
            }

            fn visit_str<E>(self, value: &str) -> Result<ContentBlocks, E>
            where
                E: de::Error,
            {
                Ok(ContentBlocks::from(value))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<ContentBlocks, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut blocks = Vec::new();
                while let Some(value) = seq.next_element::<serde_json::Value>()? {
                    if let Some(text) = value.as_str() {
                        blocks.push(ContentBlock::Text(
                            TextContentBlock::builder().text(text).build(),
                        ));
                    } else {
                        match serde_json::from_value::<ContentBlock>(value.clone()) {
                            Ok(block) => blocks.push(block),
                            Err(_) => {
                                let mut error_value = HashMap::new();
                                error_value.insert("original_json".to_string(), value);
                                blocks.push(ContentBlock::NonStandard(
                                    NonStandardContentBlock::builder()
                                        .value(error_value)
                                        .build(),
                                ));
                            }
                        }
                    }
                }
                Ok(ContentBlocks(blocks))
            }
        }

        deserializer.deserialize_any(ContentBlocksVisitor)
    }
}

impl ContentBlocks {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn into_inner(self) -> Vec<ContentBlock> {
        self.0
    }

    pub fn as_text(&self) -> String {
        self.to_string()
    }

    pub fn as_text_ref(&self) -> &str {
        if self.0.len() == 1
            && let ContentBlock::Text(ref t) = self.0[0]
        {
            return &t.text;
        }
        ""
    }

    pub fn has_images(&self) -> bool {
        self.0.iter().any(|b| matches!(b, ContentBlock::Image(_)))
    }

    pub fn as_json_values(&self) -> Vec<serde_json::Value> {
        self.0
            .iter()
            .filter_map(|block| serde_json::to_value(block).ok())
            .collect()
    }
}

// --- Deref / DerefMut: makes ContentBlocks behave like Vec<ContentBlock> ---

impl std::ops::Deref for ContentBlocks {
    type Target = Vec<ContentBlock>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ContentBlocks {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// --- IntoIterator: for block in content / for block in &content ---

impl IntoIterator for ContentBlocks {
    type Item = ContentBlock;
    type IntoIter = std::vec::IntoIter<ContentBlock>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a ContentBlocks {
    type Item = &'a ContentBlock;
    type IntoIter = std::slice::Iter<'a, ContentBlock>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut ContentBlocks {
    type Item = &'a mut ContentBlock;
    type IntoIter = std::slice::IterMut<'a, ContentBlock>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

// --- FromIterator / Extend: collect() and extend() support ---

impl FromIterator<ContentBlock> for ContentBlocks {
    fn from_iter<I: IntoIterator<Item = ContentBlock>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl Extend<ContentBlock> for ContentBlocks {
    fn extend<I: IntoIterator<Item = ContentBlock>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

// --- From conversions ---

impl From<Vec<ContentBlock>> for ContentBlocks {
    fn from(blocks: Vec<ContentBlock>) -> Self {
        Self(blocks)
    }
}

impl From<ContentBlocks> for Vec<ContentBlock> {
    fn from(blocks: ContentBlocks) -> Self {
        blocks.0
    }
}

impl From<&str> for ContentBlocks {
    fn from(s: &str) -> Self {
        if s.is_empty() {
            Self(vec![])
        } else {
            Self(vec![ContentBlock::Text(
                TextContentBlock::builder().text(s).build(),
            )])
        }
    }
}

impl From<String> for ContentBlocks {
    fn from(s: String) -> Self {
        if s.is_empty() {
            Self(vec![])
        } else {
            Self(vec![ContentBlock::Text(
                TextContentBlock::builder().text(s).build(),
            )])
        }
    }
}

impl From<&String> for ContentBlocks {
    fn from(s: &String) -> Self {
        Self::from(s.as_str())
    }
}

impl From<ContentBlock> for ContentBlocks {
    fn from(block: ContentBlock) -> Self {
        Self(vec![block])
    }
}

// --- Index: content[0] support ---

impl std::ops::Index<usize> for ContentBlocks {
    type Output = ContentBlock;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for ContentBlocks {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

// --- Display ---

impl std::fmt::Display for ContentBlocks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let texts: Vec<&str> = self
            .0
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect();
        write!(f, "{}", texts.join(" "))
    }
}

// --- AsRef / Borrow: for APIs that accept &[ContentBlock] ---

impl AsRef<[ContentBlock]> for ContentBlocks {
    fn as_ref(&self) -> &[ContentBlock] {
        &self.0
    }
}

impl std::borrow::Borrow<[ContentBlock]> for ContentBlocks {
    fn borrow(&self) -> &[ContentBlock] {
        &self.0
    }
}

// --- PartialEq<str>: compare text content with string literals ---

impl PartialEq<str> for ContentBlocks {
    fn eq(&self, other: &str) -> bool {
        // Single text block: compare directly without allocating
        if self.0.len() == 1
            && let ContentBlock::Text(ref t) = self.0[0]
        {
            return t.text == other;
        }
        // Empty content vs empty string
        if self.0.is_empty() {
            return other.is_empty();
        }
        // Fall back to joining text blocks
        let mut remaining = other;
        let mut first = true;
        for block in &self.0 {
            if let ContentBlock::Text(t) = block {
                if !first {
                    if !remaining.starts_with(' ') {
                        return false;
                    }
                    remaining = &remaining[1..];
                }
                if !remaining.starts_with(&t.text[..]) {
                    return false;
                }
                remaining = &remaining[t.text.len()..];
                first = false;
            }
        }
        remaining.is_empty()
    }
}

impl PartialEq<&str> for ContentBlocks {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_content_block_serialization() {
        let block = ContentBlock::Text(TextContentBlock::builder().text("Hello, world!").build());
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"text\":\"Hello, world!\""));
    }

    #[test]
    fn test_text_content_block_builder() {
        let block = TextContentBlock::builder().text("Test").build();
        assert_eq!(block.text, "Test");
        assert!(block.id.unwrap().starts_with("lc_"));
    }

    #[test]
    fn test_image_content_block_builder() {
        let block = ImageContentBlock::builder()
            .url("https://example.com/image.png".to_string())
            .mime_type("image/png".to_string())
            .build()
            .unwrap();
        assert_eq!(block.url.as_ref().unwrap(), "https://example.com/image.png");
        assert_eq!(block.mime_type.as_ref().unwrap(), "image/png");
    }

    #[test]
    fn test_image_content_block_validation_error() {
        let result = ImageContentBlock::builder().build();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Must provide one of: url, base64, or file_id"
        );
    }

    #[test]
    fn test_reasoning_content_block() {
        let block = ReasoningContentBlock::builder()
            .reasoning("Thinking...")
            .build();
        assert_eq!(block.reasoning(), Some("Thinking..."));
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
        let block = ContentBlock::Text(TextContentBlock::builder().text("Hello").build());
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"text\""));
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
            id: None,
            text: "It's sunny.".to_string(),
            annotations: Some(vec![citation, non_standard]),
            index: None,
            extras: None,
        };

        let wrapped = ContentBlock::Text(text_block);
        let json = serde_json::to_value(&wrapped).unwrap();
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
    fn test_annotation_citation_builder() {
        let citation = Annotation::citation()
            .url("https://example.com".to_string())
            .title("Title".to_string())
            .start_index(0)
            .end_index(10)
            .cited_text("Cited text".to_string())
            .call();

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
