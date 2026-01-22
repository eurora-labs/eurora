use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PasswordCredentials {
    pub id: Uuid,
    pub user_id: Uuid,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub password_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUser {
    pub id: Uuid,
    pub username: Option<String>,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub email_verified: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePassword {
    pub password_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OAuthCredentials {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub provider_user_id: String,
    pub access_token: Option<Vec<u8>>,
    pub refresh_token: Option<Vec<u8>>,
    pub access_token_expiry: Option<DateTime<Utc>>,
    pub scope: Option<String>,
    pub issued_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    #[serde(skip_serializing)]
    pub token_hash: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOAuthCredentials {
    pub user_id: Uuid,
    pub provider: String,
    pub provider_user_id: String,
    pub access_token: Option<Vec<u8>>,
    pub refresh_token: Option<Vec<u8>>,
    pub access_token_expiry: Option<DateTime<Utc>>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRefreshToken {
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOAuthCredentials {
    pub access_token: Option<Vec<u8>>,
    pub refresh_token: Option<Vec<u8>>,
    pub access_token_expiry: Option<DateTime<Utc>>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OAuthState {
    pub id: Uuid,
    pub state: String,
    pub pkce_verifier: String,
    pub redirect_uri: String,
    pub ip_address: Option<ipnet::IpNet>,
    pub consumed: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOAuthState {
    pub state: String,
    pub pkce_verifier: String,
    pub redirect_uri: String,
    pub ip_address: Option<ipnet::IpNet>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LoginToken {
    pub id: Uuid,
    pub token: String,
    pub consumed: bool,
    pub expires_at: DateTime<Utc>,
    pub user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLoginToken {
    pub token: String,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLoginToken {
    pub user_id: Uuid,
}

// =============================================================================
// Asset Types
// =============================================================================

/// Represents a file asset (screenshots, attachments, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Asset {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub mime_type: String,
    pub size_bytes: Option<i64>,
    pub checksum_sha256: Option<Vec<u8>>,
    pub storage_backend: String,
    pub storage_uri: String,
    pub status: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request for creating a new asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAsset {
    pub id: Uuid,
    pub name: String,
    pub checksum_sha256: Option<Vec<u8>>,
    pub size_bytes: Option<i64>,
    pub storage_uri: String,
    pub mime_type: String,
    pub metadata: Option<serde_json::Value>,
}

/// Request for updating an asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAsset {
    pub checksum_sha256: Option<Vec<u8>>,
    pub size_bytes: Option<i64>,
    pub storage_uri: Option<String>,
    pub mime_type: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Message-asset link
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MessageAsset {
    pub message_id: Uuid,
    pub asset_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Activity-asset link
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ActivityAsset {
    pub activity_id: Uuid,
    pub asset_id: Uuid,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// Activity Types
// =============================================================================

/// Represents a user activity (application/process usage)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Activity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub icon_asset_id: Option<Uuid>,
    pub process_name: String,
    pub window_title: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request for creating a new activity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewActivity {
    pub id: Option<Uuid>,
    pub user_id: Uuid,
    pub name: String,
    pub icon_asset_id: Option<Uuid>,
    pub process_name: String,
    pub window_title: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

/// Request for updating an existing activity
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateActivity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: Option<String>,
    pub icon_asset_id: Option<Uuid>,
    pub process_name: Option<String>,
    pub window_title: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
}

/// Request for listing activities with pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListActivities {
    pub user_id: Uuid,
    pub limit: u32,
    pub offset: u32,
}

/// Request for getting activities by time range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetActivitiesByTimeRange {
    pub user_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub limit: u32,
    pub offset: u32,
}

/// Request for updating activity end time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateActivityEndTime {
    pub activity_id: Uuid,
    pub user_id: Uuid,
    pub ended_at: DateTime<Utc>,
}

// =============================================================================
// Conversation Types
// =============================================================================

/// Database representation of a conversation
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Conversation {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request for creating a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewConversation {
    pub id: Option<Uuid>,
    pub user_id: Uuid,
    pub title: String,
}

/// Request for getting a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetConversation {
    pub id: Uuid,
    pub user_id: Uuid,
}

/// Request for updating a conversation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateConversation {
    pub title: Option<String>,
}

/// Request for listing conversations with pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListConversations {
    pub user_id: Uuid,
    pub limit: u32,
    pub offset: u32,
}

// =============================================================================
// Message Types
// =============================================================================

/// Message type enum matching the database enum
/// Corresponds to agent-chain BaseMessage variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "message_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    Human,
    System,
    Ai,
    Tool,
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageType::Human => write!(f, "human"),
            MessageType::System => write!(f, "system"),
            MessageType::Ai => write!(f, "ai"),
            MessageType::Tool => write!(f, "tool"),
        }
    }
}

/// Database representation of a message within a conversation
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub user_id: Uuid,
    pub message_type: MessageType,
    /// Content stored as JSONB
    /// For human: MessageContent (can be {"Text": "..."} or {"Parts": [...]})
    /// For system/ai/tool: Simple string stored as JSON string
    pub content: serde_json::Value,
    /// For ToolMessage: the ID of the tool call this responds to
    pub tool_call_id: Option<String>,
    /// For AIMessage: JSON array of ToolCall objects
    pub tool_calls: Option<serde_json::Value>,
    /// Additional metadata as JSON object
    pub additional_kwargs: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request for creating a new message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMessage {
    pub id: Option<Uuid>,
    pub conversation_id: Uuid,
    pub user_id: Uuid,
    pub message_type: MessageType,
    pub content: serde_json::Value,
    pub tool_call_id: Option<String>,
    pub tool_calls: Option<serde_json::Value>,
    pub additional_kwargs: Option<serde_json::Value>,
}

/// Request for updating an existing message
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateMessage {
    pub content: Option<serde_json::Value>,
    pub tool_call_id: Option<String>,
    pub tool_calls: Option<serde_json::Value>,
    pub additional_kwargs: Option<serde_json::Value>,
}

/// Request for listing messages with pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListMessages {
    pub conversation_id: Uuid,
    pub user_id: Uuid,
    pub limit: u32,
    pub offset: u32,
}

// =============================================================================
// Junction Types (Activity-Conversation)
// =============================================================================

/// Activity-conversation link
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ActivityConversation {
    pub activity_id: Uuid,
    pub conversation_id: Uuid,
    pub created_at: DateTime<Utc>,
}
