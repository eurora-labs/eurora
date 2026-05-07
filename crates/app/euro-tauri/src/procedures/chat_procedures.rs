use agent_chain_core::messages::ContentBlock;
use euro_activity::types::ContextChip;
use euro_timeline::TimelineManager;
use tauri::{Manager, Runtime, ipc::Channel};
use thread_core::{ChatSendRequest, ChatServerMessage};
use tokio::sync::{Mutex, mpsc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::error::AppError;
use crate::shared_types::{ActiveStreamTokens, SharedThreadManager};

/// Per-turn host context returned by `chat.collect_context`.
///
/// `content_blocks` are inlined directly — large payloads are rewritten into
/// asset references server-side at chat-turn time, so the wire format here
/// can carry raw bytes/text without the client having to round-trip them.
#[taurpc::ipc_type]
pub struct ChatContext {
    pub content_blocks: Vec<ContentBlock>,
    pub asset_chips: Vec<ContextChip>,
}

// Trait method docs are not supported by `#[taurpc::procedures]` — see the
// `ChatApiImpl` resolvers below for behavior notes.
#[taurpc::procedures(path = "chat")]
pub trait ChatApi {
    async fn collect_context<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
    ) -> Result<ChatContext, String>;

    async fn send_query<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        channel: Channel<ChatServerMessage>,
        request: ChatSendRequest,
    ) -> Result<(), String>;

    async fn regenerate<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        ai_message_id: String,
        channel: Channel<ChatServerMessage>,
    ) -> Result<(), String>;

    async fn cancel_query<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
    ) -> Result<(), String>;
}

#[derive(Clone)]
pub struct ChatApiImpl;

const STREAM_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

#[taurpc::resolvers]
impl ChatApi for ChatApiImpl {
    async fn collect_context<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        _thread_id: String,
    ) -> Result<ChatContext, String> {
        let timeline_state: tauri::State<Mutex<TimelineManager>> = app_handle
            .try_state()
            .ok_or(AppError::Unavailable("Timeline"))?;

        let timeline = timeline_state.lock().await;

        // Refreshing the activity is best-effort: a missing tab or stale
        // browser bridge shouldn't abort the chat turn — we just contribute
        // no fresh context for it.
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

    async fn send_query<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        channel: Channel<ChatServerMessage>,
        request: ChatSendRequest,
    ) -> Result<(), String> {
        let thread_state: tauri::State<SharedThreadManager> = app_handle
            .try_state()
            .ok_or(AppError::Unavailable("Thread manager"))?;
        let tokens_state: tauri::State<ActiveStreamTokens> = app_handle
            .try_state()
            .ok_or(AppError::Unavailable("Active stream tokens"))?;

        let thread_uuid =
            Uuid::parse_str(&thread_id).map_err(|e| format!("Invalid thread_id: {e}"))?;

        let cancel = CancellationToken::new();
        tokens_state
            .lock()
            .await
            .insert(thread_id.clone(), cancel.clone());

        let thread = thread_state.inner();
        let stream_result = forward_chat_stream(
            thread
                .chat_stream(thread_uuid, request, cancel.clone())
                .await,
            cancel.clone(),
            &channel,
        )
        .await;

        tokens_state.lock().await.remove(&thread_id);
        stream_result
    }

    async fn regenerate<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        ai_message_id: String,
        channel: Channel<ChatServerMessage>,
    ) -> Result<(), String> {
        let thread_state: tauri::State<SharedThreadManager> = app_handle
            .try_state()
            .ok_or(AppError::Unavailable("Thread manager"))?;
        let tokens_state: tauri::State<ActiveStreamTokens> = app_handle
            .try_state()
            .ok_or(AppError::Unavailable("Active stream tokens"))?;

        let thread_uuid =
            Uuid::parse_str(&thread_id).map_err(|e| format!("Invalid thread_id: {e}"))?;
        let ai_message_uuid =
            Uuid::parse_str(&ai_message_id).map_err(|e| format!("Invalid ai_message_id: {e}"))?;

        let cancel = CancellationToken::new();
        tokens_state
            .lock()
            .await
            .insert(thread_id.clone(), cancel.clone());

        let thread = thread_state.inner();
        let stream_result = forward_chat_stream(
            thread
                .chat_regenerate(thread_uuid, ai_message_uuid, cancel.clone())
                .await,
            cancel.clone(),
            &channel,
        )
        .await;

        tokens_state.lock().await.remove(&thread_id);
        stream_result
    }

    async fn cancel_query<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
    ) -> Result<(), String> {
        let tokens_state: tauri::State<ActiveStreamTokens> = app_handle
            .try_state()
            .ok_or(AppError::Unavailable("Active stream tokens"))?;

        if let Some(token) = tokens_state.lock().await.remove(&thread_id) {
            token.cancel();
            tracing::debug!("Cancelled stream for thread {thread_id}");
        }

        Ok(())
    }
}

async fn forward_chat_stream(
    open_result: euro_thread::Result<
        mpsc::UnboundedReceiver<euro_thread::Result<ChatServerMessage>>,
    >,
    cancel: CancellationToken,
    channel: &Channel<ChatServerMessage>,
) -> Result<(), String> {
    let mut rx = open_result.map_err(|e| format!("Failed to open chat stream: {e}"))?;

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
