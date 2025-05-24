use crate::shared_types::{SharedOpenAIClient, SharedTimeline};
use futures::StreamExt;
use tauri::ipc::Channel;
use tauri::{Manager, Runtime};
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

#[taurpc::procedures]
pub trait QueryApi {
    async fn send_query<R: Runtime>(
        app_handle: tauri::AppHandle<R>,
        channel: Channel<ResponseChunk>,
        query: Query,
    ) -> Result<String, String>;
}

#[derive(Clone)]
pub struct QueryApiImpl;

#[taurpc::resolvers]
impl QueryApi for QueryApiImpl {
    async fn send_query<R: Runtime>(
        self,
        app_handle: tauri::AppHandle<R>,
        channel: Channel<ResponseChunk>,
        query: Query,
    ) -> Result<String, String> {
        let timeline_state: tauri::State<SharedTimeline> = app_handle.state();
        let timeline = timeline_state.inner();
        let title: String = "Placeholder Title".to_string();

        let mut messages = timeline.construct_asset_messages();

        messages.push(eur_prompt_kit::LLMMessage {
            role: eur_prompt_kit::Role::User,
            content: eur_prompt_kit::MessageContent::Text(eur_prompt_kit::TextContent {
                text: query.text.clone(),
            }),
        });

        let state: tauri::State<SharedOpenAIClient> = app_handle.state();
        let mut guard = state.lock().await;
        let client = guard
            .as_mut()
            .ok_or_else(|| "OpenAI client not initialized".to_string())?;

        // Create new conversation and store it in SQLite
        eprintln!("Creating new conversation with title: {}", title);

        let mut complete_response = String::new();

        let mut stream = client.video_question(messages).await?;

        channel
            .send(ResponseChunk {
                chunk: "".to_string(),
            })
            .map_err(|e| format!("Failed to send response: {e}"))?;

        while let Some(Ok(chunk)) = stream.next().await {
            for message in chunk.choices {
                let Some(message) = message.delta.content else {
                    continue;
                };
                // Append to the complete response
                complete_response.push_str(&message);

                channel
                    .send(ResponseChunk { chunk: message })
                    .map_err(|e| format!("Failed to send response: {e}"))?;
            }
        }

        Ok("".to_string())
    }
}
