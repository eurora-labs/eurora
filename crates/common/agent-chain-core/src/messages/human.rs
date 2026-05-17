use bon::bon;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "specta")]
use specta_typescript::Unknown;

use super::base::{
    AnyMessage, BaseMessage, BaseMessageChunk, get_msg_title_repr, is_interactive_env,
};
use super::content::{ContentBlock, ContentBlocks};
use super::system::SystemMessageChunk;
use crate::load::Serializable;
use crate::utils::merge::{merge_dicts, merge_lists};

#[cfg(feature = "specta")]
type JsonObjectTs = HashMap<String, Unknown>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct HumanMessage {
    pub content: ContentBlocks,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "specta", specta(type = JsonObjectTs))]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
    #[serde(default)]
    #[cfg_attr(feature = "specta", specta(type = JsonObjectTs))]
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
        use crate::messages::block_translators::anthropic::convert_input_to_standard_blocks as anthropic_convert;
        use crate::messages::block_translators::openai::convert_to_v1_from_chat_completions_input;

        let raw_values = self.content.as_json_values();

        let mut blocks = convert_to_v1_from_chat_completions_input(&raw_values);
        blocks = anthropic_convert(&blocks);

        blocks
            .into_iter()
            .map(ContentBlock::from_value_or_non_standard)
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct HumanMessageChunk {
    pub content: ContentBlocks,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "specta", specta(type = JsonObjectTs))]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
    #[serde(default)]
    #[cfg_attr(feature = "specta", specta(type = JsonObjectTs))]
    pub response_metadata: HashMap<String, serde_json::Value>,
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
        "human_chunk"
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
        let left = self.content.as_json_values();
        let right = other.content.as_json_values();

        let content: ContentBlocks =
            match merge_lists(Some(left.clone()), vec![Some(right.clone())]) {
                Ok(Some(merged)) => merged
                    .into_iter()
                    .map(ContentBlock::from_value_or_non_standard)
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
        "human_chunk"
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
