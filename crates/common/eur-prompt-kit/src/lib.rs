use anyhow::Result;
use image::DynamicImage;
use llm::{
    builder::{LLMBackend, LLMBuilder},
    chat::ChatMessage,
};

mod service;

#[derive(Debug, Default, Copy, Clone)]
pub enum EurLLMService {
    #[default]
    OpenAI,
    Anthropic,
    Google,
    Eurora,
    Local,
}

impl From<EurLLMService> for LLMBackend {
    fn from(value: EurLLMService) -> Self {
        match value {
            EurLLMService::OpenAI => LLMBackend::OpenAI,
            EurLLMService::Anthropic => LLMBackend::Anthropic,
            EurLLMService::Google => LLMBackend::Google,
            _ => LLMBackend::OpenAI,
        }
    }
}

pub enum Role {
    System,
    User,
    Assistant,
}

// pub enum ImageSource {
//     DynamicImage(DynamicImage),
//     Bytes(Vec<u8>),
//     Path(std::path::PathBuf),
//     Uri(String),
// }

pub struct TextContent {
    pub text: String,
}

pub struct ImageContent {
    pub text: Option<String>,
    pub image: DynamicImage,
}

pub enum MessageContent {
    Text(TextContent),
    Image(ImageContent),
}

pub struct LLMMessage {
    pub role: Role,
    pub content: MessageContent,
}

pub struct LLMRequest {
    pub service: EurLLMService,
    pub endpoint: String,
    pub model: String,

    pub messages: Vec<LLMMessage>,
    // Add extra parameters when functionality expands
}

impl From<LLMMessage> for ChatMessage {
    fn from(value: LLMMessage) -> Self {
        let mut message = match value.role {
            Role::System => ChatMessage::assistant(),
            Role::User => ChatMessage::user(),
            Role::Assistant => ChatMessage::assistant(),
        };

        message = match value.content {
            MessageContent::Text(TextContent { text }) => message.content(text),
            MessageContent::Image(ImageContent { text, image }) => message.content(text.unwrap()),
        };

        message.build()
    }
}

impl From<ChatMessage> for LLMMessage {
    fn from(value: ChatMessage) -> Self {
        LLMMessage {
            role: Role::User,
            content: MessageContent::Text(TextContent {
                text: value.content.to_string(),
            }),
        }
    }
}

pub async fn anonymize_text(text: String) -> Result<String> {
    // Send messages to self-hosted LLM with instruction to remove personal data
    // TODO: Switch back-end to local
    let llm = LLMBuilder::new()
        .backend(LLMBackend::OpenAI)
        .model("gpt-4.5-turbo")
        .temperature(0.7)
        .stream(false)
        .build()
        .expect("Failed to build LLM (OpenAI)");
    let messages = vec![
        ChatMessage::user()
            .content("Anonymize the text and remove any personal data from the next message: ")
            .build(),
        ChatMessage::user().content(text).build(),
    ];

    let response = match llm.chat(&messages).await {
        Ok(response) => response,
        Err(e) => return Err(e.into()),
    };

    Ok(response.text().unwrap_or_default())
}
