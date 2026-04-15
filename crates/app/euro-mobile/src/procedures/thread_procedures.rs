use crate::error::ResultExt;
use crate::shared_types::SharedThreadManager;
use chrono::{TimeZone, Utc};
use euro_thread::ListThreadsRequest;
use proto_gen::agent_chain::BaseMessageWithSibling;
use proto_gen::thread::CreateThreadRequest;
use tauri::{Manager, Runtime};

#[taurpc::ipc_type]
pub struct Thread {
    pub id: Option<String>,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[taurpc::procedures(
    path = "thread",
    export_to = "../../../apps/mobile/src/lib/bindings/bindings.ts"
)]
pub trait ThreadApi {
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
        all_variants: bool,
    ) -> Result<Vec<BaseMessageWithSibling>, String>;

    async fn switch_branch<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        message_id: String,
        direction: i32,
    ) -> Result<Vec<BaseMessageWithSibling>, String>;

    async fn generate_title<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        content: String,
    ) -> Result<Thread, String>;
}

fn thread_manager<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<tauri::State<'_, SharedThreadManager>, String> {
    app_handle
        .try_state::<SharedThreadManager>()
        .ok_or_else(|| "Thread manager not available".to_string())
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
        let thread_state = thread_manager(&app_handle)?;
        let thread_manager = thread_state.lock().await;

        let threads = thread_manager
            .list_threads(ListThreadsRequest { limit, offset })
            .await
            .map_err(|e| e.to_string())?;

        Ok(threads.into_iter().map(|thread| thread.into()).collect())
    }

    async fn create<R: Runtime>(self, app_handle: tauri::AppHandle<R>) -> Result<Thread, String> {
        let thread_state = thread_manager(&app_handle)?;
        let thread_manager = thread_state.lock().await;
        let thread = thread_manager
            .create(CreateThreadRequest {
                title: "New Chat".to_string(),
            })
            .await
            .map_err(|e| e.to_string())?;
        Ok(thread.into())
    }

    async fn delete<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
    ) -> Result<(), String> {
        let thread_state = thread_manager(&app_handle)?;
        let thread_manager = thread_state.lock().await;
        thread_manager
            .delete_thread(thread_id)
            .await
            .map_err(|e| e.to_string())
    }

    async fn get_messages<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        limit: u32,
        offset: u32,
        all_variants: bool,
    ) -> Result<Vec<BaseMessageWithSibling>, String> {
        let thread_state = thread_manager(&app_handle)?;
        let thread_manager = thread_state.lock().await;
        let response = thread_manager
            .get_messages(thread_id, limit, offset, all_variants)
            .await
            .ctx("Failed to get messages")?;
        Ok(response.messages)
    }

    async fn switch_branch<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        message_id: String,
        direction: i32,
    ) -> Result<Vec<BaseMessageWithSibling>, String> {
        let thread_state = thread_manager(&app_handle)?;
        let thread_manager = thread_state.lock().await;
        let response = thread_manager
            .switch_branch(thread_id, message_id, direction)
            .await
            .map_err(|e| e.to_string())?;

        Ok(response.messages)
    }

    async fn generate_title<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        content: String,
    ) -> Result<Thread, String> {
        let thread_state = thread_manager(&app_handle)?;
        let thread_manager = thread_state.lock().await;
        let thread = thread_manager
            .generate_thread_title(thread_id, content)
            .await
            .map_err(|e| e.to_string())?;
        Ok(thread.into())
    }
}

impl From<proto_gen::thread::ProtoThread> for Thread {
    fn from(thread: proto_gen::thread::ProtoThread) -> Self {
        let created_at = thread
            .created_at
            .map(|ts| {
                Utc.timestamp_opt(ts.seconds, ts.nanos as u32)
                    .single()
                    .unwrap_or_default()
                    .to_rfc3339()
            })
            .unwrap_or_default();
        let updated_at = thread
            .updated_at
            .map(|ts| {
                Utc.timestamp_opt(ts.seconds, ts.nanos as u32)
                    .single()
                    .unwrap_or_default()
                    .to_rfc3339()
            })
            .unwrap_or_default();
        Thread {
            id: thread.id.into(),
            title: thread.title,
            created_at,
            updated_at,
        }
    }
}
