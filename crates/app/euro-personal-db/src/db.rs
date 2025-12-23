//! Database manager for euro-personal-db.
//!
//! Provides operations for storing and retrieving conversations, messages,
//! activities, and assets using SQLite.

use std::{ffi::c_char, str::FromStr, time::Duration};

use agent_chain_core::BaseMessage;
use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, Utc};
use euro_secret::{Sensitive, secret};
use libsqlite3_sys::sqlite3_auto_extension;
use rand::{TryRngCore, rngs::OsRng};
use sqlite_vec::sqlite3_vec_init;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions},
    types::Uuid,
};
use tracing::{debug, error};

use crate::{
    NewAsset, NewConversation, NewMessage, NewMessageAsset, UpdateConversation,
    types::{Activity, Asset, Conversation, Message, MessageAsset},
};

/// Key handle for database encryption.
pub const PERSONAL_DB_KEY_HANDLE: &str = "PERSONAL_DB_KEY";

/// Database manager for the personal database.
#[derive(Debug, Clone)]
pub struct PersonalDatabaseManager {
    pub pool: SqlitePool,
}

impl PersonalDatabaseManager {
    /// Create a new database manager with the given path.
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
                    *mut *mut c_char,
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

        let pool = match SqlitePoolOptions::new()
            .max_connections(50)
            .min_connections(3)
            .acquire_timeout(Duration::from_secs(10))
            .connect_with(opts.clone())
            .await
        {
            Ok(pool) => pool,
            Err(e) => {
                // Delete the file and try again
                let _ = std::fs::remove_file(database_path);
                error!("Failed to connect to database: {}", e);

                SqlitePoolOptions::new()
                    .max_connections(50)
                    .min_connections(3)
                    .acquire_timeout(Duration::from_secs(10))
                    .connect_with(opts)
                    .await?
            }
        };

        let db_manager = PersonalDatabaseManager { pool };

        // Run migrations after establishing the connection
        Self::run_migrations(&db_manager.pool)
            .await
            .map_err(|e| sqlx::Error::Migrate(Box::new(e)))?;

