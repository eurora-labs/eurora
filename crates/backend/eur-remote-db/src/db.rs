use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::{
    migrate::MigrateDatabase,
    postgres::{PgPool, PgPoolOptions},
};
use tracing::debug;
use uuid::Uuid;

use crate::types::{
    CreateLoginTokenRequest, CreateOAuthCredentialsRequest, CreateOAuthStateRequest,
    CreateRefreshTokenRequest, CreateUserRequest, LoginToken, OAuthCredentials, OAuthState,
    PasswordCredentials, RefreshToken, UpdateOAuthCredentialsRequest, UpdatePasswordRequest,
    UpdateUserRequest, User,
};
#[derive(Debug)]
pub struct DatabaseManager {
    pub pool: PgPool,
}

impl DatabaseManager {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
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

    async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
        let mut migrator = sqlx::migrate!("./src/migrations");
        migrator.set_ignore_missing(true);
        match migrator.run(pool).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    // User management methods
    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User, sqlx::Error> {
        let user_id = Uuid::new_v4();
        let password_id = Uuid::new_v4();
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

    pub async fn get_user_by_id(&self, user_id: Uuid) -> Result<User, sqlx::Error> {
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

    pub async fn get_user_by_username(&self, username: &str) -> Result<User, sqlx::Error> {
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

    pub async fn get_user_by_email(&self, email: &str) -> Result<User, sqlx::Error> {
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

    pub async fn update_user(
        &self,
        user_id: Uuid,
        request: UpdateUserRequest,
    ) -> Result<User, sqlx::Error> {
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

    pub async fn delete_user(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
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

    pub async fn list_users(&self) -> Result<Vec<User>, sqlx::Error> {
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
    pub async fn get_password_credentials(
        &self,
        user_id: Uuid,
    ) -> Result<PasswordCredentials, sqlx::Error> {
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
    ) -> Result<PasswordCredentials, sqlx::Error> {
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
    ) -> Result<Option<User>, sqlx::Error> {
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

    pub async fn verify_email(&self, user_id: Uuid) -> Result<User, sqlx::Error> {
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
    pub async fn user_exists_by_username(&self, username: &str) -> Result<bool, sqlx::Error> {
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

    pub async fn user_exists_by_email(&self, email: &str) -> Result<bool, sqlx::Error> {
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
    ) -> Result<OAuthCredentials, sqlx::Error> {
        let id = Uuid::new_v4();
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
    ) -> Result<OAuthCredentials, sqlx::Error> {
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
    ) -> Result<OAuthCredentials, sqlx::Error> {
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
    ) -> Result<OAuthCredentials, sqlx::Error> {
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
    ) -> Result<User, sqlx::Error> {
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
    ) -> Result<RefreshToken, sqlx::Error> {
        let id = Uuid::new_v4();
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

    pub async fn get_refresh_token_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<RefreshToken, sqlx::Error> {
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

    pub async fn revoke_refresh_token(&self, token_hash: &str) -> Result<(), sqlx::Error> {
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

    pub async fn revoke_all_user_refresh_tokens(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
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

    pub async fn cleanup_expired_refresh_tokens(&self) -> Result<u64, sqlx::Error> {
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
    ) -> Result<OAuthState, sqlx::Error> {
        let id = Uuid::new_v4();
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

    pub async fn get_oauth_state_by_state(&self, state: &str) -> Result<OAuthState, sqlx::Error> {
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

    pub async fn consume_oauth_state(&self, state: &str) -> Result<(), sqlx::Error> {
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

    pub async fn cleanup_expired_oauth_states(&self) -> Result<u64, sqlx::Error> {
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
    ) -> Result<LoginToken, sqlx::Error> {
        let id = Uuid::new_v4();
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

    pub async fn get_login_token_by_token(&self, token: &str) -> Result<LoginToken, sqlx::Error> {
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

    pub async fn consume_login_token(&self, token: &str) -> Result<LoginToken, sqlx::Error> {
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

    pub async fn cleanup_expired_login_tokens(&self) -> Result<u64, sqlx::Error> {
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
}
