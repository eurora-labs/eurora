use crate::{
    GetConversation, MessageType, PaginationParams,
    error::{DbError, DbResult},
    types::{
        Activity, ActivityAsset, Asset, AssetStatus, Conversation, CreateLoginToken,
        CreateOAuthCredentials, CreateOAuthState, CreateRefreshToken, GetActivitiesByTimeRange,
        ListActivities, ListConversations, LoginToken, Message, NewActivity, NewAsset,
        NewConversation, NewUser, OAuthCredentials, OAuthProvider, OAuthState, PasswordCredentials,
        RefreshToken, UpdateActivity, UpdateActivityEndTime, UpdateOAuthCredentials, User,
    },
};
use bon::bon;
use chrono::Utc;
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

    pub async fn create_user(&self, request: NewUser) -> DbResult<User> {
        let user_id = Uuid::now_v7();
        let now = Utc::now();

        let mut tx = self.pool.begin().await?;

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
        .bind(now) // updated_at is NOT NULL, set to now initially
        .fetch_one(&mut *tx)
        .await?;

        // OAuth-only users don't have password credentials
        if let Some(ref password_hash) = request.password_hash {
            let password_id = Uuid::now_v7();
            sqlx::query(
                r#"
                INSERT INTO password_credentials (id, user_id, password_hash, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(password_id)
            .bind(user_id)
            .bind(password_hash)
            .bind(now) // created_at
            .bind(now) // updated_at is NOT NULL, set to now initially
            .execute(&mut *tx)
            .await?;
        }

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

    pub async fn get_password_credentials(&self, user_id: Uuid) -> DbResult<PasswordCredentials> {
        let credentials = sqlx::query_as::<_, PasswordCredentials>(
            r#"
            SELECT id, user_id, password_hash, created_at, updated_at
            FROM password_credentials
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(credentials)
    }

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
        .bind(request.provider)
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
        provider: OAuthProvider,
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
        provider: OAuthProvider,
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

    pub async fn get_refresh_token_by_hash(&self, token_hash: &[u8]) -> DbResult<RefreshToken> {
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

    pub async fn revoke_refresh_token(&self, token_hash: &[u8]) -> DbResult<RefreshToken> {
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

    pub async fn create_oauth_state(&self, request: CreateOAuthState) -> DbResult<OAuthState> {
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
        .bind(&request.state)
        .bind(&request.pkce_verifier)
        .bind(&request.redirect_uri)
        .bind(request.ip_address)
        .bind(false) // consumed defaults to false
        .bind(now)
        .bind(now)
        .bind(request.expires_at)
        .bind(&request.nonce)
        .fetch_one(&self.pool)
        .await?;

        Ok(oauth_state)
    }

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

    pub async fn create_login_token(&self, request: CreateLoginToken) -> DbResult<LoginToken> {
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
        .bind(&request.token_hash)
        .bind(request.expires_at)
        .bind(request.user_id)
        .bind(false) // consumed starts as false
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(login_token)
    }

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

    /// Look up a login token by hash regardless of consumed status (still checks expiry).
    /// Used to detect already-consumed tokens for idempotent retry handling.
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

    pub async fn consume_login_token(&self, token_hash: &[u8]) -> DbResult<LoginToken> {
        let now = Utc::now();

        let login_token = sqlx::query_as::<_, LoginToken>(
            r#"
            UPDATE login_tokens
            SET consumed = true, updated_at = $2
            WHERE token_hash = $1 AND consumed = false AND expires_at > now()
            RETURNING id, token_hash, consumed, expires_at, user_id, created_at, updated_at
            "#,
        )
        .bind(token_hash)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(login_token)
    }

    // =========================================================================
    // Activity Management Methods
    // =========================================================================

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

    pub async fn list_activities(
        &self,
        request: ListActivities,
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
            .bind(request.user_id)
            .bind(params.limit())
            .bind(params.offset())
            .fetch_all(&self.pool)
            .await?;

        Ok(activities)
    }

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

    pub async fn get_activities_by_time_range(
        &self,
        request: GetActivitiesByTimeRange,
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
            .bind(request.user_id)
            .bind(request.start_time)
            .bind(request.end_time)
            .bind(params.limit())
            .bind(params.offset())
            .fetch_all(&self.pool)
            .await?;

        Ok(activities)
    }

    // =========================================================================
    // Asset Management Methods
    // =========================================================================

    pub async fn create_asset(&self, request: NewAsset) -> DbResult<Asset> {
        let id = request.id.unwrap_or_else(Uuid::now_v7);
        let now = Utc::now();
        let metadata = request.metadata.unwrap_or_else(|| serde_json::json!({}));
        let status = request.status.unwrap_or(AssetStatus::Uploaded);

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
        .bind(&request.storage_backend)
        .bind(&request.storage_uri)
        .bind(status)
        .bind(&metadata)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(asset)
    }

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

    // =========================================================================
    // Conversation Management Methods
    // =========================================================================

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

    #[builder]
    pub async fn update_conversation(
        &self,
        id: Uuid,
        user_id: Uuid,
        title: String,
    ) -> DbResult<Conversation> {
        let now = Utc::now();

        let conversation = sqlx::query_as::<_, Conversation>(
            r#"
            UPDATE conversations
            SET title = $1, updated_at = $2
            WHERE id = $3 AND user_id = $4
            RETURNING id, user_id, title, created_at, updated_at
            "#,
        )
        .bind(&title)
        .bind(now)
        .bind(id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(conversation)
    }

    pub async fn list_conversations(
        &self,
        request: ListConversations,
        params: PaginationParams,
    ) -> DbResult<Vec<Conversation>> {
        let query = format!(
            r#"
            SELECT id, user_id, title, created_at, updated_at
            FROM conversations
            WHERE user_id = $1
            ORDER BY id {}
            LIMIT $2 OFFSET $3
            "#,
            params.order()
        );

        let conversations = sqlx::query_as::<_, Conversation>(&query)
            .bind(request.user_id)
            .bind(params.limit())
            .bind(params.offset())
            .fetch_all(&self.pool)
            .await?;

        Ok(conversations)
    }

    // =========================================================================
    // Message Management Methods
    // =========================================================================

    #[builder]
    pub async fn create_message(
        &self,
        id: Option<Uuid>,
        conversation_id: Uuid,
        user_id: Uuid,
        message_type: MessageType,
        content: serde_json::Value,
        tool_call_id: Option<String>,
        tool_calls: Option<serde_json::Value>,
        additional_kwargs: Option<serde_json::Value>,
        hidden_from_ui: Option<bool>,
    ) -> DbResult<Message> {
        let id = id.unwrap_or_else(Uuid::now_v7);
        let now = Utc::now();
        let additional_kwargs = additional_kwargs.unwrap_or_else(|| serde_json::json!({}));
        let hidden_from_ui = hidden_from_ui.unwrap_or(false);

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
                INSERT INTO messages (id, conversation_id, user_id, message_type, content, tool_call_id, tool_calls, additional_kwargs, hidden_from_ui, created_at, updated_at)
                SELECT $1, vc.id, $3, $4, $5, $6, $7, $8, $9, $10, $11
                FROM verified_conversation vc
                RETURNING id, conversation_id, user_id, message_type, content, tool_call_id, tool_calls, additional_kwargs, hidden_from_ui, created_at, updated_at
            )
            SELECT * FROM inserted_message
            "#,
        )
        .bind(id)
        .bind(conversation_id)
        .bind(user_id)
        .bind(message_type)
        .bind(&content)
        .bind(&tool_call_id)
        .bind(&tool_calls)
        .bind(&additional_kwargs)
        .bind(hidden_from_ui)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(message)
    }

    #[builder]
    /// `only_visible`: `None` = all, `Some(true)` = visible only, `Some(false)` = hidden only.
    pub async fn list_messages(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
        params: Option<PaginationParams>,
        only_visible: Option<bool>,
    ) -> DbResult<Vec<Message>> {
        let params = params.unwrap_or_default();

        let messages = match only_visible {
            Some(visible) => {
                let hidden_from_ui = !visible;
                let query = format!(
                    r#"
                    SELECT m.id, m.conversation_id, m.user_id, m.message_type, m.content, m.tool_call_id, m.tool_calls, m.additional_kwargs, m.hidden_from_ui, m.created_at, m.updated_at
                    FROM messages m
                    WHERE m.conversation_id = $1 AND m.user_id = $2 AND m.hidden_from_ui = $3
                    ORDER BY m.id {}
                    LIMIT $4 OFFSET $5
                    "#,
                    params.order()
                );
                sqlx::query_as::<_, Message>(&query)
                    .bind(conversation_id)
                    .bind(user_id)
                    .bind(hidden_from_ui)
                    .bind(params.limit())
                    .bind(params.offset())
                    .fetch_all(&self.pool)
                    .await?
            }
            None => {
                let query = format!(
                    r#"
                    SELECT m.id, m.conversation_id, m.user_id, m.message_type, m.content, m.tool_call_id, m.tool_calls, m.additional_kwargs, m.hidden_from_ui, m.created_at, m.updated_at
                    FROM messages m
                    WHERE m.conversation_id = $1 AND m.user_id = $2
                    ORDER BY m.id {}
                    LIMIT $3 OFFSET $4
                    "#,
                    params.order()
                );
                sqlx::query_as::<_, Message>(&query)
                    .bind(conversation_id)
                    .bind(user_id)
                    .bind(params.limit())
                    .bind(params.offset())
                    .fetch_all(&self.pool)
                    .await?
            }
        };

        Ok(messages)
    }

    // =========================================================================
    // Stripe Billing Methods
    // =========================================================================

    pub async fn is_webhook_event_processed(&self, event_id: &str) -> DbResult<bool> {
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM stripe.webhook_events WHERE event_id = $1")
                .bind(event_id)
                .fetch_one(&self.pool)
                .await?;

        Ok(count.0 > 0)
    }

    pub async fn record_webhook_event(&self, event_id: &str, event_type: &str) -> DbResult<()> {
        sqlx::query(
            r#"
            INSERT INTO stripe.webhook_events (event_id, event_type)
            VALUES ($1, $2)
            ON CONFLICT (event_id) DO NOTHING
            "#,
        )
        .bind(event_id)
        .bind(event_type)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_billing_state_for_user(
        &self,
        user_id: Uuid,
    ) -> DbResult<Option<crate::AccountBillingState>> {
        let result = sqlx::query_as::<_, crate::AccountBillingState>(
            "SELECT abs.*
             FROM account_billing_state abs
             JOIN accounts a ON a.id = abs.account_id
             WHERE a.owner_user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(result)
    }

    /// If `email` matches an existing user, `app_user_id` is set automatically.
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

        let app_user_id: Option<Uuid> = if let Some(email) = email {
            sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
                .bind(email)
                .fetch_optional(&self.pool)
                .await?
        } else {
            None
        };

        sqlx::query(
            r#"
            INSERT INTO stripe.customers (id, app_user_id, email, created_at, updated_at, raw_data)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (id) DO UPDATE
            SET email = COALESCE(EXCLUDED.email, stripe.customers.email),
                app_user_id = COALESCE(EXCLUDED.app_user_id, stripe.customers.app_user_id),
                raw_data = EXCLUDED.raw_data,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(customer_id)
        .bind(app_user_id)
        .bind(email)
        .bind(now)
        .bind(now)
        .bind(raw_data)
        .execute(executor)
        .await?;

        Ok(())
    }

    /// Upserts an account for the user, linking to a Stripe customer. Returns the account ID.
    pub async fn ensure_account_for_user<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
        stripe_customer_id: &str,
    ) -> DbResult<Uuid>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let now = Utc::now();

        let account_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO accounts (owner_user_id, name, stripe_customer_id, created_at, updated_at)
            SELECT $1, u.username, $2, $3, $4
            FROM users u WHERE u.id = $1
            ON CONFLICT (owner_user_id) DO UPDATE
            SET stripe_customer_id = EXCLUDED.stripe_customer_id,
                updated_at = EXCLUDED.updated_at
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(stripe_customer_id)
        .bind(now)
        .bind(now)
        .fetch_one(executor)
        .await?;

        Ok(account_id)
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
        let canceled_at_ts = canceled_at.and_then(|ts| chrono::DateTime::from_timestamp(ts, 0));
        let period_start = chrono::DateTime::from_timestamp(current_period_start, 0).unwrap_or(now);
        let period_end = chrono::DateTime::from_timestamp(current_period_end, 0).unwrap_or(now);

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

    /// Replaces all existing items for the subscription with the provided set.
    pub async fn sync_stripe_subscription_items<'e, E>(
        &self,
        executor: E,
        subscription_id: &str,
        items: &[(String, String, Option<i64>, serde_json::Value)], // (item_id, price_id, quantity, raw_data)
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

        // Single CTE to delete + insert in one round-trip (executor is consumed after one use)
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
}
