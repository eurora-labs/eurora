use agent_chain_core::messages::{ContentBlock, ContentBlocks, TextContentBlock};
use agent_chain_core::proto::ProtoAiMessageChunk;
use euro_timeline::TimelineManager;
use futures::StreamExt;
use tauri::{Manager, Runtime, ipc::Channel};
use tokio::sync::Mutex;

use crate::shared_types::SharedThreadManager;

#[taurpc::ipc_type]
pub struct Query {
    text: String,
    assets: Vec<String>,
    parent_message_id: Option<String>,
}

#[taurpc::procedures(path = "chat")]
pub trait ChatApi {
    async fn send_query<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        thread_id: String,
        channel: Channel<ProtoAiMessageChunk>,
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
        channel: Channel<ProtoAiMessageChunk>,
        query: Query,
    ) -> Result<String, String> {
        let thread_state: tauri::State<SharedThreadManager> = app_handle
            .try_state()
            .ok_or_else(|| "Thread manager not available".to_string())?;
        let timeline_state: tauri::State<Mutex<TimelineManager>> = app_handle
            .try_state()
            .ok_or_else(|| "Timeline not available".to_string())?;

        let mut asset_chips_json: Option<String> = None;
        let mut context_blocks = ContentBlocks::new();

        {
            let timeline = timeline_state.lock().await;
            let mut thread_manager = thread_state.lock().await;

            let _ = timeline.refresh_current_activity().await;

            if timeline.save_current_activity_to_service().await.is_ok() && !query.assets.is_empty()
            {
                if let Some(chip) = timeline.get_context_chip().await {
                    asset_chips_json = serde_json::to_string(&vec![chip]).ok();
                }

                let mut all_blocks = ContentBlocks::new();

                let asset_blocks = timeline.construct_messages_from_last_asset().await;
                all_blocks.extend(asset_blocks.into_inner());

                let snapshot_blocks = timeline.construct_messages_from_last_snapshot().await;
                all_blocks.extend(snapshot_blocks.into_inner());

                if !all_blocks.is_empty() {
                    match thread_manager
                        .save_preliminary_content_blocks(thread_id.clone(), all_blocks)
                        .await
                    {
                        Ok(returned) => context_blocks = returned,
                        Err(e) => tracing::warn!("Failed to save preliminary blocks: {e}"),
                    }
                }
            }
        }

        let user_text_block: ContentBlock =
            TextContentBlock::builder().text(&query.text).build().into();
        context_blocks.push(user_text_block);

        tracing::debug!("Sending chat stream");
        let stream_result = {
            let mut thread_manager = thread_state.lock().await;
            thread_manager
                .chat_stream(
                    thread_id.clone(),
                    context_blocks,
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
                                let proto_chunk: ProtoAiMessageChunk = chunk.into();
                                if let Err(e) = channel.send(proto_chunk) {
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

        Ok(String::new())
    }
}
