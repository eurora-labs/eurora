use ferrous_llm_core::{Message, MessageContent, Role};
use futures::StreamExt;
use tauri::{Manager, Runtime, ipc::Channel};
use tracing::info;

use crate::shared_types::{SharedPromptKitService, SharedTimeline};
#[taurpc::ipc_type]
pub struct ResponseChunk {
    chunk: String,
}
// enum ResponseChunk<'a> {
//     #[serde(rename_all = "camelCase")]
//     Message { message: &'a str },
//     #[serde(rename_all = "camelCase")]
//     Append { chunk: &'a str },
// }
#[taurpc::ipc_type]
pub struct Query {
    text: String,
    assets: Vec<String>,
}

#[taurpc::procedures(path = "chat")]
pub trait ChatApi {
    async fn send_query<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
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
        channel: Channel<ResponseChunk>,
        query: Query,
    ) -> Result<String, String> {
        let timeline_state: tauri::State<SharedTimeline> = app_handle.state();
        let timeline = timeline_state.inner();
        let title: String = "Placeholder Title".to_string();

        let mut messages: Vec<Message> = Vec::new();
        if query.assets.len() > 0 {
            messages = timeline.construct_asset_messages();
            messages.extend(timeline.construct_snapshot_messages());
        }

        messages.push(Message {
            role: Role::User,
            content: MessageContent::Text(query.text.clone()),
        });

        let state: tauri::State<SharedPromptKitService> = app_handle.state();
        let mut guard = state.lock().await;
        let client = guard
            .as_mut()
            .ok_or_else(|| "PromptKitService not initialized".to_string())?;

        // Create new conversation and store it in SQLite
        info!("Creating new conversation with title: {}", title);

        let mut complete_response = String::new();

        // Send initial empty chunk to signal start of streaming
        channel
            .send(ResponseChunk {
                chunk: "".to_string(),
            })
            .map_err(|e| format!("Failed to send initial response: {e}"))?;

        info!("Sending chat stream");
        match client.chat_stream(messages).await {
            Ok(mut stream) => {
                info!("Starting to consume stream...");

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
                        info!("Stream completed successfully");
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

        Ok(complete_response)
    }
}
