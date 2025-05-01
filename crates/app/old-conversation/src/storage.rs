use crate::conversation::{Asset, ChatMessage, Conversation};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Pool error: {0}")]
    Pool(#[from] r2d2::Error),
    #[error("Conversation not found: {0}")]
    NotFound(String),
}

pub struct ConversationStorage {
    pool: Pool<SqliteConnectionManager>,
}

impl ConversationStorage {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let manager = SqliteConnectionManager::file(path);
        let pool = Pool::new(manager)?;

        // Initialize schema
        let conn = pool.get()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS conversation (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS chat_message (
                id TEXT PRIMARY KEY,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                visible INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                conversation_id TEXT NOT NULL,
                FOREIGN KEY (conversation_id) REFERENCES conversation (id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS asset (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                asset_type TEXT NOT NULL,
                content BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (conversation_id) REFERENCES conversation (id)
            )",
            [],
        )?;

        Ok(Self { pool })
    }

    pub fn save_conversation(&self, conversation: &Conversation) -> Result<(), StorageError> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;

        // Check if this is a new conversation or an update
        let is_new = tx.query_row(
            "SELECT COUNT(*) FROM conversation WHERE id = ?1",
            params![conversation.id],
            |row| row.get::<_, i64>(0),
        )? == 0;

        // If this is a new conversation, check if we need to delete the oldest one
        if is_new {
            // Get the current count of conversations
            let count: i64 =
                tx.query_row("SELECT COUNT(*) FROM conversation", [], |row| row.get(0))?;

            // If we already have 5 or more conversations, delete the oldest one
            if count >= 5 {
                // Find the oldest conversation by created_at
                let oldest_id: String = tx.query_row(
                    "SELECT id FROM conversation ORDER BY created_at ASC LIMIT 1",
                    [],
                    |row| row.get(0),
                )?;

                // Delete the oldest conversation and all its associated data
                self.delete_conversation_within_transaction(&tx, &oldest_id)?;
            }
        }

        tx.execute(
            "INSERT OR REPLACE INTO conversation (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                conversation.id,
                conversation.title,
                conversation.created_at,
                conversation.updated_at
            ],
        )?;

        // Delete existing messages for this conversation to handle updates
        tx.execute(
            "DELETE FROM chat_message WHERE conversation_id = ?1",
            params![conversation.id],
        )?;

