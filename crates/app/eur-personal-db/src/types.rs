use chrono::{DateTime, Utc};
use ferrous_llm_core::{Message, MessageContent, Role};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::FromRow;

#[derive(FromRow, Debug, Serialize, Deserialize, Type, Clone)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(FromRow, Debug, Serialize, Deserialize, Type)]
pub struct ChatMessage {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,

    pub has_assets: bool,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Activity table structure
#[derive(FromRow, Debug, Clone)]
pub struct Activity {
    pub id: String,
    pub name: String,
    pub icon_path: Option<String>,
    pub process_name: String,
    pub started_at: String,
    pub ended_at: Option<String>,
}

/// Activity conversation table structure
#[derive(Clone, Debug)]
pub struct ActivityConversation {
    pub activity_id: String,
    pub conversation_id: String,
    pub created_at: String,
}

/// Conversation with activities
#[derive(Clone, Debug)]
pub struct ConversationWithActivity {
    pub conversation: Conversation,
    pub activities: Vec<Activity>,
}

/// Activity asset table structure
#[derive(FromRow, Debug, Clone)]
pub struct Asset {
    pub id: String,
    pub activity_id: Option<String>,
    pub relative_path: String,
    pub absolute_path: String,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(FromRow, Debug)]
pub struct ChatMessageAsset {
    pub chat_message_id: String,
    pub asset_id: String,
    pub created_at: String,
}

impl From<ChatMessage> for Message {
    fn from(value: ChatMessage) -> Self {
        Message {
            role: match value.role.as_str() {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                _ => Role::System,
            },
            content: MessageContent::Text(value.content),
        }
    }
}
