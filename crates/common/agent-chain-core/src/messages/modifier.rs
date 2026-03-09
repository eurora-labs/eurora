use bon::bon;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;

use crate::MessageContent;

use super::base::{BaseMessage, get_msg_title_repr, is_interactive_env};
use super::content::ContentBlock;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct RemoveMessage {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub response_metadata: HashMap<String, serde_json::Value>,
}

impl BaseMessage for RemoveMessage {
    fn id(&self) -> Option<String> {
        Some(self.id.clone())
    }

    fn content(&self) -> &MessageContent {
        MessageContent::empty()
    }

    fn name(&self) -> Option<String> {
        self.name.clone()
    }

    fn set_id(&mut self, id: String) {
        self.id = id;
    }

    fn message_type(&self) -> &'static str {
        "remove"
    }

    fn additional_kwargs(&self) -> &HashMap<String, serde_json::Value> {
        &self.additional_kwargs
    }

    fn response_metadata(&self) -> &HashMap<String, serde_json::Value> {
        &self.response_metadata
    }
}

impl Serialize for RemoveMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut field_count = 4; // type, content, id, additional_kwargs, response_metadata
        if self.name.is_some() {
            field_count += 1;
        }
        field_count += 1; // response_metadata

        let mut map = serializer.serialize_map(Some(field_count))?;
        map.serialize_entry("type", "remove")?;
        map.serialize_entry("content", "")?;
        map.serialize_entry("id", &self.id)?;
        if self.name.is_some() {
            map.serialize_entry("name", &self.name)?;
        }
        map.serialize_entry("additional_kwargs", &self.additional_kwargs)?;
        map.serialize_entry("response_metadata", &self.response_metadata)?;

        map.end()
    }
}

#[bon]
impl RemoveMessage {
    #[builder]
    pub fn new(
        id: impl Into<String>,
        name: Option<String>,
        #[builder(default)] additional_kwargs: HashMap<String, serde_json::Value>,
        #[builder(default)] response_metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            id: id.into(),
            name,
            additional_kwargs,
            response_metadata,
        }
    }

    pub fn content_blocks(&self) -> Vec<ContentBlock> {
        vec![]
    }

    pub fn pretty_repr(&self, html: bool) -> String {
        let title = get_msg_title_repr("Remove Message", html);
        let name_line = if let Some(name) = &self.name {
            format!("\nName: {}", name)
        } else {
            String::new()
        };
        format!("{}{}\n\n{}", title, name_line, self.content())
    }

    pub fn pretty_print(&self) {
        println!("{}", self.pretty_repr(is_interactive_env()));
    }
}
