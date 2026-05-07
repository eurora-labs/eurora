use crate::shared_types::SharedThreadManager;
use tauri::{Manager, Runtime};
use thread_core::MessageNode;
use uuid::Uuid;

#[taurpc::ipc_type]
pub struct Thread {
    pub id: Option<String>,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[taurpc::ipc_type]
pub struct SearchThreadResultView {
    pub id: String,
    pub title: String,
    pub rank: f32,
}

#[taurpc::ipc_type]
pub struct SearchMessageResultView {
    pub id: String,
    pub thread_id: String,
    pub message_type: String,
    pub snippet: String,
    pub rank: f32,
}

#[taurpc::procedures(path = "thread")]
pub trait ThreadApi {
    #[taurpc(event)]
    async fn new_thread_added(thread: Thread);

    #[taurpc(event)]
    async fn thread_title_changed(thread: Thread);

    #[taurpc(event)]
    async fn current_thread_changed(thread: Thread);

    async fn list<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Thread>, String>;

    async fn create<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<Thread, String>;

    async fn delete<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
    ) -> Result<(), String>;

    async fn get_messages<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<MessageNode>, String>;

    async fn switch_branch<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        message_id: String,
        direction: i32,
    ) -> Result<Vec<MessageNode>, String>;

    async fn generate_title<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
    ) -> Result<Thread, String>;

    async fn search_threads<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        query: String,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<SearchThreadResultView>, String>;

    async fn search_messages<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        query: String,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<SearchMessageResultView>, String>;
}

fn thread_manager<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<tauri::State<'_, SharedThreadManager>, String> {
    app_handle
        .try_state::<SharedThreadManager>()
        .ok_or_else(|| "Thread manager not available".to_string())
}

fn parse_uuid(field: &str, value: &str) -> Result<Uuid, String> {
    Uuid::parse_str(value).map_err(|e| format!("Invalid {field}: {e}"))
}

#[derive(Clone)]
pub struct ThreadApiImpl;

#[taurpc::resolvers]
impl ThreadApi for ThreadApiImpl {
    async fn list<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<Thread>, String> {
        let manager = thread_manager(&app_handle)?;
        let threads = manager
            .list_threads(limit, offset)
            .await
            .map_err(|e| e.to_string())?;
        Ok(threads.into_iter().map(Thread::from).collect())
    }

    async fn create<R: Runtime>(self, app_handle: tauri::AppHandle<R>) -> Result<Thread, String> {
        let manager = thread_manager(&app_handle)?;
        let thread = manager.create(None).await.map_err(|e| e.to_string())?;
        Ok(Thread::from(thread))
    }

    async fn delete<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
    ) -> Result<(), String> {
        let manager = thread_manager(&app_handle)?;
        let id = parse_uuid("thread_id", &thread_id)?;
        manager.delete_thread(id).await.map_err(|e| e.to_string())
    }

    async fn get_messages<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<MessageNode>, String> {
        let manager = thread_manager(&app_handle)?;
        let id = parse_uuid("thread_id", &thread_id)?;
        manager
            .get_messages(id, limit, offset)
            .await
            .map_err(|e| e.to_string())
    }

    async fn switch_branch<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        message_id: String,
        direction: i32,
    ) -> Result<Vec<MessageNode>, String> {
        let manager = thread_manager(&app_handle)?;
        let thread_id = parse_uuid("thread_id", &thread_id)?;
        let message_id = parse_uuid("message_id", &message_id)?;
        manager
            .switch_branch(thread_id, message_id, direction)
            .await
            .map_err(|e| e.to_string())
    }

    async fn generate_title<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
    ) -> Result<Thread, String> {
        let manager = thread_manager(&app_handle)?;
        let id = parse_uuid("thread_id", &thread_id)?;
        let thread = manager
            .generate_thread_title(id)
            .await
            .map_err(|e| e.to_string())?;
        Ok(Thread::from(thread))
    }

    async fn search_threads<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        query: String,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<SearchThreadResultView>, String> {
        let manager = thread_manager(&app_handle)?;
        let response = manager
            .search_threads(query, limit, offset)
            .await
            .map_err(|e| e.to_string())?;
        Ok(response
            .results
            .into_iter()
            .map(|r| SearchThreadResultView {
                id: r.id.to_string(),
                title: r.title,
                rank: r.rank,
            })
            .collect())
    }

    async fn search_messages<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        query: String,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<SearchMessageResultView>, String> {
        let manager = thread_manager(&app_handle)?;
        let response = manager
            .search_messages(query, limit, offset)
            .await
            .map_err(|e| e.to_string())?;
        Ok(response
            .results
            .into_iter()
            .map(|r| SearchMessageResultView {
                id: r.id.to_string(),
                thread_id: r.thread_id.to_string(),
                message_type: r.message_type,
                snippet: r.snippet,
                rank: r.rank,
            })
            .collect())
    }
}

impl From<thread_core::Thread> for Thread {
    fn from(thread: thread_core::Thread) -> Self {
        Self {
            id: Some(thread.id.to_string()),
            title: thread.title,
            created_at: thread.created_at.to_rfc3339(),
            updated_at: thread.updated_at.to_rfc3339(),
        }
    }
}