        Ok(db_manager)
    }

    async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
        let migrator = sqlx::migrate!("src/migrations");
        migrator.run(pool).await
    }

    // ==================== Conversation Operations ====================

    /// Create an empty conversation.
    pub async fn insert_empty_conversation(&self) -> Result<Conversation, sqlx::Error> {
        self.insert_conversation(NewConversation::default()).await
    }

    /// Create a new conversation.
    pub async fn insert_conversation(
        &self,
        new_conversation: NewConversation,
    ) -> Result<Conversation, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let created_at = new_conversation.created_at.unwrap_or_else(Utc::now);
        let updated_at = created_at;

        sqlx::query(
            r#"
            INSERT INTO conversation (id, title, created_at, updated_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&new_conversation.title)
        .bind(created_at)
        .bind(updated_at)
        .execute(&self.pool)
        .await?;

        Ok(Conversation {
            id,
            title: new_conversation.title,
            created_at,
            updated_at,
        })
    }

    /// Update an existing conversation.
    pub async fn update_conversation(
        &self,
        conversation: UpdateConversation,
    ) -> Result<Conversation, sqlx::Error> {
        let updated_at = Utc::now();
        let created_at: DateTime<Utc> = sqlx::query_scalar(
            r#"
            UPDATE conversation
            SET title = ?, updated_at = ?
            WHERE id = ?
            RETURNING created_at
            "#,
        )
        .bind(&conversation.title)
        .bind(updated_at)
        .bind(&conversation.id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;

        Ok(Conversation {
            id: conversation.id,
            title: conversation.title,
            created_at,
            updated_at,
        })
    }

    /// Get a conversation by ID.
    pub async fn get_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Conversation, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, title, created_at, updated_at
            FROM conversation
            WHERE id = ?
            "#,
        )
        .bind(conversation_id)
        .fetch_one(&self.pool)
        .await
    }

    /// List conversations with pagination.
    pub async fn list_conversations(
        &self,
        limit: u16,
        offset: u16,
    ) -> Result<Vec<Conversation>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, title, created_at, updated_at
            FROM conversation
            ORDER BY updated_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Delete a conversation and all its messages.
    pub async fn delete_conversation(&self, conversation_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM conversation WHERE id = ?")
            .bind(conversation_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ==================== Message Operations ====================

    /// Insert a message from a NewMessage DTO.
    pub async fn insert_message(&self, new_message: NewMessage) -> Result<Message, sqlx::Error> {
        let id = new_message.id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let created_at = new_message.created_at.unwrap_or_else(Utc::now);
        let updated_at = new_message.updated_at.unwrap_or(created_at);
        let additional_kwargs = new_message
            .additional_kwargs
            .unwrap_or_else(|| "{}".to_string());

        sqlx::query(
            r#"
            INSERT INTO message (id, conversation_id, message_type, content, tool_call_id, tool_calls, additional_kwargs, sequence_num, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&new_message.conversation_id)
        .bind(&new_message.message_type)
        .bind(&new_message.content)
        .bind(&new_message.tool_call_id)
        .bind(&new_message.tool_calls)
        .bind(&additional_kwargs)
        .bind(new_message.sequence_num)
        .bind(created_at)
        .bind(updated_at)
        .execute(&self.pool)
        .await?;

        Ok(Message {
            id,
            conversation_id: new_message.conversation_id,
            message_type: new_message.message_type,
            content: new_message.content,
            tool_call_id: new_message.tool_call_id,
            tool_calls: new_message.tool_calls,
            additional_kwargs,
            sequence_num: new_message.sequence_num,
            created_at,
            updated_at,
        })
    }

    /// Insert an agent-chain BaseMessage directly.
    ///
    /// This is the primary method for saving messages from agent-chain conversations.
    pub async fn insert_base_message(
        &self,
        conversation_id: &str,
        message: &BaseMessage,
        sequence_num: i64,
    ) -> Result<Message, sqlx::Error> {
        let db_message =
            Message::from_base_message(message, conversation_id.to_string(), sequence_num)
                .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO message (id, conversation_id, message_type, content, tool_call_id, tool_calls, additional_kwargs, sequence_num, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&db_message.id)
        .bind(&db_message.conversation_id)
        .bind(&db_message.message_type)
        .bind(&db_message.content)
        .bind(&db_message.tool_call_id)
        .bind(&db_message.tool_calls)
        .bind(&db_message.additional_kwargs)
        .bind(db_message.sequence_num)
        .bind(db_message.created_at)
        .bind(db_message.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(db_message)
    }

    /// Insert multiple agent-chain BaseMessages at once.
    ///
    /// Returns the sequence number of the last inserted message.
    /// Uses a transaction to ensure atomicity - either all messages are inserted or none.
    pub async fn insert_base_messages(
        &self,
        conversation_id: &str,
        messages: &[BaseMessage],
    ) -> Result<i64, sqlx::Error> {
        // Start a transaction
        let mut tx = self.pool.begin().await?;

        // Get the current max sequence number within the transaction
        let start_seq: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(sequence_num), -1) + 1 FROM message WHERE conversation_id = ?",
        )
        .bind(conversation_id)
        .fetch_one(&mut *tx)
        .await?;

        // Insert all messages within the transaction
        for (i, message) in messages.iter().enumerate() {
            let sequence_num = start_seq + i as i64;
            let db_message =
                Message::from_base_message(message, conversation_id.to_string(), sequence_num)
                    .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;

            sqlx::query(
                r#"
                INSERT INTO message (id, conversation_id, message_type, content, tool_call_id, tool_calls, additional_kwargs, sequence_num, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&db_message.id)
            .bind(&db_message.conversation_id)
            .bind(&db_message.message_type)
            .bind(&db_message.content)
            .bind(&db_message.tool_call_id)
            .bind(&db_message.tool_calls)
            .bind(&db_message.additional_kwargs)
            .bind(db_message.sequence_num)
            .bind(db_message.created_at)
            .bind(db_message.updated_at)
            .execute(&mut *tx)
            .await?;
        }

        // Commit the transaction
        tx.commit().await?;

        Ok(start_seq + messages.len() as i64 - 1)
    }

    /// Get all messages for a conversation.
    pub async fn get_messages(&self, conversation_id: &str) -> Result<Vec<Message>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, conversation_id, message_type, content, tool_call_id, tool_calls, additional_kwargs, sequence_num, created_at, updated_at
            FROM message
            WHERE conversation_id = ?
            ORDER BY sequence_num ASC
            "#,
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Get all messages for a conversation as agent-chain BaseMessages.
    pub async fn get_base_messages(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<BaseMessage>, sqlx::Error> {
        let messages = self.get_messages(conversation_id).await?;
        messages
            .into_iter()
            .map(|m| {
                m.to_base_message()
                    .map_err(|e| sqlx::Error::Protocol(e.to_string()))
            })
            .collect()
    }

    /// Get a conversation with all its messages as BaseMessages.
    pub async fn get_conversation_with_messages(
        &self,
        conversation_id: &str,
    ) -> Result<(Conversation, Vec<BaseMessage>), sqlx::Error> {
        let conversation = self.get_conversation(conversation_id).await?;
        let messages = self.get_base_messages(conversation_id).await?;
        debug!(
            "Loaded conversation {} with {} messages",
            conversation_id,
            messages.len()
        );
        Ok((conversation, messages))
    }

    /// Get the next sequence number for a conversation.
    pub async fn get_next_sequence_num(&self, conversation_id: &str) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT COALESCE(MAX(sequence_num), -1) + 1 FROM message WHERE conversation_id = ?",
        )
        .bind(conversation_id)
        .fetch_one(&self.pool)
        .await
    }

    // ==================== Activity Operations ====================

    /// List activities with pagination.
    pub async fn list_activities(
        &self,
        limit: u16,
        offset: u16,
    ) -> Result<Vec<Activity>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, name, icon_path, process_name, started_at, ended_at
            FROM activity
            ORDER BY started_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Insert a new activity.
    pub async fn insert_activity(&self, activity: &Activity) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO activity (id, name, icon_path, process_name, started_at, ended_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&activity.id)
        .bind(&activity.name)
        .bind(&activity.icon_path)
        .bind(&activity.process_name)
        .bind(&activity.started_at)
        .bind(&activity.ended_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Update activity end time.
    pub async fn update_activity_end_time(
        &self,
        activity_id: &str,
        ended_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE activity
            SET ended_at = ?
            WHERE id = ?
            "#,
        )
        .bind(ended_at.to_rfc3339())
        .bind(activity_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get the last active (not ended) activity.
    pub async fn get_last_active_activity(&self) -> Result<Option<Activity>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, name, icon_path, process_name, started_at, ended_at
            FROM activity
            WHERE ended_at IS NULL
            ORDER BY started_at DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await
    }

    // ==================== Asset Operations ====================

    /// Insert a new asset.
    pub async fn insert_asset(&self, na: &NewAsset) -> Result<Asset, sqlx::Error> {
        let id = na.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
        let created_at = na.created_at.unwrap_or_else(Utc::now);
        let updated_at = na.updated_at.unwrap_or(created_at);

        sqlx::query(
            r#"
            INSERT INTO asset (id, activity_id, relative_path, absolute_path, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&na.activity_id)
        .bind(&na.relative_path)
        .bind(&na.absolute_path)
        .bind(created_at)
        .bind(updated_at)
        .execute(&self.pool)
        .await?;

        // Create message-asset link if message_id is provided
        if let Some(message_id) = &na.message_id {
            self.insert_message_asset(&NewMessageAsset {
                message_id: message_id.clone(),
                asset_id: id.clone(),
            })
            .await?;
        }

        Ok(Asset {
            id,
            activity_id: na.activity_id.clone(),
            relative_path: na.relative_path.clone(),
            absolute_path: na.absolute_path.clone(),
            created_at,
            updated_at,
        })
    }

    /// Insert a message-asset link.
    pub async fn insert_message_asset(
        &self,
        nma: &NewMessageAsset,
    ) -> Result<MessageAsset, sqlx::Error> {
        let created_at = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO message_asset (message_id, asset_id, created_at)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(&nma.message_id)
        .bind(&nma.asset_id)
        .bind(created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(MessageAsset {
            message_id: nma.message_id.clone(),
            asset_id: nma.asset_id.clone(),
            created_at: created_at.to_rfc3339(),
        })
    }

    /// Get assets linked to a message.
    pub async fn get_assets_by_message_id(
        &self,
        message_id: &str,
    ) -> Result<Vec<Asset>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT a.id, a.activity_id, a.relative_path, a.absolute_path, a.created_at, a.updated_at
            FROM asset a
            INNER JOIN message_asset ma ON a.id = ma.asset_id
            WHERE ma.message_id = ?
            "#,
        )
        .bind(message_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Insert an asset for an activity.
    ///
    /// Uses the provided `asset.id` to insert the asset record.
    /// The `asset.id` must be a valid non-empty string (typically a UUID).
    pub async fn insert_activity_asset(
        &self,
        activity_id: &str,
        asset: &Asset,
    ) -> Result<(), sqlx::Error> {
        if asset.id.is_empty() {
            return Err(sqlx::Error::Protocol(
                "asset.id must not be empty".to_string(),
            ));
        }

        sqlx::query(
            r#"
            INSERT INTO asset (id, activity_id, relative_path, absolute_path, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&asset.id)
        .bind(activity_id)
        .bind(&asset.relative_path)
        .bind(&asset.absolute_path)
        .bind(asset.created_at)
        .bind(asset.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

/// Initialize or retrieve the database encryption key.
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
            &euro_secret::Sensitive(b64_key.clone()),
            secret::Namespace::Global,
        )
        .map_err(|e| anyhow!("Failed to persist key: {}", e))?;
        Ok(Sensitive(b64_key))
    }
}
