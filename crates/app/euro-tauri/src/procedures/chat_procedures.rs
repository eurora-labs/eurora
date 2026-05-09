use agent_chain_core::messages::ContentBlock;
use euro_activity::types::ContextChip;
use euro_timeline::TimelineManager;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager, ipc::Channel};
use thread_core::{ChatSendRequest, ChatServerMessage};
use tokio::sync::{Mutex, mpsc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::error::ResultExt;
use crate::shared_types::{ActiveStreamTokens, SharedThreadManager};

const STREAM_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

/// Per-turn host context returned by `chat_collect_context`.
///
/// `content_blocks` are inlined directly — large payloads are rewritten into
/// asset references server-side at chat-turn time, so the wire format here
/// can carry raw bytes/text without the client having to round-trip them.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ChatContext {
    pub content_blocks: Vec<ContentBlock>,
    pub asset_chips: Vec<ContextChip>,
}

fn thread_manager(app_handle: &AppHandle) -> Result<tauri::State<'_, SharedThreadManager>, String> {
    app_handle
        .try_state::<SharedThreadManager>()
        .ok_or_else(|| "Thread manager not available".to_string())
}

fn active_stream_tokens(
    app_handle: &AppHandle,
) -> Result<tauri::State<'_, ActiveStreamTokens>, String> {
    app_handle
        .try_state::<ActiveStreamTokens>()
        .ok_or_else(|| "Active stream tokens not available".to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn chat_collect_context(
    app_handle: AppHandle,
    _thread_id: Uuid,
) -> Result<ChatContext, String> {
    let timeline_state = app_handle
        .try_state::<Mutex<TimelineManager>>()
        .ok_or_else(|| "Timeline not available".to_string())?;

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
) -> Result<(), String> {
    let thread_state = thread_manager(&app_handle)?;
    let tokens_state = active_stream_tokens(&app_handle)?;

    let cancel = CancellationToken::new();
    tokens_state.lock().await.insert(thread_id, cancel.clone());

    let thread = thread_state.inner();
    let stream_result = forward_chat_stream(
        thread.chat_stream(thread_id, request, cancel.clone()).await,
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
) -> Result<(), String> {
    let thread_state = thread_manager(&app_handle)?;
    let tokens_state = active_stream_tokens(&app_handle)?;

    let cancel = CancellationToken::new();
    tokens_state.lock().await.insert(thread_id, cancel.clone());

    let thread = thread_state.inner();
    let stream_result = forward_chat_stream(
        thread
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
pub async fn chat_cancel_query(app_handle: AppHandle, thread_id: Uuid) -> Result<(), String> {
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
) -> Result<(), String> {
    let mut rx = open_result.ctx("Failed to open chat stream")?;

    let stream_future = async {
        while let Some(item) = rx.recv().await {
            match item {
                Ok(event) => {
                    let is_terminal = matches!(
                        &event,
                        ChatServerMessage::Final { .. } | ChatServerMessage::Error { .. }
                    );
                    channel
                        .send(event)
                        .map_err(|e| format!("Failed to forward chat event: {e}"))?;
                    if is_terminal {
                        return Ok(());
                    }
                }
                Err(euro_thread::Error::Cancelled) => return Ok(()),
                Err(e) => return Err(format!("Stream error: {e}")),
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
            Err(format!(
                "Stream processing timed out after {} seconds",
                STREAM_TIMEOUT.as_secs()
            ))
        }
    }
}
