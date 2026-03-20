use bon::bon;
use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt;

use super::base::{
    AnyMessage, BaseMessage, BaseMessageChunk, get_msg_title_repr, is_interactive_env,
};
use super::content::{ContentBlock, ContentBlocks, TextContentBlock};
use super::system::SystemMessageChunk;
use crate::load::Serializable;
use crate::utils::merge::{merge_dicts, merge_lists};

#[derive(Debug, Clone, PartialEq)]
pub struct HumanMessage {
    pub content: ContentBlocks,
    pub id: Option<String>,
    pub name: Option<String>,
    pub additional_kwargs: HashMap<String, serde_json::Value>,
    pub response_metadata: HashMap<String, serde_json::Value>,
}

impl BaseMessage for HumanMessage {
    fn id(&self) -> Option<String> {
        self.id.clone()
    }

    fn content(&self) -> &ContentBlocks {
        &self.content
    }

    fn name(&self) -> Option<String> {
        self.name.clone()
    }

    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }

    fn message_type(&self) -> &'static str {
        "human"
    }

    fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value> {
        &self.additional_kwargs
    }

    fn response_metadata(&self) -> &HashMap<String, serde_json::Value> {
        &self.response_metadata
    }
}

impl Serialize for HumanMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut field_count = 4;
        if self.name.is_some() {
            field_count += 1;
        }
        field_count += 1;

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

impl<'de> Deserialize<'de> for HumanMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct HumanMessageVisitor;

        impl<'de> Visitor<'de> for HumanMessageVisitor {
            type Value = HumanMessage;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a HumanMessage object")
            }

            fn visit_map<M>(self, mut map: M) -> Result<HumanMessage, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut content: Option<ContentBlocks> = None;
                let mut id: Option<String> = None;
                let mut name: Option<String> = None;
                let mut additional_kwargs: Option<HashMap<String, serde_json::Value>> = None;
                let mut response_metadata: Option<HashMap<String, serde_json::Value>> = None;

                while let Some(key) = map.next_key::<&str>()? {
                    match key {
                        "content" => content = Some(map.next_value()?),
                        "id" => id = map.next_value()?,
                        "name" => name = map.next_value()?,
                        "additional_kwargs" => additional_kwargs = Some(map.next_value()?),
                        "response_metadata" => response_metadata = Some(map.next_value()?),
                        "type" => {
                            let _ = map.next_value::<de::IgnoredAny>()?;
                        }
                        _ => {
                            let _ = map.next_value::<de::IgnoredAny>()?;
                        }
                    }
                }

                Ok(HumanMessage {
                    content: content.unwrap_or_default(),
                    id,
                    name,
                    additional_kwargs: additional_kwargs.unwrap_or_default(),
                    response_metadata: response_metadata.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_map(HumanMessageVisitor)
    }
}

#[bon]
impl HumanMessage {
    #[builder]
    pub fn new(
        content: impl Into<ContentBlocks>,
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

    pub fn has_images(&self) -> bool {
        self.content
            .iter()
            .any(|b| matches!(b, ContentBlock::Image(_)))
    }

    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn pretty_repr(&self, html: bool) -> String {
        let title = get_msg_title_repr("Human Message", html);
        let name_line = if let Some(name) = &self.name {
            format!("\nName: {}", name)
        } else {
            String::new()
        };
        format!("{}{}\n\n{}", title, name_line, self.text())
    }

    pub fn pretty_print(&self) {
        println!("{}", self.pretty_repr(is_interactive_env()));
    }

    /// Returns content blocks after applying provider-specific input translations.
    /// This normalizes provider-specific formats (OpenAI image_url, Anthropic source blocks, etc.)
    /// into the standard ContentBlock representation.
    pub fn content_blocks(&self) -> Vec<ContentBlock> {
        use super::content::{
            AudioContentBlock, FileContentBlock, ImageContentBlock, InvalidToolCallBlock,
            NonStandardContentBlock, PlainTextContentBlock, ReasoningContentBlock, ServerToolCall,
            ServerToolCallChunk, ServerToolResult, ToolCallBlock, ToolCallChunkBlock,
            VideoContentBlock,
        };
        use crate::messages::block_translators::anthropic::convert_input_to_standard_blocks as anthropic_convert;
        use crate::messages::block_translators::openai::convert_to_v1_from_chat_completions_input;

        // Convert stored blocks to JSON for the translators
        let raw_values: Vec<serde_json::Value> = self
            .content
            .iter()
            .filter_map(|block| serde_json::to_value(block).ok())
            .collect();

        // Apply input-side translators (these handle provider-specific formats
        // that may have been stored as NonStandard blocks)
        let mut blocks = convert_to_v1_from_chat_completions_input(&raw_values);
        blocks = anthropic_convert(&blocks);

        blocks
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
                        index: v.get("index").and_then(|i| serde_json::from_value(i.clone()).ok()),
                    })
                })
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HumanMessageChunk {
    pub content: ContentBlocks,
    pub id: Option<String>,
    pub name: Option<String>,
    pub additional_kwargs: HashMap<String, serde_json::Value>,
    pub response_metadata: HashMap<String, serde_json::Value>,
}

impl Serialize for HumanMessageChunk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut field_count = 4;
        if self.name.is_some() {
            field_count += 1;
        }
        field_count += 1;

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

impl<'de> Deserialize<'de> for HumanMessageChunk {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct HumanMessageChunkVisitor;

        impl<'de> Visitor<'de> for HumanMessageChunkVisitor {
            type Value = HumanMessageChunk;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a HumanMessageChunk object")
            }

