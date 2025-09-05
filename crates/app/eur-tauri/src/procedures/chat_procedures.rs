use eur_personal_db::{Asset, Conversation, NewAsset, PersonalDatabaseManager};
use eur_timeline::TimelineManager;
use ferrous_llm_core::{Message, MessageContent, Role};
use futures::StreamExt;
use tauri::{Manager, Runtime, ipc::Channel};
use tracing::info;

use crate::shared_types::{SharedCurrentConversation, SharedPromptKitService};
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
    #[taurpc(event)]
    async fn current_conversation_changed(conversation: Conversation);

    async fn switch_conversation<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        conversation_id: String,
    ) -> Result<Conversation, String>;

    async fn send_query<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        conversation_id: String,
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
        conversation_id: String,
        channel: Channel<ResponseChunk>,
        query: Query,
    ) -> Result<String, String> {
        let personal_db: &PersonalDatabaseManager =
            app_handle.state::<PersonalDatabaseManager>().inner();
        let timeline_state: tauri::State<async_mutex::Mutex<TimelineManager>> = app_handle.state();
        let timeline = timeline_state.lock().await;

        let title: String = "Placeholder Title".to_string();
        let mut messages: Vec<Message> = Vec::new();

        // Add previous messages from this conversation
        if let Ok(previous_messages) = personal_db.get_chat_messages(&conversation_id).await {
            let chat_message_id = previous_messages
                .last()
                .map(|m| m.id.clone())
                .unwrap_or_default();

            // Collect assets for all messages that have them
            let mut previous_assets: Vec<eur_personal_db::Asset> = Vec::new();
            for message in &previous_messages {
                if message.has_assets
                    && let Ok(assets) = personal_db.get_assets_by_chat_message_id(&message.id).await
                {
                    previous_assets.extend(assets);
                }
            }

            let previous_messages = previous_messages
                .into_iter()
                .map(|message| message.into())
                .collect::<Vec<Message>>();

            // let assets = personal_db
            //     .get_assets_by_chat_message_id(&chat_message_id)
            //     .await
            //     .map_err(|e| format!("Failed to get assets: {}", e))?;

            messages.extend(previous_messages);
        }

        let has_assets = !query.assets.is_empty();

        if has_assets {
            messages = timeline.construct_asset_messages().await;
            messages.extend(timeline.construct_snapshot_messages().await);
        }

        let user_message = Message {
            role: Role::User,
            content: MessageContent::Text(query.text.clone()),
        };

        // Insert chat message into db
        let chat_message = personal_db
            .insert_chat_message_from_message(
                conversation_id.as_str(),
                user_message.clone(),
                has_assets,
            )
            .await
            .map_err(|e| format!("Failed to insert chat message: {e}"))?;

        let infos = timeline
            .save_assets_to_disk()
            .await
            .map_err(|e| format!("Failed to save assets: {e}"))?;

        for info in infos {
            let relative = info.file_path.to_string_lossy().into_owned();
            let absolute = info.absolute_path.to_string_lossy().into_owned();
            personal_db
                .insert_asset(&NewAsset {
                    id: None,
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

        // let mut db_activity = timeline
        //     .get_db_activity()
        //     .await
        //     .expect("Failed to get db activity");

        // // Insert activity into db
        // personal_db
        //     .insert_activity(&db_activity)
        //     .await
        //     .expect("Failed to insert activity");

        messages.push(user_message);

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

        personal_db
            .insert_chat_message_from_message(
                conversation_id.as_str(),
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
