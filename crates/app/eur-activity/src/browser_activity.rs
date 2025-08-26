use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use eur_native_messaging::{Channel, TauriIpcClient, create_grpc_ipc_client};
use eur_proto::{
    ipc::{
        self, ProtoArticleSnapshot, ProtoArticleState, ProtoPdfState, ProtoTweet,
        ProtoTwitterSnapshot, ProtoTwitterState, ProtoYoutubeSnapshot, ProtoYoutubeState,
        StateRequest,
    },
    shared::ProtoImageFormat,
};
use ferrous_llm_core::{ContentPart, ImageSource, Message, MessageContent, Role};
use image::DynamicImage;
use tokio::sync::Mutex;
use tracing::info;

use crate::{ActivityAsset, ActivityError, ActivitySnapshot, ActivityStrategy, ContextChip};

/// Helper function to safely load images from protocol buffer data
fn load_image_from_proto(
    proto_image: eur_proto::shared::ProtoImage,
) -> Result<DynamicImage, ActivityError> {
    let format = ProtoImageFormat::try_from(proto_image.format)
        .map_err(|_| ActivityError::ProtocolBuffer("Invalid image format".to_string()))?;

    let image = match format {
        ProtoImageFormat::Png => {
            image::load_from_memory_with_format(&proto_image.data, image::ImageFormat::Png)?
        }
        ProtoImageFormat::Jpeg => {
            image::load_from_memory_with_format(&proto_image.data, image::ImageFormat::Jpeg)?
        }
        ProtoImageFormat::Webp => {
            image::load_from_memory_with_format(&proto_image.data, image::ImageFormat::WebP)?
        }
        _ => image::load_from_memory(&proto_image.data)?,
    };

    Ok(image)
}

#[derive(Debug, Clone)]
struct TranscriptLine {
    text: String,
    start: f32,
    _duration: f32,
}

#[derive(Debug)]
struct YoutubeAsset {
    pub id: String,
    pub _url: String,
    pub title: String,
    pub transcript: Vec<TranscriptLine>,
    pub _current_time: f32,
}

struct ArticleAsset {
    pub id: String,
    pub _url: String,
    pub title: String,
    pub content: String,
}

struct TwitterAsset {
    pub id: String,
    pub _url: String,
    pub title: String,
    pub tweets: Vec<TwitterTweet>,
    pub _timestamp: String,
}

#[derive(Debug, Clone)]
pub struct TwitterTweet {
    pub text: String,
    pub _timestamp: Option<String>,
    pub author: Option<String>,
}

impl YoutubeAsset {
    pub fn try_from(state: ProtoYoutubeState) -> Result<Self, ActivityError> {
        Ok(YoutubeAsset {
            id: uuid::Uuid::new_v4().to_string(),
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
        })
    }
}

impl From<ProtoYoutubeState> for YoutubeAsset {
    fn from(state: ProtoYoutubeState) -> Self {
        // For backward compatibility, use the safe version but panic on error
        // This should be replaced with proper error handling in calling code
        Self::try_from(state).expect("Failed to convert ProtoYoutubeState to YoutubeAsset")
    }
}

impl ArticleAsset {
    pub fn try_from(state: ProtoArticleState) -> Result<Self, ActivityError> {
        Ok(ArticleAsset {
            id: uuid::Uuid::new_v4().to_string(),
            _url: "".to_string(),
            title: "article asset".to_string(),
            content: state.text_content,
        })
    }
}

impl From<ProtoArticleState> for ArticleAsset {
    fn from(article: ProtoArticleState) -> Self {
        Self::try_from(article).expect("Failed to convert ProtoArticleState to ArticleAsset")
    }
}

impl TwitterAsset {
    pub fn try_from(state: ProtoTwitterState) -> Result<Self, ActivityError> {
        let tweets: Vec<TwitterTweet> = state
            .tweets
            .into_iter()
            .map(|tweet| TwitterTweet {
                text: tweet.text,
                _timestamp: tweet.timestamp,
                author: tweet.author,
            })
            .collect();

        Ok(TwitterAsset {
            id: uuid::Uuid::new_v4().to_string(),
            _url: state.url,
            title: state.title,
            tweets,
            _timestamp: state.timestamp,
        })
    }
}

impl From<ProtoTwitterState> for TwitterAsset {
    fn from(state: ProtoTwitterState) -> Self {
        Self::try_from(state).expect("Failed to convert ProtoTwitterState to TwitterAsset")
    }
}

impl ActivityAsset for YoutubeAsset {
    fn get_name(&self) -> &String {
        &self.title
    }

    fn get_icon(&self) -> Option<&String> {
        None
    }

