use crate::shared_types::SharedCurrentConversation;
use chrono::Utc;
use eur_personal_db::{Conversation, NewConversation, PersonalDatabaseManager};
use ferrous_llm_core::Message;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{Manager, Runtime};

#[taurpc::procedures(path = "personal_db.conversation")]
pub trait ConversationApi {
    #[taurpc(event)]
    async fn new_conversation_added(conversation: Conversation);

    async fn list<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        limit: u16,
        offset: u16,
    ) -> Result<Vec<Conversation>, String>;

    async fn create<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<Conversation, String>;

    async fn get_messages<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        conversation_id: String,
    ) -> Result<Vec<Message>, String>;
}

#[derive(Clone)]
pub struct ConversationApiImpl;

#[taurpc::resolvers]
impl ConversationApi for ConversationApiImpl {
    async fn list<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        limit: u16,
        offset: u16,
    ) -> Result<Vec<Conversation>, String> {
        let personal_db = app_handle.state::<PersonalDatabaseManager>().inner();
        personal_db
            .list_conversations(limit, offset)
            .await
            .map_err(|e| e.to_string())
    }

    async fn create<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<Conversation, String> {
        let personal_db = app_handle.state::<PersonalDatabaseManager>().inner();

        let conversation = personal_db
            .insert_empty_conversation()
            .await
            .map_err(|e| e.to_string())?;

        let current = app_handle.state::<SharedCurrentConversation>();
        let mut guard = current.lock().await;
        *guard = Some(conversation.clone());

        TauRpcConversationApiEventTrigger::new(app_handle.clone())
            .new_conversation_added(conversation.clone())
            .map_err(|e| e.to_string())?;

        Ok(conversation)
    }

    async fn get_messages<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        conversation_id: String,
    ) -> Result<Vec<Message>, String> {
        let personal_db = app_handle.state::<PersonalDatabaseManager>().inner();

        let chat_messages = personal_db
            .get_chat_messages(&conversation_id)
            .await
            .map_err(|e| format!("Failed to get chat messages: {e}"))?;

        Ok(chat_messages
            .into_iter()
            .map(|message| message.into())
            .collect::<Vec<Message>>())
    }
}
