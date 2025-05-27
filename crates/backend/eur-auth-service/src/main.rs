//! The Eurora monolith server that hosts the gRPC service for questions.

use anyhow::{Result, anyhow};
use dotenv::dotenv;
use eur_proto::proto_auth_service::proto_auth_service_server::{
    ProtoAuthService, ProtoAuthServiceServer,
};
use eur_proto::proto_auth_service::{LoginRequest, LoginResponse};
use std::env;
use tonic::{Request, Response, Status, transport::Server};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[derive(Default, Debug)]
struct AuthService {}

#[tonic::async_trait]
impl ProtoAuthService for AuthService {
    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        todo!()
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
    let port = env::var("AUTH_SERVICE_PORT").unwrap_or_else(|_| "50052".to_string());

    let addr = format!("0.0.0.0:{}", port)
        .parse()
        .map_err(|e| anyhow!("Failed to parse address: {}", e))?;

    // Create service
    let service = AuthService::default();

    info!("Using auth service at {}", addr);

    // Start the gRPC server
    Server::builder()
        .add_service(ProtoAuthServiceServer::new(service))
        .serve(addr)
        .await
        .map_err(|e| anyhow!("Server error: {}", e))?;

    Ok(())
}
