use std::collections::HashMap;

use crate::{ActivityAsset, ActivitySnapshot, ActivityStrategy, ContextChip};
use anyhow::Result;
use async_trait::async_trait;
use eur_native_messaging::{Channel, TauriIpcClient, create_grpc_ipc_client};
use eur_proto::ipc::{
    self, ProtoArticleState, ProtoPdfState, ProtoYoutubeSnapshot, ProtoYoutubeState, StateRequest,
};
use eur_proto::shared::ProtoImageFormat;

use image::DynamicImage;
use tokio::sync::Mutex;

use eur_prompt_kit::{ImageContent, LLMMessage, MessageContent, Role, TextContent};

#[derive(Debug, Clone)]
struct TranscriptLine {
    text: String,
    start: f32,
    _duration: f32,
}

struct YoutubeAsset {
    pub _url: String,
    pub title: String,
    pub transcript: Vec<TranscriptLine>,
    pub _current_time: f32,
    pub video_frame: DynamicImage,
}

struct ArticleAsset {
    pub _url: String,
    pub title: String,
    pub content: String,
}

impl From<ProtoYoutubeState> for YoutubeAsset {
    fn from(state: ProtoYoutubeState) -> Self {
        // eprintln!("Converting ProtoYoutubeState to YoutubeAsset");
        // eprintln!("ProtoYoutubeState: {:?}", state);
        YoutubeAsset {
            _url: state.url,
            title: "transcript asset".to_string(),
            transcript: state
                .transcript
                .into_iter()
                .map(|line| TranscriptLine {
                    text: line.text,
                    start: line.start,
                    _duration: line.duration,
                })
                .collect(),
            _current_time: state.current_time,
            video_frame: {
                let proto_image = state.video_frame.unwrap();
                // Directly load the image using the logic from eur-proto's From impl
                match ProtoImageFormat::try_from(proto_image.format).unwrap_or_default() {
                    ProtoImageFormat::Png => image::load_from_memory_with_format(
                        &proto_image.data,
                        image::ImageFormat::Png,
                    )
                    .expect("Failed to load PNG image from proto"),
                    ProtoImageFormat::Jpeg => image::load_from_memory_with_format(
                        &proto_image.data,
                        image::ImageFormat::Jpeg,
                    )
                    .expect("Failed to load JPEG image from proto"),
                    ProtoImageFormat::Webp => image::load_from_memory_with_format(
                        &proto_image.data,
                        image::ImageFormat::WebP,
                    )
                    .expect("Failed to load WebP image from proto"),
                    _ => image::load_from_memory(&proto_image.data)
                        .expect("Failed to load image from proto"),
                }
            },
        }
    }
}

impl From<ProtoArticleState> for ArticleAsset {
    fn from(article: ProtoArticleState) -> Self {
        ArticleAsset {
            _url: "".to_string(),
            // title: article.title,
            title: "article asset".to_string(),
            content: article.content,
        }
    }
}

impl ActivityAsset for YoutubeAsset {
    fn get_name(&self) -> &String {
        &self.title
    }

    fn get_icon(&self) -> Option<&String> {
        None
    }

    fn construct_message(&self) -> LLMMessage {
        LLMMessage {
            role: Role::User,
            content: MessageContent::Image(ImageContent {
                text: Some(format!(
                    "I am watching a video and have a question about it. \
                Here's the transcript of the video: \n {}",
                    self.transcript
                        .iter()
                        .map(|line| format!("{} ({}s)", line.text, line.start))
                        .collect::<Vec<_>>()
                        .join("\n")
                )),
                image: self.video_frame.clone(),
            }),
        }
    }

    fn get_context_chip(&self) -> Option<ContextChip> {
        Some(ContextChip {
            extension_id: "9370B14D-B61C-4CE2-BDE7-B18684E8731A".to_string(),
            attrs: HashMap::from([("text".to_string(), self.title.clone())]),
            icon: None,
            position: Some(0),
        })
    }
}

impl ActivityAsset for ArticleAsset {
    fn get_name(&self) -> &String {
        &self.title
    }

    fn get_icon(&self) -> Option<&String> {
        None
    }

    fn construct_message(&self) -> LLMMessage {
        LLMMessage {
            role: Role::User,
            content: MessageContent::Text(TextContent {
                text: format!(
                    "I am reading an article and have a question about it. \
                Here's the text content of the article: \n {}",
                    self.content
                ),
            }),
        }
    }

    fn get_context_chip(&self) -> Option<ContextChip> {
        Some(ContextChip {
            extension_id: "None".to_string(),
            attrs: HashMap::new(),
            icon: None,
            position: Some(0),
        })
    }
}

pub struct ArticleSnapshot {
    pub highlight: Option<String>,
}

impl ActivitySnapshot for ArticleSnapshot {
    fn construct_message(&self) -> LLMMessage {
        LLMMessage {
            role: Role::User,
            content: MessageContent::Text(TextContent {
                text: format!(
                    "I highlighted the following text: \n {}",
                    self.highlight.clone().unwrap_or_default()
                ),
            }),
        }
    }

    fn get_updated_at(&self) -> u64 {
        todo!()
    }

    fn get_created_at(&self) -> u64 {
        todo!()
    }
}

struct YoutubeSnapshot {
    pub video_frame: DynamicImage,
}