    fn construct_message(&self) -> Message {
        Message {
            role: Role::User,
            content: MessageContent::Text(format!(
                "I am watching a video with id {} and have a question about it. \
                Here's the transcript of the video: \n {}",
                self.id,
                self.transcript
                    .iter()
                    .map(|line| format!("{} ({}s)", line.text, line.start))
                    .collect::<Vec<_>>()
                    .join("\n")
            )),
        }
    }

    fn get_context_chip(&self) -> Option<ContextChip> {
        Some(ContextChip {
            id: self.id.clone(),
            name: "video".to_string(),
            // extension_id: "9370B14D-B61C-4CE2-BDE7-B18684E8731A".to_string(),
            extension_id: "7c7b59bb-d44d-431a-9f4d-64240172e092".to_string(),
            attrs: HashMap::new(),
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

    fn construct_message(&self) -> Message {
        Message {
            role: Role::User,
            content: MessageContent::Text(format!(
                "I am reading an article and have a question about it. \
                Here's the text content of the article: \n {}",
                self.content
            )),
        }
    }

    fn get_context_chip(&self) -> Option<ContextChip> {
        Some(ContextChip {
            id: self.id.clone(),
            name: "article".to_string(),
            extension_id: "309f0906-d48c-4439-9751-7bcf915cdfc5".to_string(),
            attrs: HashMap::new(),
            icon: None,
            position: Some(0),
        })
    }
}

impl ActivityAsset for TwitterAsset {
    fn get_name(&self) -> &String {
        &self.title
    }

    fn get_icon(&self) -> Option<&String> {
        None
    }

    fn construct_message(&self) -> Message {
        let tweet_texts: Vec<String> = self
            .tweets
            .iter()
            .map(|tweet| {
                let mut text = tweet.text.clone();
                if let Some(author) = &tweet.author {
                    text = format!("@{}: {}", author, text);
                }
                text
            })
            .collect();

        Message {
            role: Role::User,
            content: MessageContent::Text(format!(
                "I am looking at Twitter content and have a question about it. \
                Here are the tweets I'm seeing: \n\n{}",
                tweet_texts.join("\n\n")
            )),
        }
    }

    fn get_context_chip(&self) -> Option<ContextChip> {
        Some(ContextChip {
            id: self.id.clone(),
            name: "twitter".to_string(),
            extension_id: "2c434895-d32c-485f-8525-c4394863b83a".to_string(),
            attrs: HashMap::new(),
            icon: None,
            position: Some(0),
        })
    }
}

pub struct TwitterSnapshot {
    pub tweets: Vec<TwitterTweet>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl From<ProtoTwitterSnapshot> for TwitterSnapshot {
    fn from(snapshot: ProtoTwitterSnapshot) -> Self {
        TwitterSnapshot {
            tweets: snapshot
                .tweets
                .into_iter()
                .map(|tweet| TwitterTweet::from(tweet.clone()))
                .collect(),
            created_at: chrono::Utc::now().timestamp() as u64,
            updated_at: chrono::Utc::now().timestamp() as u64,
        }
    }
}

impl From<ProtoTweet> for TwitterTweet {
    fn from(tweet: ProtoTweet) -> Self {
        TwitterTweet {
            text: tweet.text,
            _timestamp: tweet.timestamp,
            author: tweet.author,
        }
    }
}

impl ActivitySnapshot for TwitterSnapshot {
    fn construct_message(&self) -> Message {
        let tweet_texts: Vec<String> = self
            .tweets
            .iter()
            .map(|tweet| {
                let mut text = tweet.text.clone();
                if let Some(author) = &tweet.author {
                    text = format!("@{}: {}", author, text);
                }
                text
            })
            .collect();

        Message {
            role: Role::User,
            content: MessageContent::Text(format!(
                "I am looking at Twitter content and have a question about it. \
                Here are the tweets I'm seeing: \n\n{}",
                tweet_texts.join("\n\n")
            )),
        }
    }

    fn get_updated_at(&self) -> u64 {
        self.updated_at
    }

    fn get_created_at(&self) -> u64 {
        self.created_at
    }
}

impl TwitterSnapshot {
    pub fn new(tweets: Vec<TwitterTweet>) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            tweets,
            created_at: now,
            updated_at: now,
        }
    }
}

pub struct ArticleSnapshot {
    pub highlight: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl From<ProtoArticleSnapshot> for ArticleSnapshot {
    fn from(snapshot: ProtoArticleSnapshot) -> Self {
        ArticleSnapshot {
            highlight: Some(snapshot.highlighted_content),
            created_at: chrono::Utc::now().timestamp() as u64,
            updated_at: chrono::Utc::now().timestamp() as u64,
        }
    }
}

impl ArticleSnapshot {
    pub fn new(highlight: Option<String>) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            highlight,
            created_at: now,
            updated_at: now,
        }
    }
}

impl ActivitySnapshot for ArticleSnapshot {
    fn construct_message(&self) -> Message {
        Message {
            role: Role::User,
            content: MessageContent::Text(format!(
                "I highlighted the following text: \n {}",
                self.highlight.clone().unwrap_or_default()
            )),
        }
    }

