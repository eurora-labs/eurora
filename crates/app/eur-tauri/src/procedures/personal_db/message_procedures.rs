use crate::shared_types::SharedCurrentConversation;
use chrono::Utc;
use eur_personal_db::{Conversation, PersonalDatabaseManager};
use ferrous_llm_core::Message;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{Manager, Runtime};
use tracing::info;

#[taurpc::procedures(path = "personal_db.message")]
pub trait MessageApi {
    async fn get<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        conversation_id: String,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<Message>, String>;
}

#[derive(Clone)]
pub struct MessageApiImpl;

#[taurpc::resolvers]
impl MessageApi for MessageApiImpl {
    async fn get<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        conversation_id: String,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<Message>, String> {
        let personal_db = app_handle.state::<PersonalDatabaseManager>().inner();

        let chat_messages = personal_db
            .get_chat_messages(&conversation_id)
            .await
            .expect("Failed to get chat messages");

        Ok(chat_messages
            .into_iter()
            .map(|message| message.into())
            .collect::<Vec<Message>>())
    }
}
