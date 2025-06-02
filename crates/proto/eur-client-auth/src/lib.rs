//! Client for interacting with the Auth Service API
mod client;

pub use client::AuthClient;
use tonic::transport::Channel;

/// Create an auth client builder
pub fn auth_client_builder() -> eur_client_grpc::ClientBuilder {
    eur_client_grpc::client_builder()
}

/// Create an auth client with the default configuration
pub async fn create_auth_client() -> anyhow::Result<AuthClient> {
    let channel = Channel::from_static("http://[::1]:50051").connect().await?;

    AuthClient::new(channel)
}