    fn get_updated_at(&self) -> u64 {
        self.updated_at
    }

    fn get_created_at(&self) -> u64 {
        self.created_at
    }
}

struct YoutubeSnapshot {
    pub video_frame: DynamicImage,
    pub created_at: u64,
    pub updated_at: u64,
}

impl YoutubeSnapshot {
    pub fn try_from(snapshot: ProtoYoutubeSnapshot) -> Result<Self, ActivityError> {
        let proto_image = snapshot
            .video_frame
            .ok_or_else(|| ActivityError::ProtocolBuffer("Missing video frame data".to_string()))?;

        let video_frame = load_image_from_proto(proto_image)?;
        let now = chrono::Utc::now().timestamp() as u64;

        Ok(YoutubeSnapshot {
            video_frame,
            created_at: now,
            updated_at: now,
        })
    }
}

impl From<ProtoYoutubeSnapshot> for YoutubeSnapshot {
    fn from(snapshot: ProtoYoutubeSnapshot) -> Self {
        // For backward compatibility, use the safe version but panic on error
        // This should be replaced with proper error handling in calling code
        Self::try_from(snapshot).expect("Failed to convert ProtoYoutubeSnapshot to YoutubeSnapshot")
    }
}

impl ActivitySnapshot for YoutubeSnapshot {
    fn construct_message(&self) -> Message {
        Message {
            role: Role::User,
            // content: MessageContent::Multimodal(vec![ContentPart {
            //     text: Text{ Some("This is last frame of the video".to_string())},
            //     image: self.video_frame.clone(),
            // }]),
            content: MessageContent::Multimodal(vec![
                ContentPart::Text {
                    text: "This is last frame of the video".to_string(),
                },
                ContentPart::Image {
                    image_source: ImageSource::DynamicImage(self.video_frame.clone()),
                    detail: None,
                },
            ]),
        }
    }

    fn get_updated_at(&self) -> u64 {
        self.updated_at
    }

    fn get_created_at(&self) -> u64 {
        self.created_at
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
        // Return different arrays based on platform
        #[cfg(target_os = "windows")]
        let processes = vec![
            "firefox.exe",
            "firefox-bin.exe",
            "firefox-esr.exe",
            "chrome.exe",
            "chromium.exe",
            "chromium-browser.exe",
            "brave.exe",
            "brave-browser.exe",
            "opera.exe",
            "vivaldi.exe",
            "edge.exe",
            "msedge.exe",
            "librewolf.exe",
        ];
        #[cfg(target_os = "linux")]
        let processes = vec![
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
            "librewolf",
        ];
        #[cfg(target_os = "macos")]
        let processes = vec!["Google Chrome"];
        processes
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
                info!("Collected Youtube state");
                return Ok(vec![Box::new(YoutubeAsset::from(youtube.clone()))]);
            }
            Some(ipc::state_response::State::Article(article)) => {
                info!("Collected Article state");
                return Ok(vec![Box::new(ArticleAsset::from(article.clone()))]);
            }
            Some(ipc::state_response::State::Pdf(_pdf)) => {
                info!("Collected Pdf state (not implemented yet)");
                // PDF handling could be implemented here if needed
            }
            Some(ipc::state_response::State::Twitter(twitter)) => {
                info!("Collected Twitter state");
                return Ok(vec![Box::new(TwitterAsset::from(twitter.clone()))]);
            }
            None => {
                info!("No state received from browser");
            }
        }