impl From<ProtoYoutubeSnapshot> for YoutubeSnapshot {
    fn from(snapshot: ProtoYoutubeSnapshot) -> Self {
        YoutubeSnapshot {
            video_frame: {
                let proto_image = snapshot.video_frame.unwrap();
                // Directly load the image using the logic from eur-proto's From impl
                match ProtoImageFormat::try_from(proto_image.format).unwrap_or_default() {
                    ProtoImageFormat::Png => image::load_from_memory_with_format(
                        &proto_image.data,
                        image::ImageFormat::Png,
                    )
                    .expect("Failed to load PNG image from proto"),
                    ProtoImageFormat::Jpeg => image::load_from_memory_with_format(
                        &proto_image.data,
                        image::ImageFormat::Jpeg,
                    )
                    .expect("Failed to load JPEG image from proto"),
                    ProtoImageFormat::Webp => image::load_from_memory_with_format(
                        &proto_image.data,
                        image::ImageFormat::WebP,
                    )
                    .expect("Failed to load WebP image from proto"),
                    _ => image::load_from_memory(&proto_image.data)
                        .expect("Failed to load image from proto"),
                }
            },
        }
    }
}

impl ActivitySnapshot for YoutubeSnapshot {
    fn construct_message(&self) -> LLMMessage {
        LLMMessage {
            role: Role::User,
            content: MessageContent::Image(ImageContent {
                text: None,
                image: self.video_frame.clone(),
            }),
        }
    }

    fn get_updated_at(&self) -> u64 {
        todo!()
    }

    fn get_created_at(&self) -> u64 {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub enum BrowserState {
    Youtube(ProtoYoutubeState),
    Article(ProtoArticleState),
    Pdf(ProtoPdfState),
}

impl BrowserState {
    pub fn content_type(&self) -> String {
        match self {
            BrowserState::Youtube(_) => "youtube".to_string(),
            BrowserState::Article(_) => "article".to_string(),
            BrowserState::Pdf(_) => "pdf".to_string(),
        }
    }
    pub fn youtube(self) -> Option<ProtoYoutubeState> {
        match self {
            BrowserState::Youtube(youtube) => Some(youtube),
            _ => None,
        }
    }

    pub fn article(self) -> Option<ProtoArticleState> {
        match self {
            BrowserState::Article(article) => Some(article),
            _ => None,
        }
    }

    pub fn pdf(self) -> Option<ProtoPdfState> {
        match self {
            BrowserState::Pdf(pdf) => Some(pdf),
            _ => None,
        }
    }
}
pub struct BrowserStrategy {
    client: Mutex<TauriIpcClient<Channel>>,

    name: String,
    icon: String,
    process_name: String,
}

impl BrowserStrategy {
    /// Returns a static list of process names supported by this strategy
    pub fn get_supported_processes() -> Vec<&'static str> {
        vec![
            "firefox",
            "firefox-bin",
            "firefox-esr",
            "chrome",
            "chromium",
            "chromium-browser",
            "brave",
            "brave-browser",
            "opera",
            "vivaldi",
            "edge",
            "msedge",
            "safari",
        ]
    }

    /// Create a new BrowserStrategy with the given name
    pub async fn new(name: String, icon: String, process_name: String) -> Result<Self> {
        let client = create_grpc_ipc_client().await?;

        Ok(Self {
            name,
            icon,
            process_name,
            client: Mutex::new(client),
        })
    }

    /// Get the raw client if needed for other operations
    pub async fn get_client(&self) -> TauriIpcClient<Channel> {
        self.client.lock().await.clone()
    }
}

#[async_trait]
impl ActivityStrategy for BrowserStrategy {
    async fn retrieve_assets(&mut self) -> Result<Vec<Box<dyn crate::ActivityAsset>>> {
        // Get the client
        let mut client = self.client.lock().await.clone();

        // Make a direct gRPC call to get the state
        let request = StateRequest {};
        let response = client.get_state(request).await?;
        let state_response = response.into_inner();

        // Process the response
        match &state_response.state {
            Some(ipc::state_response::State::Youtube(youtube)) => {
                eprintln!("Collected Youtube state");
                return Ok(vec![Box::new(YoutubeAsset::from(youtube.clone()))]);
            }
            Some(ipc::state_response::State::Article(article)) => {
                eprintln!("Collected Article state");
                return Ok(vec![Box::new(ArticleAsset::from(article.clone()))]);
            }
            Some(ipc::state_response::State::Pdf(_pdf)) => {
                eprintln!("Collected Pdf state (not implemented yet)");
                // PDF handling could be implemented here if needed
            }
            None => {
                eprintln!("No state received from browser");
            }
        }

        // Return empty vector if no matching state was found
        Ok(vec![])
    }

    async fn retrieve_snapshots(&mut self) -> Result<Vec<Box<dyn crate::ActivitySnapshot>>> {
        eprintln!("Retrieving snapshots from browser");
        let mut client = self.client.lock().await.clone();

        // Make a direct gRPC call to get the state
        let request = StateRequest {};
        let response = client.get_snapshot(request).await?;
        let state_response = response.into_inner();

        match &state_response.snapshot {
            Some(ipc::snapshot_response::Snapshot::Youtube(youtube)) => {
                return Ok(vec![Box::new(YoutubeSnapshot::from(youtube.clone()))]);
            }
            None => {
                eprintln!("No snapshot received from browser");
            }
        }

        // Return empty vector if no matching state was found
        Ok(vec![])
    }

    fn gather_state(&self) -> String {
        todo!()
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn get_icon(&self) -> &String {
        &self.icon
    }

    fn get_process_name(&self) -> &String {
        &self.process_name
    }
}
