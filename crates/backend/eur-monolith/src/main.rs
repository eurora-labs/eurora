use dotenv::dotenv;
use eur_auth::JwtConfig;
use eur_auth_service::AuthService;
use eur_ocr_service::OcrService;
use eur_proto::{
    proto_auth_service::proto_auth_service_server::ProtoAuthServiceServer,
    proto_ocr_service::proto_ocr_service_server::ProtoOcrServiceServer,
};
use eur_remote_db::DatabaseManager;
use std::sync::Arc;
use tonic::transport::Server;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv().ok();
    // Initialize sentry
    let sentry_dsn = std::env::var("SENTRY_MONOLITH_DSN")
        .expect("SENTRY_MONOLITH_DSN environment variable must be set");
    let _guard = sentry::init((
        sentry_dsn,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            send_default_pii: true,
            ..Default::default()
        },
    ));

    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    let database_url = std::env::var("REMOTE_DATABASE_URL")
        .expect("REMOTE_DATABASE_URL environment variable must be set");
    let db_manager = Arc::new(DatabaseManager::new(&database_url).await?);

    // Create shared JWT configuration
    let jwt_config = JwtConfig::default();

    let addr = std::env::var("MONOLITH_ADDR")
        .unwrap_or_else(|_| "[::1]:50051".to_string())
        .parse()
        .expect("Invalid MONOLITH_ADDR format");
    let ocr_service = OcrService::new(Some(jwt_config.clone()));
    let auth_service = AuthService::new(db_manager, Some(jwt_config));
    tracing::info!("Starting gRPC server at {}", addr);
    Server::builder()
        .add_service(ProtoOcrServiceServer::new(ocr_service))
        .add_service(ProtoAuthServiceServer::new(auth_service))
        .serve_with_shutdown(addr, async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install CTRL+C signal handler");
            tracing::info!("Shutting down gracefully...");
        })
        .await?;

    Ok(())
}
