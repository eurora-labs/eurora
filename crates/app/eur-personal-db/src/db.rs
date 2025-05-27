use chrono::{DateTime, Utc};
use libsqlite3_sys::sqlite3_auto_extension;
use sqlite_vec::sqlite3_vec_init;
use sqlx::Column;
use sqlx::TypeInfo;
use sqlx::migrate::MigrateDatabase;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::types::Uuid;
use std::time::Duration;
use tracing::debug;

use crate::types::{Activity, ActivityAsset, ChatMessage, Conversation};

pub struct DatabaseManager {
    pub pool: SqlitePool,
}

impl DatabaseManager {
    pub async fn new(database_path: &str) -> Result<Self, sqlx::Error> {
        debug!(
            "Initializing DatabaseManager with database path: {}",
            database_path
        );
        let connection_string = format!("sqlite:{}", database_path);

        unsafe {
            sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
        }

        // Create the database if it doesn't exist
        if !sqlx::Sqlite::database_exists(&connection_string).await? {
            sqlx::Sqlite::create_database(&connection_string).await?;
        }

        let pool = SqlitePoolOptions::new()
            .max_connections(50)
            .min_connections(3) // Minimum number of idle connections
            .acquire_timeout(Duration::from_secs(10))
            .connect(&connection_string)
            .await?;

        // Enable WAL mode
        sqlx::query("PRAGMA journal_mode = WAL;")
            .execute(&pool)
            .await?;

        // Enable SQLite's query result caching
        // PRAGMA cache_size = -2000; -- Set cache size to 2MB
        // PRAGMA temp_store = MEMORY; -- Store temporary tables and indices in memory
        sqlx::query("PRAGMA cache_size = -2000;")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA temp_store = MEMORY;")
            .execute(&pool)
            .await?;

        let db_manager = DatabaseManager { pool };

        // Run migrations after establishing the connection
        Self::run_migrations(&db_manager.pool).await?;

        Ok(db_manager)
    }

    async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        let mut migrator = sqlx::migrate!("./src/migrations");
        migrator.set_ignore_missing(true);
        match migrator.run(pool).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
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

    pub async fn list_conversations(&self) -> Result<Vec<Conversation>, sqlx::Error> {
        let conversations = sqlx::query_as(
            r#"
            SELECT id, title, created_at, updated_at
            FROM conversation
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(conversations)
    }

    pub async fn get_conversation_with_messages(
        &self,
        conversation_id: &str,
    ) -> Result<(Conversation, Vec<ChatMessage>), sqlx::Error> {
        let conversation = self.get_conversation(conversation_id).await?;
        // let messages = self.get_chat_messages(conversation_id).await?;

        // let conversation = self.get_conversation(conversation_id).await?;
        // let messages = self.get_chat_messages(conversation_id);

        // Use future to wait in parallel
        // let (conversation, messages) = try_join!(conversation, messages)?;

        eprintln!("Conversation: {:?}", conversation);
        // eprintln!("Messages: {:?}", messages);

        // Ok((conversation, messages))
        Ok((conversation, vec![]))
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
        let id = sqlx::query(
            r#"
            INSERT INTO chat_message (id, conversation_id, role, content, visible, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(conversation_id)
        .bind(role)
        .bind(content)
        .bind(visible)
        .bind(created_at)
        .bind(updated_at)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();

        Ok(ChatMessage {
            id: id.to_string(),
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
            SELECT id, role, content, visible, created_at, updated_at
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
            INSERT INTO activity (id, name, app_name, window_name, started_at, ended_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(activity.id.clone())
        .bind(activity.name.clone())
        .bind(activity.app_name.clone())
        .bind(activity.window_name.clone())
        .bind(activity.started_at.clone())
        .bind(activity.ended_at.clone())
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
            INSERT INTO activity_asset (activity_id, data, created_at, updated_At)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(activity_id)
        .bind(asset.data.clone())
        .bind(asset.created_at.clone())
        .bind(asset.updated_at.clone())
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
