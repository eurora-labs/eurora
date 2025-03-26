use anyhow::{Context, Result};
use eur_client_grpc::ClientBuilder;
use eur_proto::ipc::{ProtoArticleState, ProtoPdfState, ProtoYoutubeState};
use eur_proto::questions_service::{
    ArticleQuestionRequest, PdfQuestionRequest, ProtoChatMessage, VideoQuestionRequest,
    questions_service_client::QuestionsServiceClient,
};
use tonic::transport::Channel;

/// Client for interacting with the Questions Service
pub struct QuestionsClient {
    client: QuestionsServiceClient<Channel>,
}

impl QuestionsClient {
    /// Create a new client with a channel
    pub fn new(channel: Channel) -> Result<Self> {
        Ok(Self {
            client: QuestionsServiceClient::new(channel),
        })
    }

    /// Ask a question about a video transcript
    pub async fn ask_video_question(
        &mut self,
        state: ProtoYoutubeState,
        question: &str,
    ) -> Result<String> {
        // Create a user message with the question
        let user_message = ProtoChatMessage {
            role: "user".to_string(),
            content: question.to_string(),
        };

        let request = tonic::Request::new(VideoQuestionRequest {
            messages: vec![user_message],
            state: Some(state),
        });

        let response = self
            .client
            .video_question(request) // Changed from ask_question to video_question to match proto
            .await
            .context("Failed to get answer from questions service")?;

        Ok(response.into_inner().response) // Changed from answer to response
    }

    /// Ask a question about an article
    pub async fn ask_article_question(
        &mut self,
        state: ProtoArticleState,
        question: &str,
    ) -> Result<String> {
        let user_message = ProtoChatMessage {
            role: "user".to_string(),
            content: question.to_string(),
        };

        let request = tonic::Request::new(ArticleQuestionRequest {
            messages: vec![user_message],
            state: Some(state),
        });

        let response = self
            .client
            .article_question(request)
            .await
            .context("Failed to get answer from questions service")?;

        Ok(response.into_inner().response)
    }

    /// Ask a question about a PDF document
    pub async fn ask_pdf_question(
        &mut self,
        state: ProtoPdfState,
        question: &str,
    ) -> Result<String> {
        let user_message = ProtoChatMessage {
            role: "user".to_string(),
            content: question.to_string(),
        };

        let request = tonic::Request::new(PdfQuestionRequest {
            messages: vec![user_message],
            state: Some(state),
        });

        let response = self
            .client
            .pdf_question(request)
            .await
            .context("Failed to get answer from questions service")?;

        Ok(response.into_inner().response)
    }
}

/// Extension trait for ClientBuilder to create Questions clients
pub trait QuestionsClientBuilderExt {
    /// Create a questions service client
    async fn questions_client(&self) -> Result<QuestionsClient>;
}

impl QuestionsClientBuilderExt for ClientBuilder {
    async fn questions_client(&self) -> Result<QuestionsClient> {
        let channel = Channel::from_static("http://[::1]:50051").connect().await?;
        QuestionsClient::new(channel)
    }
}
