use crate::error::DbResult;
use crate::{
    GetConversation,
    types::{
        Activity, ActivityAsset, Asset, Conversation, CreateLoginToken, CreateOAuthCredentials,
        CreateOAuthState, CreateRefreshToken, GetActivitiesByTimeRange, ListActivities,
        ListConversations, ListMessages, LoginToken, Message, NewActivity, NewAsset,
        NewConversation, NewMessage, NewUser, OAuthCredentials, OAuthState, PasswordCredentials,
        RefreshToken, UpdateActivity, UpdateActivityEndTime, UpdateOAuthCredentials, User,
    },
};
use chrono::{DateTime, Utc};
use sqlx::{
    migrate::MigrateDatabase,
    postgres::{PgPool, PgPoolOptions},
};
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug)]
pub struct DatabaseManager {
    pub pool: PgPool,
}

impl DatabaseManager {
    pub async fn new(database_url: &str) -> DbResult<Self> {
        // Create the database if it doesn't exist
        if !sqlx::Postgres::database_exists(database_url).await? {
            sqlx::Postgres::create_database(database_url).await?;
        }

        let pool = PgPoolOptions::new()
            .max_connections(50)
            .min_connections(3) // Minimum number of idle connections
            .acquire_timeout(Duration::from_secs(10))
            .connect(database_url)
            .await?;

        let db_manager = DatabaseManager { pool };

        // Run migrations after establishing the connection
        Self::run_migrations(&db_manager.pool).await?;

        Ok(db_manager)
    }

    async fn run_migrations(pool: &PgPool) -> DbResult<()> {
        let migrator = sqlx::migrate!("./src/migrations");
        migrator.run(pool).await?;
        Ok(())
    }

    // User management methods
    pub async fn create_user(&self, request: NewUser) -> DbResult<User> {
        let user_id = Uuid::now_v7();
        let password_id = Uuid::now_v7();
        let now = Utc::now();

        // Start a transaction to ensure both user and password_credentials are created atomically
        let mut tx = self.pool.begin().await?;

        // Insert user
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, username, email, display_name, email_verified, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, username, email, display_name, email_verified, created_at, updated_at
            "#,
        )
        .bind(user_id)
        .bind(&request.username)
        .bind(&request.email)
        .bind(&request.display_name)
        .bind(false) // email_verified defaults to false
        .bind(now)
        .bind(None::<DateTime<Utc>>) // updated_at is null initially
        .fetch_one(&mut *tx)
        .await?;

        // Insert password credentials
        sqlx::query(
            r#"
            INSERT INTO password_credentials (id, user_id, password_hash, updated_at)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(password_id)
        .bind(user_id)
        .bind(&request.password_hash)
        .bind(None::<DateTime<Utc>>) // updated_at is null initially
        .execute(&mut *tx)
        .await?;

        // Commit the transaction
        tx.commit().await?;

        Ok(user)
    }

