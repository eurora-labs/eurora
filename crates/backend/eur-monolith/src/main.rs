//! The Eurora monolith server that hosts the gRPC service for questions.

use anyhow::{Result, anyhow};
use dotenv::dotenv;
use eur_proto::questions_service::questions_service_server::{
    QuestionsService, QuestionsServiceServer,
};
use eur_proto::questions_service::{
    ArticleQuestionRequest, ArticleQuestionResponse, PdfQuestionRequest, PdfQuestionResponse,
    VideoQuestionRequest, VideoQuestionResponse,
};
use std::env;
use std::sync::Arc;
use tonic::{Request, Response, Status, transport::Server};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

// Our monolith implementation that forwards requests to the questions service
#[derive(Debug)]
struct VideoQuestionServiceImpl {
    questions_service_url: String,
}

impl VideoQuestionServiceImpl {
    fn new(questions_service_url: String) -> Self {
        Self {
            questions_service_url,
        }
    }
}

#[tonic::async_trait]
impl QuestionsService for VideoQuestionServiceImpl {
    async fn video_question(
        &self,
        request: Request<VideoQuestionRequest>,
    ) -> Result<Response<VideoQuestionResponse>, Status> {
        info!("Received video question request in monolith");

        let request_inner = request.into_inner();

        // Create a client to the questions service
        let mut client = eur_questions_service_client(&self.questions_service_url).await?;

        // Forward the request to the service
        match client.video_question(request_inner).await {
            Ok(response) => Ok(response),
            Err(e) => {
                let error_msg = format!("Error forwarding video question: {}", e);
                Err(Status::internal(error_msg))
            }
        }
    }

    async fn article_question(
        &self,
        request: Request<ArticleQuestionRequest>,
    ) -> Result<Response<ArticleQuestionResponse>, Status> {
        info!("Received article question request in monolith");

        let request_inner = request.into_inner();

        // Create a client to the questions service
        let mut client = eur_questions_service_client(&self.questions_service_url).await?;

        // Forward the request to the service
        match client.article_question(request_inner).await {
            Ok(response) => Ok(response),
            Err(e) => {
                let error_msg = format!("Error forwarding article question: {}", e);
                Err(Status::internal(error_msg))
            }
        }
    }

    async fn pdf_question(
        &self,
        request: Request<PdfQuestionRequest>,
    ) -> Result<Response<PdfQuestionResponse>, Status> {
        info!("Received PDF question request in monolith");

        let request_inner = request.into_inner();

        // Create a client to the questions service
        let mut client = eur_questions_service_client(&self.questions_service_url).await?;

        // Forward the request to the service
        match client.pdf_question(request_inner).await {
            Ok(response) => Ok(response),
            Err(e) => {
                let error_msg = format!("Error forwarding PDF question: {}", e);
                Err(Status::internal(error_msg))
            }
        }
    }
}

// Helper function to create a gRPC client to the questions service
async fn eur_questions_service_client(
    url: &str,
) -> Result<
    eur_proto::questions_service::questions_service_client::QuestionsServiceClient<
        tonic::transport::Channel,
    >,
    Status,
> {
    match eur_proto::questions_service::questions_service_client::QuestionsServiceClient::connect(
        url.to_string(),
    )
    .await
    {
        Ok(client) => Ok(client),
        Err(e) => {
            let error_msg = format!("Failed to connect to questions service: {}", e);
            Err(Status::unavailable(error_msg))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv().ok();

    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    // Get configuration from environment variables
    let port = env::var("MONOLITH_PORT").unwrap_or_else(|_| "50052".to_string());
    let questions_service_url =
        env::var("QUESTIONS_SERVICE_URL").unwrap_or_else(|_| "http://[::1]:50051".to_string());

    let addr = format!("0.0.0.0:{}", port)
        .parse()
        .map_err(|e| anyhow!("Failed to parse address: {}", e))?;

    // Create service
    let service = VideoQuestionServiceImpl::new(questions_service_url.clone());

    info!("Monolith server starting on {}", addr);
    info!("Using questions service at {}", questions_service_url);

    // Start the gRPC server
    Server::builder()
        .add_service(QuestionsServiceServer::new(service))
        .serve(addr)
        .await
        .map_err(|e| anyhow!("Server error: {}", e))?;

    Ok(())
}
