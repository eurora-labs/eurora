use agent_chain_core::messages::ContentBlock;
use euro_activity::types::ContextChip;
use euro_timeline::TimelineManager;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager, ipc::Channel};
use thiserror::Error;
use thread_core::{
    ChatSendRequest, ChatServerMessage, MessageNode, SearchMessageResult, SearchThreadResult,
    Thread,
};
use tokio::sync::{Mutex, mpsc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::shared_types::{ActiveStreamTokens, SharedThreadManager};

const STREAM_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

/// Typed error surface for the CRUD/search `thread_*` IPC commands.
/// Externally tagged so the JS side can branch on `error.type` without
/// parsing strings. `NotFound` lifts `euro_thread::Error::ThreadNotFound`
/// to a dedicated variant so the UI can render an empty state instead of
/// a generic toast.
#[derive(Debug, Error, Serialize, Type)]
#[serde(tag = "type", content = "data")]
pub enum ThreadError {
    #[error("thread not found")]
    NotFound,
    #[error("backend unreachable: {0}")]
    Backend(String),
    #[error("bad response: {0}")]
    BadResponse(String),
    #[error("state unavailable: {0}")]
    StateUnavailable(&'static str),
    #[error("internal: {0}")]
    Internal(String),
}

impl From<euro_thread::Error> for ThreadError {
    fn from(err: euro_thread::Error) -> Self {
        use euro_thread::Error as E;
        match err {
            E::ThreadNotFound => ThreadError::NotFound,
            E::Transport(ref e) => ThreadError::Backend(e.to_string()),
            E::WebSocket(ref e) => ThreadError::Backend(e.to_string()),
            E::Service { .. } => ThreadError::BadResponse(err.to_string()),
            E::Encode(e) | E::Decode(e) => ThreadError::BadResponse(e.to_string()),
            E::Auth(_) | E::InvalidUrl(_) | E::ChatProtocol(_) | E::Cancelled => {
                ThreadError::Internal(err.to_string())
            }
        }
    }
}

/// Typed error surface for the streaming `chat_*` IPC commands.
/// Externally tagged so the JS side can branch on `error.type`.
/// `Cancelled` is split out from the rest so the UI can suppress its
/// own cancel-induced errors instead of showing a toast for them; an
/// upstream `euro_thread::Error` (e.g. a deleted thread mid-stream)
/// is wrapped in `Thread` so the JS side can drill into the same
/// `ThreadError` variants it handles for CRUD calls.
#[derive(Debug, Error, Serialize, Type)]
#[serde(tag = "type", content = "data")]
pub enum StreamError {
    #[error("cancelled")]
    Cancelled,
    #[error("stream timed out after {0} seconds")]
    Timeout(u32),
    #[error("channel: {0}")]
    Channel(String),
    #[error("state unavailable: {0}")]
    StateUnavailable(&'static str),
    #[error(transparent)]
    Thread(ThreadError),
}

impl From<euro_thread::Error> for StreamError {
    fn from(err: euro_thread::Error) -> Self {
        match err {
            euro_thread::Error::Cancelled => StreamError::Cancelled,
            other => StreamError::Thread(ThreadError::from(other)),
        }
    }
}

/// Per-turn host context returned by `chat_collect_context`.
///
/// `content_blocks` are inlined directly — large payloads are rewritten
/// into asset references server-side at chat-turn time, so the wire
/// format here can carry raw bytes/text without the client having to
/// round-trip them.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChatContext {
    pub content_blocks: Vec<ContentBlock>,
    pub asset_chips: Vec<ContextChip>,
}

/// Look up the shared `ThreadManager`. Generic over the caller's error
/// type so both CRUD (`ThreadError`) and streaming (`StreamError`)
/// commands share the same lookup without duplicating the body.
fn thread_manager<E>(
    app_handle: &AppHandle,
    state_unavailable: fn(&'static str) -> E,
) -> Result<tauri::State<'_, SharedThreadManager>, E> {
    app_handle
        .try_state::<SharedThreadManager>()
        .ok_or_else(|| state_unavailable("thread manager"))
}

fn active_stream_tokens(
    app_handle: &AppHandle,
) -> Result<tauri::State<'_, ActiveStreamTokens>, StreamError> {
    app_handle
        .try_state::<ActiveStreamTokens>()
        .ok_or(StreamError::StateUnavailable("active stream tokens"))
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

#[tauri::command]
#[specta::specta]
pub async fn chat_collect_context(
    app_handle: AppHandle,
    _thread_id: Uuid,
) -> Result<ChatContext, StreamError> {
    let timeline_state = app_handle
        .try_state::<Mutex<TimelineManager>>()
        .ok_or(StreamError::StateUnavailable("timeline"))?;

    let timeline = timeline_state.lock().await;

    // Refreshing the activity is best-effort: a missing tab or stale browser
    // bridge shouldn't abort the chat turn — we just contribute no fresh
    // context for it.
    if let Err(e) = timeline.refresh_current_activity().await {
        tracing::debug!("collect_context: refresh failed: {e}");
    }
    if timeline.save_current_activity_to_service().await.is_err() {
        return Ok(ChatContext {
            content_blocks: Vec::new(),
            asset_chips: Vec::new(),
        });
    }

    let asset_blocks = timeline.construct_messages_from_last_asset().await;
    let snapshot_blocks = timeline.construct_messages_from_last_snapshot().await;

    let mut content_blocks: Vec<ContentBlock> =
        Vec::with_capacity(asset_blocks.len() + snapshot_blocks.len());
    content_blocks.extend(asset_blocks.into_inner());
    content_blocks.extend(snapshot_blocks.into_inner());

    let asset_chips = timeline
        .get_context_chip()
        .await
        .map(|chip| vec![chip])
        .unwrap_or_default();

    Ok(ChatContext {
        content_blocks,
        asset_chips,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn chat_send_query(
    app_handle: AppHandle,
    thread_id: Uuid,
    channel: Channel<ChatServerMessage>,
    request: ChatSendRequest,
) -> Result<(), StreamError> {
    let thread_state = thread_manager(&app_handle, StreamError::StateUnavailable)?;
    let tokens_state = active_stream_tokens(&app_handle)?;

    let cancel = CancellationToken::new();
    tokens_state.lock().await.insert(thread_id, cancel.clone());

    let stream_result = forward_chat_stream(
        thread_state
            .chat_stream(thread_id, request, cancel.clone())
            .await,
        cancel.clone(),
        &channel,
    )
    .await;

    tokens_state.lock().await.remove(&thread_id);
    stream_result
}

#[tauri::command]
#[specta::specta]
pub async fn chat_regenerate(
    app_handle: AppHandle,
    thread_id: Uuid,
    ai_message_id: Uuid,
    channel: Channel<ChatServerMessage>,
) -> Result<(), StreamError> {
    let thread_state = thread_manager(&app_handle, StreamError::StateUnavailable)?;
    let tokens_state = active_stream_tokens(&app_handle)?;

    let cancel = CancellationToken::new();
    tokens_state.lock().await.insert(thread_id, cancel.clone());

    let stream_result = forward_chat_stream(
        thread_state
            .chat_regenerate(thread_id, ai_message_id, cancel.clone())
            .await,
        cancel.clone(),
        &channel,
    )
    .await;

    tokens_state.lock().await.remove(&thread_id);
    stream_result
}

#[tauri::command]
#[specta::specta]
pub async fn chat_cancel_query(app_handle: AppHandle, thread_id: Uuid) -> Result<(), StreamError> {
    let tokens_state = active_stream_tokens(&app_handle)?;

    if let Some(token) = tokens_state.lock().await.remove(&thread_id) {
        token.cancel();
        tracing::debug!("Cancelled stream for thread {thread_id}");
    }

    Ok(())
}

async fn forward_chat_stream(
    open_result: euro_thread::Result<
        mpsc::UnboundedReceiver<euro_thread::Result<ChatServerMessage>>,
    >,
    cancel: CancellationToken,
    channel: &Channel<ChatServerMessage>,
) -> Result<(), StreamError> {
    let mut rx = open_result?;

    let stream_future = async {
        while let Some(item) = rx.recv().await {
            let event = item?;
            let is_terminal = matches!(
                &event,
                ChatServerMessage::Final { .. } | ChatServerMessage::Error { .. }
            );
            channel
                .send(event)
                .map_err(|e| StreamError::Channel(e.to_string()))?;
            if is_terminal {
                return Ok(());
            }
        }
        Ok(())
    };

    match tokio::time::timeout(STREAM_TIMEOUT, stream_future).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => {
            cancel.cancel();
            Err(e)
        }
        Err(_) => {
            cancel.cancel();
            Err(StreamError::Timeout(STREAM_TIMEOUT.as_secs() as u32))
        }
    }
}
