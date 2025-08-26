use crate::util::get_db_path;
use eur_settings::AppSettings;
use tracing::info;

use async_mutex::Mutex;
use eur_personal_db::{Conversation, PersonalDatabaseManager};
use eur_prompt_kit::PromptKitService;
pub type SharedPromptKitService = Mutex<Option<PromptKitService>>;
pub type SharedAppSettings = Mutex<AppSettings>;
pub type SharedCurrentConversation = Option<Conversation>;

pub async fn create_shared_database_manager(
    app_handle: &tauri::AppHandle,
) -> PersonalDatabaseManager {
    let db_path = get_db_path(app_handle);
    PersonalDatabaseManager::new(&db_path)
        .await
        .map_err(|e| {
            info!("Failed to create database manager: {}", e);
            e
        })
        .unwrap()
}
