use bon::bon;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;

use super::base::merge_content;
use super::content::MessageContent;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct FunctionMessage {
    pub content: MessageContent,
    pub name: String,
    pub id: Option<String>,
    #[serde(default)]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub response_metadata: HashMap<String, serde_json::Value>,
}

impl Serialize for FunctionMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(6))?;

        map.serialize_entry("type", "function")?;
        map.serialize_entry("content", &self.content)?;
        map.serialize_entry("name", &self.name)?;
        map.serialize_entry("id", &self.id)?;
        map.serialize_entry("additional_kwargs", &self.additional_kwargs)?;
        map.serialize_entry("response_metadata", &self.response_metadata)?;

        map.end()
    }
}

#[bon]
impl FunctionMessage {
    #[builder]
    pub fn new(
        content: impl Into<MessageContent>,
        name: impl Into<String>,
        id: Option<String>,
        #[builder(default)] additional_kwargs: HashMap<String, serde_json::Value>,
        #[builder(default)] response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            content: content.into(),
            name: name.into(),
            id,
            additional_kwargs,
            response_metadata,
        }
    }

    pub fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }

    pub fn message_type(&self) -> &'static str {
        "function"
    }

    pub fn text(&self) -> String {
        self.content.as_text()
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct FunctionMessageChunk {
    pub content: MessageContent,
    pub name: String,
    pub id: Option<String>,
    #[serde(default)]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub response_metadata: HashMap<String, serde_json::Value>,
}

impl Serialize for FunctionMessageChunk {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(6))?;

        map.serialize_entry("type", "FunctionMessageChunk")?;
        map.serialize_entry("content", &self.content)?;
        map.serialize_entry("name", &self.name)?;
        map.serialize_entry("id", &self.id)?;
        map.serialize_entry("additional_kwargs", &self.additional_kwargs)?;
        map.serialize_entry("response_metadata", &self.response_metadata)?;

        map.end()
    }
}

#[bon]
impl FunctionMessageChunk {
    #[builder]
    pub fn new(
        content: impl Into<MessageContent>,
        name: impl Into<String>,
        id: Option<String>,
        #[builder(default)] additional_kwargs: HashMap<String, serde_json::Value>,
        #[builder(default)] response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            content: content.into(),
            name: name.into(),
            id,
            additional_kwargs,
            response_metadata,
        }
    }

    pub fn message_type(&self) -> &'static str {
        "FunctionMessageChunk"
    }

    pub fn text(&self) -> String {
        self.content.as_text()
    }

    pub fn concat(&self, other: &FunctionMessageChunk) -> FunctionMessageChunk {
        if self.name != other.name {
            panic!("Cannot concatenate FunctionMessageChunks with different names");
        }

        let content: MessageContent =
            merge_content(self.content.as_text_ref(), other.content.as_text_ref()).into();

        let mut additional_kwargs = self.additional_kwargs.clone();
        for (k, v) in &other.additional_kwargs {
            additional_kwargs.insert(k.clone(), v.clone());
        }

        let mut response_metadata = self.response_metadata.clone();
        for (k, v) in &other.response_metadata {
            response_metadata.insert(k.clone(), v.clone());
        }

        FunctionMessageChunk {
            content,
            name: self.name.clone(),
            id: self.id.clone().or_else(|| other.id.clone()),
            additional_kwargs,
            response_metadata,
        }
    }

    pub fn to_message(&self) -> FunctionMessage {
        FunctionMessage {
            content: self.content.clone(),
            name: self.name.clone(),
            id: self.id.clone(),
            additional_kwargs: self.additional_kwargs.clone(),
            response_metadata: self.response_metadata.clone(),
        }
    }
}

impl std::ops::Add for FunctionMessageChunk {
    type Output = FunctionMessageChunk;

    fn add(self, other: FunctionMessageChunk) -> FunctionMessageChunk {
        self.concat(&other)
    }
}
