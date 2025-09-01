use std::{str::FromStr, time::Duration};

use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, Utc};
use eur_secret::{Sensitive, secret};
use libsqlite3_sys::sqlite3_auto_extension;
use rand::{TryRngCore, rngs::OsRng};
use sqlite_vec::sqlite3_vec_init;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions},
    types::Uuid,
};
use tracing::{debug, info};

use crate::types::{Activity, ActivityAsset, ChatMessage, Conversation};

pub struct PersonalDatabaseManager {
    pub pool: SqlitePool,
}

impl PersonalDatabaseManager {
    pub async fn new(database_path: &str) -> Result<Self, sqlx::Error> {
        debug!(
            "Initializing PersonalDatabaseManager with database path: {}",
            database_path
        );
        let connection_string = format!("sqlite:{}", database_path);
        debug!("Initializing database connection");

        unsafe {
            sqlite3_auto_extension(Some(std::mem::transmute::<
                *const (),
                unsafe extern "C" fn(
                    *mut libsqlite3_sys::sqlite3,
                    *mut *mut i8,
                    *const libsqlite3_sys::sqlite3_api_routines,
                ) -> i32,
            >(sqlite3_vec_init as *const ())));
        }

        let mut opts = SqliteConnectOptions::from_str(&connection_string)?
            .pragma("journal_mode", "WAL")
            .pragma("cache_size", "2000")
            .pragma("temp_store", "MEMORY")
            .create_if_missing(true);

        // The database for development is unencrypted
        if cfg!(not(debug_assertions)) {
            let key = init_key().map_err(|e| sqlx::Error::Configuration(e.into()))?;
            opts = opts
                .pragma("key", format!("'{}'", key.0))
                .pragma("kdf_iter", "64000")
                .pragma("cipher_page_size", "4096")
                .pragma("cipher_hmac_algorithm", "HMAC_SHA512")
                .pragma("cipher_kdf_algorithm", "PBKDF2_HMAC_SHA512");
        }

        let pool = SqlitePoolOptions::new()
            .max_connections(50)
            .min_connections(3)
            .acquire_timeout(Duration::from_secs(10))
            .connect_with(opts)
            .await?;

        let db_manager = PersonalDatabaseManager { pool };

        // Run migrations after establishing the connection and setting up encryption
        Self::run_migrations(&db_manager.pool)
            .await
            .map_err(|e| sqlx::Error::Migrate(Box::new(e)))?;

        Ok(db_manager)
    }

    async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
        let migrator = sqlx::migrate!("src/migrations");
        migrator.run(pool).await
    }

    pub async fn insert_conversation(
        &self,
        title: &str,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Conversation, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO conversation (id, title, created_at, updated_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(id.clone())
        .bind(title)
        .bind(created_at)
        .bind(updated_at)
        .execute(&self.pool)
        .await?;

        Ok(Conversation {
            id: id.to_string(),
            title: title.to_string(),
            created_at,
            updated_at,
        })
    }

    pub async fn get_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Conversation, sqlx::Error> {
        let conversation = sqlx::query_as(
            r#"
            SELECT id, title, created_at, updated_at
            FROM conversation
            WHERE id = ?
             "#,
        )
        .bind(conversation_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(conversation)
    }

    pub async fn list_conversations(
        &self,
        limit: u16,
        offset: u16,
    ) -> Result<Vec<Conversation>, sqlx::Error> {
        let conversations = sqlx::query_as(
            r#"
            SELECT id, title, created_at, updated_at
            FROM conversation
            ORDER BY created_at DESC
            LIMIT $1,$2
            "#,
        )
        .bind(offset)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(conversations)
    }

    pub async fn get_conversation_with_messages(
        &self,
        conversation_id: &str,
    ) -> Result<(Conversation, Vec<ChatMessage>), sqlx::Error> {
        let conversation = self.get_conversation(conversation_id).await?;
        let messages = self.get_chat_messages(conversation_id).await?;

        // let conversation = self.get_conversation(conversation_id).await?;
        // let messages = self.get_chat_messages(conversation_id);

        // Use future to wait in parallel
        // let (conversation, messages) = try_join!(conversation, messages)?;

        info!("Conversation: {:?}", conversation);
        // info!("Messages: {:?}", messages);

        Ok((conversation, messages))
        // Ok((conversation, vec![]))
    }

    pub async fn insert_chat_message(
        &self,
        conversation_id: &str,
        role: &str,
        content: &str,
        visible: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<ChatMessage, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let result = sqlx::query(
            r#"
            INSERT INTO chat_message (id, conversation_id, role, content, visible, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(conversation_id)
        .bind(role)
        .bind(content)
        .bind(visible)
        .bind(created_at)
        .bind(updated_at)
        .execute(&self.pool)
        .await?;

        Ok(ChatMessage {
            id,
            conversation_id: conversation_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            visible,
            created_at,
            updated_at,
        })
    }

    pub async fn get_chat_messages(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<ChatMessage>, sqlx::Error> {
        let messages = sqlx::query_as(
            r#"
            SELECT id, conversation_id, role, content, visible, created_at, updated_at
            FROM chat_message
            WHERE conversation_id = ?
            "#,
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(messages)
    }

    pub async fn insert_activity(&self, activity: &Activity) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO activity (id, name, conversation_id, icon_path, process_name, started_at, ended_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(activity.id.clone())
        .bind(activity.name.clone())
        .bind(activity.conversation_id.clone())
        .bind(activity.icon_path.clone())
        .bind(activity.process_name.clone())
        .bind(activity.start.clone())
        .bind(activity.end.clone())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn insert_activity_asset(
        &self,
        activity_id: &str,
        asset: &ActivityAsset,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO activity_asset (id, activity_id, data, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(activity_id)
        .bind(asset.data.clone())
        .bind(asset.created_at.clone())
        .bind(asset.updated_at.clone())
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

pub const PERSONAL_DB_KEY_HANDLE: &str = "PERSONAL_DB_KEY";

fn init_key() -> Result<Sensitive<String>> {
    let key = secret::retrieve(PERSONAL_DB_KEY_HANDLE, secret::Namespace::Global)
        .map_err(|e| anyhow!("Failed to retrieve key: {}", e))?;
    if let Some(key) = key {
        Ok(key)
    } else {
        let mut key = [0u8; 32];
        OsRng
            .try_fill_bytes(&mut key)
            .map_err(|e| anyhow!("Failed to generate random key: {}", e))?;
        let b64_key = general_purpose::STANDARD.encode(key);
        secret::persist(
            PERSONAL_DB_KEY_HANDLE,
            &eur_secret::Sensitive(b64_key.clone()),
            secret::Namespace::Global,
        )
        .map_err(|e| anyhow!("Failed to persist key: {}", e))?;
        Ok(Sensitive(b64_key))
    }
}
