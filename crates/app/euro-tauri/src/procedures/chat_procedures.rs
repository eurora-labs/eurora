use agent_chain_core::{AIMessage, BaseMessage};
use euro_timeline::TimelineManager;
use futures::StreamExt;
use tauri::{Manager, Runtime, ipc::Channel};
use tokio::sync::Mutex;
use tracing::{debug, error, info};

use crate::{
    procedures::conversation_procedures::TauRpcConversationApiEventTrigger,
    shared_types::SharedConversationManager,
};

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
        conversation_id: Option<String>,
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
        conversation_id: Option<String>,
        channel: Channel<ResponseChunk>,
        query: Query,
    ) -> Result<String, String> {
        let mut conversation_id = conversation_id;

        let event = posthog_rs::Event::new_anon("send_query");
        tauri::async_runtime::spawn(async move {
            let _ = posthog_rs::capture(event).await.map_err(|e| {
                error!("Failed to capture posthog event: {}", e);
            });
        });

        // Scope for timeline and conversation_manager locks
        // These locks are released before stream processing to avoid blocking
        {
            let timeline_state: tauri::State<Mutex<TimelineManager>> = app_handle.state();
            let timeline = timeline_state.lock().await;
            let conversation_state: tauri::State<SharedConversationManager> = app_handle.state();
            let mut conversation_manager = conversation_state.lock().await;

            if conversation_id.is_none() {
                let conversation = conversation_manager
                    .ensure_remote_conversation()
                    .await
                    .expect("Failed to ensure remote conversation");
                conversation_id = Some(conversation.id().unwrap().to_string());
                TauRpcConversationApiEventTrigger::new(app_handle.clone())
                    .new_conversation_added(conversation.into())
                    .expect("Failed to trigger new conversation added event");
            }

            if timeline.save_current_activity_to_service().await.is_ok() {
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
            // timeline and conversation_manager locks are released here
        }

        // Create new thread to handle conversation title generation
        // This runs in a separate task so it doesn't block the main flow
        let title_app_handle = app_handle.clone();
        let content = query.text.clone();
        let conversation_id_for_title = conversation_id.clone();
        if let Some(id) = conversation_id_for_title {
            tokio::spawn(async move {
                let app_handle = title_app_handle.clone();
                let conversation_state: tauri::State<SharedConversationManager> =
                    app_handle.state();
                let conversation_manager = conversation_state.lock().await;
                let conversation = conversation_manager
                    .generate_conversation_title(id, content)
                    .await
                    .map_err(|e| format!("Failed to generate conversation title: {e}"));
                if let Ok(conversation) = conversation {
                    TauRpcConversationApiEventTrigger::new(title_app_handle)
                        .conversation_title_changed(conversation.into())
                        .expect("Failed to send conversation title");
                }
            });
        }

        let mut complete_response = String::new();

        // Send initial empty chunk to signal start of streaming
        channel
            .send(ResponseChunk {
                chunk: "".to_string(),
            })
            .map_err(|e| format!("Failed to send initial response: {e}"))?;

        debug!("Sending chat stream");
        // Acquire the lock just for creating the stream, then release it
        let stream_result = {
            let conversation_state: tauri::State<SharedConversationManager> = app_handle.state();
            let mut conversation_manager = conversation_state.lock().await;
            conversation_manager.chat_stream(query.text.clone()).await
            // conversation_manager lock is released here at end of scope
        };

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

        let _ai_message: BaseMessage = AIMessage::builder()
            .content(complete_response.clone())
            .build()
            .into();
        Ok(complete_response)
    }
}
