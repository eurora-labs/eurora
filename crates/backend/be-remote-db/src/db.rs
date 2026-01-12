use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::{
    migrate::MigrateDatabase,
    postgres::{PgPool, PgPoolOptions},
};
use tracing::{debug, info};
use uuid::Uuid;

use crate::error::DbResult;
use crate::types::{
    Activity, ActivityAsset, Asset, CreateActivityRequest, CreateAssetRequest,
    CreateLoginTokenRequest, CreateOAuthCredentialsRequest, CreateOAuthStateRequest,
    CreateRefreshTokenRequest, CreateUserRequest, GetActivitiesByTimeRangeRequest,
    ListActivitiesRequest, LoginToken, MessageAsset, OAuthCredentials, OAuthState,
    PasswordCredentials, RefreshToken, UpdateActivityEndTimeRequest, UpdateActivityRequest,
    UpdateAssetRequest, UpdateOAuthCredentialsRequest, UpdatePasswordRequest, UpdateUserRequest,
    User,
};
#[derive(Debug)]
pub struct DatabaseManager {
    pub pool: PgPool,
}

impl DatabaseManager {
    pub async fn new(database_url: &str) -> DbResult<Self> {
        debug!(
            "Initializing DatabaseManager with database URL: {}",
            database_url
        );

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
        let mut migrator = sqlx::migrate!("./src/migrations");
        migrator.set_ignore_missing(true);
        migrator.run(pool).await?;
        Ok(())
    }