        // Insert all messages
        for message in &conversation.messages {
            tx.execute(
                "INSERT INTO chat_message (id, role, content, visible, created_at, updated_at, conversation_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    message.id,
                    message.role,
                    message.content,
                    message.visible as i32,
                    message.created_at,
                    message.updated_at,
                    conversation.id,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    // Helper method to delete a conversation within an existing transaction
    fn delete_conversation_within_transaction(
        &self,
        tx: &rusqlite::Transaction,
        id: &str,
    ) -> Result<(), StorageError> {
        // Delete assets first due to foreign key constraint
        tx.execute("DELETE FROM asset WHERE conversation_id = ?1", params![id])?;

        // Delete messages
        tx.execute(
            "DELETE FROM chat_message WHERE conversation_id = ?1",
            params![id],
        )?;

        // Delete the conversation
        tx.execute("DELETE FROM conversation WHERE id = ?1", params![id])?;

        Ok(())
    }

    pub fn get_conversation(&self, id: &str) -> Result<Conversation, StorageError> {
        let conn = self.pool.get()?;
        let conversation = conn
            .query_row(
                "SELECT id, title,  created_at, updated_at FROM conversation WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Conversation {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        messages: Vec::new(), // We'll populate this next
                        created_at: row.get(2)?,
                        updated_at: row.get(3)?,
                    })
                },
            )
            .map_err(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => StorageError::NotFound(id.to_string()),
                err => StorageError::Database(err),
            })?;

        let mut stmt = conn.prepare(
            "SELECT id, role, content, visible, created_at, updated_at
             FROM chat_message 
             WHERE conversation_id = ?1
             ORDER BY created_at ASC",
        )?;

        let messages = stmt
            .query_map(params![id], |row| {
                Ok(ChatMessage {
                    id: row.get(0)?,
                    role: row.get(1)?,
                    content: row.get(2)?,
                    visible: row.get::<_, i32>(3)? != 0,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Conversation {
            messages,
            ..conversation
        })
    }

    pub fn list_conversations(&self) -> Result<Vec<Conversation>, StorageError> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT id FROM conversation ORDER BY created_at DESC")?;

        let conversation_ids = stmt.query_map([], |row| row.get::<_, String>(0))?;

        let mut conversations = Vec::new();
        for id_result in conversation_ids {
            let id = id_result?;
            let conversation = self.get_conversation(&id)?;
            conversations.push(conversation);
        }

        Ok(conversations)
    }

    pub fn save_asset(&self, asset: &Asset) -> Result<(), StorageError> {
        let conn = self.pool.get()?;
        let content_json = serde_json::to_vec(&asset.content).map_err(|e| {
            StorageError::Database(rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })?;

        conn.execute(
            "INSERT OR REPLACE INTO asset (id, conversation_id, asset_type, content, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                asset.id,
                asset.conversation_id,
                asset.asset_type,
                content_json,
                asset.created_at,
                asset.updated_at
            ],
        )?;

        Ok(())
    }

    pub fn get_conversation_assets(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<Asset>, StorageError> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, asset_type, content, created_at, updated_at
             FROM asset
             WHERE conversation_id = ?1
             ORDER BY created_at ASC",
        )?;

        let assets = stmt
            .query_map(params![conversation_id], |row| {
                let content_blob: Vec<u8> = row.get(3)?;
                let content = serde_json::from_slice(&content_blob)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

                Ok(Asset {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    asset_type: row.get(2)?,
                    content,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(assets)
    }

    pub fn delete_conversation(&self, id: &str) -> Result<(), StorageError> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;

        self.delete_conversation_within_transaction(&tx, id)?;

        tx.commit()?;
        Ok(())
    }

    pub fn delete_asset(&self, id: &str) -> Result<(), StorageError> {
        let conn = self.pool.get()?;
        conn.execute("DELETE FROM asset WHERE id = ?1", params![id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_conversation() -> Conversation {
        let mut conversation = Conversation::new(None, None);
        conversation
            .add_message(ChatMessage::new(
                None,
                "user".to_string(),
                "Hello".to_string(),
                true,
            ))
            .unwrap();
        conversation
    }

    #[test]
    fn test_conversation_storage() -> Result<(), StorageError> {
        let storage = ConversationStorage::new(":memory:")?;

        let conversation = create_test_conversation();
        let conversation_id = conversation.id.clone();

        // Test save
        storage.save_conversation(&conversation)?;

        // Test get
        let retrieved = storage.get_conversation(&conversation_id)?;
        assert_eq!(retrieved.id, conversation.id);
        assert_eq!(retrieved.messages.len(), 1);
        assert_eq!(retrieved.messages[0].content, "Hello");

        // Test list
        let conversations = storage.list_conversations()?;
        assert_eq!(conversations.len(), 1);

        // Test delete
        storage.delete_conversation(&conversation_id)?;
        assert!(storage.get_conversation(&conversation_id).is_err());

        Ok(())
    }

    #[test]
    fn test_conversation_limit() -> Result<(), StorageError> {
        let storage = ConversationStorage::new(":memory:")?;

        // Create 6 conversations with different creation times
        for i in 1..=6 {
            let mut conversation = Conversation::new(None, None);

            // Set created_at to different timestamps to ensure clear ordering
            // The first conversation (i=1) will be the oldest
            conversation.created_at = 1000 + (i as u64 * 100);
            conversation.updated_at = conversation.created_at;

            // Add a message to identify the conversation
            conversation
                .add_message(ChatMessage::new(
                    None,
                    "user".to_string(),
                    format!("Conversation {}", i),
                    true,
                ))
                .unwrap();

            storage.save_conversation(&conversation)?;
        }

        // Verify that only 5 conversations are stored
        let conversations = storage.list_conversations()?;
        assert_eq!(conversations.len(), 5, "Should only keep 5 conversations");

        // Verify that the oldest conversation (i=1) was deleted
        // The remaining conversations should be 2, 3, 4, 5, 6
        for conversation in &conversations {
            let message = conversation.messages.first().unwrap();
            assert!(
                !message.content.contains("Conversation 1"),
                "The oldest conversation should have been deleted"
            );
        }

        // Verify that conversations 2-6 exist
        let mut found_conversations = vec![false; 5]; // For conversations 2-6

        for conversation in conversations {
            let message = conversation.messages.first().unwrap();
            for i in 2..=6 {
                if message.content == format!("Conversation {}", i) {
                    found_conversations[i - 2] = true;
                }
            }
        }

        // All conversations 2-6 should be found
        assert!(
            found_conversations.iter().all(|&found| found),
            "All conversations 2-6 should exist"
        );

        Ok(())
    }

    #[test]
    fn test_asset_storage() -> Result<(), StorageError> {
        let storage = ConversationStorage::new(":memory:")?;
        let conversation = create_test_conversation();
        storage.save_conversation(&conversation)?;

        // Test image asset
        let image_asset = Asset::new(
            conversation.id.clone(),
            "image".to_string(),
            serde_json::json!({
                "url": "https://example.com/image.jpg",
                "alt_text": "Test image",
                "width": 800,
                "height": 600
            }),
        );
        storage.save_asset(&image_asset)?;

        // Test document asset
        let doc_asset = Asset::new(
            conversation.id.clone(),
            "document".to_string(),
            serde_json::json!({
                "url": "https://example.com/doc.pdf",
                "filename": "test.pdf",
                "mime_type": "application/pdf",
                "size_bytes": 1024
            }),
        );
        storage.save_asset(&doc_asset)?;

        // Test custom asset
        let custom_asset = Asset::new(
            conversation.id.clone(),
            "custom".to_string(),
            serde_json::json!({
                "custom_field": "value",
                "number": 42
            }),
        );
        storage.save_asset(&custom_asset)?;

        // Test retrieving assets
        let assets = storage.get_conversation_assets(&conversation.id)?;
        assert_eq!(assets.len(), 3);

        // Verify asset types and content
        let image = assets.iter().find(|a| a.asset_type == "image");
        assert!(image.is_some());
        assert_eq!(
            image.unwrap().content["url"],
            "https://example.com/image.jpg"
        );

        let doc = assets.iter().find(|a| a.asset_type == "document");
        assert!(doc.is_some());
        assert_eq!(doc.unwrap().content["filename"], "test.pdf");

        let custom = assets.iter().find(|a| a.asset_type == "custom");
        assert!(custom.is_some());
        assert_eq!(custom.unwrap().content["custom_field"], "value");

        // Test deleting individual asset
        storage.delete_asset(&image_asset.id)?;
        let assets_after_delete = storage.get_conversation_assets(&conversation.id)?;
        assert_eq!(assets_after_delete.len(), 2);

        // Test cascade delete with conversation
        storage.delete_conversation(&conversation.id)?;
        let assets_after_cascade = storage.get_conversation_assets(&conversation.id)?;
        assert_eq!(assets_after_cascade.len(), 0);

        Ok(())
    }
}
