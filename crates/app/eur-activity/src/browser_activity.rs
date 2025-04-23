use crate::{ActivityAsset, ActivityStrategy};
use anyhow::Result;
use async_trait::async_trait;
use eur_native_messaging::{Channel, TauriIpcClient, create_grpc_ipc_client};
use eur_proto::ipc::{
    self, ProtoArticleState, ProtoPdfState, ProtoYoutubeState, StateRequest, StateResponse,
};
// We don't need the alias anymore
use eur_proto::shared::ProtoImageFormat; // Import the format enum

use image::DynamicImage;
use tokio::sync::{Mutex, mpsc};
use tokio_stream::{StreamExt, wrappers::ReceiverStream};
use tonic::Streaming;

use eur_prompt_kit::{ImageContent, Message, MessageContent, Role, TextContent};

#[derive(Debug, Clone)]
struct TranscriptLine {
    text: String,
    start: f32,
    duration: f32,
}

struct YoutubeAsset {
    pub url: String,
    pub title: String,
    pub transcript: Vec<TranscriptLine>,
    pub current_time: f32,
    pub video_frame: DynamicImage,
}

struct ArticleAsset {
    pub url: String,
    pub title: String,
    pub content: String,
}

impl From<ProtoYoutubeState> for YoutubeAsset {
    fn from(state: ProtoYoutubeState) -> Self {
        YoutubeAsset {
            url: state.url,
            title: "transcript asset".to_string(),
            transcript: state
                .transcript
                .into_iter()
                .map(|line| TranscriptLine {
                    text: line.text,
                    start: line.start,
                    duration: line.duration,
                })
                .collect(),
            current_time: state.current_time,
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
            url: article.url,
            // title: article.title,
            title: "article asset".to_string(),
            content: article.content,
        }
    }
}

impl ActivityAsset for YoutubeAsset {
    // fn get_display(&self) -> DisplayAsset {
    //     DisplayAsset {
    //         name: self.title.clone(),
    //         icon: "".to_string(),
    //     }
    // }

    fn get_name(&self) -> &String {
        &self.title
    }

    fn get_icon(&self) -> Option<&String> {
        None
    }

    fn construct_message(&self) -> Message {
        Message {
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
}

impl ActivityAsset for ArticleAsset {
    fn get_name(&self) -> &String {
        &self.title
    }

    fn get_icon(&self) -> Option<&String> {
        None
    }

    fn construct_message(&self) -> Message {
        Message {
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
    stream: Mutex<Streaming<StateResponse>>,
    request_tx: mpsc::Sender<StateRequest>,

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
        let mut client = create_grpc_ipc_client().await?;

        // Create a channel for requests
        let (tx, rx) = mpsc::channel::<StateRequest>(32);
        // Convert receiver to a stream that can be used with gRPC
        let request_stream = ReceiverStream::new(rx);

        // Create a persistent bidirectional stream
        let result = client.get_state_streaming(request_stream).await?;
        let stream = result.into_inner();

        // Send initial request to get first state
        tx.send(StateRequest {}).await?;

        Ok(Self {
            name,
            icon,
            process_name,
            client: Mutex::new(client),
            stream: Mutex::new(stream),
            request_tx: tx,
        })
    }

    /// Recreate the stream if it has ended
    async fn recreate_stream(&mut self) -> Result<()> {
        eprintln!("Recreating stream");

        // Create a new client
        let mut new_client = create_grpc_ipc_client().await?;

        // Create a new channel for requests
        let (tx, rx) = mpsc::channel::<StateRequest>(32);
        let request_stream = ReceiverStream::new(rx);

        // Create a new persistent bidirectional stream
        let result = new_client.get_state_streaming(request_stream).await?;
        let new_stream = result.into_inner();

        // Update the client
        {
            let mut client_lock = self.client.lock().await;
            *client_lock = new_client;
        }

        // Update the stream
        {
            let mut stream_lock = self.stream.lock().await;
            *stream_lock = new_stream;
        }

        // Send an initial request through the new channel
        tx.send(StateRequest {}).await.map_err(|e| {
            anyhow::anyhow!("Failed to send initial request after recreation: {}", e)
        })?;

        // Update the request_tx
        // NOTE: In a proper implementation, request_tx should be behind a Mutex
        // For now, we're replacing it directly which isn't thread-safe
        // Consider updating the design to make this field a Mutex<mpsc::Sender<StateRequest>>
        self.request_tx = tx;

        Ok(())
    }

    /// Get the raw client if needed for other operations
    pub async fn get_client(&self) -> TauriIpcClient<Channel> {
        self.client.lock().await.clone()
    }
}

#[async_trait]
impl ActivityStrategy for BrowserStrategy {
    async fn retrieve_assets(&mut self) -> Result<Vec<Box<dyn crate::ActivityAsset>>> {
        // Send a request to get the latest state
        self.request_tx
            .send(StateRequest {})
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send state request: {}", e))?;

        // Get the response
        let mut stream_lock = self.stream.lock().await;

        // Try to get a message from the stream
        match stream_lock.message().await {
            Ok(Some(state_response)) => match &state_response.state {
                Some(ipc::state_response::State::Youtube(youtube)) => {
                    eprintln!("Collected Youtube state");
                    return Ok(vec![Box::new(YoutubeAsset::from(youtube.clone()))]);
                    // return Ok(Some(BrowserState::Youtube(youtube.clone())));
                }
                Some(ipc::state_response::State::Article(article)) => {
                    eprintln!("Collected Article state");
                    return Ok(vec![Box::new(ArticleAsset::from(article.clone()))]);
                }
                // Some(ipc::state_response::State::Article(article)) => {
                //     eprintln!("Collected Article state");
                //     return Ok(Some(BrowserState::Article(article.clone())));
                // }
                // Some(ipc::state_response::State::Pdf(pdf)) => {
                //     eprintln!("Collected Pdf state");
                //     return Ok(Some(BrowserState::Pdf(pdf.clone())));
                // }
                _ => {}
            },
            Ok(None) => {
                // Stream ended unexpectedly
                eprintln!("Stream ended unexpectedly, recreating...");
                drop(stream_lock); // Release the lock before creating a new stream
                self.recreate_stream().await?;
            }
            Err(e) => {
                // Error reading from stream
                eprintln!("Error reading from stream: {}, recreating...", e);
                drop(stream_lock);
                self.recreate_stream().await?;
                return Err(anyhow::anyhow!("Stream error: {}", e));
            }
        }

        // Ok(None)
        // Implementation for retrieving assets
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
