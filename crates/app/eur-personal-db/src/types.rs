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
    pub visible: bool,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Activity table structure
#[derive(FromRow, Debug)]
pub struct Activity {
    pub id: String,
    pub conversation_id: Option<String>,
    pub name: String,
    pub icon_path: Option<String>,
    pub process_name: String,
    pub start: String,
    pub end: Option<String>,
}

/// Activity asset table structure
#[derive(FromRow, Debug)]
pub struct ActivityAsset {
    pub id: String,
    pub activity_id: String,
    pub data: String, // JSON blob stored as text

    pub created_at: String,
    pub updated_at: String,
}

/// Video chunk table structure
#[derive(FromRow, Debug)]
pub struct VideoChunk {
    pub id: String,
    pub file_path: String,
}

/// Frame table structure
#[derive(FromRow, Debug)]
pub struct Frame {
    pub id: String,
    pub video_chunk_id: String,
    pub relative_index: i32,
}

/// Activity snapshot table structure
#[derive(FromRow, Debug)]
pub struct ActivitySnapshot {
    pub id: String,
    pub frame_id: String,
    pub activity_id: String,
}

/// Frame text table structure
#[derive(FromRow, Debug)]
pub struct FrameText {
    pub id: String,
    pub frame_id: String,
    pub text: String,
    pub text_json: Option<String>,
    pub ocr_engine: String,
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
