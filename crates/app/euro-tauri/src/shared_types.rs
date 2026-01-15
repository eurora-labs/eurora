use anyhow::Result;
use euro_conversation::ConversationManager;
use euro_personal_db::{Conversation, PersonalDatabaseManager};
use euro_settings::AppSettings;
use prompt_kit::PromptKitService;
use tokio::sync::Mutex;
use tracing::error;

use crate::util::get_db_path;
pub type SharedPromptKitService = Mutex<Option<PromptKitService>>;
pub type SharedAppSettings = Mutex<AppSettings>;
pub type SharedCurrentConversation = Mutex<Option<Conversation>>;
pub type SharedConversationManager = Mutex<ConversationManager>;

pub async fn create_shared_database_manager(
    app_handle: &tauri::AppHandle,
) -> Result<PersonalDatabaseManager> {
    let db_path = get_db_path(app_handle);
    PersonalDatabaseManager::new(&db_path).await.map_err(|e| {
        error!("Failed to create database manager: {}", e);
        e.into()
    })
    // PersonalDatabaseManager::new(&db_path)
    //     .await
    //     .map_err(|e| {
    //         debug!("Failed to create database manager: {}", e);
    //         e
    //     })
    //     .unwrap()
}
