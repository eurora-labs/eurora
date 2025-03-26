//! Service for handling content-related questions using OpenAI's chat completion API.

pub mod util;

use anyhow::{Result, anyhow};
use async_openai::{
    Client,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestMessageContentPart,
        ChatCompletionRequestMessageContentPartImage, ChatCompletionRequestMessageContentPartText,
        CreateChatCompletionRequest, Role,
    },
};
use base64::Engine;
use eur_proto::questions_service::{
    ArticleQuestionRequest, ArticleQuestionResponse, ProtoChatMessage, PdfQuestionRequest,
    PdfQuestionResponse, VideoQuestionRequest, VideoQuestionResponse,
};
use eur_proto::tauri_ipc::{ProtoArticleState, ProtoPdfState};
use serde_json::Value;
use std::env;
use tracing::{debug, error, info};

use crate::util::flatten_transcript_with_highlight;

/// Initializes the OpenAI client
pub fn init_openai_client() -> Client {
    // Ensure environment variables are loaded
    dotenv::dotenv().ok();

    // Use the OpenAI API key from environment variables
    Client::new()
}

/// Converts a proto chat message to an OpenAI chat message
fn convert_chat_message(message: &ProtoChatMessage) -> ChatCompletionRequestMessage {
    let role = match message.role.as_str() {
        "user" => Role::User,
        "system" => Role::System,
        "assistant" => Role::Assistant,
        _ => Role::User, // Default to user role
    };

    ChatCompletionRequestMessage {
        role,
        content: Some(message.content.clone().into()),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    }
}

/// Process a video-related question and return an AI-generated response.
pub async fn video_question(request: VideoQuestionRequest) -> Result<VideoQuestionResponse> {
    info!("Received video question request in service");
    debug!("State: {:?}", request.state);

    let messages = request.messages;
    let state = request
        .state
        .ok_or_else(|| anyhow!("Missing state in request"))?;

    let proto_image = state
        .video_frame
        .ok_or_else(|| anyhow!("Missing video frame in state"))?;

    // Convert the image data from bytes to a base64 string
    let image_base64 = base64::engine::general_purpose::STANDARD.encode(&proto_image.data);

    let transcript_str = state.transcript.clone();
    let transcript: Vec<Value> = serde_json::from_str(&transcript_str)
        .map_err(|e| anyhow!("Failed to parse transcript: {}", e))?;

    let current_time = state.current_time;

    let flat_transcript = flatten_transcript_with_highlight(&transcript, current_time, None);
    info!("Finished processing request");

    let completion = send_video_request_to_llm(&messages, &image_base64, &flat_transcript).await?;
    info!("Finished sending request to LLM");

    let content = completion.ok_or_else(|| anyhow!("No response content from LLM"))?;
    debug!("Response: {}", content);

    Ok(VideoQuestionResponse { response: content })
}

/// Process an article-related question and return an AI-generated response.
pub async fn article_question(request: ArticleQuestionRequest) -> Result<ArticleQuestionResponse> {
    info!("Received article question request in service");

    let messages = request.messages;
    let state = request
        .state
        .ok_or_else(|| anyhow!("Missing state in request"))?;

    let completion = send_article_request_to_llm(&messages, &state).await?;
    info!("Finished sending request to LLM");

    let content = completion.ok_or_else(|| anyhow!("No response content from LLM"))?;

    Ok(ArticleQuestionResponse { response: content })
}

/// Process a PDF-related question and return an AI-generated response.
pub async fn pdf_question(request: PdfQuestionRequest) -> Result<PdfQuestionResponse> {
    info!("Received PDF question request in service");
    debug!("State: {:?}", request.state);

    let messages = request.messages;
    let state = request
        .state
        .ok_or_else(|| anyhow!("Missing state in request"))?;

    let completion = send_pdf_request_to_llm(&messages, &state).await?;
    info!("Finished sending request to LLM");

    let content = completion.ok_or_else(|| anyhow!("No response content from LLM"))?;

    Ok(PdfQuestionResponse { response: content })
}

/// Send a video-related question to OpenAI's LLM and return the completion response.
async fn send_video_request_to_llm(
    messages: &[ProtoChatMessage],
    image_base64: &str,
    flat_transcript: &str,
) -> Result<Option<String>> {
    info!("Sending video request to LLM");
    debug!("Transcript: {}", flat_transcript);

    let client = init_openai_client();

    // Create initial system message
    let system_message = ChatCompletionRequestMessage {
        role: Role::System,
        content: Some("You are a helpful assistant.".into()),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    };

    // Create the message with transcript and image
    let message_parts = vec![
        ChatCompletionRequestMessageContentPart::Text(
            ChatCompletionRequestMessageContentPartText {
                text: format!(
                    "I am watching a video and have a question about it. \
                    I attached the screenshot of the last moment in the video. \
                    Here's the transcript of the whole video. \
                    The current line is denoted with %HIGHLIGHT% tag:\n {}",
                    flat_transcript
                ),
                r#type: "text".into(),
            },
        ),
        ChatCompletionRequestMessageContentPart::Image(
            ChatCompletionRequestMessageContentPartImage {
                image_url: format!("data:image/jpeg;base64,{}", image_base64),
                r#type: "image_url".into(),
                detail: Some("high".into()),
            },
        ),
    ];

    let user_message = ChatCompletionRequestMessage {
        role: Role::User,
        content: Some(message_parts.into()),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    };

    // Convert all remaining messages from the request
    let chat_messages: Vec<ChatCompletionRequestMessage> =
        messages.iter().map(convert_chat_message).collect();

    // Combine all messages
    let mut all_messages = vec![system_message, user_message];
    all_messages.extend(chat_messages);

    // Create the chat completion request
    let request = CreateChatCompletionRequest {
        model: env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string()),
        messages: all_messages,
        ..Default::default()
    };

    // Send the request to OpenAI
    match client.chat().create(request).await {
        Ok(response) => {
            if let Some(choice) = response.choices.first() {
                if let Some(content) = &choice.message.content {
                    return Ok(Some(content.clone()));
                }
            }

            error!("No content in response from OpenAI");
            Ok(None)
        }
        Err(e) => {
            error!("Error sending request to OpenAI: {}", e);
            Err(anyhow!("OpenAI API error: {}", e))
        }
    }
}

