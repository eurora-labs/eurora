use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
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
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub password_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub email_verified: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePasswordRequest {
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
pub struct CreateOAuthCredentialsRequest {
    pub user_id: Uuid,
    pub provider: String,
    pub provider_user_id: String,
    pub access_token: Option<Vec<u8>>,
    pub refresh_token: Option<Vec<u8>>,
    pub access_token_expiry: Option<DateTime<Utc>>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRefreshTokenRequest {
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateOAuthCredentialsRequest {
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
pub struct CreateOAuthStateRequest {
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
pub struct CreateLoginTokenRequest {
    pub token: String,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLoginTokenRequest {
    pub user_id: Uuid,
}
