use base64::prelude::*;
use base64::{Engine as _, engine::general_purpose};
use config::{Config, Environment, File};
use dotenv::dotenv;
use eur_prompt_kit::{ImageContent, Message, MessageContent, Role};
use eur_util::flatten_transcript_with_highlight;
use futures::Stream;
use image;
use openai_api_rs::v1::chat_completion::{
    self, ChatCompletionRequest, ChatCompletionResponseForStream,
};
use openai_api_rs::v1::error::APIError;
use serde::Deserialize;
use std::io::Cursor;
use std::sync::OnceLock;
use thiserror::Error;
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to load configuration: {0}")]
    LoadError(#[from] config::ConfigError),
    #[error("OpenAI API key not found")]
    MissingApiKey,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    openai_api_key: String,
}

static SETTINGS: OnceLock<Settings> = OnceLock::new();

impl Settings {
    pub fn global() -> &'static Settings {
        SETTINGS.get_or_init(|| {
            Self::new().unwrap_or_else(|err| {
                panic!("Failed to initialize settings: {}", err);
            })
        })
    }

    fn new() -> Result<Self, ConfigError> {
        // Load .env file if it exists
        dotenv().ok();

        // Build configuration
        let config = Config::builder()
            // Start with default configuration
            .set_default("openai_api_key", "")?
            // Add in settings from the config directory if it exists
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name("config/local").required(false))
            // Add in settings from environment variables (with prefix "EUR_")
            // E.g., `EUR_OPENAI_API_KEY=value`
            .add_source(Environment::with_prefix("EUR"))
            .build()?;

        let settings: Settings = config.try_deserialize()?;

        if settings.openai_api_key.is_empty() {
            return Err(ConfigError::MissingApiKey);
        }

        Ok(settings)
    }

    pub fn openai_api_key(&self) -> &str {
        &self.openai_api_key
    }
}
use eur_proto::ipc::{ProtoArticleState, ProtoPdfState, ProtoYoutubeState};
use eur_proto::questions_service::ProtoChatMessage;
use openai_api_rs::v1::api::OpenAIClient;
use openai_api_rs::v1::common::GPT4_O_LATEST;

pub struct OpenAI {
    client: OpenAIClient,
}

impl Default for OpenAI {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenAI {
    pub fn new() -> Self {
        let settings = Settings::global();

        let client = OpenAIClient::builder()
            .with_api_key(settings.openai_api_key().to_string())
            .build()
            .unwrap();
        Self { client }
    }

    /// Creates a new OpenAI client with the provided API key
    pub fn with_api_key(api_key: &str) -> Self {
        let client = OpenAIClient::builder()
            .with_api_key(api_key.to_string())
            .build()
            .unwrap();
        Self { client }
    }

    pub async fn video_question_temp(
        &mut self,
        messages: Vec<Message>,
    ) -> Result<impl Stream<Item = Result<ChatCompletionResponseForStream, APIError>>, String> {
        if messages.is_empty() {
            return Err("Messages cannot be empty".to_string());
        }

        let mut openai_messages = Vec::new();

        // Process the first message (assumed to be image + text)
        let first_message = &messages[0];
        if let MessageContent::Image(image_content) = &first_message.content {
            // if let MessageContent::Text(image_content) = &first_message.content {
            let mut image_data: Vec<u8> = Vec::new();
            image_content
                .image
                .write_to(&mut Cursor::new(&mut image_data), image::ImageFormat::Png)
                .unwrap();

            let image_base64 = general_purpose::STANDARD.encode(image_data);

            let image_url = format!("data:image/jpeg;base64,{image_base64}"); // Assuming JPEG

            let mut content_parts = vec![];

            // Add text part if present
            if let Some(text) = &image_content.text {
                // if let text = &image_content.text {
                content_parts.push(chat_completion::ImageUrl {
                    r#type: chat_completion::ContentType::text,
                    text: Some(text.clone()),
                    image_url: None,
                });
            }

            // Add image part
            content_parts.push(chat_completion::ImageUrl {
                r#type: chat_completion::ContentType::image_url,
                text: None,
                image_url: Some(chat_completion::ImageUrlType { url: image_url }),
            });

            openai_messages.push(chat_completion::ChatCompletionMessage {
                role: match first_message.role {
                    Role::User => chat_completion::MessageRole::user,
                    Role::System => chat_completion::MessageRole::system,
                    Role::Assistant => chat_completion::MessageRole::assistant, // Should not happen for first message usually
                },
                content: chat_completion::Content::ImageUrl(content_parts),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });
        } else {
            return Err("First message must be of type MessageContent::Image".to_string());
        }

        // Process subsequent messages (conversation history)
        for message in messages.iter().skip(1) {
            match &message.content {
                MessageContent::Text(text_message) => {
                    openai_messages.push(chat_completion::ChatCompletionMessage {
                        role: match message.role {
                            Role::User => chat_completion::MessageRole::user,
                            Role::Assistant => chat_completion::MessageRole::assistant,
                            Role::System => chat_completion::MessageRole::system, // Less common in history
                        },
                        content: chat_completion::Content::Text(text_message.text.clone()),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    });
                }
                MessageContent::Image(_) => {
                    // Handle or return error if subsequent messages contain images,
                    // as standard chat history usually doesn't mix images like this.
                    return Err(
                        "Subsequent messages cannot contain images in this context".to_string()
                    );
                }
            }
        }

        let req =
            ChatCompletionRequest::new(GPT4_O_LATEST.to_string(), openai_messages).stream(true);

        self.client
            .chat_completion_stream(req)
            .await
            .map_err(|e| format!("Failed to create chat completion stream: {}", e))
    }

