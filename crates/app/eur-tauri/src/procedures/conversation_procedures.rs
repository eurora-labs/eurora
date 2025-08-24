use chrono::Utc;
use eur_personal_db::{Conversation, PersonalDatabaseManager};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{Manager, Runtime};
use tracing::info;

#[taurpc::procedures(path = "conversation")]
pub trait ConversationApi {
    async fn create<R: Runtime>(app_handle: tauri::AppHandle<R>) -> Result<Conversation, String>;
}

#[derive(Clone)]
pub struct ConversationApiImpl;

#[taurpc::resolvers]
impl ConversationApi for ConversationApiImpl {
    async fn create<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
    ) -> Result<Conversation, String> {
        let personal_db = app_handle.state::<PersonalDatabaseManager>().inner();
        // Set title to current time string
        let title = Utc::now().to_rfc3339();

        let conversation = personal_db
            .insert_conversation(&title, Utc::now(), Utc::now())
            .await
            .map_err(|e| e.to_string())?;

        Ok(conversation)
    }

    // async fn ask_video_question<R: Runtime>(
    //     self,
    //     app_handle: tauri::AppHandle<R>,
    //     mut conversation_id: String,
    //     query: QueryAssets,
    //     channel: Channel<DownloadEvent>,
    // ) -> Result<String, String> {
    //     info!("Asking question: {}", query.text);
    //     info!("Conversation ID: {}", conversation_id);

    //     let db = app_handle.state::<SharedPersonalDb>().clone();

    //     // Get the timeline from app state
    //     let timeline_state: tauri::State<SharedTimeline> = app_handle.state();
    //     let timeline = timeline_state.inner();

    //     let title: String = "Placeholder Title".to_string();

    //     let mut messages = timeline.construct_asset_messages();

    //     messages.push(eur_prompt_kit::LLMMessage {
    //         role: eur_prompt_kit::Role::User,
    //         content: eur_prompt_kit::MessageContent::Text(eur_prompt_kit::TextContent {
    //             text: query.text.clone(),
    //         }),
    //     });

    //     let state: tauri::State<SharedOpenAIClient> = app_handle.state();
    //     let mut guard = state.lock().await;
    //     let client = guard
    //         .as_mut()
    //         .ok_or_else(|| "OpenAI client not initialized".to_string())?;

    //     if conversation_id == "NEW" {
    //         // Create new conversation and store it in SQLite
    //         info!("Creating new conversation with title: {}", title);

    //         let conversation = db
    //             .insert_conversation(&title, Utc::now(), Utc::now())
    //             .await
    //             .map_err(|e| format!("Failed to insert conversation: {}", e))?;

    //         conversation_id = conversation.id.clone();

    //         info!("New conversation ID: {}", conversation_id);

    //         db.insert_chat_message(
    //             &conversation_id,
    //             "USER",
    //             &query.text,
    //             true,
    //             Utc::now(),
    //             Utc::now(),
    //         )
    //         .await
    //         .map_err(|e| format!("Failed to insert chat message: {}", e))?;
    //     }

    //     let mut complete_response = String::new();

    //     let mut stream = client.video_question(messages).await?;

    //     channel
    //         .send(DownloadEvent {
    //             event_data: DownloadEventData::Message {
    //                 message: "".to_string(),
    //             },
    //         })
    //         .map_err(|e| format!("Failed to send response: {e}"))?;

    //     while let Some(Ok(chunk)) = stream.next().await {
    //         for message in chunk.choices {
    //             let Some(message) = message.delta.content else {
    //                 continue;
    //             };
    //             // Append to the complete response
    //             complete_response.push_str(&message);

    //             channel
    //                 .send(DownloadEvent {
    //                     event_data: DownloadEventData::Append { chunk: message },
    //                 })
    //                 .map_err(|e| format!("Failed to send response: {e}"))?;
    //         }
    //     }

    //     // After the stream ends, add the complete response as a ChatMessage to the conversation
    //     if !complete_response.is_empty() {
    //         db.insert_chat_message(
    //             &conversation_id,
    //             "SYSTEM",
    //             &complete_response,
    //             true,
    //             Utc::now(),
    //             Utc::now(),
    //         )
    //         .await
    //         .map_err(|e| format!("Failed to insert chat message: {}", e))?;
    //         info!(
    //             "Added assistant response to conversation {}",
    //             conversation_id
    //         );
    //     }

    //     Ok("test".into())
    // }

    // async fn continue_conversation<R: Runtime>(
    //     self,
    //     app_handle: tauri::AppHandle<R>,
    //     conversation_id: String,
    //     question: String,
    //     channel: Channel<DownloadEvent>,
    // ) -> Result<(), String> {
    //     info!("Continuing conversation: {}", conversation_id);
    //     info!("Asking question: {}", question);

    //     let db = app_handle.state::<SharedPersonalDb>().clone();

    //     db.insert_chat_message(
    //         &conversation_id,
    //         "user",
    //         &question,
    //         true,
    //         Utc::now(),
    //         Utc::now(),
    //     )
    //     .await
    //     .unwrap();

    //     let chat_messages = db
    //         .get_chat_messages(&conversation_id)
    //         .await
    //         .map_err(|e| format!("Failed to get previous messages: {}", e))
    //         .unwrap();

    //     // Get the OpenAI client
    //     let state: tauri::State<SharedOpenAIClient> = app_handle.state();
    //     let mut guard = state.lock().await;
    //     let client = guard
    //         .as_mut()
    //         .ok_or_else(|| "OpenAI client not initialized".to_string())?;

    //     let messages = chat_messages
    //         .iter()
    //         .map(|msg| {
    //             let mut role = eur_prompt_kit::Role::System;
    //             if msg.role == "user" {
    //                 role = eur_prompt_kit::Role::User;
    //             }

    //             eur_prompt_kit::LLMMessage {
    //                 role,
    //                 content: eur_prompt_kit::MessageContent::Text(eur_prompt_kit::TextContent {
    //                     text: msg.content.clone(),
    //                 }),
    //             }
    //         })
    //         .collect();

    //     let mut stream = client.video_question(messages).await?;
    //     channel
    //         .send(DownloadEvent {
    //             event_data: DownloadEventData::Message {
    //                 message: "".to_string(),
    //             },
    //         })
    //         .map_err(|e| format!("Failed to send response: {e}"))?;
    //     // Collect the complete response
    //     let mut complete_response = String::new();

    //     while let Some(Ok(chunk)) = stream.next().await {
    //         for message in chunk.choices {
    //             let Some(message) = message.delta.content else {
    //                 continue;
    //             };
    //             // Append to the complete response
    //             complete_response.push_str(&message);

    //             channel
    //                 .send(DownloadEvent {
    //                     event_data: DownloadEventData::Append { chunk: message },
    //                 })
    //                 .map_err(|e| format!("Failed to send response: {e}"))?;
    //         }
    //     }

    //     // After the stream ends, add the complete response as a ChatMessage to the conversation
    //     if !complete_response.is_empty() {
    //         db.insert_chat_message(
    //             &conversation_id,
    //             "SYSTEM",
    //             &complete_response,
    //             true,
    //             Utc::now(),
    //             Utc::now(),
    //         )
    //         .await
    //         .unwrap();
    //         info!(
    //             "Added assistant response to conversation {}",
    //             conversation_id
    //         );
    //     }

    //     Ok(())
    // }

    // async fn get_current_conversation<R: Runtime>(
    //     self,
    //     app_handle: tauri::AppHandle<R>,
    // ) -> Result<Conversation, String> {
    //     let db = app_handle.state::<SharedPersonalDb>().clone();
    //     let current_conversation_id = app_handle
    //         .state::<crate::SharedCurrentConversationId>()
    //         .clone();
    //     Ok(db.get_conversation(&current_conversation_id).await.unwrap())
    // }

    // async fn get_conversation_with_messages<R: Runtime>(
    //     self,
    //     app_handle: tauri::AppHandle<R>,
    //     conversation_id: String,
    // ) -> Result<(Conversation, Vec<ChatMessage>), String> {
    //     let db = app_handle.state::<SharedPersonalDb>().clone();
    //     let conversation = db
    //         .get_conversation_with_messages(&conversation_id)
    //         .await
    //         .unwrap();

    //     Ok(conversation)
    // }

    // async fn switch_conversation<R: Runtime>(
    //     self,
    //     app_handle: tauri::AppHandle<R>,
    //     conversation_id: String,
    // ) -> Result<(), String> {
    //     Ok(())
    // }

    // async fn list_conversations<R: Runtime>(
    //     self,
    //     app_handle: tauri::AppHandle<R>,
    // ) -> Result<Vec<Conversation>, String> {
    //     let db = app_handle.state::<SharedPersonalDb>().clone();
    //     let conversations = db.list_conversations().await.unwrap();
    //     Ok(conversations)
    // }
}