        // Return empty vector if no matching state was found
        Ok(vec![])
    }

    async fn retrieve_snapshots(&mut self) -> Result<Vec<Box<dyn crate::ActivitySnapshot>>> {
        info!("Retrieving snapshots from browser");
        let mut client = self.client.lock().await.clone();

        // Make a direct gRPC call to get the state
        let request = StateRequest {};
        let response = client.get_snapshot(request).await?;
        let state_response = response.into_inner();

        match &state_response.snapshot {
            Some(ipc::snapshot_response::Snapshot::Youtube(youtube)) => {
                return Ok(vec![Box::new(YoutubeSnapshot::from(youtube.clone()))]);
            }
            Some(ipc::snapshot_response::Snapshot::Article(article)) => {
                return Ok(vec![Box::new(ArticleSnapshot::from(article.clone()))]);
            }
            Some(ipc::snapshot_response::Snapshot::Twitter(twitter)) => {
                return Ok(vec![Box::new(TwitterSnapshot::from(twitter.clone()))]);
            }
            None => {
                info!("No snapshot received from browser");
            }
        }

        // Return empty vector if no matching state was found
        Ok(vec![])
    }

    fn gather_state(&self) -> String {
        let state = serde_json::json!({
            "process_name": self.process_name,
            "name": self.name,
            "timestamp": chrono::Utc::now().timestamp(),
            "status": "active",
            "strategy_type": "browser",
            "supported_content": ["youtube", "article", "pdf"]
        });

        state.to_string()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ActivitySnapshot;

    #[test]
    fn test_browser_strategy_supported_processes() {
        let supported = BrowserStrategy::get_supported_processes();
        assert!(supported.contains(&"firefox"));
        assert!(supported.contains(&"chrome"));
        assert!(supported.contains(&"safari"));
        assert!(!supported.is_empty());
    }

    #[test]
    fn test_article_snapshot_creation() {
        let snapshot = ArticleSnapshot::new(Some("Test highlight".to_string()));

        assert_eq!(snapshot.highlight, Some("Test highlight".to_string()));
        assert!(snapshot.created_at > 0);
        assert!(snapshot.updated_at > 0);
        assert_eq!(snapshot.created_at, snapshot.updated_at);
    }

    #[test]
    fn test_article_snapshot_timestamps() {
        let snapshot = ArticleSnapshot::new(None);

        assert_eq!(snapshot.get_created_at(), snapshot.created_at);
        assert_eq!(snapshot.get_updated_at(), snapshot.updated_at);
    }

    #[test]
    fn test_article_snapshot_message_construction() {
        let snapshot = ArticleSnapshot::new(Some("Important text".to_string()));
        let message = snapshot.construct_message();

        match message.content {
            MessageContent::Text(text_content) => {
                assert!(text_content.contains("Important text"));
                assert!(text_content.contains("highlighted"));
            }
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_article_asset_creation() {
        let article_state = ProtoArticleState {
            content: "Test article content".to_string(),
            text_content: "Test text content".to_string(),
            selected_text: None,
            title: "Test Title".to_string(),
            site_name: "Test Site".to_string(),
            language: "en".to_string(),
            excerpt: "Test excerpt".to_string(),
            length: 100,
        };

        let asset = ArticleAsset::from(article_state);
        assert_eq!(asset.title, "article asset");
        assert_eq!(asset.content, "Test article content");
        assert!(!asset.id.is_empty());
    }

    #[test]
    fn test_article_asset_context_chip() {
        let article_state = ProtoArticleState {
            content: "Test content".to_string(),
            text_content: "Test text content".to_string(),
            selected_text: None,
            title: "Test Title".to_string(),
            site_name: "Test Site".to_string(),
            language: "en".to_string(),
            excerpt: "Test excerpt".to_string(),
            length: 50,
        };

        let asset = ArticleAsset::from(article_state);
        let chip = asset.get_context_chip().unwrap();

        assert_eq!(chip.name, "article");
        assert_eq!(chip.extension_id, "None");
        assert!(!chip.id.is_empty());
    }

    #[test]
    fn test_browser_state_content_type() {
        let youtube_state = BrowserState::Youtube(ProtoYoutubeState {
            url: "test".to_string(),
            title: "Test Video".to_string(),
            transcript: vec![],
            current_time: 0.0,
        });

        assert_eq!(youtube_state.content_type(), "youtube");

        let article_state = BrowserState::Article(ProtoArticleState {
            content: "test".to_string(),
            text_content: "Test text content".to_string(),
            selected_text: None,
            title: "Test Title".to_string(),
            site_name: "Test Site".to_string(),
            language: "en".to_string(),
            excerpt: "Test excerpt".to_string(),
            length: 10,
        });

        assert_eq!(article_state.content_type(), "article");
    }

    #[test]
    fn test_load_image_from_proto_invalid_format() {
        let proto_image = eur_proto::shared::ProtoImage {
            data: vec![1, 2, 3], // Invalid image data
            format: 999,         // Invalid format
            width: 100,
            height: 100,
        };

        let result = load_image_from_proto(proto_image);
        assert!(result.is_err());

        match result.unwrap_err() {
            ActivityError::ProtocolBuffer(_) => {} // Expected
            _ => panic!("Expected ProtocolBuffer error"),
        }
    }
}
