use tauri::Runtime;

#[taurpc::procedures(path = "personal_db.message")]
pub trait MessageApi {
    async fn get<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        conversation_id: String,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<String>, String>;
}

#[derive(Clone)]
pub struct MessageApiImpl;

#[taurpc::resolvers]
impl MessageApi for MessageApiImpl {
    async fn get<R: Runtime>(
        self,
        _app_handle: tauri::AppHandle<R>,
        _conversation_id: String,
        _limit: Option<u32>,
        _offset: Option<u32>,
    ) -> Result<Vec<String>, String> {
        Ok(vec![])
        // let personal_db = app_handle.state::<PersonalDatabaseManager>().inner();

        // personal_db
        //     .get_base_messages(&conversation_id)
        //     .await
        //     .map_err(|e| format!("Failed to get chat messages: {e}"))
    }
}
