use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl std::fmt::Display for SortOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortOrder::Asc => write!(f, "ASC"),
            SortOrder::Desc => write!(f, "DESC"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    offset: u32,
    limit: u32,
    order: SortOrder,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self::new(0, 5, "DESC".to_string())
    }
}

impl PaginationParams {
    pub const MAX_LIMIT: u32 = 100;

    pub fn new(offset: u32, limit: u32, order: String) -> Self {
        let order = match order.to_lowercase().as_str() {
            "asc" => SortOrder::Asc,
            "desc" => SortOrder::Desc,
            _ => panic!("Invalid sort order"),
        };
        Self {
            offset,
            limit: limit.min(Self::MAX_LIMIT),
            order,
        }
    }

    pub fn offset(&self) -> i64 {
        self.offset as i64
    }

    pub fn limit(&self) -> i64 {
        self.limit as i64
    }

    pub fn order(&self) -> &SortOrder {
        &self.order
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PasswordCredentials {
    pub id: Uuid,
    pub user_id: Uuid,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub password_hash: Option<String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "oauth_provider", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum OAuthProvider {
    Google,
    Github,
}

impl std::fmt::Display for OAuthProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuthProvider::Google => write!(f, "google"),
            OAuthProvider::Github => write!(f, "github"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OAuthCredentials {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: OAuthProvider,
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
    pub token_hash: Vec<u8>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOAuthCredentials {
    pub user_id: Uuid,
    pub provider: OAuthProvider,
    pub provider_user_id: String,
    pub access_token: Option<Vec<u8>>,
    pub refresh_token: Option<Vec<u8>>,
    pub access_token_expiry: Option<DateTime<Utc>>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRefreshToken {
    pub user_id: Uuid,
    pub token_hash: Vec<u8>,
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
    #[serde(skip_serializing)]
    pub pkce_verifier: Vec<u8>,
    pub redirect_uri: String,
    pub ip_address: Option<ipnet::IpNet>,
    pub consumed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    #[serde(skip_serializing)]
    pub nonce: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOAuthState {
    pub state: String,
    pub pkce_verifier: Vec<u8>,
    pub redirect_uri: String,
    pub ip_address: Option<ipnet::IpNet>,
    pub expires_at: DateTime<Utc>,
    pub nonce: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LoginToken {
    pub id: Uuid,
    #[serde(skip_serializing)]
    pub token_hash: Vec<u8>,
    pub consumed: bool,
    pub expires_at: DateTime<Utc>,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLoginToken {
    pub token_hash: Vec<u8>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[sqlx(type_name = "asset_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AssetStatus {
    Pending,
    #[default]
    Uploaded,
    Processing,
    Ready,
    Failed,
    Deleted,
}

impl std::fmt::Display for AssetStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetStatus::Pending => write!(f, "pending"),
            AssetStatus::Uploaded => write!(f, "uploaded"),
            AssetStatus::Processing => write!(f, "processing"),
            AssetStatus::Ready => write!(f, "ready"),
            AssetStatus::Failed => write!(f, "failed"),
            AssetStatus::Deleted => write!(f, "deleted"),
        }
    }
}

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
    pub status: AssetStatus,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAsset {
    pub id: Option<Uuid>,
    pub user_id: Uuid,
    pub name: String,
    pub checksum_sha256: Option<Vec<u8>>,
    pub size_bytes: Option<i64>,
    pub storage_uri: String,
    pub storage_backend: String,
    pub mime_type: String,
    pub status: Option<AssetStatus>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAsset {
    pub checksum_sha256: Option<Vec<u8>>,
    pub size_bytes: Option<i64>,
    pub storage_uri: Option<String>,
    pub mime_type: Option<String>,
    pub status: Option<AssetStatus>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MessageAsset {
    pub message_id: Uuid,
    pub asset_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ActivityAsset {
    pub activity_id: Uuid,
    pub asset_id: Uuid,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// Activity Types
// =============================================================================

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListActivities {
    pub user_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetActivitiesByTimeRange {
    pub user_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateActivityEndTime {
    pub activity_id: Uuid,
    pub user_id: Uuid,
    pub ended_at: DateTime<Utc>,
}

// =============================================================================
// Conversation Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Conversation {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewConversation {
    pub id: Option<Uuid>,
    pub user_id: Uuid,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetConversation {
    pub id: Uuid,
    pub user_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateConversation {
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListConversations {
    pub user_id: Uuid,
}

// =============================================================================
// Message Types
// =============================================================================

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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub user_id: Uuid,
    pub message_type: MessageType,
    pub content: serde_json::Value,
    pub tool_call_id: Option<String>,
    pub tool_calls: Option<serde_json::Value>,
    pub additional_kwargs: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateMessage {
    pub content: Option<serde_json::Value>,
    pub tool_call_id: Option<String>,
    pub tool_calls: Option<serde_json::Value>,
    pub additional_kwargs: Option<serde_json::Value>,
}

// =============================================================================
// Junction Types (Activity-Conversation)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ActivityConversation {
    pub activity_id: Uuid,
    pub conversation_id: Uuid,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// Billing Types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AccountBillingState {
    pub account_id: Uuid,
    pub stripe_subscription_id: Option<String>,
    pub status: Option<String>,
    pub current_period_start: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub cancel_at_period_end: Option<bool>,
    pub plan_id: Option<String>,
    pub plan_name: Option<String>,
    pub max_users: Option<i32>,
    pub max_projects: Option<i32>,
}