            fn visit_map<M>(self, mut map: M) -> Result<HumanMessageChunk, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut content: Option<ContentBlocks> = None;
                let mut id: Option<String> = None;
                let mut name: Option<String> = None;
                let mut additional_kwargs: Option<HashMap<String, serde_json::Value>> = None;
                let mut response_metadata: Option<HashMap<String, serde_json::Value>> = None;

                while let Some(key) = map.next_key::<&str>()? {
                    match key {
                        "content" => content = Some(map.next_value()?),
                        "id" => id = map.next_value()?,
                        "name" => name = map.next_value()?,
                        "additional_kwargs" => additional_kwargs = Some(map.next_value()?),
                        "response_metadata" => response_metadata = Some(map.next_value()?),
                        "type" => {
                            let _ = map.next_value::<de::IgnoredAny>()?;
                        }
                        _ => {
                            let _ = map.next_value::<de::IgnoredAny>()?;
                        }
                    }
                }

                Ok(HumanMessageChunk {
                    content: content.unwrap_or_default(),
                    id,
                    name,
                    additional_kwargs: additional_kwargs.unwrap_or_default(),
                    response_metadata: response_metadata.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_map(HumanMessageChunkVisitor)
    }
}

#[bon]
impl HumanMessageChunk {
    #[builder]
    pub fn new(
        content: impl Into<ContentBlocks>,
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

    pub fn message_type(&self) -> &'static str {
        "HumanMessageChunk"
    }

    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn concat(&self, other: &HumanMessageChunk) -> HumanMessageChunk {
        // Merge content blocks: serialize to Value, use merge_lists, deserialize back
        let left: Vec<serde_json::Value> = self
            .content
            .iter()
            .filter_map(|b| serde_json::to_value(b).ok())
            .collect();
        let right: Vec<serde_json::Value> = other
            .content
            .iter()
            .filter_map(|b| serde_json::to_value(b).ok())
            .collect();

        let content: ContentBlocks =
            match merge_lists(Some(left.clone()), vec![Some(right.clone())]) {
                Ok(Some(merged)) => merged
                    .into_iter()
                    .filter_map(|v| serde_json::from_value(v).ok())
                    .collect(),
                _ => {
                    let mut blocks = self.content.clone();
                    blocks.extend(other.content.iter().cloned());
                    blocks
                }
            };

        let additional_kwargs = {
            let left_val = serde_json::to_value(&self.additional_kwargs).unwrap_or_default();
            let right_val = serde_json::to_value(&other.additional_kwargs).unwrap_or_default();
            match merge_dicts(left_val, vec![right_val]) {
                Ok(merged) => serde_json::from_value(merged).unwrap_or_default(),
                Err(_) => self.additional_kwargs.clone(),
            }
        };

        let response_metadata = {
            let left_val = serde_json::to_value(&self.response_metadata).unwrap_or_default();
            let right_val = serde_json::to_value(&other.response_metadata).unwrap_or_default();
            match merge_dicts(left_val, vec![right_val]) {
                Ok(merged) => serde_json::from_value(merged).unwrap_or_default(),
                Err(_) => self.response_metadata.clone(),
            }
        };

        HumanMessageChunk {
            content,
            id: self.id.clone().or_else(|| other.id.clone()),
            name: self.name.clone().or_else(|| other.name.clone()),
            additional_kwargs,
            response_metadata,
        }
    }

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
        iter.reduce(|a, b| a + b).unwrap_or_else(|| {
            HumanMessageChunk::builder()
                .content(ContentBlocks::new())
                .build()
        })
    }
}

impl BaseMessageChunk for HumanMessageChunk {
    fn id(&self) -> Option<String> {
        self.id.clone()
    }
    fn content(&self) -> &ContentBlocks {
        &self.content
    }
    fn name(&self) -> Option<String> {
        self.name.clone()
    }
    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
    fn message_type(&self) -> &'static str {
        "HumanMessageChunk"
    }
    fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value> {
        &self.additional_kwargs
    }
    fn response_metadata(&self) -> &HashMap<String, serde_json::Value> {
        &self.response_metadata
    }
    fn to_message(&self) -> AnyMessage {
        AnyMessage::HumanMessage(self.to_message())
    }
}

impl From<HumanMessageChunk> for HumanMessage {
    fn from(chunk: HumanMessageChunk) -> Self {
        chunk.to_message()
    }
}

impl std::ops::Add<SystemMessageChunk> for HumanMessageChunk {
    type Output = HumanMessageChunk;

    fn add(self, other: SystemMessageChunk) -> HumanMessageChunk {
        let other_as_human = HumanMessageChunk {
            content: other.content,
            id: other.id,
            name: other.name,
            additional_kwargs: other.additional_kwargs,
            response_metadata: other.response_metadata,
        };
        self.concat(&other_as_human)
    }
}

impl Serializable for HumanMessage {
    fn is_lc_serializable() -> bool {
        true
    }

    fn get_lc_namespace() -> Vec<String> {
        vec![
            "langchain".to_string(),
            "schema".to_string(),
            "messages".to_string(),
        ]
    }
}

impl Serializable for HumanMessageChunk {
    fn is_lc_serializable() -> bool {
        true
    }

    fn get_lc_namespace() -> Vec<String> {
        vec![
            "langchain".to_string(),
            "schema".to_string(),
            "messages".to_string(),
        ]
    }
}

submit_constructor!(HumanMessage);
submit_constructor!(HumanMessageChunk);
