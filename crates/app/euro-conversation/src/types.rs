use agent_chain_core::BaseMessage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    id: Option<Uuid>,
    title: String,
    messages: Vec<BaseMessage>,

    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
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