    // User management methods
    pub async fn create_user(&self, request: CreateUserRequest) -> DbResult<User> {
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

    pub async fn update_user(&self, user_id: Uuid, request: UpdateUserRequest) -> DbResult<User> {
        let now = Utc::now();

        let user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET username = COALESCE($2, username),
                email = COALESCE($3, email),
                display_name = COALESCE($4, display_name),
                email_verified = COALESCE($5, email_verified),
                updated_at = $6
            WHERE id = $1
            RETURNING id, username, email, display_name, email_verified, created_at, updated_at
            "#,
        )
        .bind(user_id)
        .bind(&request.username)
        .bind(&request.email)
        .bind(&request.display_name)
        .bind(request.email_verified)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn delete_user(&self, user_id: Uuid) -> DbResult<()> {
        // Due to CASCADE DELETE constraint, this will also delete password_credentials
        sqlx::query(
            r#"
            DELETE FROM users
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_users(&self) -> DbResult<Vec<User>> {
        let users = sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, email, display_name, email_verified, created_at, updated_at
            FROM users
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
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

    pub async fn update_password(
        &self,
        user_id: Uuid,
        request: UpdatePasswordRequest,
    ) -> DbResult<PasswordCredentials> {
        let now = Utc::now();

        let credentials = sqlx::query_as::<_, PasswordCredentials>(
            r#"
            UPDATE password_credentials
            SET password_hash = $2,
                updated_at = $3
            WHERE user_id = $1
            RETURNING id, user_id, password_hash, updated_at
            "#,
        )
        .bind(user_id)
        .bind(&request.password_hash)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(credentials)
    }

    // Authentication helper methods
    pub async fn authenticate_user(
        &self,
        username_or_email: &str,
        password_hash: &str,
    ) -> DbResult<Option<User>> {
        let user_result = sqlx::query_as::<_, User>(
            r#"
            SELECT u.id, u.username, u.email, u.display_name, u.email_verified, u.created_at, u.updated_at
            FROM users u
            INNER JOIN password_credentials pc ON u.id = pc.user_id
            WHERE (u.username = $1 OR u.email = $1) AND pc.password_hash = $2
            "#,
        )
        .bind(username_or_email)
        .bind(password_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user_result)
    }

    pub async fn verify_email(&self, user_id: Uuid) -> DbResult<User> {
        let now = Utc::now();

        let user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET email_verified = true,
                updated_at = $2
            WHERE id = $1
            RETURNING id, username, email, display_name, email_verified, created_at, updated_at
            "#,
        )
        .bind(user_id)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
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
        request: CreateOAuthCredentialsRequest,
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

    pub async fn get_oauth_credentials_by_provider_user_id(
        &self,
        provider: &str,
        provider_user_id: &str,
    ) -> DbResult<OAuthCredentials> {
        let oauth_creds = sqlx::query_as::<_, OAuthCredentials>(
            r#"
            SELECT id, user_id, provider, provider_user_id, access_token,
                   refresh_token, access_token_expiry, scope, issued_at, created_at, updated_at
            FROM oauth_credentials
            WHERE provider = $1 AND provider_user_id = $2
            "#,
        )
        .bind(provider)
        .bind(provider_user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(oauth_creds)
    }

    pub async fn update_oauth_credentials(
        &self,
        id: Uuid,
        request: UpdateOAuthCredentialsRequest,
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
        request: CreateRefreshTokenRequest,
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

    pub async fn revoke_refresh_token(&self, token_hash: &str) -> DbResult<()> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE refresh_tokens
            SET revoked = true, updated_at = $2
            WHERE token_hash = $1
            "#,
        )
        .bind(token_hash)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn revoke_all_user_refresh_tokens(&self, user_id: Uuid) -> DbResult<()> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE refresh_tokens
            SET revoked = true, updated_at = $2
            WHERE user_id = $1 AND revoked = false
            "#,
        )
        .bind(user_id)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn cleanup_expired_refresh_tokens(&self) -> DbResult<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM refresh_tokens
            WHERE expires_at < now()
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    // OAuth state management methods
    pub async fn create_oauth_state(
        &self,
        request: CreateOAuthStateRequest,
    ) -> DbResult<OAuthState> {
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

    pub async fn consume_oauth_state(&self, state: &str) -> DbResult<()> {
        sqlx::query(
            r#"
            UPDATE oauth_state
            SET consumed = true
            WHERE state = $1
            "#,
        )
        .bind(state)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn cleanup_expired_oauth_states(&self) -> DbResult<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM oauth_state
            WHERE expires_at < now()
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    // Login token management methods
    pub async fn create_login_token(
        &self,
        request: CreateLoginTokenRequest,
    ) -> DbResult<LoginToken> {
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
            WHERE token = $1 AND expires_at > now()
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

    pub async fn cleanup_expired_login_tokens(&self) -> DbResult<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM login_tokens
            WHERE expires_at < now()
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    // =========================================================================
    // Activity Management Methods
    // =========================================================================

    /// Create a new activity
    pub async fn create_activity(&self, request: CreateActivityRequest) -> DbResult<Activity> {
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

    /// Get an activity by ID
    pub async fn get_activity(&self, activity_id: Uuid) -> DbResult<Activity> {
        let activity = sqlx::query_as::<_, Activity>(
            r#"
            SELECT id, user_id, name, icon_asset_id, process_name, window_title, started_at, ended_at, created_at, updated_at
            FROM activities
            WHERE id = $1
            "#,
        )
        .bind(activity_id)
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
    pub async fn list_activities(
        &self,
        request: ListActivitiesRequest,
    ) -> DbResult<(Vec<Activity>, u64)> {
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

        // Get total count
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM activities WHERE user_id = $1
            "#,
        )
        .bind(request.user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok((activities, count.0 as u64))
    }

    /// Update an existing activity
    pub async fn update_activity(&self, request: UpdateActivityRequest) -> DbResult<Activity> {
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
    pub async fn update_activity_end_time(
        &self,
        request: UpdateActivityEndTimeRequest,
    ) -> DbResult<()> {
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
    pub async fn delete_activity(&self, activity_id: Uuid, user_id: Uuid) -> DbResult<()> {
        sqlx::query(
            r#"
            DELETE FROM activities
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(activity_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get activities by time range for a user
    pub async fn get_activities_by_time_range(
        &self,
        request: GetActivitiesByTimeRangeRequest,
    ) -> DbResult<(Vec<Activity>, u64)> {
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

        // Get total count for the time range
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM activities
            WHERE user_id = $1
              AND started_at >= $2
              AND started_at <= $3
            "#,
        )
        .bind(request.user_id)
        .bind(request.start_time)
        .bind(request.end_time)
        .fetch_one(&self.pool)
        .await?;

        Ok((activities, count.0 as u64))
    }

    // =========================================================================
    // Asset Management Methods
    // =========================================================================

    /// Create a new asset
    pub async fn create_asset(
        &self,
        user_id: Uuid,
        request: CreateAssetRequest,
    ) -> DbResult<Asset> {
        info!("create asset request: {:?}", request.clone());
        let id = request.id;
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
        .bind(user_id)
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

    /// Get an asset by ID
    pub async fn get_asset(&self, asset_id: Uuid) -> DbResult<Asset> {
        let asset = sqlx::query_as::<_, Asset>(
            r#"
            SELECT id, user_id, name, mime_type, size_bytes, checksum_sha256, storage_backend, storage_uri, status, metadata, created_at, updated_at
            FROM assets
            WHERE id = $1
            "#,
        )
        .bind(asset_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(asset)
    }

    /// Get an asset by ID for a specific user
    pub async fn get_asset_for_user(&self, asset_id: Uuid, user_id: Uuid) -> DbResult<Asset> {
        let asset = sqlx::query_as::<_, Asset>(
            r#"
            SELECT id, user_id, name, mime_type, size_bytes, checksum_sha256, storage_backend, storage_uri, status, metadata, created_at, updated_at
            FROM assets
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(asset_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(asset)
    }

    /// List assets for a user with pagination
    pub async fn list_assets(
        &self,
        user_id: Uuid,
        limit: u32,
        offset: u32,
    ) -> DbResult<(Vec<Asset>, u64)> {
        // Clamp limit to max 100
        let limit = limit.clamp(1, 100);

        let assets = sqlx::query_as::<_, Asset>(
            r#"
            SELECT id, user_id, name, mime_type, size_bytes, checksum_sha256, storage_backend, storage_uri, status, metadata, created_at, updated_at
            FROM assets
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

        // Get total count
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM assets WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok((assets, count.0 as u64))
    }

    /// Update an asset
    pub async fn update_asset(
        &self,
        asset_id: Uuid,
        user_id: Uuid,
        request: UpdateAssetRequest,
    ) -> DbResult<Asset> {
        let now = Utc::now();

        let asset = sqlx::query_as::<_, Asset>(
            r#"
            UPDATE assets
            SET checksum_sha256 = COALESCE($3, checksum_sha256),
                size_bytes = COALESCE($4, size_bytes),
                storage_uri = COALESCE($5, storage_uri),
                mime_type = COALESCE($6, mime_type),
                metadata = COALESCE($7, metadata),
                updated_at = $8
            WHERE id = $1 AND user_id = $2
            RETURNING id, user_id, name, mime_type, size_bytes, checksum_sha256, storage_backend, storage_uri, status, metadata, created_at, updated_at
            "#,
        )
        .bind(asset_id)
        .bind(user_id)
        .bind(&request.checksum_sha256)
        .bind(request.size_bytes)
        .bind(&request.storage_uri)
        .bind(&request.mime_type)
        .bind(&request.metadata)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(asset)
    }

    /// Delete an asset
    pub async fn delete_asset(&self, asset_id: Uuid, user_id: Uuid) -> DbResult<()> {
        sqlx::query(
            r#"
            DELETE FROM assets
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(asset_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get assets by message ID
    pub async fn get_assets_by_message_id(
        &self,
        message_id: Uuid,
        user_id: Uuid,
    ) -> DbResult<Vec<Asset>> {
        let assets = sqlx::query_as::<_, Asset>(
            r#"
            SELECT a.id, a.user_id, a.name, a.mime_type, a.size_bytes, a.checksum_sha256, a.storage_backend, a.storage_uri, a.status, a.metadata, a.created_at, a.updated_at
            FROM assets a
            INNER JOIN message_assets ma ON a.id = ma.asset_id
            WHERE ma.message_id = $1 AND a.user_id = $2
            "#,
        )
        .bind(message_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(assets)
    }

    /// Get assets by activity ID
    pub async fn get_assets_by_activity_id(
        &self,
        activity_id: Uuid,
        user_id: Uuid,
    ) -> DbResult<Vec<Asset>> {
        let assets = sqlx::query_as::<_, Asset>(
            r#"
            SELECT a.id, a.user_id, a.name, a.mime_type, a.size_bytes, a.checksum_sha256, a.storage_backend, a.storage_uri, a.status, a.metadata, a.created_at, a.updated_at
            FROM assets a
            INNER JOIN activity_assets aa ON a.id = aa.asset_id
            WHERE aa.activity_id = $1 AND a.user_id = $2
            "#,
        )
        .bind(activity_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(assets)
    }

    /// Link an asset to a message
    pub async fn link_asset_to_message(
        &self,
        message_id: Uuid,
        asset_id: Uuid,
    ) -> DbResult<MessageAsset> {
        let now = Utc::now();

        let message_asset = sqlx::query_as::<_, MessageAsset>(
            r#"
            INSERT INTO message_assets (message_id, asset_id, created_at)
            VALUES ($1, $2, $3)
            RETURNING message_id, asset_id, created_at
            "#,
        )
        .bind(message_id)
        .bind(asset_id)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(message_asset)
    }

    /// Unlink an asset from a message
    pub async fn unlink_asset_from_message(
        &self,
        message_id: Uuid,
        asset_id: Uuid,
    ) -> DbResult<()> {
        sqlx::query(
            r#"
            DELETE FROM message_assets
            WHERE message_id = $1 AND asset_id = $2
            "#,
        )
        .bind(message_id)
        .bind(asset_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Link an asset to an activity
    pub async fn link_asset_to_activity(
        &self,
        activity_id: Uuid,
        asset_id: Uuid,
    ) -> DbResult<ActivityAsset> {
        let now = Utc::now();

        let activity_asset = sqlx::query_as::<_, ActivityAsset>(
            r#"
            INSERT INTO activity_assets (activity_id, asset_id, created_at)
            VALUES ($1, $2, $3)
            RETURNING activity_id, asset_id, created_at
            "#,
        )
        .bind(activity_id)
        .bind(asset_id)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(activity_asset)
    }

    /// Unlink an asset from an activity
    pub async fn unlink_asset_from_activity(
        &self,
        activity_id: Uuid,
        asset_id: Uuid,
    ) -> DbResult<()> {
        sqlx::query(
            r#"
            DELETE FROM activity_assets
            WHERE activity_id = $1 AND asset_id = $2
            "#,
        )
        .bind(activity_id)
        .bind(asset_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Find asset by SHA256 hash for deduplication
    pub async fn find_asset_by_sha256(
        &self,
        user_id: Uuid,
        checksum_sha256: &[u8],
    ) -> DbResult<Option<Asset>> {
        let asset = sqlx::query_as::<_, Asset>(
            r#"
            SELECT id, user_id, name, mime_type, size_bytes, checksum_sha256, storage_backend, storage_uri, status, metadata, created_at, updated_at
            FROM assets
            WHERE user_id = $1 AND checksum_sha256 = $2
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .bind(checksum_sha256)
        .fetch_optional(&self.pool)
        .await?;

        Ok(asset)
    }
}
