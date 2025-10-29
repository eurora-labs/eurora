use anyhow::Result;
use eur_personal_db::{Conversation, PersonalDatabaseManager};
use eur_prompt_kit::PromptKitService;
use eur_settings::AppSettings;
use tokio::sync::Mutex;
use tracing::error;

use crate::util::get_db_path;
pub type SharedPromptKitService = Mutex<Option<PromptKitService>>;
pub type SharedAppSettings = Mutex<AppSettings>;
pub type SharedCurrentConversation = Mutex<Option<Conversation>>;

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
