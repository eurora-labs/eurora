use agent_chain_core::{AIMessage, AnyMessage};
use euro_timeline::TimelineManager;
use futures::StreamExt;
use tauri::{Manager, Runtime, ipc::Channel};
use tokio::sync::Mutex;

use crate::shared_types::SharedThreadManager;

#[taurpc::ipc_type]
pub struct ResponseChunk {
    chunk: String,
    reasoning: Option<String>,
}

#[taurpc::ipc_type]
pub struct Query {
    text: String,
    assets: Vec<String>,
    parent_message_id: Option<String>,
    image_asset_ids: Option<Vec<ImageAssetId>>,
}

#[taurpc::ipc_type]
pub struct ImageAssetId {
    asset_id: String,
    mime_type: String,
}

#[taurpc::procedures(path = "chat")]
pub trait ChatApi {
    async fn send_query<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        channel: Channel<ResponseChunk>,
        query: Query,
    ) -> Result<String, String>;
}

#[derive(Clone)]
pub struct ChatApiImpl;

#[taurpc::resolvers]
impl ChatApi for ChatApiImpl {
    async fn send_query<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        channel: Channel<ResponseChunk>,
        query: Query,
    ) -> Result<String, String> {
        let thread_state: tauri::State<SharedThreadManager> = app_handle
            .try_state()
            .ok_or_else(|| "Thread manager not available".to_string())?;
        let timeline_state: tauri::State<Mutex<TimelineManager>> = app_handle
            .try_state()
            .ok_or_else(|| "Timeline not available".to_string())?;

        let mut asset_chips_json: Option<String> = None;

        {
            let timeline = timeline_state.lock().await;
            let mut thread_manager = thread_state.lock().await;

            let _ = timeline.refresh_current_activity().await;

            if timeline.save_current_activity_to_service().await.is_ok() && !query.assets.is_empty()
            {
                let chips = timeline.get_context_chips().await;
                if !chips.is_empty() {
                    asset_chips_json = serde_json::to_string(&chips).ok();
                }

                let mut messages = Vec::new();
                let asset_messages = timeline.construct_messages_from_last_asset().await;
                if let Some(last_asset_message) = asset_messages.last() {
                    messages.push(last_asset_message);
                }

                let snapshot_messages = timeline.construct_messages_from_last_snapshot().await;
                if let Some(last_snapshot_message) = snapshot_messages.last() {
                    messages.push(last_snapshot_message);
                }

                for message in messages {
                    match &message {
                        AnyMessage::SystemMessage(m) => {
                            let _ = thread_manager
                                .add_system_message(thread_id.clone(), m)
                                .await;
                        }
                        AnyMessage::HumanMessage(m) => {
                            let _ = thread_manager
                                .add_hidden_human_message(thread_id.clone(), m)
                                .await;
                        }
                        _ => {
                            tracing::warn!("Unexpected message type in asset context");
                        }
                    }
                }
            }
        }

        let mut complete_response = String::new();

        tracing::debug!("Sending chat stream");
        let stream_result = {
            let mut thread_manager = thread_state.lock().await;
            thread_manager
                .chat_stream(
                    thread_id.clone(),
                    query.text.clone(),
                    query.parent_message_id.clone(),
                    asset_chips_json,
                )
                .await
        };

        match stream_result {
            Ok(mut stream) => {
                tracing::debug!("Starting to consume stream...");

                let timeout_duration = std::time::Duration::from_secs(300);
                let stream_future = async {
                    while let Some(result) = stream.next().await {
                        match result {
                            Ok(chunk) => {
                                let content = chunk.content.to_string();
                                let reasoning = chunk
                                    .additional_kwargs
                                    .get("reasoning_content")
                                    .and_then(|v| v.as_str())
                                    .map(String::from);

                                if content.is_empty() && reasoning.is_none() {
                                    continue;
                                }

                                complete_response.push_str(&content);

                                if let Err(e) = channel.send(ResponseChunk {
                                    chunk: content,
                                    reasoning,
                                }) {
                                    return Err(format!("Failed to send response chunk: {e}"));
                                }
                            }
                            Err(e) => {
                                return Err(format!("Stream error: {e}"));
                            }
                        }
                    }
                    Ok(())
                };

                match tokio::time::timeout(timeout_duration, stream_future).await {
                    Ok(Ok(())) => {
                        tracing::debug!("Stream completed successfully");
                    }
                    Ok(Err(e)) => {
                        return Err(e);
                    }
                    Err(_) => {
                        return Err("Stream processing timed out after 5 minutes".to_string());
                    }
                }
            }
            Err(e) => {
                return Err(format!("Failed to create chat stream: {e}"));
            }
        }

        let _ai_message: AnyMessage = AIMessage::builder()
            .content(complete_response.clone())
            .build()
            .into();
        Ok(complete_response)
    }
}
