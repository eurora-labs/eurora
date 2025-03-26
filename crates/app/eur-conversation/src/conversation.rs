use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub conversation_id: String,
    pub asset_type: String,
    pub content: Value,
    pub created_at: u64,
    pub updated_at: u64,
}

impl Asset {
    pub fn new(conversation_id: String, asset_type: String, content: Value) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id: Uuid::new_v4().to_string(),
            conversation_id,
            asset_type,
            content,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub visible: bool,
    pub created_at: u64,
    pub updated_at: u64,
}

impl Conversation {
    pub fn new(id: Option<String>, title: Option<String>) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id: id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            title: title.unwrap_or_else(|| "New Conversation".to_string()),
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_message(&mut self, message: ChatMessage) -> Result<(), anyhow::Error> {
        self.messages.push(message);
        self.updated_at = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Ok(())
    }

    pub fn last_message(&self) -> Option<&ChatMessage> {
        self.messages.last()
    }
}

impl ChatMessage {
    pub fn new(id: Option<String>, role: String, content: String, visible: bool) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id: id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            role,
            content,
            visible,
            created_at: now,
            updated_at: now,
        }
    }
}
