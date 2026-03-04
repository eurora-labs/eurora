use agent_chain_core::{AIMessage, BaseMessage};
use euro_timeline::TimelineManager;
use futures::StreamExt;
use tauri::{Manager, Runtime, ipc::Channel};
use tokio::sync::Mutex;

use crate::{
    error::ResultExt, procedures::thread_procedures::TauRpcThreadApiEventTrigger,
    shared_types::SharedThreadManager,
};

#[taurpc::ipc_type]
pub struct ResponseChunk {
    chunk: String,
    reasoning: Option<String>,
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
        let thread_state: tauri::State<SharedThreadManager> = app_handle
            .try_state()
            .ok_or_else(|| "Thread manager not available".to_string())?;
        let timeline_state: tauri::State<Mutex<TimelineManager>> = app_handle
            .try_state()
            .ok_or_else(|| "Timeline not available".to_string())?;

        let mut thread_id = thread_id;
        let mut is_new_thread = false;

        {
            let timeline = timeline_state.lock().await;
            let mut thread_manager = thread_state.lock().await;

            if thread_id.is_none() {
                let thread = thread_manager
                    .ensure_remote_thread()
                    .await
                    .ctx("Failed to ensure remote thread")?;
                thread_id = Some(thread.id().unwrap().to_string());
                is_new_thread = true;
                TauRpcThreadApiEventTrigger::new(app_handle.clone())
                    .new_thread_added(thread.into())
                    .ctx("Failed to trigger new thread added event")?;
            }

            if timeline.save_current_activity_to_service().await.is_ok() && !query.assets.is_empty()
            {
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
                        _ => {
                            tracing::warn!("Unexpected message type in asset context");
                        }
                    }
                }
            }
        }

        let content = query.text.clone();
        let thread_id_for_title = thread_id.clone();
        if let (true, Some(id)) = (is_new_thread, thread_id_for_title) {
            let title_app_handle = app_handle.clone();
            tokio::spawn(async move {
                let result = {
                    let thread_state: tauri::State<SharedThreadManager> = title_app_handle.state();
                    let thread_manager = thread_state.lock().await;
                    thread_manager.generate_thread_title(id, content).await
                };
                match result {
                    Ok(thread) => {
                        let _ = TauRpcThreadApiEventTrigger::new(title_app_handle)
                            .thread_title_changed(thread.into());
                    }
                    Err(e) => {
                        tracing::error!("Failed to generate thread title: {e}");
                    }
                }
            });
        }

        let mut complete_response = String::new();

        tracing::debug!("Sending chat stream");
        let stream_result = {
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

        let _ai_message: BaseMessage = AIMessage::builder()
            .content(complete_response.clone())
            .build()
            .into();
        Ok(complete_response)
    }
}
