use bon::bon;
use chrono::{DateTime, Utc};
use sqlx::{
    migrate::MigrateDatabase,
    postgres::{PgPool, PgPoolOptions},
};
use std::time::Duration;
use uuid::Uuid;

use crate::{
    MessageType, PaginationParams,
    error::{DbError, DbResult},
    types::{
        Activity, ActivityAsset, Asset, AssetStatus, LoginToken, Message, OAuthCredentials,
        OAuthProvider, OAuthState, PasswordCredentials, RefreshToken, SearchResultMessage,
        SearchResultThread, Thread, TokenUsage, User,
    },
};

pub const DEFAULT_TOKEN_LIMIT: i64 = 50_000;

fn build_prefix_tsquery(query: &str) -> Option<String> {
    let terms: Vec<String> = query
        .split_whitespace()
        .filter_map(|w| {
            let sanitized: String = w.chars().filter(|c| c.is_alphanumeric()).collect();
            if sanitized.is_empty() {
                None
            } else {
                Some(format!("{}:*", sanitized))
            }
        })
        .collect();

    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" & "))
    }
}

fn regex_escape(query: &str) -> String {
    let mut escaped = String::with_capacity(query.len() * 2);
    for c in query.chars() {
        if "\\.*+?()[]{}|^$".contains(c) {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

#[derive(Debug)]
pub struct DatabaseManager {
    pub pool: PgPool,
}

#[bon]
impl DatabaseManager {
    pub async fn new(database_url: &str) -> DbResult<Self> {
        if !sqlx::Postgres::database_exists(database_url).await? {
            sqlx::Postgres::create_database(database_url).await?;
        }

        let pool = PgPoolOptions::new()
            .max_connections(50)
            .min_connections(3)
            .acquire_timeout(Duration::from_secs(10))
            .connect(database_url)
            .await?;

        let db_manager = DatabaseManager { pool };

        Self::run_migrations(&db_manager.pool).await?;

        Ok(db_manager)
    }

    async fn run_migrations(pool: &PgPool) -> DbResult<()> {
        let migrator = sqlx::migrate!("./src/migrations");
        migrator.run(pool).await?;
        Ok(())
    }

    #[builder]
    pub async fn create_user(
        &self,
        email: String,
        display_name: Option<String>,
        password_hash: Option<String>,
    ) -> DbResult<User> {
        let user_id = Uuid::now_v7();
        let now = Utc::now();

        let mut tx = self.pool.begin().await?;

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, email, display_name, email_verified, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, email, display_name, email_verified, created_at, updated_at
            "#,
        )
        .bind(user_id)
        .bind(&email)
        .bind(&display_name)
        .bind(false)
        .bind(now)
        .bind(now)
        .fetch_one(&mut *tx)
        .await?;

        if let Some(ref password_hash) = password_hash {
            sqlx::query(
                r#"
                INSERT INTO password_credentials (user_id, password_hash, created_at, updated_at)
                VALUES ($1, $2, $3, $4)
                "#,
            )
            .bind(user_id)
            .bind(password_hash)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(user)
    }

    #[builder]
    pub async fn create_user_with_oauth(
        &self,
        email: String,
        display_name: Option<String>,
        email_verified: bool,
        provider: OAuthProvider,
        provider_user_id: String,
        access_token: Option<Vec<u8>>,
        refresh_token: Option<Vec<u8>>,
        access_token_expiry: Option<DateTime<Utc>>,
        scope: Option<String>,
    ) -> DbResult<User> {
        let user_id = Uuid::now_v7();
        let oauth_cred_id = Uuid::now_v7();
        let now = Utc::now();

        let mut tx = self.pool.begin().await?;

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, email, display_name, email_verified, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, email, display_name, email_verified, created_at, updated_at
            "#,
        )
        .bind(user_id)
        .bind(&email)
        .bind(&display_name)
        .bind(email_verified)
        .bind(now)
        .bind(now)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO oauth_credentials (
                id, user_id, provider, provider_user_id, access_token,
                refresh_token, access_token_expiry, scope, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(oauth_cred_id)
        .bind(user_id)
        .bind(provider)
        .bind(&provider_user_id)
        .bind(&access_token)
        .bind(&refresh_token)
        .bind(access_token_expiry)
        .bind(&scope)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(user)
    }

    #[builder]
    pub async fn get_user(&self, id: Option<Uuid>, email: Option<String>) -> DbResult<User> {
        let (clause, bind_value) = match (id, email) {
            (Some(id), _) => ("id = $1::uuid", id.to_string()),
            (_, Some(email)) => ("email = $1", email),
            _ => {
                return Err(DbError::Internal(
                    "get_user requires at least one filter".into(),
                ));
            }
        };

        let query = format!(
            "SELECT id, email, display_name, email_verified, created_at, updated_at FROM users WHERE {clause}"
        );

        let user = sqlx::query_as::<_, User>(&query)
            .bind(bind_value)
            .fetch_one(&self.pool)
            .await?;

        Ok(user)
    }

    #[builder]
    pub async fn get_password_credentials(&self, user_id: Uuid) -> DbResult<PasswordCredentials> {
        let credentials = sqlx::query_as::<_, PasswordCredentials>(
            r#"
            SELECT user_id, password_hash, created_at, updated_at
            FROM password_credentials
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(credentials)
    }

    #[builder]
    pub async fn user_exists_by_email(&self, email: &str) -> DbResult<bool> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM users WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0 > 0)
    }

    #[builder]
    pub async fn create_oauth_credentials(
        &self,
        user_id: Uuid,
        provider: OAuthProvider,
        provider_user_id: String,
        access_token: Option<Vec<u8>>,
        refresh_token: Option<Vec<u8>>,
        access_token_expiry: Option<DateTime<Utc>>,
        scope: Option<String>,
    ) -> DbResult<OAuthCredentials> {
        let id = Uuid::now_v7();
        let now = Utc::now();

        let oauth_creds = sqlx::query_as::<_, OAuthCredentials>(
            r#"
            INSERT INTO oauth_credentials (
                id, user_id, provider, provider_user_id, access_token,
                refresh_token, access_token_expiry, scope, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, user_id, provider, provider_user_id, access_token,
                      refresh_token, access_token_expiry, scope, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(provider)
        .bind(&provider_user_id)
        .bind(&access_token)
        .bind(&refresh_token)
        .bind(access_token_expiry)
        .bind(&scope)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(oauth_creds)
    }

    #[builder]
    pub async fn get_oauth_credentials_by_provider_and_user(
        &self,
        provider: OAuthProvider,
        user_id: Uuid,
    ) -> DbResult<OAuthCredentials> {
        let oauth_creds = sqlx::query_as::<_, OAuthCredentials>(
            r#"
            SELECT id, user_id, provider, provider_user_id, access_token,
                   refresh_token, access_token_expiry, scope, created_at, updated_at
            FROM oauth_credentials
            WHERE provider = $1 AND user_id = $2
            "#,
        )
        .bind(provider)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(oauth_creds)
    }

    #[builder]
    pub async fn get_oauth_provider_for_user(
        &self,
        user_id: Uuid,
    ) -> DbResult<Option<OAuthProvider>> {
        let result: Option<(OAuthProvider,)> = sqlx::query_as(
            r#"
            SELECT provider FROM oauth_credentials WHERE user_id = $1 LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|(p,)| p))
    }

    #[builder]
    pub async fn update_oauth_credentials(
        &self,
        id: Uuid,
        access_token: Option<Vec<u8>>,
        refresh_token: Option<Vec<u8>>,
        access_token_expiry: Option<DateTime<Utc>>,
        scope: Option<String>,
    ) -> DbResult<OAuthCredentials> {
        let now = Utc::now();

        let oauth_creds = sqlx::query_as::<_, OAuthCredentials>(
            r#"
            UPDATE oauth_credentials
            SET access_token = COALESCE($2, access_token),
                refresh_token = COALESCE($3, refresh_token),
                access_token_expiry = COALESCE($4, access_token_expiry),
                scope = COALESCE($5, scope),
                updated_at = $6
            WHERE id = $1
            RETURNING id, user_id, provider, provider_user_id, access_token,
                      refresh_token, access_token_expiry, scope, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&access_token)
        .bind(&refresh_token)
        .bind(access_token_expiry)
        .bind(&scope)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(oauth_creds)
    }

    #[builder]
    pub async fn get_user_by_oauth_provider(
        &self,
        provider: OAuthProvider,
        provider_user_id: &str,
    ) -> DbResult<User> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT u.id, u.email, u.display_name, u.email_verified, u.created_at, u.updated_at
            FROM users u
            INNER JOIN oauth_credentials oc ON u.id = oc.user_id
            WHERE oc.provider = $1 AND oc.provider_user_id = $2
            "#,
        )
        .bind(provider)
        .bind(provider_user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    #[builder]
    pub async fn create_refresh_token(
        &self,
        user_id: Uuid,
        token_hash: Vec<u8>,
        expires_at: DateTime<Utc>,
    ) -> DbResult<RefreshToken> {
        let id = Uuid::now_v7();
        let now = Utc::now();

        let refresh_token = sqlx::query_as::<_, RefreshToken>(
            r#"
            INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, revoked, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, user_id, token_hash, expires_at, revoked, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&token_hash)
        .bind(expires_at)
        .bind(false)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(refresh_token)
    }

    #[builder]
    pub async fn get_refresh_token_by_hash(&self, token_hash: &[u8]) -> DbResult<RefreshToken> {
        let refresh_token = sqlx::query_as::<_, RefreshToken>(
            r#"
            SELECT id, user_id, token_hash, expires_at, revoked, created_at, updated_at
            FROM refresh_tokens
            WHERE token_hash = $1 AND revoked = false AND expires_at > now()
            "#,
        )
        .bind(token_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(refresh_token)
    }

    #[builder]
    pub async fn revoke_refresh_token(&self, token_hash: &[u8]) -> DbResult<RefreshToken> {
        let now = Utc::now();

        let refresh_token = sqlx::query_as::<_, RefreshToken>(
            r#"
            UPDATE refresh_tokens
            SET revoked = true, updated_at = $2
            WHERE token_hash = $1 AND revoked = false
            RETURNING id, user_id, token_hash, expires_at, revoked, created_at, updated_at
            "#,
        )
        .bind(token_hash)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(refresh_token)
    }

    #[builder]
    pub async fn create_oauth_state(
        &self,
        state: String,
        pkce_verifier: Vec<u8>,
        redirect_uri: String,
        ip_address: Option<ipnet::IpNet>,
        expires_at: DateTime<Utc>,
        nonce: Vec<u8>,
    ) -> DbResult<OAuthState> {
        let id = Uuid::now_v7();
        let now = Utc::now();

        let oauth_state = sqlx::query_as::<_, OAuthState>(
            r#"
            INSERT INTO oauth_state (id, state, pkce_verifier, redirect_uri, ip_address, consumed, created_at, updated_at, expires_at, nonce)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, state, pkce_verifier, redirect_uri, ip_address, consumed, created_at, updated_at, expires_at, nonce
            "#,
        )
        .bind(id)
        .bind(&state)
        .bind(&pkce_verifier)
        .bind(&redirect_uri)
        .bind(ip_address)
        .bind(false)
        .bind(now)
        .bind(now)
        .bind(expires_at)
        .bind(&nonce)
        .fetch_one(&self.pool)
        .await?;

        Ok(oauth_state)
    }

    #[builder]
    pub async fn get_oauth_state_by_state(&self, state: &str) -> DbResult<OAuthState> {
        let oauth_state = sqlx::query_as::<_, OAuthState>(
            r#"
            SELECT id, state, pkce_verifier, redirect_uri, ip_address, consumed, created_at, updated_at, expires_at, nonce
            FROM oauth_state
            WHERE state = $1 AND consumed = false AND expires_at > now()
            "#,
        )
        .bind(state)
        .fetch_one(&self.pool)
        .await?;

        Ok(oauth_state)
    }

    #[builder]
    pub async fn consume_oauth_state(&self, state: &str) -> DbResult<OAuthState> {
        let now = Utc::now();

        let oauth_state = sqlx::query_as::<_, OAuthState>(
            r#"
            UPDATE oauth_state
            SET consumed = true, updated_at = $2
            WHERE state = $1 AND consumed = false AND expires_at > now()
            RETURNING id, state, pkce_verifier, redirect_uri, ip_address, consumed, created_at, updated_at, expires_at, nonce
            "#,
        )
        .bind(state)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(oauth_state)
    }

    #[builder]
    pub async fn create_login_token(
        &self,
        token_hash: Vec<u8>,
        user_id: Uuid,
        expires_at: DateTime<Utc>,
    ) -> DbResult<LoginToken> {
        let id = Uuid::now_v7();
        let now = Utc::now();

        let login_token = sqlx::query_as::<_, LoginToken>(
            r#"
            INSERT INTO login_tokens (id, token_hash, expires_at, user_id, consumed, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, token_hash, consumed, expires_at, user_id, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&token_hash)
        .bind(expires_at)
        .bind(user_id)
        .bind(false)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(login_token)
    }

    #[builder]
    pub async fn get_login_token_by_hash(&self, token_hash: &[u8]) -> DbResult<LoginToken> {
        let login_token = sqlx::query_as::<_, LoginToken>(
            r#"
            SELECT id, token_hash, consumed, expires_at, user_id, created_at, updated_at
            FROM login_tokens
            WHERE token_hash = $1 AND consumed = false AND expires_at > now()
            "#,
        )
        .bind(token_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(login_token)
    }

    #[builder]
    pub async fn get_login_token_by_hash_any(&self, token_hash: &[u8]) -> DbResult<LoginToken> {
        let login_token = sqlx::query_as::<_, LoginToken>(
            r#"
            SELECT id, token_hash, consumed, expires_at, user_id, created_at, updated_at
            FROM login_tokens
            WHERE token_hash = $1 AND expires_at > now()
            "#,
        )
        .bind(token_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(login_token)
    }

    #[builder]
    pub async fn cleanup_expired_auth_data(&self) -> DbResult<()> {
        let deleted_states = sqlx::query_scalar::<_, i64>(
            "WITH deleted AS (DELETE FROM oauth_state WHERE expires_at < now() - interval '1 hour' RETURNING 1) SELECT count(*) FROM deleted",
        )
        .fetch_one(&self.pool)
        .await?;

        let deleted_login_tokens = sqlx::query_scalar::<_, i64>(
            "WITH deleted AS (DELETE FROM login_tokens WHERE expires_at < now() - interval '1 hour' RETURNING 1) SELECT count(*) FROM deleted",
        )
        .fetch_one(&self.pool)
        .await?;

        let deleted_refresh_tokens = sqlx::query_scalar::<_, i64>(
            "WITH deleted AS (DELETE FROM refresh_tokens WHERE revoked = true AND created_at < now() - interval '30 days' RETURNING 1) SELECT count(*) FROM deleted",
        )
        .fetch_one(&self.pool)
        .await?;

        if deleted_states > 0 || deleted_login_tokens > 0 || deleted_refresh_tokens > 0 {
            tracing::info!(
                "Cleaned up expired auth data: {} oauth_states, {} login_tokens, {} refresh_tokens",
                deleted_states,
                deleted_login_tokens,
                deleted_refresh_tokens,
            );
        }

        Ok(())
    }

    #[builder]
    pub async fn create_activity(
        &self,
        id: Option<Uuid>,
        user_id: Uuid,
        name: String,
        icon_asset_id: Option<Uuid>,
        process_name: String,
        window_title: String,
        started_at: DateTime<Utc>,
        ended_at: Option<DateTime<Utc>>,
    ) -> DbResult<Activity> {
        let id = id.unwrap_or_else(Uuid::now_v7);
        let now = Utc::now();

        let activity = sqlx::query_as::<_, Activity>(
            r#"
            INSERT INTO activities (id, user_id, name, icon_asset_id, process_name, window_title, started_at, ended_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, user_id, name, icon_asset_id, process_name, window_title, started_at, ended_at, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&name)
        .bind(icon_asset_id)
        .bind(&process_name)
        .bind(&window_title)
        .bind(started_at)
        .bind(ended_at)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(activity)
    }

    #[builder]
    pub async fn get_activity_for_user(
        &self,
        activity_id: Uuid,
        user_id: Uuid,
    ) -> DbResult<Activity> {
        let activity = sqlx::query_as::<_, Activity>(
            r#"
            SELECT id, user_id, name, icon_asset_id, process_name, window_title, started_at, ended_at, created_at, updated_at
            FROM activities
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(activity_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(activity)
    }

    #[builder]
    pub async fn list_activities(
        &self,
        user_id: Uuid,
        params: PaginationParams,
    ) -> DbResult<Vec<Activity>> {
        let query = format!(
            r#"
            SELECT id, user_id, name, icon_asset_id, process_name, window_title, started_at, ended_at, created_at, updated_at
            FROM activities
            WHERE user_id = $1
            ORDER BY started_at {}
            LIMIT $2 OFFSET $3
            "#,
            params.order()
        );

        let activities = sqlx::query_as::<_, Activity>(&query)
            .bind(user_id)
            .bind(params.limit())
            .bind(params.offset())
            .fetch_all(&self.pool)
            .await?;

        Ok(activities)
    }

    #[builder]
    pub async fn update_activity(
        &self,
        id: Uuid,
        user_id: Uuid,
        name: Option<String>,
        icon_asset_id: Option<Uuid>,
        process_name: Option<String>,
        window_title: Option<String>,
        started_at: Option<DateTime<Utc>>,
        ended_at: Option<DateTime<Utc>>,
    ) -> DbResult<Activity> {
        let now = Utc::now();

        let activity = sqlx::query_as::<_, Activity>(
            r#"
            UPDATE activities
            SET name = COALESCE($3, name),
                icon_asset_id = COALESCE($4, icon_asset_id),
                process_name = COALESCE($5, process_name),
                window_title = COALESCE($6, window_title),
                started_at = COALESCE($7, started_at),
                ended_at = COALESCE($8, ended_at),
                updated_at = $9
            WHERE id = $1 AND user_id = $2
            RETURNING id, user_id, name, icon_asset_id, process_name, window_title, started_at, ended_at, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&name)
        .bind(icon_asset_id)
        .bind(&process_name)
        .bind(&window_title)
        .bind(started_at)
        .bind(ended_at)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(activity)
    }

    #[builder]
    pub async fn update_activity_end_time(
        &self,
        activity_id: Uuid,
        user_id: Uuid,
        ended_at: DateTime<Utc>,
    ) -> DbResult<()> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE activities
            SET ended_at = $3, updated_at = $4
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(activity_id)
        .bind(user_id)
        .bind(ended_at)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[builder]
    pub async fn get_last_active_activity(&self, user_id: Uuid) -> DbResult<Option<Activity>> {
        let activity = sqlx::query_as::<_, Activity>(
            r#"
            SELECT id, user_id, name, icon_asset_id, process_name, window_title, started_at, ended_at, created_at, updated_at
            FROM activities
            WHERE user_id = $1 AND ended_at IS NULL
            ORDER BY started_at DESC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(activity)
    }

    #[builder]
    pub async fn delete_activity(&self, activity_id: Uuid, user_id: Uuid) -> DbResult<Activity> {
        let activity = sqlx::query_as::<_, Activity>(
            r#"
            DELETE FROM activities
            WHERE id = $1 AND user_id = $2
            RETURNING id, user_id, name, icon_asset_id, process_name, window_title, started_at, ended_at, created_at, updated_at
            "#,
        )
        .bind(activity_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(activity)
    }

    #[builder]
    pub async fn get_activities_by_time_range(
        &self,
        user_id: Uuid,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        params: PaginationParams,
    ) -> DbResult<Vec<Activity>> {
        let query = format!(
            r#"
            SELECT id, user_id, name, icon_asset_id, process_name, window_title, started_at, ended_at, created_at, updated_at
            FROM activities
            WHERE user_id = $1
              AND started_at >= $2
              AND started_at <= $3
            ORDER BY started_at {}
            LIMIT $4 OFFSET $5
            "#,
            params.order()
        );

        let activities = sqlx::query_as::<_, Activity>(&query)
            .bind(user_id)
            .bind(start_time)
            .bind(end_time)
            .bind(params.limit())
            .bind(params.offset())
            .fetch_all(&self.pool)
            .await?;

        Ok(activities)
    }

    #[builder]
    pub async fn create_asset(
        &self,
        id: Option<Uuid>,
        user_id: Uuid,
        name: String,
        mime_type: String,
        size_bytes: Option<i64>,
        checksum_sha256: Option<Vec<u8>>,
        storage_backend: String,
        storage_uri: String,
        status: Option<AssetStatus>,
        metadata: Option<serde_json::Value>,
    ) -> DbResult<Asset> {
        let id = id.unwrap_or_else(Uuid::now_v7);
        let now = Utc::now();
        let metadata = metadata.unwrap_or_else(|| serde_json::json!({}));
        let status = status.unwrap_or(AssetStatus::Uploaded);

        let asset = sqlx::query_as::<_, Asset>(
            r#"
            INSERT INTO assets (id, user_id, name, mime_type, size_bytes, checksum_sha256, storage_backend, storage_uri, status, metadata, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING id, user_id, name, mime_type, size_bytes, checksum_sha256, storage_backend, storage_uri, status, metadata, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&name)
        .bind(&mime_type)
        .bind(size_bytes)
        .bind(&checksum_sha256)
        .bind(&storage_backend)
        .bind(&storage_uri)
        .bind(status)
        .bind(&metadata)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(asset)
    }

    #[builder]
    pub async fn get_asset(&self, id: Uuid) -> DbResult<Asset> {
        let asset = sqlx::query_as::<_, Asset>(
            r#"
            SELECT id, user_id, name, mime_type, size_bytes, checksum_sha256,
                   storage_backend, storage_uri, status, metadata, created_at, updated_at
            FROM assets
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(asset)
    }

    #[builder]
    pub async fn link_asset_to_activity(
        &self,
        activity_id: Uuid,
        asset_id: Uuid,
        user_id: Uuid,
    ) -> DbResult<ActivityAsset> {
        let now = Utc::now();

        let activity_asset = sqlx::query_as::<_, ActivityAsset>(
            r#"
            WITH verified_activity AS (
                SELECT id FROM activities WHERE id = $1 AND user_id = $3
            ),
            verified_asset AS (
                SELECT id FROM assets WHERE id = $2 AND user_id = $3
            )
            INSERT INTO activity_assets (activity_id, asset_id, created_at)
            SELECT va.id, vas.id, $4
            FROM verified_activity va, verified_asset vas
            RETURNING activity_id, asset_id, created_at
            "#,
        )
        .bind(activity_id)
        .bind(asset_id)
        .bind(user_id)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(activity_asset)
    }

    #[builder]
    pub async fn create_thread(
        &self,
        id: Option<Uuid>,
        user_id: Uuid,
        title: String,
    ) -> DbResult<Thread> {
        let id = id.unwrap_or_else(Uuid::now_v7);
        let now = Utc::now();

        let thread = sqlx::query_as::<_, Thread>(
            r#"
            INSERT INTO threads (id, user_id, title, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, user_id, title, active_leaf_id, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&title)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(thread)
    }

    #[builder]
    pub async fn delete_thread(&self, id: Uuid, user_id: Uuid) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM threads WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                entity: "thread",
                id: Some(id.to_string()),
            });
        }
        Ok(())
    }

    #[builder]
    pub async fn get_thread(&self, id: Uuid, user_id: Uuid) -> DbResult<Thread> {
        let thread = sqlx::query_as::<_, Thread>(
            r#"
            SELECT id, user_id, title, active_leaf_id, created_at, updated_at
            FROM threads
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(thread)
    }

    #[builder]
    pub async fn update_thread(&self, id: Uuid, user_id: Uuid, title: String) -> DbResult<Thread> {
        let now = Utc::now();

        let thread = sqlx::query_as::<_, Thread>(
            r#"
            UPDATE threads
            SET title = $1, updated_at = $2
            WHERE id = $3 AND user_id = $4
            RETURNING id, user_id, title, active_leaf_id, created_at, updated_at
            "#,
        )
        .bind(&title)
        .bind(now)
        .bind(id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(thread)
    }

    #[builder]
    pub async fn set_active_leaf(
        &self,
        id: Uuid,
        user_id: Uuid,
        active_leaf_id: Option<Uuid>,
    ) -> DbResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE threads
            SET active_leaf_id = $1
            WHERE id = $2 AND user_id = $3
              AND ($1 IS NULL OR EXISTS(
                  SELECT 1 FROM messages WHERE id = $1 AND thread_id = $2 AND user_id = $3
              ))
            "#,
        )
        .bind(active_leaf_id)
        .bind(id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::not_found_with_id("thread or active leaf", id));
        }

        Ok(())
    }

    #[builder]
    pub async fn get_sibling_info(
        &self,
        thread_id: Uuid,
        user_id: Uuid,
        message_ids: &[Uuid],
    ) -> DbResult<Vec<crate::types::SiblingInfo>> {
        if message_ids.is_empty() {
            return Ok(Vec::new());
        }

        let rows = sqlx::query_as::<_, crate::types::SiblingInfo>(
            r#"
            WITH thread_messages AS (
                SELECT m.id, m.parent_message_id, m.thread_id, m.created_at
                FROM messages m
                JOIN threads t ON t.id = m.thread_id
                WHERE m.thread_id = $2 AND t.user_id = $3
            ),
            siblings AS (
                SELECT
                    m.id,
                    COUNT(*) OVER (PARTITION BY m.parent_message_id) AS sibling_count,
                    ROW_NUMBER() OVER (PARTITION BY m.parent_message_id ORDER BY m.created_at, m.id) - 1 AS sibling_index
                FROM thread_messages m
                WHERE m.parent_message_id IS NOT NULL
            ),
            root_siblings AS (
                SELECT
                    m.id,
                    COUNT(*) OVER (PARTITION BY m.thread_id) AS sibling_count,
                    ROW_NUMBER() OVER (PARTITION BY m.thread_id ORDER BY m.created_at, m.id) - 1 AS sibling_index
                FROM thread_messages m
                WHERE m.parent_message_id IS NULL
            )
            SELECT q.message_id, q.sibling_count, q.sibling_index FROM (
                SELECT s.id AS message_id, s.sibling_count, s.sibling_index FROM siblings s WHERE s.id = ANY($1)
                UNION ALL
                SELECT r.id AS message_id, r.sibling_count, r.sibling_index FROM root_siblings r WHERE r.id = ANY($1)
            ) q
            "#,
        )
        .bind(message_ids)
        .bind(thread_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    #[builder]
    pub async fn get_adjacent_sibling(
        &self,
        thread_id: Uuid,
        user_id: Uuid,
        message_id: Uuid,
        direction: i32,
    ) -> DbResult<Option<Uuid>> {
        let row = sqlx::query_scalar::<_, Uuid>(
            r#"
            WITH target AS (
                SELECT m.id, m.parent_message_id, m.thread_id
                FROM messages m
                JOIN threads t ON t.id = m.thread_id
                WHERE m.id = $1 AND m.thread_id = $3 AND t.user_id = $4
            ),
            siblings AS (
                SELECT m.id,
                       ROW_NUMBER() OVER (ORDER BY m.created_at, m.id) AS rn,
                       COUNT(*) OVER () AS total
                FROM messages m, target t
                WHERE (t.parent_message_id IS NOT NULL AND m.parent_message_id = t.parent_message_id)
                   OR (t.parent_message_id IS NULL AND m.parent_message_id IS NULL AND m.thread_id = t.thread_id)
            )
            SELECT s2.id
            FROM siblings s1
            JOIN siblings s2 ON s2.rn = CASE
                WHEN s1.rn + $2 < 1 THEN s1.total
                WHEN s1.rn + $2 > s1.total THEN 1
                ELSE s1.rn + $2
            END
            WHERE s1.id = $1
            "#,
        )
        .bind(message_id)
        .bind(direction)
        .bind(thread_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn find_deepest_leaf(
        &self,
        thread_id: Uuid,
        user_id: Uuid,
        message_id: Uuid,
    ) -> DbResult<Uuid> {
        let leaf = sqlx::query_scalar::<_, Uuid>(
            r#"
            WITH RECURSIVE descendants AS (
                SELECT m.id, m.created_at, 0 AS depth
                FROM messages m
                JOIN threads t ON t.id = m.thread_id
                WHERE m.id = $1 AND m.thread_id = $2 AND t.user_id = $3
                UNION ALL
                SELECT m.id, m.created_at, d.depth + 1
                FROM messages m
                JOIN descendants d ON m.parent_message_id = d.id
                WHERE m.thread_id = $2
            )
            SELECT id FROM descendants ORDER BY depth DESC, created_at DESC, id DESC LIMIT 1
            "#,
        )
        .bind(message_id)
        .bind(thread_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(leaf)
    }

    #[builder]
    pub async fn list_threads(
        &self,
        user_id: Uuid,
        params: PaginationParams,
    ) -> DbResult<Vec<Thread>> {
        let query = format!(
            r#"
            SELECT id, user_id, title, active_leaf_id, created_at, updated_at
            FROM threads
            WHERE user_id = $1
            ORDER BY id {}
            LIMIT $2 OFFSET $3
            "#,
            params.order()
        );

        let threads = sqlx::query_as::<_, Thread>(&query)
            .bind(user_id)
            .bind(params.limit())
            .bind(params.offset())
            .fetch_all(&self.pool)
            .await?;

        Ok(threads)
    }

    #[builder]
    pub async fn create_message(
        &self,
        id: Option<Uuid>,
        thread_id: Uuid,
        user_id: Uuid,
        parent_message_id: Option<Uuid>,
        message_type: MessageType,
        content: String,
        tool_call_id: Option<String>,
        tool_calls: Option<serde_json::Value>,
        additional_kwargs: Option<serde_json::Value>,
        hidden_from_ui: Option<bool>,
        reasoning_blocks: Option<serde_json::Value>,
    ) -> DbResult<Message> {
        let id = id.unwrap_or_else(Uuid::now_v7);
        let now = Utc::now();
        let additional_kwargs = additional_kwargs.unwrap_or_else(|| serde_json::json!({}));
        let hidden_from_ui = hidden_from_ui.unwrap_or(false);

        let message = sqlx::query_as::<_, Message>(
            r#"
            WITH verified_thread AS (
                SELECT id FROM threads
                WHERE id = $2 AND user_id = $3
            ),
            updated_thread AS (
                UPDATE threads
                SET updated_at = $12, active_leaf_id = $1
                WHERE id = (SELECT id FROM verified_thread)
                RETURNING id
            ),
            inserted_message AS (
                INSERT INTO messages (id, thread_id, user_id, parent_message_id, message_type, content, tool_call_id, tool_calls, additional_kwargs, hidden_from_ui, reasoning_blocks, created_at, updated_at)
                SELECT $1, vc.id, $3, COALESCE($4, t.active_leaf_id), $5, $6, $7, $8, $9, $10, $11, $12, $12
                FROM verified_thread vc
                JOIN threads t ON t.id = vc.id
                RETURNING id, thread_id, user_id, parent_message_id, message_type, content, tool_call_id, tool_calls, additional_kwargs, reasoning_blocks, created_at, updated_at
            )
            SELECT * FROM inserted_message
            "#,
        )
        .bind(id)
        .bind(thread_id)
        .bind(user_id)
        .bind(parent_message_id)
        .bind(message_type)
        .bind(&content)
        .bind(&tool_call_id)
        .bind(&tool_calls)
        .bind(&additional_kwargs)
        .bind(hidden_from_ui)
        .bind(&reasoning_blocks)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(message)
    }

    #[builder]
    pub async fn list_messages(
        &self,
        thread_id: Uuid,
        user_id: Uuid,
        params: Option<PaginationParams>,
        include_visible: Option<bool>,
        include_hidden: Option<bool>,
    ) -> DbResult<Vec<Message>> {
        let params = params.unwrap_or_default();
        let include_visible = include_visible.unwrap_or(true);
        let include_hidden = include_hidden.unwrap_or(true);

        let visibility_filter = match (include_visible, include_hidden) {
            (true, true) => String::new(),
            (true, false) => " AND m.hidden_from_ui = false".to_string(),
            (false, true) => " AND m.hidden_from_ui = true".to_string(),
            (false, false) => " AND false".to_string(),
        };

        let query = format!(
            r#"
            WITH RECURSIVE branch AS (
                SELECT m.id, m.thread_id, m.user_id, m.parent_message_id, m.message_type,
                       m.content, m.tool_call_id, m.tool_calls, m.additional_kwargs,
                       m.reasoning_blocks, m.hidden_from_ui, m.created_at, m.updated_at
                FROM messages m
                JOIN threads t ON t.active_leaf_id = m.id
                WHERE t.id = $1 AND t.user_id = $2

                UNION ALL

                SELECT parent.id, parent.thread_id, parent.user_id, parent.parent_message_id,
                       parent.message_type, parent.content, parent.tool_call_id, parent.tool_calls,
                       parent.additional_kwargs, parent.reasoning_blocks, parent.hidden_from_ui,
                       parent.created_at, parent.updated_at
                FROM messages parent
                JOIN branch child ON child.parent_message_id = parent.id
                    AND parent.thread_id = $1 AND parent.user_id = $2
            )
            SELECT m.id, m.thread_id, m.user_id, m.parent_message_id, m.message_type,
                   m.content, m.tool_call_id, m.tool_calls, m.additional_kwargs,
                   m.reasoning_blocks, m.created_at, m.updated_at
            FROM branch m
            WHERE true{}
            ORDER BY m.created_at {}
            LIMIT $3 OFFSET $4
            "#,
            visibility_filter,
            params.order()
        );

        let messages = sqlx::query_as::<_, Message>(&query)
            .bind(thread_id)
            .bind(user_id)
            .bind(params.limit())
            .bind(params.offset())
            .fetch_all(&self.pool)
            .await?;

        Ok(messages)
    }

    pub async fn list_messages_by_level(
        &self,
        thread_id: Uuid,
        user_id: Uuid,
        start_level: i32,
        end_level: i32,
    ) -> DbResult<crate::types::MessageTreeResult> {
        let peek_level = end_level + 1;
        let rows = sqlx::query_as::<_, crate::types::MessageTreeNode>(
            r#"
            WITH RECURSIVE tree AS (
                SELECT m.id, m.parent_message_id, m.message_type, m.content,
                       m.additional_kwargs, m.reasoning_blocks, m.hidden_from_ui,
                       m.created_at,
                       0 AS depth
                FROM messages m
                JOIN threads t ON t.id = m.thread_id
                WHERE m.thread_id = $1 AND t.user_id = $2
                  AND m.parent_message_id IS NULL

                UNION ALL

                SELECT m.id, m.parent_message_id, m.message_type, m.content,
                       m.additional_kwargs, m.reasoning_blocks, m.hidden_from_ui,
                       m.created_at,
                       t.depth + 1
                FROM messages m
                JOIN tree t ON m.parent_message_id = t.id
                WHERE m.thread_id = $1
                  AND t.depth + 1 <= $5
            ),
            visible AS (
                SELECT id, parent_message_id, message_type, content,
                       additional_kwargs, reasoning_blocks, created_at,
                       ROW_NUMBER() OVER (
                           PARTITION BY id ORDER BY depth
                       ) AS rn,
                       depth
                FROM tree
                WHERE hidden_from_ui = false
            ),
            numbered AS (
                SELECT v.id, v.parent_message_id, v.message_type, v.content,
                       v.additional_kwargs, v.reasoning_blocks, v.created_at,
                       v.depth,
                       DENSE_RANK() OVER (ORDER BY v.depth) - 1 AS level
                FROM visible v
                WHERE v.rn = 1
            ),
            with_siblings AS (
                SELECT
                    n.id,
                    CASE WHEN p.id IS NOT NULL THEN n.parent_message_id ELSE NULL END AS parent_message_id,
                    n.message_type,
                    n.content,
                    n.additional_kwargs,
                    n.reasoning_blocks,
                    n.level,
                    COUNT(*) OVER (
                        PARTITION BY CASE WHEN p.id IS NOT NULL THEN n.parent_message_id ELSE NULL END
                    ) AS sibling_count,
                    ROW_NUMBER() OVER (
                        PARTITION BY CASE WHEN p.id IS NOT NULL THEN n.parent_message_id ELSE NULL END
                        ORDER BY n.created_at, n.id
                    ) - 1 AS sibling_index
                FROM numbered n
                LEFT JOIN numbered p ON p.id = n.parent_message_id
            )
            SELECT id, parent_message_id, message_type, content,
                   additional_kwargs, reasoning_blocks,
                   level::int4 AS level,
                   sibling_count, sibling_index
            FROM with_siblings
            WHERE level >= $3 AND level <= $4
            ORDER BY level, sibling_index
            "#,
        )
        .bind(thread_id)
        .bind(user_id)
        .bind(start_level)
        .bind(peek_level)
        .bind(peek_level)
        .fetch_all(&self.pool)
        .await?;

        let has_more = rows.iter().any(|r| r.level > end_level);
        let nodes = rows.into_iter().filter(|r| r.level <= end_level).collect();

        Ok(crate::types::MessageTreeResult { nodes, has_more })
    }

    pub async fn list_messages_by_level_from_parents(
        &self,
        thread_id: Uuid,
        user_id: Uuid,
        parent_ids: &[Uuid],
        start_level: i32,
        depth_limit: i32,
    ) -> DbResult<crate::types::MessageTreeResult> {
        let peek_depth = depth_limit + 1;
        let rows = sqlx::query_as::<_, crate::types::MessageTreeNode>(
            r#"
            WITH RECURSIVE tree AS (
                SELECT m.id, m.parent_message_id, m.message_type, m.content,
                       m.additional_kwargs, m.reasoning_blocks, m.hidden_from_ui,
                       m.created_at,
                       0 AS depth
                FROM messages m
                JOIN threads t ON t.id = m.thread_id
                WHERE m.thread_id = $1 AND t.user_id = $2
                  AND m.parent_message_id = ANY($3)

                UNION ALL

                SELECT m.id, m.parent_message_id, m.message_type, m.content,
                       m.additional_kwargs, m.reasoning_blocks, m.hidden_from_ui,
                       m.created_at,
                       t.depth + 1
                FROM messages m
                JOIN tree t ON m.parent_message_id = t.id
                WHERE m.thread_id = $1
                  AND t.depth + 1 <= $4
            ),
            visible AS (
                SELECT id, parent_message_id, message_type, content,
                       additional_kwargs, reasoning_blocks, created_at,
                       ROW_NUMBER() OVER (
                           PARTITION BY id ORDER BY depth
                       ) AS rn,
                       depth
                FROM tree
                WHERE hidden_from_ui = false
            ),
            numbered AS (
                SELECT v.id, v.parent_message_id, v.message_type, v.content,
                       v.additional_kwargs, v.reasoning_blocks, v.created_at,
                       $5 + DENSE_RANK() OVER (ORDER BY v.depth) - 1 AS level
                FROM visible v
                WHERE v.rn = 1
            ),
            with_siblings AS (
                SELECT
                    n.id,
                    n.parent_message_id,
                    n.message_type,
                    n.content,
                    n.additional_kwargs,
                    n.reasoning_blocks,
                    n.level,
                    COUNT(*) OVER (PARTITION BY n.parent_message_id) AS sibling_count,
                    ROW_NUMBER() OVER (
                        PARTITION BY n.parent_message_id
                        ORDER BY n.created_at, n.id
                    ) - 1 AS sibling_index
                FROM numbered n
            )
            SELECT id, parent_message_id, message_type, content,
                   additional_kwargs, reasoning_blocks,
                   level::int4 AS level,
                   sibling_count, sibling_index
            FROM with_siblings
            ORDER BY level, sibling_index
            "#,
        )
        .bind(thread_id)
        .bind(user_id)
        .bind(parent_ids)
        .bind(peek_depth)
        .bind(start_level)
        .fetch_all(&self.pool)
        .await?;

        let max_requested_level = start_level + depth_limit - 1;
        let has_more = rows.iter().any(|r| r.level > max_requested_level);
        let nodes = rows
            .into_iter()
            .filter(|r| r.level <= max_requested_level)
            .collect();

        Ok(crate::types::MessageTreeResult { nodes, has_more })
    }

    #[builder]
    pub async fn try_claim_webhook_event<'e, E>(
        &self,
        executor: E,
        event_id: &str,
        event_type: &str,
    ) -> DbResult<bool>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let result = sqlx::query(
            r#"
            INSERT INTO stripe.webhook_events (event_id, event_type)
            VALUES ($1, $2)
            ON CONFLICT (event_id) DO NOTHING
            "#,
        )
        .bind(event_id)
        .bind(event_type)
        .execute(executor)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    #[builder]
    pub async fn upsert_stripe_customer<'e, E>(
        &self,
        executor: E,
        customer_id: &str,
        email: Option<&str>,
        raw_data: &serde_json::Value,
    ) -> DbResult<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO stripe.customers (id, app_user_id, email, created_at, updated_at, raw_data)
            VALUES ($1, (SELECT id FROM users WHERE email = $2), $2, $3, $3, $4)
            ON CONFLICT (id) DO UPDATE
            SET email = COALESCE(EXCLUDED.email, stripe.customers.email),
                app_user_id = COALESCE(EXCLUDED.app_user_id, stripe.customers.app_user_id),
                raw_data = EXCLUDED.raw_data,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(customer_id)
        .bind(email)
        .bind(now)
        .bind(raw_data)
        .execute(executor)
        .await?;

        Ok(())
    }

    #[builder]
    pub async fn link_stripe_customer_to_user<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
        stripe_customer_id: &str,
    ) -> DbResult<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE users
            SET stripe_customer_id = $2
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(stripe_customer_id)
        .execute(executor)
        .await?;

        Ok(())
    }

    #[builder]
    pub async fn upsert_stripe_subscription<'e, E>(
        &self,
        executor: E,
        subscription_id: &str,
        customer_id: &str,
        status: &str,
        cancel_at_period_end: bool,
        canceled_at: Option<i64>,
        current_period_start: i64,
        current_period_end: i64,
        raw_data: &serde_json::Value,
    ) -> DbResult<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let now = Utc::now();
        let canceled_at_ts = canceled_at.and_then(|ts| {
            let dt = chrono::DateTime::from_timestamp(ts, 0);
            if dt.is_none() {
                tracing::warn!(ts, "malformed canceled_at timestamp in stripe subscription");
            }
            dt
        });
        let period_start = chrono::DateTime::from_timestamp(current_period_start, 0)
            .unwrap_or_else(|| {
                tracing::warn!(current_period_start, "malformed current_period_start timestamp in stripe subscription, falling back to now");
                now
            });
        let period_end = chrono::DateTime::from_timestamp(current_period_end, 0).unwrap_or_else(
            || {
                tracing::warn!(current_period_end, "malformed current_period_end timestamp in stripe subscription, falling back to now");
                now
            },
        );

        sqlx::query(
            r#"
            INSERT INTO stripe.subscriptions (
                id, customer_id, status,
                cancel_at_period_end, canceled_at,
                current_period_start, current_period_end,
                created_at, updated_at, raw_data
            )
            VALUES ($1, $2, $3::stripe.subscription_status, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (id) DO UPDATE
            SET status = $3::stripe.subscription_status,
                customer_id = EXCLUDED.customer_id,
                cancel_at_period_end = EXCLUDED.cancel_at_period_end,
                canceled_at = EXCLUDED.canceled_at,
                current_period_start = EXCLUDED.current_period_start,
                current_period_end = EXCLUDED.current_period_end,
                raw_data = EXCLUDED.raw_data,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(subscription_id)
        .bind(customer_id)
        .bind(status)
        .bind(cancel_at_period_end)
        .bind(canceled_at_ts)
        .bind(period_start)
        .bind(period_end)
        .bind(now)
        .bind(now)
        .bind(raw_data)
        .execute(executor)
        .await?;

        Ok(())
    }

    #[builder]
    pub async fn upsert_stripe_price<'e, E>(
        &self,
        executor: E,
        price_id: &str,
        currency: &str,
        unit_amount: Option<i64>,
        recurring_interval: Option<&str>,
        active: bool,
        raw_data: &serde_json::Value,
    ) -> DbResult<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO stripe.prices (id, currency, unit_amount, recurring_interval, active, created_at, updated_at, raw_data)
            VALUES ($1, $2, $3, $4, $5, $6, $6, $7)
            ON CONFLICT (id) DO UPDATE
            SET currency = EXCLUDED.currency,
                unit_amount = EXCLUDED.unit_amount,
                recurring_interval = EXCLUDED.recurring_interval,
                active = EXCLUDED.active,
                raw_data = EXCLUDED.raw_data,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(price_id)
        .bind(currency)
        .bind(unit_amount)
        .bind(recurring_interval)
        .bind(active)
        .bind(now)
        .bind(raw_data)
        .execute(executor)
        .await?;

        Ok(())
    }

    #[builder]
    pub async fn ensure_plan_price<'e, E>(
        &self,
        executor: E,
        plan_id: &str,
        stripe_price_id: &str,
    ) -> DbResult<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        sqlx::query(
            "INSERT INTO plan_prices (plan_id, stripe_price_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(plan_id)
        .bind(stripe_price_id)
        .execute(executor)
        .await?;

        Ok(())
    }

    #[builder]
    pub async fn sync_stripe_subscription_items<'e, E>(
        &self,
        executor: E,
        subscription_id: &str,
        items: &[(String, String, Option<i64>, serde_json::Value)],
    ) -> DbResult<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        if items.is_empty() {
            sqlx::query("DELETE FROM stripe.subscription_items WHERE subscription_id = $1")
                .bind(subscription_id)
                .execute(executor)
                .await?;
            return Ok(());
        }

        let mut query = String::from(
            "WITH deleted AS (DELETE FROM stripe.subscription_items WHERE subscription_id = $1) INSERT INTO stripe.subscription_items (id, subscription_id, price_id, quantity, raw_data) VALUES ",
        );

        let mut param_idx = 2u32;
        for (i, _) in items.iter().enumerate() {
            if i > 0 {
                query.push_str(", ");
            }
            query.push_str(&format!(
                "(${}, $1, ${}, ${}, ${})",
                param_idx,
                param_idx + 1,
                param_idx + 2,
                param_idx + 3,
            ));
            param_idx += 4;
        }
        query.push_str(" ON CONFLICT (id) DO UPDATE SET price_id = EXCLUDED.price_id, quantity = EXCLUDED.quantity, raw_data = EXCLUDED.raw_data");

        let mut q = sqlx::query(&query).bind(subscription_id);
        for (item_id, price_id, quantity, raw_data) in items {
            q = q
                .bind(item_id)
                .bind(price_id)
                .bind(quantity.map(|v| v as i32))
                .bind(raw_data);
        }

        q.execute(executor).await?;

        Ok(())
    }

    #[builder]
    pub async fn update_stripe_subscription_status(
        &self,
        subscription_id: &str,
        status: &str,
        cancel_at_period_end: bool,
        canceled_at: Option<i64>,
        raw_data: &serde_json::Value,
    ) -> DbResult<()> {
        let now = Utc::now();
        let canceled_at_ts = canceled_at.and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));

        let result = sqlx::query(
            r#"
            UPDATE stripe.subscriptions
            SET status = $2::stripe.subscription_status,
                cancel_at_period_end = $3,
                canceled_at = $4,
                raw_data = $5,
                updated_at = $6
            WHERE id = $1
            "#,
        )
        .bind(subscription_id)
        .bind(status)
        .bind(cancel_at_period_end)
        .bind(canceled_at_ts)
        .bind(raw_data)
        .bind(now)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::not_found_with_id("subscription", subscription_id));
        }

        Ok(())
    }

    #[builder]
    pub async fn update_stripe_subscription_status_with_executor<'e, E>(
        &self,
        executor: E,
        subscription_id: &str,
        status: &str,
        cancel_at_period_end: bool,
        canceled_at: Option<i64>,
        raw_data: &serde_json::Value,
    ) -> DbResult<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let now = Utc::now();
        let canceled_at_ts = canceled_at.and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));

        let result = sqlx::query(
            r#"
            UPDATE stripe.subscriptions
            SET status = $2::stripe.subscription_status,
                cancel_at_period_end = $3,
                canceled_at = $4,
                raw_data = $5,
                updated_at = $6
            WHERE id = $1
            "#,
        )
        .bind(subscription_id)
        .bind(status)
        .bind(cancel_at_period_end)
        .bind(canceled_at_ts)
        .bind(raw_data)
        .bind(now)
        .execute(executor)
        .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::not_found_with_id("subscription", subscription_id));
        }

        Ok(())
    }

    #[builder]
    pub async fn ensure_user_plan<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
        plan_id: &str,
    ) -> DbResult<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE users
            SET plan_id = CASE
                    WHEN (SELECT rank FROM (VALUES ('free',0),('tier1',1)) AS r(id,rank) WHERE r.id = $2)
                       > (SELECT rank FROM (VALUES ('free',0),('tier1',1)) AS r(id,rank) WHERE r.id = users.plan_id)
                    THEN $2
                    ELSE users.plan_id
                END
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(plan_id)
        .execute(executor)
        .await?;

        Ok(())
    }

    #[builder]
    pub async fn get_plan_id_for_user(&self, user_id: Uuid) -> DbResult<Option<String>> {
        let result: Option<String> = sqlx::query_scalar("SELECT plan_id FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(result)
    }

    #[builder]
    pub async fn update_plan_by_stripe_customer<'e, E>(
        &self,
        executor: E,
        stripe_customer_id: &str,
        plan_id: &str,
    ) -> DbResult<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE users
            SET plan_id = $2
            WHERE stripe_customer_id = $1
            "#,
        )
        .bind(stripe_customer_id)
        .bind(plan_id)
        .execute(executor)
        .await?;

        Ok(())
    }

    #[builder]
    pub async fn resolve_plan_for_stripe_price<'e, E>(
        &self,
        executor: E,
        price_id: &str,
    ) -> DbResult<Option<String>>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let result: Option<String> =
            sqlx::query_scalar("SELECT plan_id FROM plan_prices WHERE stripe_price_id = $1")
                .bind(price_id)
                .fetch_optional(executor)
                .await?;
        Ok(result)
    }

    #[builder]
    pub async fn record_token_usage(
        &self,
        user_id: Uuid,
        thread_id: Uuid,
        message_id: Uuid,
        input_tokens: i64,
        output_tokens: i64,
        reasoning_tokens: Option<i64>,
        cache_creation_tokens: Option<i64>,
        cache_read_tokens: Option<i64>,
    ) -> DbResult<TokenUsage> {
        let record = sqlx::query_as::<_, TokenUsage>(
            r#"
            INSERT INTO token_usage (
                user_id, thread_id, message_id,
                input_tokens, output_tokens, reasoning_tokens,
                cache_creation_tokens, cache_read_tokens
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, user_id, thread_id, message_id,
                      input_tokens, output_tokens, reasoning_tokens,
                      cache_creation_tokens, cache_read_tokens, created_at
            "#,
        )
        .bind(user_id)
        .bind(thread_id)
        .bind(message_id)
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(reasoning_tokens.unwrap_or(0))
        .bind(cache_creation_tokens.unwrap_or(0))
        .bind(cache_read_tokens.unwrap_or(0))
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    #[builder]
    pub async fn get_token_limit_and_usage(
        &self,
        user_id: Uuid,
        year_month: i32,
    ) -> DbResult<(i64, i64)> {
        let row: Option<(i64, Option<i64>)> = sqlx::query_as(
            r#"
            SELECT p.monthly_token_limit,
                   mtt.total_tokens
            FROM users u
            JOIN plans p ON p.id = u.plan_id
            LEFT JOIN monthly_token_totals mtt
                   ON mtt.user_id = u.id
                  AND mtt.year_month = $2
            WHERE u.id = $1
            "#,
        )
        .bind(user_id)
        .bind(year_month)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some((limit, used)) => Ok((limit, used.unwrap_or(0))),
            None => Ok((DEFAULT_TOKEN_LIMIT, 0)),
        }
    }

    #[builder]
    pub async fn consume_login_token_and_create_refresh_token(
        &self,
        login_token_hash: &[u8],
        user_id: Uuid,
        refresh_token_hash: Vec<u8>,
        refresh_token_expires_at: DateTime<Utc>,
    ) -> DbResult<()> {
        let now = Utc::now();
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            UPDATE login_tokens
            SET consumed = true, updated_at = $2
            WHERE token_hash = $1 AND consumed = false AND expires_at > now()
            "#,
        )
        .bind(login_token_hash)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, revoked, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(&refresh_token_hash)
        .bind(refresh_token_expires_at)
        .bind(false)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub async fn search_messages(
        &self,
        user_id: Uuid,
        query: &str,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<SearchResultMessage>> {
        let tsquery = match build_prefix_tsquery(query) {
            Some(q) => q,
            None => return Ok(vec![]),
        };
        let ilike_pattern = format!("%{}%", query.replace('%', r"\%").replace('_', r"\_"));
        let regex_pattern = regex_escape(query);
        let results = sqlx::query_as::<_, SearchResultMessage>(
            r#"
            WITH matched AS (
                SELECT id, thread_id, message_type, content,
                       search_tsv @@ to_tsquery('english', $1) AS fts_match,
                       CASE WHEN search_tsv @@ to_tsquery('english', $1)
                            THEN ts_rank(search_tsv, to_tsquery('english', $1))
                            ELSE 0.0
                       END AS rank,
                       created_at
                FROM messages
                WHERE user_id = $2
                  AND hidden_from_ui = false
                  AND (search_tsv @@ to_tsquery('english', $1) OR content ILIKE $5)
                ORDER BY rank DESC, created_at DESC
                LIMIT $3 OFFSET $4
            )
            SELECT id, thread_id, message_type,
                   regexp_replace(
                       substring(content from GREATEST(1, position(lower($7) in lower(content)) - 80) for 200),
                       $6, '<mark>\&</mark>', 'gi'
                   ) AS snippet,
                   rank, created_at
            FROM matched
            "#,
        )
        .bind(&tsquery)       // $1
        .bind(user_id)        // $2
        .bind(limit)          // $3
        .bind(offset)         // $4
        .bind(&ilike_pattern) // $5
        .bind(&regex_pattern) // $6
        .bind(query)          // $7
        .fetch_all(&self.pool)
        .await?;

        Ok(results)
    }

    pub async fn search_threads(
        &self,
        user_id: Uuid,
        query: &str,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<SearchResultThread>> {
        let tsquery = match build_prefix_tsquery(query) {
            Some(q) => q,
            None => return Ok(vec![]),
        };
        let ilike_pattern = format!("%{}%", query.replace('%', r"\%").replace('_', r"\_"));
        let results = sqlx::query_as::<_, SearchResultThread>(
            r#"
            SELECT id, title,
                   GREATEST(
                       CASE WHEN to_tsvector('english', immutable_unaccent(coalesce(title, ''))) @@ to_tsquery('english', $1)
                            THEN ts_rank(to_tsvector('english', immutable_unaccent(coalesce(title, ''))), to_tsquery('english', $1))
                            ELSE 0.0
                       END,
                       CASE WHEN similarity(coalesce(title, ''), $5) > 0.1
                            THEN similarity(coalesce(title, ''), $5)
                            ELSE 0.0
                       END
                   ) AS rank,
                   updated_at
            FROM threads
            WHERE user_id = $2
              AND (
                  to_tsvector('english', immutable_unaccent(coalesce(title, ''))) @@ to_tsquery('english', $1)
                  OR coalesce(title, '') ILIKE $6
              )
            ORDER BY rank DESC, updated_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(&tsquery)
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .bind(query)
        .bind(&ilike_pattern)
        .fetch_all(&self.pool)
        .await?;

        Ok(results)
    }
}
