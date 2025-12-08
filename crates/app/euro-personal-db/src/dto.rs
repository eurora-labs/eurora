use chrono::{DateTime, Utc};

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

#[derive(Debug)]
pub struct NewChatMessage {
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub has_assets: bool,

    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct NewConversation {
    pub title: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct UpdateConversation {
    pub id: String,
    pub title: Option<String>,
}