    pub async fn get_user_by_id(&self, user_id: Uuid) -> DbResult<User> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, email, display_name, email_verified, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn get_user_by_username(&self, username: &str) -> DbResult<User> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, email, display_name, email_verified, created_at, updated_at
            FROM users
            WHERE username = $1
            "#,
        )
        .bind(username)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn get_user_by_email(&self, email: &str) -> DbResult<User> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, email, display_name, email_verified, created_at, updated_at
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    // Password credentials management methods
    pub async fn get_password_credentials(&self, user_id: Uuid) -> DbResult<PasswordCredentials> {
        let credentials = sqlx::query_as::<_, PasswordCredentials>(
            r#"
            SELECT id, user_id, password_hash, updated_at
            FROM password_credentials
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(credentials)
    }

    // Utility methods
    pub async fn user_exists_by_username(&self, username: &str) -> DbResult<bool> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM users WHERE username = $1
            "#,
        )
        .bind(username)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0 > 0)
    }

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

    // OAuth credentials management methods
    pub async fn create_oauth_credentials(
        &self,
        request: CreateOAuthCredentials,
    ) -> DbResult<OAuthCredentials> {
        let id = Uuid::now_v7();
        let now = Utc::now();

        let oauth_creds = sqlx::query_as::<_, OAuthCredentials>(
            r#"
            INSERT INTO oauth_credentials (
                id, user_id, provider, provider_user_id, access_token,
                refresh_token, access_token_expiry, scope, issued_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, user_id, provider, provider_user_id, access_token,
                      refresh_token, access_token_expiry, scope, issued_at, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(request.user_id)
        .bind(&request.provider)
        .bind(&request.provider_user_id)
        .bind(&request.access_token)
        .bind(&request.refresh_token)
        .bind(request.access_token_expiry)
        .bind(&request.scope)
        .bind(now)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(oauth_creds)
    }

    pub async fn get_oauth_credentials_by_provider_and_user(
        &self,
        provider: &str,
        user_id: Uuid,
    ) -> DbResult<OAuthCredentials> {
        let oauth_creds = sqlx::query_as::<_, OAuthCredentials>(
            r#"
            SELECT id, user_id, provider, provider_user_id, access_token,
                   refresh_token, access_token_expiry, scope, issued_at, created_at, updated_at
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

    pub async fn update_oauth_credentials(
        &self,
        id: Uuid,
        request: UpdateOAuthCredentials,
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
                      refresh_token, access_token_expiry, scope, issued_at, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&request.access_token)
        .bind(&request.refresh_token)
        .bind(request.access_token_expiry)
        .bind(&request.scope)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(oauth_creds)
    }

    pub async fn get_user_by_oauth_provider(
        &self,
        provider: &str,
        provider_user_id: &str,
    ) -> DbResult<User> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT u.id, u.username, u.email, u.display_name, u.email_verified, u.created_at, u.updated_at
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

    // Refresh token management methods
    pub async fn create_refresh_token(
        &self,
        request: CreateRefreshToken,
    ) -> DbResult<RefreshToken> {
        let id = Uuid::now_v7();
        let now = Utc::now();

        let refresh_token = sqlx::query_as::<_, RefreshToken>(
            r#"
            INSERT INTO refresh_tokens (id, user_id, token_hash, issued_at, expires_at, revoked, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, user_id, token_hash, issued_at, expires_at, revoked, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(request.user_id)
        .bind(&request.token_hash)
        .bind(now)
        .bind(request.expires_at)
        .bind(false)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(refresh_token)
    }

    pub async fn get_refresh_token_by_hash(&self, token_hash: &str) -> DbResult<RefreshToken> {
        let refresh_token = sqlx::query_as::<_, RefreshToken>(
            r#"
            SELECT id, user_id, token_hash, issued_at, expires_at, revoked, created_at, updated_at
            FROM refresh_tokens
            WHERE token_hash = $1 AND revoked = false AND expires_at > now()
            "#,
        )
        .bind(token_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(refresh_token)
    }

    pub async fn revoke_refresh_token(&self, token_hash: &str) -> DbResult<RefreshToken> {
        let now = Utc::now();

        let refresh_token = sqlx::query_as::<_, RefreshToken>(
            r#"
            UPDATE refresh_tokens
            SET revoked = true, updated_at = $2
            WHERE token_hash = $1 AND revoked = false
            RETURNING id, user_id, token_hash, issued_at, expires_at, revoked, created_at, updated_at
            "#,
        )
        .bind(token_hash)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(refresh_token)
    }

    // OAuth state management methods
    pub async fn create_oauth_state(&self, request: CreateOAuthState) -> DbResult<OAuthState> {
        let id = Uuid::now_v7();
        let now = Utc::now();

        let oauth_state = sqlx::query_as::<_, OAuthState>(
            r#"
            INSERT INTO oauth_state (id, state, pkce_verifier, redirect_uri, ip_address, consumed, created_at, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, state, pkce_verifier, redirect_uri, ip_address, consumed, created_at, expires_at
            "#,
        )
        .bind(id)
        .bind(&request.state)
        .bind(&request.pkce_verifier)
        .bind(&request.redirect_uri)
        .bind(request.ip_address)
        .bind(false) // consumed defaults to false
        .bind(now)
        .bind(request.expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(oauth_state)
    }

    pub async fn get_oauth_state_by_state(&self, state: &str) -> DbResult<OAuthState> {
        let oauth_state = sqlx::query_as::<_, OAuthState>(
            r#"
            SELECT id, state, pkce_verifier, redirect_uri, ip_address, consumed, created_at, expires_at
            FROM oauth_state
            WHERE state = $1 AND consumed = false AND expires_at > now()
            "#,
        )
        .bind(state)
        .fetch_one(&self.pool)
        .await?;

        Ok(oauth_state)
    }

    pub async fn consume_oauth_state(&self, state: &str) -> DbResult<OAuthState> {
        let oauth_state = sqlx::query_as::<_, OAuthState>(
            r#"
            UPDATE oauth_state
            SET consumed = true
            WHERE state = $1 AND consumed = false AND expires_at > now()
            RETURNING id, state, pkce_verifier, redirect_uri, ip_address, consumed, created_at, expires_at
            "#,
        )
        .bind(state)
        .fetch_one(&self.pool)
        .await?;

        Ok(oauth_state)
    }

    // Login token management methods
    pub async fn create_login_token(&self, request: CreateLoginToken) -> DbResult<LoginToken> {
        let id = Uuid::now_v7();
        let now = Utc::now();

        let login_token = sqlx::query_as::<_, LoginToken>(
            r#"
            INSERT INTO login_tokens (id, token, expires_at, user_id, consumed, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, token, consumed, expires_at, user_id, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&request.token)
        .bind(request.expires_at)
        .bind(request.user_id)
        .bind(false) // consumed starts as false
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(login_token)
    }

    pub async fn get_login_token_by_token(&self, token: &str) -> DbResult<LoginToken> {
        let login_token = sqlx::query_as::<_, LoginToken>(
            r#"
            SELECT id, token, consumed, expires_at, user_id, created_at, updated_at
            FROM login_tokens
            WHERE token = $1 AND consumed = false AND expires_at > now()
            "#,
        )
        .bind(token)
        .fetch_one(&self.pool)
        .await?;

        Ok(login_token)
    }

    pub async fn consume_login_token(&self, token: &str) -> DbResult<LoginToken> {
        let now = Utc::now();

        let login_token = sqlx::query_as::<_, LoginToken>(
            r#"
            UPDATE login_tokens
            SET consumed = true, updated_at = $2
            WHERE token = $1 AND expires_at > now()
            RETURNING id, token, consumed, expires_at, user_id, created_at, updated_at
            "#,
        )
        .bind(token)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(login_token)
    }

    // =========================================================================
    // Activity Management Methods
    // =========================================================================

    /// Create a new activity
    pub async fn create_activity(&self, request: NewActivity) -> DbResult<Activity> {
        let id = request.id.unwrap_or_else(Uuid::now_v7);
        let now = Utc::now();

        let activity = sqlx::query_as::<_, Activity>(
            r#"
            INSERT INTO activities (id, user_id, name, icon_asset_id, process_name, window_title, started_at, ended_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, user_id, name, icon_asset_id, process_name, window_title, started_at, ended_at, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(request.user_id)
        .bind(&request.name)
        .bind(request.icon_asset_id)
        .bind(&request.process_name)
        .bind(&request.window_title)
        .bind(request.started_at)
        .bind(request.ended_at)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(activity)
    }

    /// Get an activity by ID for a specific user
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

    /// List activities for a user with pagination
    pub async fn list_activities(&self, request: ListActivities) -> DbResult<Vec<Activity>> {
        // Clamp limit to max 100
        let limit = request.limit.clamp(1, 100);

        let activities = sqlx::query_as::<_, Activity>(
            r#"
            SELECT id, user_id, name, icon_asset_id, process_name, window_title, started_at, ended_at, created_at, updated_at
            FROM activities
            WHERE user_id = $1
            ORDER BY started_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(request.user_id)
        .bind(limit as i64)
        .bind(request.offset as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(activities)
    }

    /// Update an existing activity
    pub async fn update_activity(&self, request: UpdateActivity) -> DbResult<Activity> {
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
        .bind(request.id)
        .bind(request.user_id)
        .bind(&request.name)
        .bind(request.icon_asset_id)
        .bind(&request.process_name)
        .bind(&request.window_title)
        .bind(request.started_at)
        .bind(request.ended_at)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(activity)
    }

    /// Update activity end time
    pub async fn update_activity_end_time(&self, request: UpdateActivityEndTime) -> DbResult<()> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE activities
            SET ended_at = $3, updated_at = $4
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(request.activity_id)
        .bind(request.user_id)
        .bind(request.ended_at)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get the last active (not ended) activity for a user
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

    /// Delete an activity
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

    /// Get activities by time range for a user
    pub async fn get_activities_by_time_range(
        &self,
        request: GetActivitiesByTimeRange,
    ) -> DbResult<Vec<Activity>> {
        // Clamp limit to max 100
        let limit = request.limit.clamp(1, 100);

        let activities = sqlx::query_as::<_, Activity>(
            r#"
            SELECT id, user_id, name, icon_asset_id, process_name, window_title, started_at, ended_at, created_at, updated_at
            FROM activities
            WHERE user_id = $1
              AND started_at >= $2
              AND started_at <= $3
            ORDER BY started_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(request.user_id)
        .bind(request.start_time)
        .bind(request.end_time)
        .bind(limit as i64)
        .bind(request.offset as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(activities)
    }

    // =========================================================================
    // Asset Management Methods
    // =========================================================================

    /// Create a new asset
    pub async fn create_asset(&self, request: NewAsset) -> DbResult<Asset> {
        let id = request.id.unwrap_or_else(Uuid::now_v7);
        let now = Utc::now();
        let metadata = request.metadata.unwrap_or_else(|| serde_json::json!({}));

        let asset = sqlx::query_as::<_, Asset>(
            r#"
            INSERT INTO assets (id, user_id, name, mime_type, size_bytes, checksum_sha256, storage_backend, storage_uri, status, metadata, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING id, user_id, name, mime_type, size_bytes, checksum_sha256, storage_backend, storage_uri, status, metadata, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(request.user_id)
        .bind(&request.name)
        .bind(&request.mime_type)
        .bind(request.size_bytes)
        .bind(&request.checksum_sha256)
        .bind("fs")
        .bind(&request.storage_uri)
        .bind("uploaded")
        .bind(&metadata)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(asset)
    }

    /// Link an asset to an activity
    /// Verifies that both the activity and asset belong to the specified user
    pub async fn link_asset_to_activity(
        &self,
        activity_id: Uuid,
        asset_id: Uuid,
        user_id: Uuid,
    ) -> DbResult<ActivityAsset> {
        let now = Utc::now();

        // Use CTE to verify ownership of both activity and asset before linking
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

    // =========================================================================
    // Conversation Management Methods
    // =========================================================================

    /// Create a new conversation
    pub async fn create_conversation(&self, request: NewConversation) -> DbResult<Conversation> {
        let id = request.id.unwrap_or_else(Uuid::now_v7);
        let now = Utc::now();

        let conversation = sqlx::query_as::<_, Conversation>(
            r#"
            INSERT INTO conversations (id, user_id, title, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, user_id, title, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(request.user_id)
        .bind(&request.title)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(conversation)
    }

    /// Get a conversation by ID
    pub async fn get_conversation(&self, request: GetConversation) -> DbResult<Conversation> {
        let conversation = sqlx::query_as::<_, Conversation>(
            r#"
            SELECT id, user_id, title, created_at, updated_at
            FROM conversations
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(request.id)
        .bind(request.user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(conversation)
    }

    /// List conversations for a user with pagination
    pub async fn list_conversations(
        &self,
        request: ListConversations,
    ) -> DbResult<Vec<Conversation>> {
        // Clamp limit to max 100
        let limit = request.limit.clamp(1, 100);

        let conversations = sqlx::query_as::<_, Conversation>(
            r#"
            SELECT id, user_id, title, created_at, updated_at
            FROM conversations
            WHERE user_id = $1
            ORDER BY updated_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(request.user_id)
        .bind(limit as i64)
        .bind(request.offset as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(conversations)
    }

    // =========================================================================
    // Message Management Methods
    // =========================================================================

    /// Create a new message
    pub async fn create_message(&self, request: NewMessage) -> DbResult<Message> {
        let id = request.id.unwrap_or_else(Uuid::now_v7);
        let now = Utc::now();
        let additional_kwargs = request
            .additional_kwargs
            .unwrap_or_else(|| serde_json::json!({}));

        let message = sqlx::query_as::<_, Message>(
            r#"
            WITH verified_conversation AS (
                SELECT id FROM conversations
                WHERE id = $2 AND user_id = $3
            ),
            updated_conversation AS (
                UPDATE conversations
                SET updated_at = $10
                WHERE id = (SELECT id FROM verified_conversation)
                RETURNING id
            ),
            inserted_message AS (
                INSERT INTO messages (id, conversation_id, user_id, message_type, content, tool_call_id, tool_calls, additional_kwargs, created_at, updated_at)
                SELECT $1, vc.id, $3, $4, $5, $6, $7, $8, $9, $10
                FROM verified_conversation vc
                RETURNING id, conversation_id, user_id, message_type, content, tool_call_id, tool_calls, additional_kwargs, created_at, updated_at
            )
            SELECT * FROM inserted_message
            "#,
        )
        .bind(id)
        .bind(request.conversation_id)
        .bind(request.user_id)
        .bind(request.message_type)
        .bind(&request.content)
        .bind(&request.tool_call_id)
        .bind(&request.tool_calls)
        .bind(&additional_kwargs)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(message)
    }

    /// List messages for a conversation with pagination
    pub async fn list_messages_desc(&self, request: ListMessages) -> DbResult<Vec<Message>> {
        let limit = request.limit.clamp(1, 100);

        let messages = sqlx::query_as::<_, Message>(
                    r#"
                    SELECT m.id, m.conversation_id, m.user_id, m.message_type, m.content, m.tool_call_id, m.tool_calls, m.additional_kwargs, m.created_at, m.updated_at
                    FROM messages m
                    WHERE m.conversation_id = $1 AND m.user_id = $2
                    ORDER BY m.id DESC
                    LIMIT $3 OFFSET $4
                    "#,
                )
                .bind(request.conversation_id)
                .bind(request.user_id)
                .bind(limit as i64)
                .bind(request.offset as i64)
                .fetch_all(&self.pool)
                .await?;

        Ok(messages)
    }

    /// List messages for a conversation with pagination
    pub async fn list_messages(&self, request: ListMessages) -> DbResult<Vec<Message>> {
        let limit = request.limit.clamp(1, 100);

        let messages = sqlx::query_as::<_, Message>(
                    r#"
                    SELECT m.id, m.conversation_id, m.user_id, m.message_type, m.content, m.tool_call_id, m.tool_calls, m.additional_kwargs, m.created_at, m.updated_at
                    FROM messages m
                    WHERE m.conversation_id = $1 AND m.user_id = $2
                    ORDER BY m.id ASC
                    LIMIT $3 OFFSET $4
                    "#,
                )
                .bind(request.conversation_id)
                .bind(request.user_id)
                .bind(limit as i64)
                .bind(request.offset as i64)
                .fetch_all(&self.pool)
                .await?;

        Ok(messages)
    }
}
