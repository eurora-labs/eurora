use agent_chain_core::{AIMessage, BaseMessage};
use euro_timeline::TimelineManager;
use futures::StreamExt;
use tauri::{Manager, Runtime, ipc::Channel};
use tokio::sync::Mutex;

use crate::{
    procedures::thread_procedures::TauRpcThreadApiEventTrigger, shared_types::SharedThreadManager,
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
        thread_id: Option<String>,
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
        thread_id: Option<String>,
        channel: Channel<ResponseChunk>,
        query: Query,
    ) -> Result<String, String> {
        let mut thread_id = thread_id;

        {
            let timeline_state: tauri::State<Mutex<TimelineManager>> = app_handle.state();
            let timeline = timeline_state.lock().await;
            let thread_state: tauri::State<SharedThreadManager> = app_handle.state();
            let mut thread_manager = thread_state.lock().await;

            if thread_id.is_none() {
                let thread = thread_manager
                    .ensure_remote_thread()
                    .await
                    .expect("Failed to ensure remote thread");
                thread_id = Some(thread.id().unwrap().to_string());
                TauRpcThreadApiEventTrigger::new(app_handle.clone())
                    .new_thread_added(thread.into())
                    .expect("Failed to trigger new thread added event");
            }

            if timeline.save_current_activity_to_service().await.is_ok() {
                // For now, we don't need to save assets to service, that will come later
                // when there is a way to convert assets to messages directly
                // if let Ok(infos) = timeline.save_assets_to_service_by_ids(&query.assets).await {
                //     tracing::info!("Infos: {:?}", infos);
                // }

                let has_assets = !query.assets.is_empty();

                if has_assets {
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
                            BaseMessage::System(m) => {
                                let _ = thread_manager.add_system_message(m).await;
                            }
                            BaseMessage::Human(m) => {
                                let _ = thread_manager.add_hidden_human_message(m).await;
                            }
                            _ => todo!(),
                        }
                    }
                }
            }
        }
        let title_app_handle = app_handle.clone();
        let content = query.text.clone();
        let thread_id_for_title = thread_id.clone();
        if let Some(id) = thread_id_for_title {
            tokio::spawn(async move {
                let app_handle = title_app_handle.clone();
                let thread_state: tauri::State<SharedThreadManager> = app_handle.state();
                let thread_manager = thread_state.lock().await;
                let thread = thread_manager
                    .generate_thread_title(id, content)
                    .await
                    .map_err(|e| format!("Failed to generate thread title: {e}"));
                if let Ok(thread) = thread {
                    TauRpcThreadApiEventTrigger::new(title_app_handle)
                        .thread_title_changed(thread.into())
                        .expect("Failed to send thread title");
                }
            });
        }

        let mut complete_response = String::new();

        tracing::debug!("Sending chat stream");
        let stream_result = {
            let thread_state: tauri::State<SharedThreadManager> = app_handle.state();
            let mut thread_manager = thread_state.lock().await;
            thread_manager.chat_stream(query.text.clone()).await
        };

        match stream_result {
            Ok(mut stream) => {
                tracing::debug!("Starting to consume stream...");

                let timeout_duration = std::time::Duration::from_secs(300);
                let stream_future = async {
                    while let Some(result) = stream.next().await {
                        match result {
                            Ok(chunk) => {
                                if chunk.is_empty() {
                                    continue;
                                }

                                complete_response.push_str(&chunk);

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
