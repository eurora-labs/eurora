use chrono::{DateTime, Utc};
use sqlx::migrate::MigrateDatabase;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use tracing::debug;
use uuid::Uuid;

use crate::types::{
    CreateUserRequest, PasswordCredentials, UpdatePasswordRequest, UpdateUserRequest, User,
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
            INSERT INTO password_credentials (id, user_id, password_hash, password_salt, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(password_id)
        .bind(user_id)
        .bind(&request.password_hash)
        .bind(&request.password_salt)
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
        .bind(&request.email_verified)
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
            SELECT id, user_id, password_hash, password_salt, updated_at
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
                password_salt = $3,
                updated_at = $4
            WHERE user_id = $1
            RETURNING id, user_id, password_hash, password_salt, updated_at
            "#,
        )
        .bind(user_id)
        .bind(&request.password_hash)
        .bind(&request.password_salt)
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
}
