pub mod conversation;
pub mod storage;

use anyhow::Result;
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use serde_json::Value;
use std::path::PathBuf;

pub use conversation::{Asset, ChatMessage, Conversation};
pub use storage::{ConversationStorage, StorageError};

// Re-export types expected by the Tauri app
pub type ConversationId = String;
pub type Controller = ConversationStorage;

static STORAGE: OnceCell<RwLock<Option<ConversationStorage>>> = OnceCell::new();

/// Initialize the conversation storage with the given database path
pub fn init_storage(db_path: PathBuf) -> Result<(), StorageError> {
    let storage = ConversationStorage::new(db_path)?;
    STORAGE.get_or_init(|| RwLock::new(Some(storage)));
    Ok(())
}

/// Get a reference to the conversation storage
pub fn get_storage() -> Result<&'static RwLock<Option<ConversationStorage>>, StorageError> {
    STORAGE.get().ok_or_else(|| {
        StorageError::Database(rusqlite::Error::InvalidParameterName(
            "Storage not initialized".to_string(),
        ))
    })
}

/// Creates a new conversation for a video question and stores the current browser state
pub async fn create_video_question_conversation() -> Result<ConversationId> {
    // Create a new conversation
    let conversation = Conversation::new(None, None);
    let conversation_id = conversation.id.clone();

    // Get storage
    let storage_lock = get_storage()?;
    let storage = storage_lock.read();
    let storage = storage
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Storage not initialized"))?;

    // Save the conversation
    storage.save_conversation(&conversation)?;

    Ok(conversation_id)
}

/// Add an asset to a conversation
pub fn add_asset(
    conversation_id: &str,
    asset_type: &str,
    content: Value,
) -> Result<Asset, StorageError> {
    // Create the asset
    let asset = Asset::new(conversation_id.to_string(), asset_type.to_string(), content);

    // Get storage
    let storage_lock = get_storage()?;
    let storage = storage_lock.read();
    let storage = storage.as_ref().ok_or_else(|| {
        StorageError::Database(rusqlite::Error::InvalidParameterName(
            "Storage not initialized".to_string(),
        ))
    })?;

    // Save the asset
    storage.save_asset(&asset)?;

    Ok(asset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_storage_initialization() {
        let temp_path = std::env::temp_dir().join(format!("test_{}.db", Uuid::new_v4()));

        // Initialize storage
        init_storage(temp_path.clone()).unwrap();

        // Test storage is accessible
        let storage_lock = get_storage().unwrap();
        let storage = storage_lock.read();
        assert!(storage.is_some());

        // Cleanup
        std::fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_conversation_workflow() {
        let temp_path = std::env::temp_dir().join(format!("test_{}.db", Uuid::new_v4()));
        init_storage(temp_path.clone()).unwrap();

        let storage_lock = get_storage().unwrap();
        let storage = storage_lock.read();
        let storage = storage.as_ref().unwrap();

        // Create and save a conversation
        let mut conversation = Conversation::new(None, None);
        conversation
            .add_message(ChatMessage::new(
                None,
                "user".to_string(),
                "Test message".to_string(),
                true,
            ))
            .unwrap();

        storage.save_conversation(&conversation).unwrap();

        // Retrieve and verify
        let retrieved = storage.get_conversation(&conversation.id).unwrap();
        assert_eq!(retrieved.messages.len(), 1);
        assert_eq!(retrieved.messages[0].content, "Test message");

        // Cleanup
        std::fs::remove_file(temp_path).ok();
    }
}
