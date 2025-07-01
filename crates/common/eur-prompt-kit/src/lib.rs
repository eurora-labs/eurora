use eur_proto::proto_prompt_service::ProtoChatMessage;
use image::{DynamicImage, ImageFormat};
use llm::{
    builder::LLMBackend,
    chat::{ChatMessage, ImageMime},
};

mod config;
mod service;
pub use config::{EuroraConfig, OllamaConfig, RemoteConfig};
pub use service::PromptKitService;

#[derive(Debug, Default, Copy, Clone)]
pub enum EurLLMService {
    #[default]
    OpenAI,
    Anthropic,
    Google,
    Eurora,
    Local,
    Ollama,
}

impl From<EurLLMService> for LLMBackend {
    fn from(value: EurLLMService) -> Self {
        match value {
            EurLLMService::OpenAI => LLMBackend::OpenAI,
            EurLLMService::Anthropic => LLMBackend::Anthropic,
            EurLLMService::Google => LLMBackend::Google,
            EurLLMService::Eurora => todo!("Eurora backend not implemented"),
            EurLLMService::Local => todo!("Local backend not implemented"),
            EurLLMService::Ollama => LLMBackend::Ollama,
        }
    }
}

impl From<String> for EurLLMService {
    fn from(value: String) -> Self {
        match value.as_str() {
            "openai" => EurLLMService::OpenAI,
            "anthropic" => EurLLMService::Anthropic,
            "google" => EurLLMService::Google,
            "eurora" => EurLLMService::Eurora,
            "local" => EurLLMService::Local,
            "ollama" => EurLLMService::Ollama,
            _ => EurLLMService::OpenAI,
        }
    }
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct TextContent {
    pub text: String,
}

#[derive(Debug)]
pub struct ImageContent {
    pub text: Option<String>,
    pub image: DynamicImage,
}

#[derive(Debug)]
pub enum MessageContent {
    Text(TextContent),
    Image(ImageContent),
}
#[derive(Debug)]
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
            MessageContent::Image(ImageContent { text, image }) => {
                let mut buffer = Vec::new();
                let mut cursor = std::io::Cursor::new(&mut buffer);
                image.write_to(&mut cursor, ImageFormat::Png).unwrap();

                // message.image(ImageMime::PNG, buffer)
                message = message.image(ImageMime::PNG, buffer);
                message.content(text.unwrap())
            }
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

impl From<ProtoChatMessage> for LLMMessage {
    fn from(value: ProtoChatMessage) -> Self {
        LLMMessage {
            role: match value.role.as_str() {
                "user" => Role::User,
                "system" => Role::System,
                _ => Role::User,
            },
            content: MessageContent::Text(TextContent {
                text: value.content,
            }),
        }
    }
}

impl From<LLMMessage> for ProtoChatMessage {
    fn from(value: LLMMessage) -> Self {
        ProtoChatMessage {
            role: match value.role {
                Role::User => "user".to_string(),
                Role::System => "system".to_string(),
                _ => "user".to_string(),
            },
            content: match value.content {
                MessageContent::Text(TextContent { text }) => text,
                MessageContent::Image(ImageContent { text, image: _ }) => text.unwrap(),
            },
        }
    }
}
