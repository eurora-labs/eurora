use bon::bon;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;

use super::base::{get_msg_title_repr, is_interactive_env, merge_content};
use super::content::{ContentBlock, ContentPart, KNOWN_BLOCK_TYPES, MessageContent};
use super::system::SystemMessageChunk;
use crate::load::Serializable;
use crate::utils::merge::{merge_dicts, merge_lists};

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct HumanMessage {
    pub content: MessageContent,
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub response_metadata: HashMap<String, serde_json::Value>,
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

#[bon]
impl HumanMessage {
    #[builder]
    pub fn new(
        content: impl Into<MessageContent>,
        content_blocks: Option<Vec<ContentBlock>>,
        id: Option<String>,
        name: Option<String>,
        #[builder(default)] additional_kwargs: HashMap<String, serde_json::Value>,
        #[builder(default)] response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        let resolved_content = if let Some(blocks) = content_blocks {
            let parts: Vec<ContentPart> = blocks
                .into_iter()
                .filter_map(|block| serde_json::to_value(&block).ok().map(ContentPart::Other))
                .collect();
            MessageContent::Parts(parts)
        } else {
            content.into()
        };

        Self {
            content: resolved_content,
            id,
            name,
            additional_kwargs,
            response_metadata,
        }
    }

    pub fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }

    pub fn message_type(&self) -> &'static str {
        "human"
    }

    pub fn has_images(&self) -> bool {
        self.content.has_images()
    }

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

    pub fn text(&self) -> String {
        self.content.as_text()
    }

    pub fn pretty_repr(&self, html: bool) -> String {
        let title = get_msg_title_repr("Human Message", html);
        let name_line = if let Some(name) = &self.name {
            format!("\nName: {}", name)
        } else {
            String::new()
        };
        format!("{}{}\n\n{}", title, name_line, self.content.as_text_ref())
    }

    pub fn pretty_print(&self) {
        println!("{}", self.pretty_repr(is_interactive_env()));
    }

    pub fn content_blocks(&self) -> Vec<ContentBlock> {
        use super::content::{
            AudioContentBlock, FileContentBlock, ImageContentBlock, InvalidToolCallBlock,
            NonStandardContentBlock, PlainTextContentBlock, ReasoningContentBlock, ServerToolCall,
            ServerToolCallChunk, ServerToolResult, TextContentBlock, ToolCallBlock,
            ToolCallChunkBlock, VideoContentBlock,
        };
        use crate::messages::block_translators::anthropic::convert_input_to_standard_blocks as anthropic_convert;
        use crate::messages::block_translators::openai::convert_to_v1_from_chat_completions_input;

        let mut blocks: Vec<serde_json::Value> = Vec::new();

        let items: Vec<serde_json::Value> = match &self.content {
            MessageContent::Text(s) => {
                if s.is_empty() {
                    vec![]
                } else {
                    vec![serde_json::Value::String(s.clone())]
                }
            }
            MessageContent::Parts(parts) => parts
                .iter()
                .filter_map(|p| serde_json::to_value(p).ok())
                .collect(),
        };

        for item in items {
            if let Some(s) = item.as_str() {
                blocks.push(serde_json::json!({"type": "text", "text": s}));
            } else if item.is_object() {
                let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");

                if !KNOWN_BLOCK_TYPES.contains(&item_type) || item.get("source_type").is_some() {
                    blocks.push(serde_json::json!({"type": "non_standard", "value": item}));
                } else {
                    blocks.push(item);
                }
            }
        }

        blocks = convert_to_v1_from_chat_completions_input(&blocks);
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

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct HumanMessageChunk {
    pub content: MessageContent,
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
    #[serde(default)]
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

#[bon]
impl HumanMessageChunk {
    #[builder]
    pub fn new(
        content: impl Into<MessageContent>,
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

    pub fn concat(&self, other: &HumanMessageChunk) -> HumanMessageChunk {
        let content = match (&self.content, &other.content) {
            (MessageContent::Text(a), MessageContent::Text(b)) => {
                MessageContent::Text(merge_content(a, b))
            }
            (MessageContent::Parts(a), MessageContent::Parts(b)) => {
                let left: Vec<serde_json::Value> = a
                    .iter()
                    .filter_map(|p| serde_json::to_value(p).ok())
                    .collect();
                let right: Vec<serde_json::Value> = b
                    .iter()
                    .filter_map(|p| serde_json::to_value(p).ok())
                    .collect();
                match merge_lists(Some(left.clone()), vec![Some(right.clone())]) {
                    Ok(Some(merged)) => {
                        let parts: Vec<ContentPart> = merged
                            .into_iter()
                            .filter_map(|v| serde_json::from_value(v).ok())
                            .collect();
                        MessageContent::Parts(parts)
                    }
                    _ => {
                        let mut parts = a.clone();
                        parts.extend(b.clone());
                        MessageContent::Parts(parts)
                    }
                }
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
        iter.reduce(|a, b| a + b)
            .unwrap_or_else(|| HumanMessageChunk::builder().content("").build())
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
