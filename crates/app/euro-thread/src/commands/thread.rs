use tauri::{AppHandle, Manager};
use thread_core::{MessageNode, SearchMessageResult, SearchThreadResult, Thread};
use uuid::Uuid;

use super::error::ThreadError;
use super::state::SharedThreadManager;

pub(super) fn thread_manager<E>(
    app_handle: &AppHandle,
    state_unavailable: fn(&'static str) -> E,
) -> Result<tauri::State<'_, SharedThreadManager>, E> {
    app_handle
        .try_state::<SharedThreadManager>()
        .ok_or_else(|| state_unavailable("thread manager"))
}

#[tauri::command]
#[specta::specta]
pub async fn thread_list(
    app_handle: AppHandle,
    limit: u32,
    offset: u32,
) -> Result<Vec<Thread>, ThreadError> {
    let manager = thread_manager(&app_handle, ThreadError::StateUnavailable)?;
    Ok(manager.list_threads(limit, offset).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn thread_create(app_handle: AppHandle) -> Result<Thread, ThreadError> {
    let manager = thread_manager(&app_handle, ThreadError::StateUnavailable)?;
    Ok(manager.create(None).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn thread_delete(app_handle: AppHandle, thread_id: Uuid) -> Result<(), ThreadError> {
    let manager = thread_manager(&app_handle, ThreadError::StateUnavailable)?;
    Ok(manager.delete_thread(thread_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn thread_get_messages(
    app_handle: AppHandle,
    thread_id: Uuid,
    limit: u32,
    offset: u32,
) -> Result<Vec<MessageNode>, ThreadError> {
    let manager = thread_manager(&app_handle, ThreadError::StateUnavailable)?;
    Ok(manager.get_messages(thread_id, limit, offset).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn thread_switch_branch(
    app_handle: AppHandle,
    thread_id: Uuid,
    message_id: Uuid,
    direction: i32,
) -> Result<Vec<MessageNode>, ThreadError> {
    let manager = thread_manager(&app_handle, ThreadError::StateUnavailable)?;
    Ok(manager
        .switch_branch(thread_id, message_id, direction)
        .await?)
}

#[tauri::command]
#[specta::specta]
pub async fn thread_generate_title(
    app_handle: AppHandle,
    thread_id: Uuid,
) -> Result<Thread, ThreadError> {
    let manager = thread_manager(&app_handle, ThreadError::StateUnavailable)?;
    Ok(manager.generate_thread_title(thread_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn thread_search_threads(
    app_handle: AppHandle,
    query: String,
    limit: u32,
    offset: u32,
) -> Result<Vec<SearchThreadResult>, ThreadError> {
    let manager = thread_manager(&app_handle, ThreadError::StateUnavailable)?;
    let response = manager.search_threads(query, limit, offset).await?;
    Ok(response.results)
}

#[tauri::command]
#[specta::specta]
pub async fn thread_search_messages(
    app_handle: AppHandle,
    query: String,
    limit: u32,
    offset: u32,
) -> Result<Vec<SearchMessageResult>, ThreadError> {
    let manager = thread_manager(&app_handle, ThreadError::StateUnavailable)?;
    let response = manager.search_messages(query, limit, offset).await?;
    Ok(response.results)
}
