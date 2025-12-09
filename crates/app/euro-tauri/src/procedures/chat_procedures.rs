use euro_activity::AssetFunctionality;
use euro_llm::core::{Message, MessageContent, Role};
use euro_personal_db::{Conversation, NewAsset, PersonalDatabaseManager, UpdateConversation};
use euro_timeline::TimelineManager;
use futures::StreamExt;
use tauri::{Manager, Runtime, ipc::Channel};
use tokio::sync::Mutex;
use tracing::{debug, error};

use crate::shared_types::{SharedCurrentConversation, SharedPromptKitService};

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
    #[taurpc(event)]
    async fn current_conversation_changed(conversation: Conversation);

    async fn switch_conversation<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        conversation_id: String,
    ) -> Result<Conversation, String>;

    async fn send_query<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        conversation: Conversation,
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
        conversation: Conversation,
        channel: Channel<ResponseChunk>,
        query: Query,
    ) -> Result<String, String> {
        let personal_db: &PersonalDatabaseManager =
            app_handle.state::<PersonalDatabaseManager>().inner();
        let timeline_state: tauri::State<Mutex<TimelineManager>> = app_handle.state();
        let timeline = timeline_state.lock().await;

        let event = posthog_rs::Event::new_anon("send_query");
        tauri::async_runtime::spawn(async move {
            let _ = posthog_rs::capture(event).await.map_err(|e| {
                error!("Failed to capture posthog event: {}", e);
            });
        });

        let mut messages: Vec<Message> = Vec::new();

        // Add previous messages from this conversation
        if let Ok(previous_messages) = personal_db.get_chat_messages(&conversation.id).await {
            // Collect assets for all messages that have them
            let mut previous_assets: Vec<euro_personal_db::Asset> = Vec::new();
            for message in &previous_messages {
                if message.has_assets
                    && let Ok(assets) = personal_db.get_assets_by_chat_message_id(&message.id).await
                {
                    previous_assets.extend(assets);
                }
            }

            match timeline.load_assets_from_disk(&previous_assets).await {
                Ok(recon_assets) => {
                    for asset in recon_assets {
                        let message = asset.construct_messages();
                        messages.extend(message);
                    }
                }
                Err(e) => {
                    error!("Failed to load assets: {}", e);
                }
            }

            let previous_messages = previous_messages
                .into_iter()
                .map(|message| message.into())
                .collect::<Vec<Message>>();

            messages.extend(previous_messages);
        }

        let has_assets = !query.assets.is_empty();

        if has_assets {
            messages.extend(
                timeline
                    .construct_asset_messages_by_ids(&query.assets)
                    .await,
            );
            messages.extend(
                timeline
                    .construct_snapshot_messages_by_ids(&query.assets)
                    .await,
            );
        }

        let user_message = Message {
            role: Role::User,
            content: MessageContent::Text(query.text.clone()),
        };

        // Save chat message into db
        let chat_message = personal_db
            .insert_chat_message_from_message(&conversation.id, user_message.clone(), has_assets)
            .await
            .map_err(|e| format!("Failed to insert chat message: {e}"))?;

        if conversation.title.is_none() {
            personal_db
                .update_conversation(UpdateConversation {
                    id: conversation.id.clone(),
                    title: Some(query.text.clone().chars().take(35).collect()),
                })
                .await
                .map_err(|e| format!("Failed to update conversation title: {e}"))?;
        }

        if let Ok(infos) = timeline.save_assets_to_disk_by_ids(&query.assets).await {
            for info in infos {
                let relative = info.file_path.to_string_lossy().into_owned();
                let absolute = info.absolute_path.to_string_lossy().into_owned();
                let id = info
                    .file_path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned());
                personal_db
                    .insert_asset(&NewAsset {
                        id,
                        activity_id: None,
                        relative_path: relative,
                        absolute_path: absolute,
                        chat_message_id: Some(chat_message.id.clone()),
                        created_at: Some(info.saved_at),
                        updated_at: Some(info.saved_at),
                    })
                    .await
                    .expect("Failed to insert asset info");
            }
        }

        messages.push(user_message);

        let state: tauri::State<SharedPromptKitService> = app_handle.state();
        let mut guard = state.lock().await;
        let client = guard
            .as_mut()
            .ok_or_else(|| "PromptKitService not initialized".to_string())?;

        let mut complete_response = String::new();

        // Send initial empty chunk to signal start of streaming
        channel
            .send(ResponseChunk {
                chunk: "".to_string(),
            })
            .map_err(|e| format!("Failed to send initial response: {e}"))?;

        debug!("Sending chat stream");
        match client.chat_stream(messages).await {
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

        personal_db
            .insert_chat_message_from_message(
                &conversation.id,
                Message {
                    role: Role::Assistant,
                    content: MessageContent::Text(complete_response.clone()),
                },
                false,
            )
            .await
            .map_err(|e| format!("Failed to insert chat message: {e}"))?;

        Ok(complete_response)
    }

    async fn switch_conversation<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        conversation_id: String,
    ) -> Result<Conversation, String> {
        let personal_db = app_handle.state::<PersonalDatabaseManager>().inner();

        let conversation = personal_db
            .get_conversation(&conversation_id)
            .await
            .map_err(|e| format!("Failed to get conversation: {}", e))?;

        let current = app_handle.state::<SharedCurrentConversation>();
        let mut guard = current.lock().await;
        *guard = Some(conversation.clone());

        TauRpcChatApiEventTrigger::new(app_handle.clone())
            .current_conversation_changed(conversation.clone())
            .map_err(|e| e.to_string())?;

        Ok(conversation)
    }
}
