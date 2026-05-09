use bon::bon;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

#[cfg(feature = "specta")]
use specta_typescript::Unknown;

use super::base::{BaseMessage, get_msg_title_repr, is_interactive_env};
use super::content::{ContentBlock, ContentBlocks};

#[cfg(feature = "specta")]
type JsonObjectTs = HashMap<String, Unknown>;

static EMPTY_CONTENT_BLOCKS: LazyLock<ContentBlocks> = LazyLock::new(ContentBlocks::new);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RemoveMessage {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    #[cfg_attr(feature = "specta", specta(type = JsonObjectTs))]
    pub additional_kwargs: HashMap<String, serde_json::Value>,
    #[serde(default)]
    #[cfg_attr(feature = "specta", specta(type = JsonObjectTs))]
    pub response_metadata: HashMap<String, serde_json::Value>,
}

impl BaseMessage for RemoveMessage {
    fn id(&self) -> Option<String> {
        Some(self.id.clone())
    }

    fn content(&self) -> &ContentBlocks {
        &EMPTY_CONTENT_BLOCKS
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
        format!("{}{}\n\n", title, name_line)
    }

    pub fn pretty_print(&self) {
        println!("{}", self.pretty_repr(is_interactive_env()));
    }
}
