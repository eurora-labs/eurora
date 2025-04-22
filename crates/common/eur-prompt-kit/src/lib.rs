use image::DynamicImage;

pub enum LLMService {
    OpenAI,
    Anthropic,
    Google,
    Eurora,
    Local,
}

pub enum Role {
    System,
    User,
    Assistant,
}

pub enum ImageSource {
    DynamicImage(DynamicImage),
    Bytes(Vec<u8>),
    Path(std::path::PathBuf),
    Uri(String),
}

pub struct TextContent {
    pub text: String,
}

pub struct ImageContent {
    pub text: Option<String>,
    pub image_source: ImageSource,
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
