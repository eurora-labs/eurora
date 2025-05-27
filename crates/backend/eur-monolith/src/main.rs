use dotenv::dotenv;
use eur_auth_service::AuthService;
use eur_ocr_service::OcrService;
use eur_proto::{
    proto_auth_service::proto_auth_service_server::ProtoAuthServiceServer,
    proto_ocr_service::proto_ocr_service_server::ProtoOcrServiceServer,
};
use tonic::{Request, Response, Status, transport::Server};
use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv().ok();

    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");
    let addr = "[::1]:50051".parse().unwrap();
    let ocr_service = OcrService::default();
    let auth_service = AuthService::default();
    Server::builder()
        .add_service(ProtoOcrServiceServer::new(ocr_service))
        .add_service(ProtoAuthServiceServer::new(auth_service))
        .serve(addr)
        .await?;

    Ok(())
}
