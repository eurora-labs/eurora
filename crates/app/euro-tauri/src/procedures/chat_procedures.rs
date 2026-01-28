use agent_chain_core::{AIMessage, BaseMessage};
use euro_timeline::TimelineManager;
use futures::StreamExt;
use tauri::{Manager, Runtime, ipc::Channel};
use tokio::sync::Mutex;
use tracing::{debug, error, info};

use crate::shared_types::SharedConversationManager;

#[taurpc::ipc_type]
pub struct ResponseChunk {
    chunk: String,
}

#[taurpc::ipc_type]
pub struct Query {
    text: String,
    assets: Vec<String>,
}

#[taurpc::procedures(path = "chat")]
pub trait ChatApi {
    async fn send_query<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        _conversation_id: Option<String>,
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
        _conversation_id: Option<String>,
        channel: Channel<ResponseChunk>,
        query: Query,
    ) -> Result<String, String> {
        let timeline_state: tauri::State<Mutex<TimelineManager>> = app_handle.state();
        let timeline = timeline_state.lock().await;
        let conversation_state: tauri::State<SharedConversationManager> = app_handle.state();
        let mut conversation_manager = conversation_state.lock().await;

        let event = posthog_rs::Event::new_anon("send_query");
        tauri::async_runtime::spawn(async move {
            let _ = posthog_rs::capture(event).await.map_err(|e| {
                error!("Failed to capture posthog event: {}", e);
            });
        });

        conversation_manager
            .ensure_remote_conversation()
            .await
            .expect("Failed to ensure remote conversation");

        if let Ok(_) = timeline.save_current_activity_to_service().await {
            if let Ok(infos) = timeline.save_assets_to_service_by_ids(&query.assets).await {
                info!("Infos: {:?}", infos);
            }

            let has_assets = !query.assets.is_empty();

            if has_assets {
                let mut messages = Vec::new();
                let asset_messages = timeline
                    .construct_asset_messages_by_ids(&query.assets)
                    .await;
                if let Some(last_asset_message) = asset_messages.last() {
                    messages.push(last_asset_message);
                }

                let snapshot_messages = timeline
                    .construct_snapshot_messages_by_ids(&query.assets)
                    .await;

                if let Some(last_snapshot_message) = snapshot_messages.last() {
                    messages.push(last_snapshot_message);
                }

                // Make a for loop
                for message in messages {
                    match &message {
                        BaseMessage::System(m) => {
                            let _ = conversation_manager.add_system_message(m).await;
                        }
                        BaseMessage::Human(m) => {
                            let _ = conversation_manager.add_human_message(m).await;
                        }
                        _ => todo!(),
                    }
                }
            }
        }

        let mut complete_response = String::new();

        // Send initial empty chunk to signal start of streaming
        channel
            .send(ResponseChunk {
                chunk: "".to_string(),
            })
            .map_err(|e| format!("Failed to send initial response: {e}"))?;

        debug!("Sending chat stream");
        let stream_result = conversation_manager.chat_stream(query.text.clone()).await;
        // Drop the MutexGuard to free the lock before consuming the stream,
        // so other chat operations are not blocked during stream iteration
        drop(conversation_manager);

        match stream_result {
            Ok(mut stream) => {
                debug!("Starting to consume stream...");

                // Add timeout for stream processing
                let timeout_duration = std::time::Duration::from_secs(300); // 5 minutes
                let stream_future = async {
                    while let Some(result) = stream.next().await {
                        match result {
                            Ok(chunk) => {
                                // Skip empty chunks to reduce noise
                                if chunk.is_empty() {
                                    continue;
                                }

                                // Append to the complete response
                                complete_response.push_str(&chunk);

                                // Send the chunk to the frontend
                                if let Err(e) = channel.send(ResponseChunk { chunk }) {
                                    return Err(format!("Failed to send response chunk: {e}"));
                                }
                            }
                            Err(e) => {
                                return Err(format!("Stream error: {}", e));
                            }
                        }
                    }
                    Ok(())
                };

                // Apply timeout to stream processing
                match tokio::time::timeout(timeout_duration, stream_future).await {
                    Ok(Ok(())) => {
                        debug!("Stream completed successfully");
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
                return Err(format!("Failed to create chat stream: {}", e));
            }
        }

        let _ai_message: BaseMessage = AIMessage::new(complete_response.clone()).into();
        Ok(complete_response)
    }
}
