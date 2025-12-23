use agent_chain_core::BaseMessage;
use euro_personal_db::PersonalDatabaseManager;
use tauri::{Manager, Runtime};

#[taurpc::procedures(path = "personal_db.message")]
pub trait MessageApi {
    async fn get<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        conversation_id: String,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<BaseMessage>, String>;
}

#[derive(Clone)]
pub struct MessageApiImpl;

#[taurpc::resolvers]
impl MessageApi for MessageApiImpl {
    async fn get<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        conversation_id: String,
        _limit: Option<u32>,
        _offset: Option<u32>,
    ) -> Result<Vec<BaseMessage>, String> {
        let personal_db = app_handle.state::<PersonalDatabaseManager>().inner();

        let chat_messages = personal_db
            .get_chat_messages(&conversation_id)
            .await
            .map_err(|e| format!("Failed to get chat messages: {e}"))?;

        Ok(chat_messages
            .into_iter()
            .map(|message| message.into())
            .collect::<Vec<BaseMessage>>())
    }
}