    pub async fn video_question(
        &mut self,
        messages: Vec<ProtoChatMessage>,
        state: ProtoYoutubeState,
    ) -> Result<impl Stream<Item = Result<ChatCompletionResponseForStream, APIError>>, String> {
        // Convert video frame bytes to base64
        let image_base64 = BASE64_STANDARD.encode(state.video_frame.unwrap().data);

        let flat_transcript = flatten_transcript_with_highlight(
            state.transcript,
            state.current_time,
            "%CURRENT%".to_string(),
        );

        // Create initial messages with system and user content
        let mut chat_messages = vec![chat_completion::ChatCompletionMessage {
            role: chat_completion::MessageRole::user,
            content: chat_completion::Content::ImageUrl(vec![
                chat_completion::ImageUrl {
                    r#type: chat_completion::ContentType::text,
                    text: Some(format!(
                        "I am watching a video and have a question about it. \
                        I attached the screenshot of the last moment in the video. \
                        Here's the transcript of the whole video. \
                        The current line is denoted with %CURRENT% tag:\n{}",
                        flat_transcript
                    )),
                    image_url: None,
                },
                chat_completion::ImageUrl {
                    r#type: chat_completion::ContentType::image_url,
                    text: None,
                    image_url: Some(chat_completion::ImageUrlType {
                        url: format!("data:image/jpeg;base64,{image_base64}"),
                    }),
                },
            ]),

            name: None,
            tool_calls: None,
            tool_call_id: None,
        }];

        // Add conversation history
        for message in messages.iter() {
            chat_messages.push(chat_completion::ChatCompletionMessage {
                role: if message.role == "user" {
                    chat_completion::MessageRole::user
                } else {
                    chat_completion::MessageRole::assistant
                },
                content: chat_completion::Content::Text(message.content.clone()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }

        let req = ChatCompletionRequest::new(GPT4_O_LATEST.to_string(), chat_messages).stream(true);

        self.client
            .chat_completion_stream(req)
            .await
            .map_err(|e| format!("Failed to create chat completion stream: {}", e))
    }

    pub async fn article_question(
        &mut self,
        messages: Vec<ProtoChatMessage>,
        state: ProtoArticleState,
    ) -> Result<impl Stream<Item = Result<ChatCompletionResponseForStream, APIError>>, String> {
        let mut chat_messages = vec![chat_completion::ChatCompletionMessage {
            role: chat_completion::MessageRole::user,
            content: chat_completion::Content::Text(format!(
                "I am reading an article and have a question about it. \
                Here's the text content of the article: \n{}",
                state.content
            )),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }];

        // Add highlighted text if it exists
        if !state.selected_text.is_empty() {
            chat_messages.push(chat_completion::ChatCompletionMessage {
                role: chat_completion::MessageRole::user,
                content: chat_completion::Content::Text(format!(
                    "I highlighted the following part of the article: \n{}",
                    state.selected_text
                )),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }

        for message in messages.iter() {
            chat_messages.push(chat_completion::ChatCompletionMessage {
                role: if message.role == "user" {
                    chat_completion::MessageRole::user
                } else {
                    chat_completion::MessageRole::assistant
                },
                content: chat_completion::Content::Text(message.content.clone()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }

        let req = ChatCompletionRequest::new(GPT4_O_LATEST.to_string(), chat_messages).stream(true);

        self.client
            .chat_completion_stream(req)
            .await
            .map_err(|e| format!("Failed to create chat completion stream: {}", e))
    }

    pub async fn pdf_question(
        &mut self,
        messages: Vec<ProtoChatMessage>,
        state: ProtoPdfState,
    ) -> Result<impl Stream<Item = Result<ChatCompletionResponseForStream, APIError>>, String> {
        let mut chat_messages = vec![chat_completion::ChatCompletionMessage {
            role: chat_completion::MessageRole::user,
            content: chat_completion::Content::Text(format!(
                "I am reading a PDF document and have a question about it. \n \
                Here's the text content of the current page: \n{}",
                state.content
            )),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }];

        // Add highlighted text if it exists
        if !state.selected_text.is_empty() {
            chat_messages.push(chat_completion::ChatCompletionMessage {
                role: chat_completion::MessageRole::user,
                content: chat_completion::Content::Text(format!(
                    "I highlighted the following part of the document: \n{}",
                    state.selected_text
                )),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }

        for message in messages.iter() {
            chat_messages.push(chat_completion::ChatCompletionMessage {
                role: if message.role == "user" {
                    chat_completion::MessageRole::user
                } else {
                    chat_completion::MessageRole::assistant
                },
                content: chat_completion::Content::Text(message.content.clone()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }

        let req = ChatCompletionRequest::new(GPT4_O_LATEST.to_string(), chat_messages).stream(true);

        self.client
            .chat_completion_stream(req)
            .await
            .map_err(|e| format!("Failed to create chat completion stream: {}", e))
    }

    pub async fn send_message_to_llm(&mut self, messages: Vec<String>) -> String {
        let req = ChatCompletionRequest::new(
            GPT4_O_LATEST.to_string(),
            vec![chat_completion::ChatCompletionMessage {
                role: chat_completion::MessageRole::user,
                content: chat_completion::Content::Text(messages.join("\n")),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }],
        );

        let completion = self.client.chat_completion(req).await.unwrap();

        completion.choices[0]
            .message
            .content
            .clone()
            .unwrap()
            .to_string()
    }
}
