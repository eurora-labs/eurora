use agent_chain_core::BaseMessage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    id: Option<Uuid>,
    title: String,
    messages: Vec<BaseMessage>,

    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Conversation {
    pub fn id(&self) -> Option<Uuid> {
        self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn set_id(&mut self, id: Uuid) -> Result<()> {
        if self.id.is_none() {
            self.id = Some(id);
            return Ok(());
        }
        Err(Error::SetId("Conversation ID is already set".to_string()))
    }
}

impl Default for Conversation {
    fn default() -> Self {
        let title = "New Chat".to_string();
        let created_at = Utc::now();
        let updated_at = created_at;

        Self {
            id: None,
            title,
            messages: Vec::new(),
            created_at,
            updated_at,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConversationEvent {
    NewConversation { id: Option<Uuid>, title: String },
}
