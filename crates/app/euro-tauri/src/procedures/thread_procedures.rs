use crate::error::ResultExt;
use crate::shared_types::SharedThreadManager;
use agent_chain_core::messages::prelude::*;
use euro_thread::{ListThreadsRequest, Thread};
use proto_gen::thread::{CreateThreadRequest, GetMessagesResponse};
use std::collections::HashMap;
use tauri::{Manager, Runtime};

#[taurpc::ipc_type]
pub struct ThreadView {
    pub id: Option<String>,
    pub title: String,
}

#[taurpc::ipc_type]
pub struct ReasoningBlock {
    pub r#type: String,
    pub content: Option<String>,
    pub signature: Option<String>,
}

#[taurpc::ipc_type]
pub struct MessageView {
    pub id: Option<String>,
    pub role: String,
    pub content: String,
    pub reasoning_blocks: Option<Vec<ReasoningBlock>>,
    pub sibling_count: u32,
    pub sibling_index: u32,
}

#[taurpc::procedures(path = "thread")]
pub trait ThreadApi {
    #[taurpc(event)]
    async fn new_thread_added(thread: ThreadView);

    #[taurpc(event)]
    async fn thread_title_changed(thread: ThreadView);

    #[taurpc(event)]
    async fn current_thread_changed(thread: ThreadView);

    async fn list<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ThreadView>, String>;

    async fn create<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<ThreadView, String>;

    async fn get_messages<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<MessageView>, String>;

    async fn switch_branch<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        message_id: String,
        direction: i32,
    ) -> Result<Vec<MessageView>, String>;

    async fn generate_title<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        content: String,
    ) -> Result<ThreadView, String>;
}

fn thread_manager<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<tauri::State<'_, SharedThreadManager>, String> {
    app_handle
        .try_state::<SharedThreadManager>()
        .ok_or_else(|| "Thread manager not available".to_string())
}

fn convert_response(response: GetMessagesResponse) -> Vec<MessageView> {
    let sibling_map: HashMap<String, (u32, u32)> = response
        .sibling_info
        .into_iter()
        .map(|s| (s.message_id, (s.sibling_count, s.sibling_index)))
        .collect();

    response
        .messages
        .into_iter()
        .map(AnyMessage::from)
        .filter_map(|message| match message {
            AnyMessage::SystemMessage(_) => None,
            _ => {
                let id = message.id();
                let (sibling_count, sibling_index) = id
                    .as_ref()
                    .and_then(|id| sibling_map.get(id))
                    .copied()
                    .unwrap_or((1, 0));
                let reasoning_blocks = extract_reasoning_blocks(&message);
                Some(MessageView {
                    id,
                    role: message.message_type().to_string(),
                    content: message.content().to_string(),
                    reasoning_blocks,
                    sibling_count,
                    sibling_index,
                })
            }
        })
        .collect()
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

    async fn create<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<ThreadView, String> {
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

    async fn get_messages<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<MessageView>, String> {
        let thread_state = thread_manager(&app_handle)?;
        let thread_manager = thread_state.lock().await;
        let response = thread_manager
            .get_messages(thread_id, limit, offset)
            .await
            .ctx("Failed to get messages")?;

        Ok(convert_response(response))
    }

    async fn switch_branch<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        message_id: String,
        direction: i32,
    ) -> Result<Vec<MessageView>, String> {
        let thread_state = thread_manager(&app_handle)?;
        let thread_manager = thread_state.lock().await;
        let response = thread_manager
            .switch_branch(thread_id, message_id, direction)
            .await
            .map_err(|e| e.to_string())?;

        Ok(convert_response(response))
    }

    async fn generate_title<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        content: String,
    ) -> Result<ThreadView, String> {
        let thread_state = thread_manager(&app_handle)?;
        let thread_manager = thread_state.lock().await;
        let thread = thread_manager
            .generate_thread_title(thread_id, content)
            .await
            .map_err(|e| e.to_string())?;
        Ok(thread.into())
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

fn extract_reasoning_blocks(message: &AnyMessage) -> Option<Vec<ReasoningBlock>> {
    let kwargs = message.additional_kwargs();
    let blocks = kwargs.get("reasoning_blocks")?.as_array()?;
    let result: Vec<ReasoningBlock> = blocks
        .iter()
        .filter_map(|block| {
            let block_type = block.get("type")?.as_str()?.to_string();
            let content = block
                .get("content")
                .and_then(|v| v.as_str())
                .map(String::from);
            let signature = block
                .get("signature")
                .and_then(|v| v.as_str())
                .map(String::from);
            Some(ReasoningBlock {
                r#type: block_type,
                content,
                signature,
            })
        })
        .collect();
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}
