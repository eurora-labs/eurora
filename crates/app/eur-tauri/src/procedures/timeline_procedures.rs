use eur_activity::AssetFunctionality;
use eur_personal_db::{Conversation, NewAsset, PersonalDatabaseManager, UpdateConversation};
use eur_timeline::TimelineManager;
use ferrous_llm_core::{Message, MessageContent, Role};
use futures::StreamExt;
use tauri::{Manager, Runtime, ipc::Channel};
use tracing::{debug, error};

use crate::shared_types::{SharedCurrentConversation, SharedPromptKitService};

#[taurpc::procedures(path = "timeline")]
pub trait TimelineApi {
    #[taurpc(event)]
    async fn new_app_event(name: String);

    async fn list<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<Vec<String>, String>;
}

#[derive(Clone)]
pub struct TimelineApiImpl;

#[taurpc::resolvers]
impl TimelineApi for TimelineApiImpl {
    async fn list<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<Vec<String>, String> {
        Ok(vec!["Test 1".to_string(), "Test 2".to_string()])
    }
}