/// Send an article-related question to OpenAI's LLM and return the completion response.
async fn send_article_request_to_llm(
    messages: &[ProtoChatMessage],
    state: &ProtoArticleState,
) -> Result<Option<String>> {
    info!("Sending article request to LLM");

    let client = init_openai_client();

    // Create initial system message
    let system_message = ChatCompletionRequestMessage {
        role: Role::System,
        content: Some("You are a helpful assistant.".into()),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    };

    // Create the message with article content and highlighted text
    let message_parts = vec![
        ChatCompletionRequestMessageContentPart::Text(
            ChatCompletionRequestMessageContentPartText {
                text: format!(
                    "I am reading an article and have a question about it. \
                    Here's the text content of the article:\n {}",
                    state.content
                ),
                r#type: "text".into(),
            },
        ),
        ChatCompletionRequestMessageContentPart::Text(
            ChatCompletionRequestMessageContentPartText {
                text: format!(
                    "I highlighted the following part of the article\n {}",
                    state.selectedText
                ),
                r#type: "text".into(),
            },
        ),
    ];

    let user_message = ChatCompletionRequestMessage {
        role: Role::User,
        content: Some(message_parts.into()),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    };

    // Convert all remaining messages from the request
    let chat_messages: Vec<ChatCompletionRequestMessage> =
        messages.iter().map(convert_chat_message).collect();

    // Combine all messages
    let mut all_messages = vec![system_message, user_message];
    all_messages.extend(chat_messages);

    // Create the chat completion request
    let request = CreateChatCompletionRequest {
        model: env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string()),
        messages: all_messages,
        ..Default::default()
    };

    // Send the request to OpenAI
    match client.chat().create(request).await {
        Ok(response) => {
            if let Some(choice) = response.choices.first() {
                if let Some(content) = &choice.message.content {
                    return Ok(Some(content.clone()));
                }
            }

            error!("No content in response from OpenAI");
            Ok(None)
        }
        Err(e) => {
            error!("Error sending request to OpenAI: {}", e);
            Err(anyhow!("OpenAI API error: {}", e))
        }
    }
}

/// Send a PDF-related question to OpenAI's LLM and return the completion response.
async fn send_pdf_request_to_llm(
    messages: &[ProtoChatMessage],
    state: &ProtoPdfState,
) -> Result<Option<String>> {
    info!("Sending PDF request to LLM");

    let client = init_openai_client();

    // Create initial system message
    let system_message = ChatCompletionRequestMessage {
        role: Role::System,
        content: Some("You are a helpful assistant.".into()),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    };

    // Create the message with PDF content and highlighted text
    let message_parts = vec![
        ChatCompletionRequestMessageContentPart::Text(
            ChatCompletionRequestMessageContentPartText {
                text: format!(
                    "I am reading a PDF document and have a question about it. \
                    Here's the text content of the current page:\n {}",
                    state.content
                ),
                r#type: "text".into(),
            },
        ),
        ChatCompletionRequestMessageContentPart::Text(
            ChatCompletionRequestMessageContentPartText {
                text: format!(
                    "I highlighted the following part of the document\n {}",
                    state.selectedText
                ),
                r#type: "text".into(),
            },
        ),
    ];

    let user_message = ChatCompletionRequestMessage {
        role: Role::User,
        content: Some(message_parts.into()),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    };

    // Convert all remaining messages from the request
    let chat_messages: Vec<ChatCompletionRequestMessage> =
        messages.iter().map(convert_chat_message).collect();

    // Combine all messages
    let mut all_messages = vec![system_message, user_message];
    all_messages.extend(chat_messages);

    // Create the chat completion request
    let request = CreateChatCompletionRequest {
        model: env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string()),
        messages: all_messages,
        ..Default::default()
    };

    // Send the request to OpenAI
    match client.chat().create(request).await {
        Ok(response) => {
            if let Some(choice) = response.choices.first() {
                if let Some(content) = &choice.message.content {
                    return Ok(Some(content.clone()));
                }
            }

            error!("No content in response from OpenAI");
            Ok(None)
        }
        Err(e) => {
            error!("Error sending request to OpenAI: {}", e);
            Err(anyhow!("OpenAI API error: {}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
