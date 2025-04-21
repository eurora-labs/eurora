use image::DynamicImage;

pub enum LLMService {
    OpnAI,
    Anthropic,
    Google,
    Eurora,
    Local,
}

pub enum Role {
    System,
    User,
}

pub struct TextContent {
    prefix: String,
    body: String,
    suffix: String,
}

pub struct ImageContent {
    text: Option<TextContent>,
    image: DynamicImage,
}

pub enum MessageContent {
    Text(TextContent),
    Image(ImageContent),
}

pub struct Message {
    pub role: Role,
    pub content: MessageContent,
}

pub struct LLMRequest {
    pub service: LLMService,
    pub endpoint: String,
    pub model: String,

    pub messages: Vec<Message>,
    // Add extra parameters when functionality expands
}
