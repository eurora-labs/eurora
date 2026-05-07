use bon::bon;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "specta")]
use specta_typescript::Unknown;

use super::base::{
    AnyMessage, BaseMessage, BaseMessageChunk, MergeError, get_msg_title_repr, is_interactive_env,
};
use super::content::{ContentBlock, ContentBlocks};
use crate::load::Serializable;
use crate::utils::merge::{merge_dicts, merge_lists};

#[cfg(feature = "specta")]
type JsonObjectTs = HashMap<String, Unknown>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ChatMessage {
    pub content: ContentBlocks,
    pub role: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "specta", specta(type = JsonObjectTs))]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
    #[serde(default)]
    #[cfg_attr(feature = "specta", specta(type = JsonObjectTs))]
    pub response_metadata: HashMap<String, serde_json::Value>,
}

impl BaseMessage for ChatMessage {
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
        "chat"
    }

    fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value> {
        &self.additional_kwargs
    }

    fn response_metadata(&self) -> &HashMap<String, serde_json::Value> {
        &self.response_metadata
    }
}

#[bon]
impl ChatMessage {
    #[builder]
    pub fn new(
        content: impl Into<ContentBlocks>,
        role: impl Into<String>,
        id: Option<String>,
        name: Option<String>,
        #[builder(default)] additional_kwargs: HashMap<String, serde_json::Value>,
        #[builder(default)] response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            content: content.into(),
            role: role.into(),
            id,
            name,
            additional_kwargs,
            response_metadata,
        }
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
        let title = get_msg_title_repr("Chat Message", html);
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
pub struct ChatMessageChunk {
    pub content: ContentBlocks,
    pub role: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "specta", specta(type = JsonObjectTs))]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
    #[serde(default)]
    #[cfg_attr(feature = "specta", specta(type = JsonObjectTs))]
    pub response_metadata: HashMap<String, serde_json::Value>,
}

#[bon]
impl ChatMessageChunk {
    #[builder]
    pub fn new(
        content: impl Into<ContentBlocks>,
        role: impl Into<String>,
        id: Option<String>,
        name: Option<String>,
        #[builder(default)] additional_kwargs: HashMap<String, serde_json::Value>,
        #[builder(default)] response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            content: content.into(),
            role: role.into(),
            id,
            name,
            additional_kwargs,
            response_metadata,
        }
    }

    pub fn message_type(&self) -> &'static str {
        "chat_chunk"
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

    /// Concatenate two chat chunks. Returns [`MergeError::MismatchedRole`] when
    /// `self.role` and `other.role` differ — same-role merging is the only
    /// well-defined operation.
    pub fn try_concat(&self, other: &ChatMessageChunk) -> Result<ChatMessageChunk, MergeError> {
        if self.role != other.role {
            return Err(MergeError::MismatchedRole {
                left: self.role.clone(),
                right: other.role.clone(),
            });
        }

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

        Ok(ChatMessageChunk {
            content,
            role: self.role.clone(),
            id: self.id.clone(),
            name: self.name.clone(),
            additional_kwargs,
            response_metadata,
        })
    }

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

    pub fn to_message(&self) -> ChatMessage {
        ChatMessage {
            content: self.content.clone(),
            role: self.role.clone(),
            id: self.id.clone(),
            name: self.name.clone(),
            additional_kwargs: self.additional_kwargs.clone(),
            response_metadata: self.response_metadata.clone(),
        }
    }
}

impl std::ops::Add for ChatMessageChunk {
    type Output = ChatMessageChunk;

    /// Convenience wrapper around [`ChatMessageChunk::try_concat`]. Panics if
    /// the two chunks have different roles — that's a programming error.
    /// Use `try_concat` for explicit error handling.
    fn add(self, other: ChatMessageChunk) -> ChatMessageChunk {
        self.try_concat(&other)
            .expect("merging ChatMessageChunks with mismatched role")
    }
}

impl std::iter::Sum for ChatMessageChunk {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|a, b| a + b).unwrap_or_else(|| {
            ChatMessageChunk::builder()
                .content(ContentBlocks::new())
                .role("")
                .build()
        })
    }
}

impl std::ops::Add<super::human::HumanMessageChunk> for ChatMessageChunk {
    type Output = ChatMessageChunk;

    fn add(self, other: super::human::HumanMessageChunk) -> ChatMessageChunk {
        let coerced = ChatMessageChunk {
            content: other.content,
            role: self.role.clone(),
            id: other.id,
            name: other.name,
            additional_kwargs: other.additional_kwargs,
            response_metadata: other.response_metadata,
        };
        self.try_concat(&coerced)
            .expect("ChatMessageChunk + HumanMessageChunk should always merge (role is copied)")
    }
}

impl BaseMessageChunk for ChatMessageChunk {
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
        "chat_chunk"
    }
    fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value> {
        &self.additional_kwargs
    }
    fn response_metadata(&self) -> &HashMap<String, serde_json::Value> {
        &self.response_metadata
    }
    fn to_message(&self) -> AnyMessage {
        AnyMessage::ChatMessage(self.to_message())
    }
}

impl Serializable for ChatMessage {
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

impl Serializable for ChatMessageChunk {
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

submit_constructor!(ChatMessage);
submit_constructor!(ChatMessageChunk);
