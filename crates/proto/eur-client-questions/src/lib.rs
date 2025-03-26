//! Client for interacting with the Questions Service API
mod client;

pub use client::QuestionsClient;
use tonic::transport::Channel;

/// Create a questions client builder
pub fn questions_client_builder() -> eur_client_grpc::ClientBuilder {
    eur_client_grpc::client_builder()
}

/// Create a questions client with the default configuration
pub async fn create_questions_client() -> anyhow::Result<QuestionsClient> {
    let channel = Channel::from_static("http://[::1]:50051").connect().await?;

    QuestionsClient::new(channel)
}
