use crate::error::ResultExt;
use crate::shared_types::SharedThreadManager;
use agent_chain_core::messages::prelude::*;
use euro_thread::{ListThreadsRequest, Thread};
use tauri::{Manager, Runtime};

#[taurpc::ipc_type]
pub struct ThreadView {
    pub id: Option<String>,
    pub title: String,
}

#[taurpc::ipc_type]
pub struct MessageView {
    pub id: Option<String>,
    pub role: String,
    pub content: String,
}

#[taurpc::procedures(path = "thread")]
pub trait ThreadApi {
    #[taurpc(event)]
    async fn new_thread_added(thread: ThreadView);

    #[taurpc(event)]
    async fn thread_title_changed(thread: ThreadView);

    #[taurpc(event)]
    async fn current_thread_changed(thread: ThreadView);

    async fn switch_thread<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
    ) -> Result<ThreadView, String>;

    async fn list<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ThreadView>, String>;

    async fn create_empty_thread<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
    ) -> Result<ThreadView, String>;

    async fn create<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<ThreadView, String>;

    async fn get_messages<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<MessageView>, String>;
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
    ) -> Result<Vec<ThreadView>, String> {
        let thread_state = thread_manager(&app_handle)?;
        let thread_manager = thread_state.lock().await;

        let threads = thread_manager
            .list_threads(ListThreadsRequest { limit, offset })
            .await
            .map_err(|e| e.to_string())?;

        Ok(threads.into_iter().map(|thread| thread.into()).collect())
    }

    async fn create_empty_thread<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<ThreadView, String> {
        let view: ThreadView = {
            let thread_state = thread_manager(&app_handle)?;
            let mut thread_manager = thread_state.lock().await;
            let thread = thread_manager
                .create_empty_thread()
                .await
                .map_err(|e| e.to_string())?;
            thread.into()
        };

        TauRpcThreadApiEventTrigger::new(app_handle)
            .current_thread_changed(view.clone())
            .ctx("Failed to trigger current thread changed event")?;

        Ok(view)
    }

    async fn create<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<ThreadView, String> {
        let thread_state = thread_manager(&app_handle)?;
        let _thread_manager = thread_state.lock().await;

        todo!()
    }

    async fn get_messages<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<MessageView>, String> {
        let thread_state = thread_manager(&app_handle)?;
        let thread_manager = thread_state.lock().await;
        let messages = thread_manager
            .get_messages(thread_id, limit, offset)
            .await
            .ctx("Failed to get messages")?;

        Ok(messages
            .into_iter()
            .filter_map(|message| match message {
                AnyMessage::SystemMessage(_) => None,
                _ => Some(MessageView::from(message)),
            })
            .collect())
    }

    async fn switch_thread<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
    ) -> Result<ThreadView, String> {
        let view: ThreadView = {
            let thread_state = thread_manager(&app_handle)?;
            let mut thread_manager = thread_state.lock().await;
            let thread = thread_manager
                .switch_thread(thread_id)
                .await
                .ctx("Failed to switch thread")?;
            thread.into()
        };

        TauRpcThreadApiEventTrigger::new(app_handle)
            .current_thread_changed(view.clone())
            .ctx("Failed to trigger current thread changed event")?;

        Ok(view)
    }
}

impl From<Thread> for ThreadView {
    fn from(thread: Thread) -> Self {
        ThreadView {
            id: thread.id().map(|id| id.to_string()),
            title: thread.title().to_string(),
        }
    }
}

impl From<&Thread> for ThreadView {
    fn from(thread: &Thread) -> Self {
        ThreadView {
            id: thread.id().map(|id| id.to_string()),
            title: thread.title().to_string(),
        }
    }
}

impl From<&AnyMessage> for MessageView {
    fn from(message: &AnyMessage) -> Self {
        MessageView {
            id: message.id(),
            role: message.message_type().to_string(),
            content: message.content().to_string(),
        }
    }
}

impl From<AnyMessage> for MessageView {
    fn from(message: AnyMessage) -> Self {
        MessageView {
            id: message.id(),
            role: message.message_type().to_string(),
            content: message.content().to_string(),
        }
    }
}
