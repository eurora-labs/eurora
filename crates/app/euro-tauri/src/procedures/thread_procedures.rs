use tauri::{AppHandle, Manager};
use thread_core::{MessageNode, SearchMessageResult, SearchThreadResult, Thread};
use uuid::Uuid;

use crate::error::ResultExt;
use crate::shared_types::SharedThreadManager;

fn thread_manager(app_handle: &AppHandle) -> Result<tauri::State<'_, SharedThreadManager>, String> {
    app_handle
        .try_state::<SharedThreadManager>()
        .ok_or_else(|| "Thread manager not available".to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn thread_list(
    app_handle: AppHandle,
    limit: u32,
    offset: u32,
) -> Result<Vec<Thread>, String> {
    let manager = thread_manager(&app_handle)?;
    manager
        .list_threads(limit, offset)
        .await
        .ctx("Failed to list threads")
}

#[tauri::command]
#[specta::specta]
pub async fn thread_create(app_handle: AppHandle) -> Result<Thread, String> {
    let manager = thread_manager(&app_handle)?;
    manager.create(None).await.ctx("Failed to create thread")
}

#[tauri::command]
#[specta::specta]
pub async fn thread_delete(app_handle: AppHandle, thread_id: Uuid) -> Result<(), String> {
    let manager = thread_manager(&app_handle)?;
    manager
        .delete_thread(thread_id)
        .await
        .ctx("Failed to delete thread")
}

#[tauri::command]
#[specta::specta]
pub async fn thread_get_messages(
    app_handle: AppHandle,
    thread_id: Uuid,
    limit: u32,
    offset: u32,
) -> Result<Vec<MessageNode>, String> {
    let manager = thread_manager(&app_handle)?;
    manager
        .get_messages(thread_id, limit, offset)
        .await
        .ctx("Failed to get messages")
}

#[tauri::command]
#[specta::specta]
pub async fn thread_switch_branch(
    app_handle: AppHandle,
    thread_id: Uuid,
    message_id: Uuid,
    direction: i32,
) -> Result<Vec<MessageNode>, String> {
    let manager = thread_manager(&app_handle)?;
    manager
        .switch_branch(thread_id, message_id, direction)
        .await
        .ctx("Failed to switch branch")
}

#[tauri::command]
#[specta::specta]
pub async fn thread_generate_title(
    app_handle: AppHandle,
    thread_id: Uuid,
) -> Result<Thread, String> {
    let manager = thread_manager(&app_handle)?;
    manager
        .generate_thread_title(thread_id)
        .await
        .ctx("Failed to generate thread title")
}

#[tauri::command]
#[specta::specta]
pub async fn thread_search_threads(
    app_handle: AppHandle,
    query: String,
    limit: u32,
    offset: u32,
) -> Result<Vec<SearchThreadResult>, String> {
    let manager = thread_manager(&app_handle)?;
    let response = manager
        .search_threads(query, limit, offset)
        .await
        .ctx("Failed to search threads")?;
    Ok(response.results)
}

#[tauri::command]
#[specta::specta]
pub async fn thread_search_messages(
    app_handle: AppHandle,
    query: String,
    limit: u32,
    offset: u32,
) -> Result<Vec<SearchMessageResult>, String> {
    let manager = thread_manager(&app_handle)?;
    let response = manager
        .search_messages(query, limit, offset)
        .await
        .ctx("Failed to search messages")?;
    Ok(response.results)
}
