use chrono::{DateTime, Utc};
use ferrous_llm_core::{Message, MessageContent, Role};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::FromRow;

#[derive(Debug)]
pub struct NewAsset {
    pub id: Option<String>,
    pub activity_id: Option<String>,
    pub relative_path: String,
    pub absolute_path: String,
    pub chat_message_id: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct NewChatMessageAsset {
    pub chat_message_id: String,
    pub asset_id: String,
}
