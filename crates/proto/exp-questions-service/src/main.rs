//! Binary for running the questions service gRPC server.

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
use tonic::{Request, Response, Status, transport::Server};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

// Import the library module
use eur_questions_service as service_lib;

#[derive(Debug, Default)]
struct QuestionsServiceImpl {}

// Implement the QuestionsService trait required by Tonic/gRPC
#[tonic::async_trait]
impl QuestionsService for QuestionsServiceImpl {
    async fn video_question(
        &self,
        request: Request<VideoQuestionRequest>,
    ) -> Result<Response<VideoQuestionResponse>, Status> {
        eprintln!("Received video question request");

        // Get the inner request from the gRPC wrapper
        let request = request.into_inner();

        // Call our service implementation
        match service_lib::video_question(request).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => {
                let error_msg = format!("Failed to process video question: {}", e);
                tracing::error!("{}", error_msg);
                Err(Status::internal(error_msg))
            }
        }
    }

    async fn article_question(
        &self,
        request: Request<ArticleQuestionRequest>,
    ) -> Result<Response<ArticleQuestionResponse>, Status> {
        eprintln!("Received article question request");

        // Get the inner request from the gRPC wrapper
        let request = request.into_inner();

        // Call our service implementation
        match service_lib::article_question(request).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => {
                let error_msg = format!("Failed to process article question: {}", e);
                tracing::error!("{}", error_msg);
                Err(Status::internal(error_msg))
            }
        }
    }

    async fn pdf_question(
        &self,
        request: Request<PdfQuestionRequest>,
    ) -> Result<Response<PdfQuestionResponse>, Status> {
        eprintln!("Received PDF question request");

        // Get the inner request from the gRPC wrapper
        let request = request.into_inner();

        // Call our service implementation
        match service_lib::pdf_question(request).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => {
                let error_msg = format!("Failed to process PDF question: {}", e);
                tracing::error!("{}", error_msg);
                Err(Status::internal(error_msg))
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    // Initialize the tracing subscriber
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    // Get the address to run the server on
    let port = env::var("QUESTIONS_SERVICE_PORT").unwrap_or_else(|_| "50051".to_string());
    let addr = format!("0.0.0.0:{}", port)
        .parse()
        .map_err(|e| anyhow!("Failed to parse address: {}", e))?;

    eprintln!("Questions service starting on {}", addr);

    // Create the service
    let service = QuestionsServiceImpl::default();

    // Run the server
    Server::builder()
        .add_service(QuestionsServiceServer::new(service))
        .serve(addr)
        .await
        .map_err(|e| anyhow!("Failed to run server: {}", e))?;

    Ok(())
}
