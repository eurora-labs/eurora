//! Data Transfer Objects for euro-personal-db.
//!
//! These DTOs are used for inserting and updating records in the database.

use chrono::{DateTime, Utc};

/// DTO for creating a new conversation.
#[derive(Debug, Clone, Default)]
pub struct NewConversation {
    pub title: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

/// DTO for updating an existing conversation.
#[derive(Debug, Clone)]
pub struct UpdateConversation {
    pub id: String,
    pub title: Option<String>,
}

/// DTO for creating a new message.
#[derive(Debug, Clone)]
pub struct NewMessage {
    pub id: Option<String>,
    pub conversation_id: String,
    pub message_type: String,
    /// JSON-encoded content
    pub content: String,
    /// For ToolMessage only
    pub tool_call_id: Option<String>,
    /// JSON-encoded tool calls for AIMessage
    pub tool_calls: Option<String>,
    /// JSON-encoded additional kwargs
    pub additional_kwargs: Option<String>,
    pub sequence_num: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// DTO for creating a new asset.
#[derive(Debug, Clone)]
pub struct NewAsset {
    pub id: Option<String>,
    pub activity_id: Option<String>,
    pub relative_path: String,
    pub absolute_path: String,
    /// If provided, creates a message_asset link
    pub message_id: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// DTO for creating a message-asset link.
#[derive(Debug, Clone)]
pub struct NewMessageAsset {
    pub message_id: String,
    pub asset_id: String,
}
