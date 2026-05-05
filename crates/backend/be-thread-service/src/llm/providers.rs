use std::sync::Arc;

use agent_chain::{BaseChatModel, BaseTool, ollama::ChatOllama, openai::ChatOpenAI};

use crate::tools::firecrawl_tools;

const BASE_NEBUL_URL: &str = "https://api.inference.nebul.io/v1";

/// LLM providers selected at startup. The chat and title providers always
/// exist; the vision provider is only present in cloud mode and brings its
/// own bundled tool set (Firecrawl).
pub struct Providers {
    pub chat: Arc<dyn BaseChatModel + Send + Sync>,
    pub title: Arc<dyn BaseChatModel + Send + Sync>,
    pub vision: Option<VisionConfig>,
}

pub struct VisionConfig {
    pub model: Arc<dyn BaseChatModel + Send + Sync>,
    pub default_tools: Vec<Arc<dyn BaseTool>>,
}

/// Construct provider clients from environment variables.
///
/// Local mode (`RUNNING_EURORA_FULLY_LOCAL=true`) uses Ollama for both chat
/// and title generation and disables vision/tools. Cloud mode uses Nebul's
/// OpenAI-compatible inference endpoint and binds Firecrawl tools to the
/// vision model.
pub fn build_providers() -> Providers {
    let local_mode = std::env::var("RUNNING_EURORA_FULLY_LOCAL")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if local_mode {
        let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2".to_string());
        let host = std::env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| "http://host.docker.internal:11434".to_string());
        let chat: Arc<dyn BaseChatModel + Send + Sync> =
            Arc::new(ChatOllama::builder().model(&model).base_url(&host).build());
        let title: Arc<dyn BaseChatModel + Send + Sync> =
            Arc::new(ChatOllama::builder().model(&model).base_url(&host).build());
        Providers {
            chat,
            title,
            vision: None,
        }
    } else {
        let api_key =
            std::env::var("NEBUL_API_KEY").expect("NEBUL_API_KEY environment variable must be set");

        let chat: Arc<dyn BaseChatModel + Send + Sync> = Arc::new(
            ChatOpenAI::builder()
                .model(std::env::var("NEBUL_MODEL").expect("NEBUL_MODEL must be set"))
                .reasoning_effort("medium")
                .api_base(BASE_NEBUL_URL)
                .api_key(&api_key)
                .use_responses_api(false)
                .build(),
        );

        let title: Arc<dyn BaseChatModel + Send + Sync> = Arc::new(
            ChatOpenAI::builder()
                .model(std::env::var("NEBUL_TITLE_MODEL").expect("NEBUL_TITLE_MODEL must be set"))
                .api_base(BASE_NEBUL_URL)
                .api_key(&api_key)
                .build(),
        );

        let vision_model: Arc<dyn BaseChatModel + Send + Sync> = Arc::new(
            ChatOpenAI::builder()
                .model(std::env::var("NEBUL_VISION_MODEL").expect("NEBUL_VISION_MODEL must be set"))
                .api_base(BASE_NEBUL_URL)
                .api_key(&api_key)
                .use_responses_api(false)
                .build(),
        );

        Providers {
            chat,
            title,
            vision: Some(VisionConfig {
                model: vision_model,
                default_tools: firecrawl_tools(),
            }),
        }
    }
}
